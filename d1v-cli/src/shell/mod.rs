use std::io::{self, IsTerminal};
use std::time::Duration;

use anyhow::anyhow;
use clap::Args;
use crossterm::terminal::{self, disable_raw_mode, enable_raw_mode};
use d1v_api::{CreateShellSessionRequest, ShellConnection};
use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
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

impl From<protocol::ProtocolError> for Error {
    fn from(error: protocol::ProtocolError) -> Self {
        anyhow!(error).into()
    }
}

#[derive(Debug, Clone, Args)]
pub struct ShellArgs {
    /// Project ID; omit to open the workspace root
    pub project_id: Option<String>,

    /// Organization workspace ID (workspace-root shells only)
    #[arg(long, value_name = "ID")]
    pub organization_id: Option<u64>,
}

struct RawTerminal;

impl RawTerminal {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawTerminal {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

pub async fn run(ctx: &Context, args: ShellArgs) -> Result<()> {
    validate_interactive(&args, ctx.output.format)?;
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

fn validate_interactive(args: &ShellArgs, format: Format) -> Result<()> {
    if args.project_id.is_some() && args.organization_id.is_some() {
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
    run_terminal_with_io(
        connection,
        initial_cols,
        initial_rows,
        tokio::io::stdin(),
        tokio::io::stdout(),
        true,
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

    let _raw_terminal = interactive_tty.then(RawTerminal::enter).transpose()?;
    let mut input = vec![0_u8; 16 * 1024];
    let mut size = (initial_cols, initial_rows);
    let mut resize_interval = tokio::time::interval(Duration::from_millis(100));
    resize_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

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
                            ServerControl::Ready { .. }
                            | ServerControl::Cwd { .. }
                            | ServerControl::Pong { .. } => {}
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
            organization_id: Some(42),
        };
        assert!(validate_interactive(&args, Format::Text).is_err());
    }

    #[test]
    fn rejects_json_before_tty_check() {
        let args = ShellArgs {
            project_id: None,
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

        let exit_code =
            run_terminal_with_io(connection, 80, 24, input_reader, output_writer, false)
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
}
