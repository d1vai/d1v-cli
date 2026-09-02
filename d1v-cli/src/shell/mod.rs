use std::io::{self, IsTerminal, Read};
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};
use std::time::Duration;

use anyhow::anyhow;
use clap::Args;
use crossterm::terminal::{self, disable_raw_mode, enable_raw_mode};
use d1v_api::{CreateShellSessionRequest, ShellConnection};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt, ReadBuf};
use tokio::sync::mpsc;
use tokio::time::MissedTickBehavior;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::http::header::SEC_WEBSOCKET_PROTOCOL;

use crate::Context;
use crate::error::{Error, Result};
use crate::output::Format;

pub mod protocol;

use protocol::{ClientControl, ServerControl};

const MAX_CAPTURE_BYTES: usize = 16 * 1024 * 1024;
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(20);

impl From<protocol::ProtocolError> for Error {
    fn from(error: protocol::ProtocolError) -> Self {
        anyhow!(error).into()
    }
}

#[derive(Debug, Clone, Args)]
pub struct ShellArgs {
    /// Project ID; omit to use D1V_PROJECT_ID from `.env`
    pub project_id: Option<String>,

    /// Explicitly open the workspace root instead of the current project
    #[arg(long, conflicts_with = "project_id")]
    pub workspace: bool,

    /// Organization workspace ID (workspace-root shells only)
    #[arg(long, value_name = "ID")]
    pub organization_id: Option<u64>,
}

#[derive(Debug, Clone, Args)]
#[command(trailing_var_arg = true)]
pub struct ExecArgs {
    /// Project directory target; omit to use D1V_PROJECT_ID from `.env`
    #[arg(long, value_name = "ID")]
    pub project_id: Option<String>,

    /// Explicitly execute at the workspace root instead of the current project
    #[arg(long, conflicts_with = "project_id")]
    pub workspace: bool,

    /// Organization workspace ID (workspace-root exec only)
    #[arg(long, value_name = "ID")]
    pub organization_id: Option<u64>,

    /// Command and arguments, conventionally separated with --
    #[arg(required = true, allow_hyphen_values = true)]
    pub command: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ExecResult {
    session_id: String,
    project_id: Option<String>,
    cwd: String,
    exit_code: i32,
    stdout: String,
    stderr: String,
}

struct RawTerminal;

impl RawTerminal {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        Ok(Self)
    }
}

struct ThreadedStdin {
    receiver: mpsc::Receiver<io::Result<Vec<u8>>>,
    pending: Vec<u8>,
    offset: usize,
}

impl ThreadedStdin {
    fn spawn() -> io::Result<Self> {
        let (sender, receiver) = mpsc::channel(8);
        std::thread::Builder::new()
            .name("d1v-terminal-stdin".into())
            .spawn(move || {
                let mut stdin = io::stdin();
                loop {
                    let mut buffer = vec![0_u8; 16 * 1024];
                    match stdin.read(&mut buffer) {
                        Ok(count) => {
                            buffer.truncate(count);
                            let eof = count == 0;
                            if sender.blocking_send(Ok(buffer)).is_err() || eof {
                                return;
                            }
                        }
                        Err(error) => {
                            let _ = sender.blocking_send(Err(error));
                            return;
                        }
                    }
                }
            })?;
        Ok(Self {
            receiver,
            pending: Vec::new(),
            offset: 0,
        })
    }
}

impl AsyncRead for ThreadedStdin {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        loop {
            if self.offset < self.pending.len() {
                let count = buffer
                    .remaining()
                    .min(self.pending.len().saturating_sub(self.offset));
                let end = self.offset + count;
                buffer.put_slice(&self.pending[self.offset..end]);
                self.offset = end;
                return Poll::Ready(Ok(()));
            }
            match self.receiver.poll_recv(cx) {
                Poll::Ready(Some(Ok(payload))) => {
                    self.pending = payload;
                    self.offset = 0;
                    if self.pending.is_empty() {
                        return Poll::Ready(Ok(()));
                    }
                }
                Poll::Ready(Some(Err(error))) => return Poll::Ready(Err(error)),
                Poll::Ready(None) => return Poll::Ready(Ok(())),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl Drop for RawTerminal {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

pub async fn run(ctx: &Context, args: ShellArgs) -> Result<()> {
    validate_interactive(&args, ctx.output.format)?;
    let mut args = args;
    if !args.workspace && args.project_id.is_none() {
        args.project_id = crate::workspace::resolve_env_project_id(None)?.or_else(|| {
            std::env::var("D1V_PROJECT_ID")
                .ok()
                .filter(|id| !id.trim().is_empty())
        });
    }
    let (cols, rows) = normalize_terminal_size(terminal::size().unwrap_or((120, 40)));
    eprintln!("Connecting to D1V terminal...");

    let connection = create_session(ctx, &args, cols, rows).await?;
    let session_id = connection.session_id.clone();
    let result = run_terminal(connection, cols, rows).await;
    let cleanup = ctx.client.shell().close(&session_id).await;

    if let Err(error) = cleanup
        && result.is_ok()
    {
        return Err(error.into());
    }
    let exit_code = result?;
    if exit_code == 0 {
        Ok(())
    } else {
        Err(Error::RemoteExit(exit_code))
    }
}

pub async fn run_exec(ctx: &Context, args: ExecArgs) -> Result<()> {
    validate_exec(&args)?;
    let mut args = args;
    if !args.workspace && args.project_id.is_none() {
        args.project_id = crate::workspace::resolve_env_project_id(None)?.or_else(|| {
            std::env::var("D1V_PROJECT_ID")
                .ok()
                .filter(|id| !id.trim().is_empty())
        });
    }
    let mut request = CreateShellSessionRequest::exec(args.command.clone());
    let api = ctx.client.shell();
    let connection = if let Some(project_id) = args.project_id.as_deref() {
        request.target = d1v_api::ShellTarget::Project;
        api.create_project(project_id, &request).await?
    } else {
        api.create_workspace(args.organization_id, &request).await?
    };
    let session_id = connection.session_id.clone();
    let project_id = connection.project_id.clone();
    let cwd = connection.cwd.clone();
    let capture = matches!(ctx.output.format, Format::Json);
    let transport_result = run_exec_connection(connection, capture).await;
    let cleanup_result = api.close(&session_id).await;
    let (exit_code, stdout, stderr) = transport_result?;

    if capture {
        let result = ExecResult {
            session_id,
            project_id,
            cwd,
            exit_code,
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
        };
        ctx.present(crate::text::Text::new(), &result)?;
    }
    if let Err(error) = cleanup_result
        && exit_code == 0
    {
        return Err(error.into());
    }
    if exit_code == 0 {
        Ok(())
    } else {
        Err(Error::RemoteExit(exit_code))
    }
}

fn validate_exec(args: &ExecArgs) -> Result<()> {
    if (args.project_id.is_some() || !args.workspace) && args.organization_id.is_some() {
        return Err(anyhow!("--organization-id cannot be used with --project-id").into());
    }
    if args.command.is_empty() {
        return Err(anyhow!("exec command is required after --").into());
    }
    let mut total_bytes = 0;
    for argument in &args.command {
        let argument_bytes = argument.len();
        if argument.is_empty() || argument.contains('\0') || argument_bytes > 4096 {
            return Err(anyhow!(
                "exec arguments must be non-empty, NUL-free, and at most 4096 bytes"
            )
            .into());
        }
        total_bytes += argument_bytes;
    }
    if args.command.len() > 128 || total_bytes > 32768 {
        return Err(anyhow!("exec command exceeds the argument limit").into());
    }
    Ok(())
}

fn validate_interactive(args: &ShellArgs, format: Format) -> Result<()> {
    if (args.project_id.is_some() || !args.workspace) && args.organization_id.is_some() {
        return Err(anyhow!("--organization-id cannot be used with PROJECT_ID").into());
    }
    if matches!(format, Format::Json) {
        return Err(anyhow!("d1v shell requires text output; omit --format json").into());
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(anyhow!("d1v shell requires an interactive terminal").into());
    }
    Ok(())
}

fn normalize_terminal_size((cols, rows): (u16, u16)) -> (u16, u16) {
    (cols.clamp(20, 500), rows.clamp(5, 200))
}

async fn create_session(
    ctx: &Context,
    args: &ShellArgs,
    cols: u16,
    rows: u16,
) -> Result<ShellConnection> {
    let api = ctx.client.shell();
    if let Some(project_id) = args.project_id.as_deref() {
        Ok(api
            .create_project(project_id, &CreateShellSessionRequest::project(cols, rows))
            .await?)
    } else {
        Ok(api
            .create_workspace(
                args.organization_id,
                &CreateShellSessionRequest::workspace(cols, rows),
            )
            .await?)
    }
}

async fn run_terminal(
    connection: ShellConnection,
    initial_cols: u16,
    initial_rows: u16,
) -> Result<i32> {
    let _raw_terminal = RawTerminal::enter()?;
    run_terminal_with_io(
        connection,
        initial_cols,
        initial_rows,
        ThreadedStdin::spawn()?,
        tokio::io::stdout(),
        true,
        HEARTBEAT_INTERVAL,
    )
    .await
}

async fn run_terminal_with_io<R, W>(
    connection: ShellConnection,
    initial_cols: u16,
    initial_rows: u16,
    mut stdin: R,
    mut stdout: W,
    interactive_tty: bool,
    heartbeat_period: Duration,
) -> Result<i32>
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut request = connection
        .websocket_url
        .as_str()
        .into_client_request()
        .map_err(|error| anyhow!("invalid terminal WebSocket URL: {error}"))?;
    request.headers_mut().insert(
        SEC_WEBSOCKET_PROTOCOL,
        HeaderValue::from_static(protocol::SUBPROTOCOL),
    );
    request.headers_mut().insert(
        "x-d1v-shell-ticket",
        HeaderValue::from_str(&connection.connection_ticket)
            .map_err(|_| anyhow!("invalid terminal connection ticket"))?,
    );

    let (mut socket, response) = connect_async(request)
        .await
        .map_err(|error| anyhow!("terminal WebSocket connection failed: {error}"))?;
    if response
        .headers()
        .get(SEC_WEBSOCKET_PROTOCOL)
        .and_then(|value| value.to_str().ok())
        != Some(protocol::SUBPROTOCOL)
    {
        return Err(anyhow!("terminal server did not negotiate d1v-terminal.v1").into());
    }

    socket
        .send(Message::Text(
            protocol::encode_control(&ClientControl::open(initial_cols, initial_rows))?.into(),
        ))
        .await
        .map_err(|error| anyhow!("failed to open terminal stream: {error}"))?;

    let mut input = vec![0_u8; 16 * 1024];
    let mut size = (initial_cols, initial_rows);
    let mut resize_interval = tokio::time::interval(Duration::from_millis(100));
    resize_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut heartbeat_interval = tokio::time::interval_at(
        tokio::time::Instant::now() + heartbeat_period,
        heartbeat_period,
    );
    heartbeat_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut heartbeat_sequence = 0_i64;
    let mut awaiting_heartbeat = None;

    loop {
        tokio::select! {
            read = stdin.read(&mut input) => {
                let count = read?;
                if count == 0 {
                    socket.send(Message::Text(
                        protocol::encode_control(&ClientControl::Detach)?.into()
                    )).await.map_err(|error| anyhow!("failed to detach terminal: {error}"))?;
                    return Ok(0);
                }
                for chunk in input[..count].chunks(protocol::MAX_BINARY_PAYLOAD_BYTES) {
                    socket.send(Message::Binary(protocol::encode_input(chunk)?.into()))
                        .await
                        .map_err(|error| anyhow!("failed to send terminal input: {error}"))?;
                }
            }
            message = socket.next() => {
                match message {
                    Some(Ok(Message::Binary(frame))) => {
                        stdout.write_all(protocol::decode_output(&frame)?).await?;
                        stdout.flush().await?;
                    }
                    Some(Ok(Message::Text(frame))) => {
                        match protocol::decode_control(frame.as_ref())? {
                            ServerControl::Ready { .. } | ServerControl::Cwd { .. } => {}
                            ServerControl::Pong { timestamp } => {
                                if awaiting_heartbeat == Some(timestamp) {
                                    awaiting_heartbeat = None;
                                }
                            }
                            ServerControl::Exit { code, .. } => {
                                stdout.flush().await?;
                                return Ok(code.unwrap_or(1));
                            }
                            ServerControl::Error { code, retryable } => {
                                let suffix = if retryable { " (retryable)" } else { "" };
                                return Err(anyhow!("terminal server error: {code}{suffix}").into());
                            }
                        }
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        socket.send(Message::Pong(payload)).await
                            .map_err(|error| anyhow!("failed to answer terminal ping: {error}"))?;
                    }
                    Some(Ok(Message::Close(frame))) => {
                        let reason = frame.map(|value| value.reason.to_string()).unwrap_or_default();
                        return Err(anyhow!("terminal connection closed: {reason}").into());
                    }
                    Some(Ok(Message::Pong(_) | Message::Frame(_))) => {}
                    Some(Err(error)) => {
                        return Err(anyhow!("terminal WebSocket error: {error}").into());
                    }
                    None => {
                        return Err(anyhow!("terminal connection ended before exit status").into());
                    }
                }
            }
            _ = resize_interval.tick(), if interactive_tty => {
                if let Ok(next_size) = terminal::size()
                    && let next_size = normalize_terminal_size(next_size)
                    && next_size != size
                {
                    size = next_size;
                    socket.send(Message::Text(
                        protocol::encode_control(&ClientControl::Resize {
                            cols: size.0,
                            rows: size.1,
                        })?.into()
                    )).await.map_err(|error| anyhow!("failed to resize terminal: {error}"))?;
                }
            }
            _ = heartbeat_interval.tick() => {
                if awaiting_heartbeat.is_some() {
                    return Err(anyhow!("terminal application heartbeat timed out").into());
                }
                heartbeat_sequence = heartbeat_sequence.saturating_add(1);
                awaiting_heartbeat = Some(heartbeat_sequence);
                socket.send(Message::Text(
                    protocol::encode_control(&ClientControl::Ping {
                        timestamp: heartbeat_sequence,
                    })?
                )).await.map_err(|error| anyhow!("failed to send terminal heartbeat: {error}"))?;
            }
        }
    }
}

async fn run_exec_connection(
    connection: ShellConnection,
    capture: bool,
) -> Result<(i32, Vec<u8>, Vec<u8>)> {
    run_exec_connection_with_heartbeat(connection, capture, HEARTBEAT_INTERVAL).await
}

async fn run_exec_connection_with_heartbeat(
    connection: ShellConnection,
    capture: bool,
    heartbeat_period: Duration,
) -> Result<(i32, Vec<u8>, Vec<u8>)> {
    let mut request = connection
        .websocket_url
        .as_str()
        .into_client_request()
        .map_err(|error| anyhow!("invalid terminal WebSocket URL: {error}"))?;
    request.headers_mut().insert(
        SEC_WEBSOCKET_PROTOCOL,
        HeaderValue::from_static(protocol::SUBPROTOCOL),
    );
    request.headers_mut().insert(
        "x-d1v-shell-ticket",
        HeaderValue::from_str(&connection.connection_ticket)
            .map_err(|_| anyhow!("invalid terminal connection ticket"))?,
    );
    let (mut socket, response) = connect_async(request)
        .await
        .map_err(|error| anyhow!("exec WebSocket connection failed: {error}"))?;
    if response
        .headers()
        .get(SEC_WEBSOCKET_PROTOCOL)
        .and_then(|value| value.to_str().ok())
        != Some(protocol::SUBPROTOCOL)
    {
        return Err(anyhow!("terminal server did not negotiate d1v-terminal.v1").into());
    }
    socket
        .send(Message::Text(
            protocol::encode_control(&ClientControl::open(120, 40))?.into(),
        ))
        .await
        .map_err(|error| anyhow!("failed to open exec stream: {error}"))?;

    let mut captured_stdout = Vec::new();
    let mut captured_stderr = Vec::new();
    let mut stdout = tokio::io::stdout();
    let mut stderr = tokio::io::stderr();
    let mut heartbeat_interval = tokio::time::interval_at(
        tokio::time::Instant::now() + heartbeat_period,
        heartbeat_period,
    );
    heartbeat_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut heartbeat_sequence = 0_i64;
    let mut awaiting_heartbeat = None;
    loop {
        tokio::select! {
        message = socket.next() => match message {
            Some(message) => match message {
            Ok(Message::Binary(frame)) => {
                let (channel, payload) = protocol::decode_server_binary(&frame)?;
                if capture {
                    let destination = match channel {
                        protocol::ServerBinaryChannel::Output => &mut captured_stdout,
                        protocol::ServerBinaryChannel::Stderr => &mut captured_stderr,
                    };
                    if destination.len().saturating_add(payload.len()) > MAX_CAPTURE_BYTES {
                        return Err(anyhow!(
                            "exec output exceeds the 16 MiB JSON capture limit; use text output"
                        )
                        .into());
                    }
                    destination.extend_from_slice(payload);
                } else {
                    match channel {
                        protocol::ServerBinaryChannel::Output => {
                            stdout.write_all(payload).await?;
                            stdout.flush().await?;
                        }
                        protocol::ServerBinaryChannel::Stderr => {
                            stderr.write_all(payload).await?;
                            stderr.flush().await?;
                        }
                    }
                }
            }
            Ok(Message::Text(frame)) => match protocol::decode_control(frame.as_ref())? {
                ServerControl::Ready { .. } | ServerControl::Cwd { .. } => {}
                ServerControl::Pong { timestamp } => {
                    if awaiting_heartbeat == Some(timestamp) {
                        awaiting_heartbeat = None;
                    }
                }
                ServerControl::Exit { code, .. } => {
                    stdout.flush().await?;
                    stderr.flush().await?;
                    return Ok((code.unwrap_or(1), captured_stdout, captured_stderr));
                }
                ServerControl::Error { code, retryable } => {
                    let suffix = if retryable { " (retryable)" } else { "" };
                    return Err(anyhow!("exec server error: {code}{suffix}").into());
                }
            },
            Ok(Message::Ping(payload)) => {
                socket
                    .send(Message::Pong(payload))
                    .await
                    .map_err(|error| anyhow!("failed to answer exec ping: {error}"))?;
            }
            Ok(Message::Close(frame)) => {
                let reason = frame
                    .map(|value| value.reason.to_string())
                    .unwrap_or_default();
                return Err(anyhow!("exec connection closed: {reason}").into());
            }
            Ok(Message::Pong(_) | Message::Frame(_)) => {}
            Err(error) => return Err(anyhow!("exec WebSocket error: {error}").into()),
            },
            None => return Err(anyhow!("exec connection ended before exit status").into()),
        },
        _ = heartbeat_interval.tick() => {
            if awaiting_heartbeat.is_some() {
                return Err(anyhow!("exec application heartbeat timed out").into());
            }
            heartbeat_sequence = heartbeat_sequence.saturating_add(1);
            awaiting_heartbeat = Some(heartbeat_sequence);
            socket.send(Message::Text(
                protocol::encode_control(&ClientControl::Ping {
                    timestamp: heartbeat_sequence,
                })?
            )).await.map_err(|error| anyhow!("failed to send exec heartbeat: {error}"))?;
        }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use futures_util::{SinkExt, StreamExt};
    use jiff::Timestamp;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_hdr_async;
    use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};

    use super::*;

    #[test]
    fn rejects_organization_with_project() {
        let args = ShellArgs {
            project_id: Some("project_1".into()),
            workspace: false,
            organization_id: Some(42),
        };
        assert!(validate_interactive(&args, Format::Text).is_err());
    }

    #[test]
    fn rejects_json_before_tty_check() {
        let args = ShellArgs {
            project_id: None,
            workspace: false,
            organization_id: None,
        };
        assert!(validate_interactive(&args, Format::Json).is_err());
    }

    #[test]
    fn clamps_terminal_size_to_api_contract() {
        assert_eq!(normalize_terminal_size((0, 1)), (20, 5));
        assert_eq!(normalize_terminal_size((120, 40)), (120, 40));
        assert_eq!(normalize_terminal_size((u16::MAX, u16::MAX)), (500, 200));
    }

    #[test]
    fn validates_exec_scope_and_argument_limits() {
        let conflicting_scope = ExecArgs {
            project_id: Some("project_1".into()),
            workspace: false,
            organization_id: Some(42),
            command: vec!["true".into()],
        };
        assert!(validate_exec(&conflicting_scope).is_err());

        let empty_argument = ExecArgs {
            project_id: None,
            workspace: false,
            organization_id: None,
            command: vec![String::new()],
        };
        assert!(validate_exec(&empty_argument).is_err());

        let too_many_arguments = ExecArgs {
            project_id: None,
            workspace: false,
            organization_id: None,
            command: vec!["x".into(); 129],
        };
        assert!(validate_exec(&too_many_arguments).is_err());
    }

    #[tokio::test]
    async fn relays_binary_pty_over_authenticated_websocket() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let saw_auth = Arc::new(Mutex::new(false));
        let server_auth = saw_auth.clone();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket =
                accept_hdr_async(stream, move |request: &Request, mut response: Response| {
                    assert_eq!(
                        request.headers().get("x-d1v-shell-ticket").unwrap(),
                        "secret-ticket"
                    );
                    assert_eq!(
                        request.headers().get(SEC_WEBSOCKET_PROTOCOL).unwrap(),
                        protocol::SUBPROTOCOL
                    );
                    response.headers_mut().insert(
                        SEC_WEBSOCKET_PROTOCOL,
                        HeaderValue::from_static(protocol::SUBPROTOCOL),
                    );
                    *server_auth.lock().unwrap() = true;
                    Ok(response)
                })
                .await
                .unwrap();

            let open = socket.next().await.unwrap().unwrap().into_text().unwrap();
            assert_eq!(
                open,
                protocol::encode_control(&ClientControl::open(80, 24)).unwrap()
            );
            socket
                .send(Message::Text(
                    r#"{"type":"ready","session_id":"sh_test","cwd":"/workspace-root"}"#.into(),
                ))
                .await
                .unwrap();
            let input = socket.next().await.unwrap().unwrap().into_data();
            assert_eq!(input.as_slice(), b"\x00echo ready\n");
            socket
                .send(Message::Binary(b"\x01ready\r\n".as_slice().into()))
                .await
                .unwrap();
            let heartbeat = socket.next().await.unwrap().unwrap().into_text().unwrap();
            let heartbeat: serde_json::Value = serde_json::from_str(&heartbeat).unwrap();
            assert_eq!(heartbeat["type"], "ping");
            socket
                .send(Message::Text(
                    serde_json::json!({
                        "type": "pong",
                        "timestamp": heartbeat["timestamp"],
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
            socket
                .send(Message::Text(
                    r#"{"type":"exit","code":7,"signal":null}"#.into(),
                ))
                .await
                .unwrap();
        });

        let connection = ShellConnection {
            session_id: "sh_test".into(),
            workspace_scope: "user:1".into(),
            project_id: None,
            runtime_provider: "fabric".into(),
            node_id: "node_test".into(),
            cwd: "/workspace-root".into(),
            transport: d1v_api::ShellTransport::Direct,
            websocket_url: format!("ws://{address}/ws/terminal/sh_test"),
            connection_ticket: "secret-ticket".into(),
            ticket_expires_at: "2026-08-22T12:00:30Z".parse::<Timestamp>().unwrap(),
        };
        let (mut input_writer, input_reader) = tokio::io::duplex(1024);
        let (output_writer, mut output_reader) = tokio::io::duplex(1024);
        input_writer.write_all(b"echo ready\n").await.unwrap();

        let exit_code = run_terminal_with_io(
            connection,
            80,
            24,
            input_reader,
            output_writer,
            false,
            Duration::from_millis(500),
        )
        .await
        .unwrap();
        drop(input_writer);
        let mut output = Vec::new();
        output_reader.read_to_end(&mut output).await.unwrap();
        server.await.unwrap();

        assert!(*saw_auth.lock().unwrap());
        assert_eq!(output, b"ready\r\n");
        assert_eq!(exit_code, 7);
    }

    #[tokio::test]
    async fn captures_exec_channels_and_nonzero_exit_over_authenticated_websocket() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let saw_auth = Arc::new(Mutex::new(false));
        let server_auth = saw_auth.clone();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket =
                accept_hdr_async(stream, move |request: &Request, mut response: Response| {
                    assert_eq!(
                        request.headers().get("x-d1v-shell-ticket").unwrap(),
                        "exec-secret-ticket"
                    );
                    assert_eq!(
                        request.headers().get(SEC_WEBSOCKET_PROTOCOL).unwrap(),
                        protocol::SUBPROTOCOL
                    );
                    response.headers_mut().insert(
                        SEC_WEBSOCKET_PROTOCOL,
                        HeaderValue::from_static(protocol::SUBPROTOCOL),
                    );
                    *server_auth.lock().unwrap() = true;
                    Ok(response)
                })
                .await
                .unwrap();

            let open = socket.next().await.unwrap().unwrap().into_text().unwrap();
            assert_eq!(
                open,
                protocol::encode_control(&ClientControl::open(120, 40)).unwrap()
            );
            socket
                .send(Message::Text(
                    r#"{"type":"ready","session_id":"sh_exec","cwd":"/workspace-root/project_1"}"#
                        .into(),
                ))
                .await
                .unwrap();
            let heartbeat = socket.next().await.unwrap().unwrap().into_text().unwrap();
            let heartbeat: serde_json::Value = serde_json::from_str(&heartbeat).unwrap();
            assert_eq!(heartbeat["type"], "ping");
            socket
                .send(Message::Text(
                    serde_json::json!({
                        "type": "pong",
                        "timestamp": heartbeat["timestamp"],
                    })
                    .to_string(),
                ))
                .await
                .unwrap();
            socket
                .send(Message::Binary(b"\x01command output\n".as_slice().into()))
                .await
                .unwrap();
            socket
                .send(Message::Binary(b"\x02command error\n".as_slice().into()))
                .await
                .unwrap();
            socket
                .send(Message::Text(
                    r#"{"type":"exit","code":23,"signal":null}"#.into(),
                ))
                .await
                .unwrap();
        });

        let connection = ShellConnection {
            session_id: "sh_exec".into(),
            workspace_scope: "user:1".into(),
            project_id: Some("project_1".into()),
            runtime_provider: "fabric".into(),
            node_id: "node_test".into(),
            cwd: "/workspace-root/project_1".into(),
            transport: d1v_api::ShellTransport::Direct,
            websocket_url: format!("ws://{address}/ws/terminal/sh_exec"),
            connection_ticket: "exec-secret-ticket".into(),
            ticket_expires_at: "2026-08-22T12:00:30Z".parse::<Timestamp>().unwrap(),
        };
        let (exit_code, stdout, stderr) =
            run_exec_connection_with_heartbeat(connection, true, Duration::from_millis(500))
                .await
                .unwrap();
        server.await.unwrap();

        assert!(*saw_auth.lock().unwrap());
        assert_eq!(exit_code, 23);
        assert_eq!(stdout, b"command output\n");
        assert_eq!(stderr, b"command error\n");

        let result = ExecResult {
            session_id: "sh_exec".into(),
            project_id: Some("project_1".into()),
            cwd: "/workspace-root/project_1".into(),
            exit_code,
            stdout: String::from_utf8(stdout).unwrap(),
            stderr: String::from_utf8(stderr).unwrap(),
        };
        assert_eq!(
            serde_json::to_value(result).unwrap(),
            serde_json::json!({
                "session_id": "sh_exec",
                "project_id": "project_1",
                "cwd": "/workspace-root/project_1",
                "exit_code": 23,
                "stdout": "command output\n",
                "stderr": "command error\n"
            })
        );
    }
}
