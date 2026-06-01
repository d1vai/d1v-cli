use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use clap::{Args, Subcommand};
use futures_util::{SinkExt, StreamExt};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::{Mutex, mpsc};
use tokio::time::sleep;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
use url::Url;

use crate::Context;
use crate::config::Config;
use crate::error::Result;
use crate::text::{Line, Text};
use crate::theme;
use crate::token::TokenSource;
use crate::runtime_install;
use crate::workspace;

const DEFAULT_HOME_DIR: &str = ".d1v/agent/home";
const DEFAULT_OPCODE_HEALTH: &str = "http://127.0.0.1:9191/health";
const DEFAULT_OPCODE_BASE: &str = "http://127.0.0.1:9191";

type TunnelWriter =
    futures_util::stream::SplitSink<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>, Message>;

static TUNNELS: std::sync::LazyLock<Mutex<HashMap<String, Arc<Mutex<TunnelWriter>>>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Subcommand)]
pub enum AgentCommand {
    InitHome(InitHomeArgs),
    Pair(PairArgs),
    Start(StartArgs),
    Status,
    Project {
        #[command(subcommand)]
        command: AgentProjectCommand,
    },
    InitRuntime(InitRuntimeArgs),
}

#[derive(Subcommand)]
pub enum AgentProjectCommand {
    Create(ProjectCreateArgs),
    Import(ProjectImportArgs),
    Bind(ProjectBindArgs),
}

#[derive(Args)]
pub struct InitHomeArgs {
    #[arg(long, default_value = DEFAULT_HOME_DIR)]
    pub path: PathBuf,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub device_id: Option<String>,
}

#[derive(Args)]
pub struct PairArgs {
    #[arg(long)]
    pub code: String,
    #[arg(long)]
    pub name: Option<String>,
}

#[derive(Args)]
pub struct StartArgs {
    #[arg(long)]
    pub opcode_bin: Option<PathBuf>,
}

#[derive(Args)]
pub struct ProjectCreateArgs {
    #[arg(long)]
    pub project_id: String,
    #[arg(long)]
    pub name: String,
    #[arg(long)]
    pub path: Option<PathBuf>,
}

#[derive(Args)]
pub struct ProjectImportArgs {
    #[arg(long)]
    pub path: PathBuf,
    #[arg(long)]
    pub project_id: Option<String>,
}

#[derive(Args)]
pub struct ProjectBindArgs {
    #[arg(long)]
    pub project_id: String,
    #[arg(long)]
    pub path: PathBuf,
}

#[derive(Args)]
pub struct InitRuntimeArgs {
    #[arg(long)]
    pub project_id: String,
    #[arg(long)]
    pub path: PathBuf,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub device_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AgentConfig {
    device_id: String,
    device_name: String,
    home_root: Option<String>,
    opcode_base_url: String,
    project_bindings: Vec<ProjectBinding>,
}

pub(crate) struct RuntimeDoctorConfig {
    pub device_id: String,
    pub home_root: Option<String>,
}

pub(crate) fn load_runtime_doctor_config() -> Result<RuntimeDoctorConfig> {
    let config = load_agent_config()?;
    Ok(RuntimeDoctorConfig {
        device_id: config.device_id,
        home_root: config.home_root,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectBinding {
    project_id: String,
    workspace_root: String,
}

fn agent_dir() -> Result<PathBuf> {
    Ok(Config::dir()?.join("agent"))
}

fn agent_config_path() -> Result<PathBuf> {
    Ok(agent_dir()?.join("config.json"))
}

fn load_agent_config() -> Result<AgentConfig> {
    let path = agent_config_path()?;
    if !path.exists() {
        return Ok(AgentConfig {
            device_id: format!("dev-{}", uuid_like()),
            device_name: default_device_name(),
            home_root: None,
            opcode_base_url: DEFAULT_OPCODE_BASE.to_string(),
            project_bindings: Vec::new(),
        });
    }
    Ok(serde_json::from_str(&fs::read_to_string(path)?).map_err(|e| anyhow!(e))?)
}

fn save_agent_config(config: &AgentConfig) -> Result {
    let path = agent_config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(config).map_err(|e| anyhow!(e))?)?;
    Ok(())
}

fn uuid_like() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    format!("{nanos:x}")
}

fn default_device_name() -> String {
    let host = std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "local-device".to_string());
    format!("{} ({})", host, std::env::consts::OS)
}

fn canonical_string(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}

fn ensure_home_layout(path: &Path) -> Result {
    fs::create_dir_all(path)?;
    fs::create_dir_all(path.join("projects"))?;
    fs::create_dir_all(path.join("cache"))?;
    fs::create_dir_all(path.join("logs"))?;
    fs::create_dir_all(path.join("tmp"))?;
    fs::create_dir_all(path.join("metadata"))?;
    Ok(())
}

async fn authed_http_client(ctx: &Context) -> Result<reqwest::Client> {
    let token = ctx
        .tokens
        .lookup()?
        .ok_or_else(|| anyhow!("missing auth token"))?;
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        format!("Bearer {}", token.expose_secret())
            .parse()
            .map_err(|e: reqwest::header::InvalidHeaderValue| anyhow!(e))?,
    );
    headers.insert(
        reqwest::header::ACCEPT,
        "application/json"
            .parse()
            .map_err(|e: reqwest::header::InvalidHeaderValue| anyhow!(e))?,
    );
    Ok(reqwest::Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| anyhow!(e))?)
}

fn base_url(ctx: &Context) -> String {
    ctx.client.base_url().trim_end_matches('/').to_string()
}

async fn put_device_home(ctx: &Context, device_id: &str, home_root: &str) -> Result {
    let client = authed_http_client(ctx).await?;
    let resp = client
        .put(format!("{}/api/devices/{}/home", base_url(ctx), device_id))
        .json(&json!({ "workspace_root": home_root }))
        .send()
        .await
        .map_err(|e| anyhow!(e))?;
    if !resp.status().is_success() {
        return Err(anyhow!("failed to update device home: {}", resp.status()).into());
    }
    Ok(())
}

async fn put_project_runtime(
    ctx: &Context,
    project_id: &str,
    device_id: &str,
) -> Result {
    let client = authed_http_client(ctx).await?;
    let resp = client
        .put(format!(
            "{}/api/devices/projects/{}/runtime",
            base_url(ctx),
            project_id
        ))
        .json(&json!({
            "runtime_type": "local",
            "device_id": device_id,
        }))
        .send()
        .await
        .map_err(|e| anyhow!(e))?;
    if !resp.status().is_success() {
        return Err(anyhow!("failed to bind project runtime: {}", resp.status()).into());
    }
    Ok(())
}

async fn put_project_binding(
    ctx: &Context,
    project_id: &str,
    workspace_root: &str,
) -> Result {
    let client = authed_http_client(ctx).await?;
    let resp = client
        .put(format!(
            "{}/api/devices/projects/{}/binding",
            base_url(ctx),
            project_id
        ))
        .json(&json!({ "workspace_root": workspace_root }))
        .send()
        .await
        .map_err(|e| anyhow!(e))?;
    if !resp.status().is_success() {
        return Err(anyhow!("failed to bind project path: {}", resp.status()).into());
    }
    Ok(())
}

async fn complete_pairing(ctx: &Context, code: &str, config: &AgentConfig) -> Result {
    let client = authed_http_client(ctx).await?;
    let resp = client
        .post(format!("{}/api/devices/pair/complete", base_url(ctx)))
        .json(&json!({
            "pairing_code": code,
            "device_id": config.device_id,
            "name": config.device_name,
            "public_key": format!("local-agent-{}", config.device_id),
            "os_type": std::env::consts::OS,
            "capabilities": {
                "runtime": "opcode-api",
                "transport": "relay",
                "home_root": config.home_root,
            },
            "workspace_root": config.home_root,
        }))
        .send()
        .await
        .map_err(|e| anyhow!(e))?;
    if !resp.status().is_success() {
        return Err(anyhow!("failed to pair device: {}", resp.status()).into());
    }
    if let Some(home_root) = &config.home_root {
        put_device_home(ctx, &config.device_id, home_root).await?;
    }
    Ok(())
}

fn upsert_binding(config: &mut AgentConfig, project_id: &str, workspace_root: &str) {
    config.project_bindings.retain(|item| item.project_id != project_id);
    config.project_bindings.push(ProjectBinding {
        project_id: project_id.to_string(),
        workspace_root: workspace_root.to_string(),
    });
}

async fn init_workspace_binding(
    ctx: &Context,
    project_id: &str,
    path: &Path,
    name: Option<String>,
) -> Result {
    workspace::init(
        ctx,
        workspace::InitArgs {
            path: path.to_path_buf(),
            name,
            project_id: Some(project_id.to_string()),
            force: false,
            dry_run: false,
        },
    )
    .await
}

async fn maybe_spawn_opcode(
    ctx: &Context,
    config: &AgentConfig,
    opcode_bin: Option<&Path>,
    cloud_control_url: &str,
) -> Result<Option<Child>> {
    let health = reqwest::get(DEFAULT_OPCODE_HEALTH).await;
    if let Ok(resp) = health && resp.status().is_success() {
        return Ok(None);
    }

    let bin = if let Some(path) = opcode_bin {
        path.to_path_buf()
    } else if let Ok(raw) = std::env::var("D1V_OPCODE_API_BIN") {
        PathBuf::from(raw)
    } else {
        runtime_install::ensure_runtime_installed(ctx, None).await?
    };

    let workspace_root = config
        .home_root
        .clone()
        .unwrap_or_else(|| Config::dir().map(|p| p.join("agent/home").display().to_string()).unwrap_or_else(|_| DEFAULT_HOME_DIR.to_string()));

    let child = Command::new(bin)
        .env("WORKSPACE_ROOT", workspace_root)
        .env("OPCODE_RUNTIME_MODE", "cloud-managed")
        .env("OPCODE_DEVICE_ID", &config.device_id)
        .env("OPCODE_CLOUD_CONTROL_URL", cloud_control_url)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;

    for _ in 0..20 {
        let health = reqwest::get(DEFAULT_OPCODE_HEALTH).await;
        if let Ok(resp) = health && resp.status().is_success() {
            return Ok(Some(child));
        }
        sleep(Duration::from_secs(1)).await;
    }

    Err(anyhow!("opcode-api did not become healthy on {}", DEFAULT_OPCODE_HEALTH).into())
}
async fn relay_local_http(base: &str, payload: &serde_json::Value) -> serde_json::Value {
    let method = payload.get("method").and_then(|v| v.as_str()).unwrap_or("GET");
    let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("/");
    let query = payload.get("query").cloned().unwrap_or_else(|| json!({}));
    let json_body = payload.get("json").cloned();
    let timeout = payload.get("timeout").and_then(|v| v.as_f64()).unwrap_or(90.0);

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs_f64(timeout))
        .build()
    {
        Ok(client) => client,
        Err(err) => return json!({"status_code": 500, "message": err.to_string(), "body": null}),
    };

    let mut req = client.request(
        reqwest::Method::from_bytes(method.as_bytes()).unwrap_or(reqwest::Method::GET),
        format!("{}{}", base.trim_end_matches('/'), path),
    );
    if let Some(map) = query.as_object() {
        req = req.query(map);
    }
    if let Some(body) = json_body {
        req = req.json(&body);
    }

    match req.send().await {
        Ok(resp) => {
            let status = resp.status();
            let body = match resp.bytes().await {
                Ok(bytes) if bytes.is_empty() => serde_json::Value::Null,
                Ok(bytes) => serde_json::from_slice::<serde_json::Value>(&bytes)
                    .unwrap_or_else(|_| json!({"raw": String::from_utf8_lossy(&bytes).to_string()})),
                Err(err) => json!({"error": err.to_string()}),
            };
            json!({
                "status_code": status.as_u16(),
                "message": status.canonical_reason().unwrap_or("ok"),
                "body": body,
            })
        }
        Err(err) => json!({"status_code": 502, "message": err.to_string(), "body": null}),
    }
}

async fn open_local_ws_tunnel(
    opcode_base: String,
    tunnel_id: String,
    session_id: String,
    sender: mpsc::UnboundedSender<Message>,
) {
    let ws_url = format!(
        "{}/ws/claude/{}",
        opcode_base
            .replace("http://", "ws://")
            .replace("https://", "wss://")
            .trim_end_matches('/'),
        session_id
    );

    match connect_async(ws_url.as_str()).await {
        Ok((socket, _)) => {
            let (write, mut read) = socket.split();
            let write = Arc::new(Mutex::new(write));
            let _ = sender.send(Message::Text(
                json!({"type":"ws_event","tunnel_id":tunnel_id,"event":"open"}).to_string(),
            ));
            TUNNELS.lock().await.insert(tunnel_id.clone(), write.clone());

            while let Some(message) = read.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        let _ = sender.send(Message::Text(
                            json!({"type":"ws_event","tunnel_id":tunnel_id,"event":"text","text":text}).to_string(),
                        ));
                    }
                    Ok(Message::Binary(bytes)) => {
                        let _ = sender.send(Message::Text(
                            json!({"type":"ws_event","tunnel_id":tunnel_id,"event":"bytes","bytes_base64":STANDARD.encode(bytes)}).to_string(),
                        ));
                    }
                    Ok(Message::Close(_)) => break,
                    Ok(_) => {}
                    Err(err) => {
                        let _ = sender.send(Message::Text(
                            json!({"type":"ws_event","tunnel_id":tunnel_id,"event":"error","detail":err.to_string()}).to_string(),
                        ));
                        break;
                    }
                }
            }
        }
        Err(err) => {
            let _ = sender.send(Message::Text(
                json!({"type":"ws_event","tunnel_id":tunnel_id,"event":"error","detail":err.to_string()}).to_string(),
            ));
        }
    }

    TUNNELS.lock().await.remove(&tunnel_id);
    let _ = sender.send(Message::Text(
        json!({"type":"ws_event","tunnel_id":tunnel_id,"event":"close"}).to_string(),
    ));
}

pub async fn run(ctx: &Context, command: AgentCommand) -> Result {
    match command {
        AgentCommand::InitHome(args) => {
            let mut config = load_agent_config()?;
            ensure_home_layout(&args.path)?;
            let home_root = canonical_string(&args.path);
            config.home_root = Some(home_root.clone());
            if let Some(name) = args.name {
                config.device_name = name;
            }
            if let Some(device_id) = args.device_id {
                config.device_id = device_id;
            }
            save_agent_config(&config)?;
            ctx.success(format!("Initialized agent home at {}", home_root));
            Ok(())
        }
        AgentCommand::Pair(args) => {
            let mut config = load_agent_config()?;
            if let Some(name) = args.name {
                config.device_name = name;
            }
            complete_pairing(ctx, &args.code, &config).await?;
            save_agent_config(&config)?;
            ctx.success(format!("Paired device {}", config.device_id));
            Ok(())
        }
        AgentCommand::Project { command } => match command {
            AgentProjectCommand::Create(args) => {
                let mut config = load_agent_config()?;
                let home_root = config
                    .home_root
                    .clone()
                    .ok_or_else(|| anyhow!("agent home is not initialized; run `d1v agent init-home` first"))?;
                let path = args
                    .path
                    .unwrap_or_else(|| PathBuf::from(&home_root).join("projects").join(&args.name));
                fs::create_dir_all(&path)?;
                init_workspace_binding(ctx, &args.project_id, &path, Some(args.name.clone())).await?;
                let root = canonical_string(&path);
                upsert_binding(&mut config, &args.project_id, &root);
                save_agent_config(&config)?;
                put_project_runtime(ctx, &args.project_id, &config.device_id).await?;
                put_project_binding(ctx, &args.project_id, &root).await?;
                ctx.success(format!("Created local project directory at {}", root));
                Ok(())
            }
            AgentProjectCommand::Import(args) => {
                let mut config = load_agent_config()?;
                let root = canonical_string(&args.path);
                if let Some(project_id) = args.project_id.as_deref() {
                    init_workspace_binding(ctx, project_id, &args.path, None).await?;
                    upsert_binding(&mut config, project_id, &root);
                    save_agent_config(&config)?;
                    put_project_runtime(ctx, project_id, &config.device_id).await?;
                    put_project_binding(ctx, project_id, &root).await?;
                    ctx.success(format!("Imported and bound local project {}", root));
                } else {
                    ctx.success(format!("Imported local project directory {}", root));
                }
                Ok(())
            }
            AgentProjectCommand::Bind(args) => {
                let mut config = load_agent_config()?;
                let root = canonical_string(&args.path);
                init_workspace_binding(ctx, &args.project_id, &args.path, None).await?;
                upsert_binding(&mut config, &args.project_id, &root);
                save_agent_config(&config)?;
                put_project_runtime(ctx, &args.project_id, &config.device_id).await?;
                put_project_binding(ctx, &args.project_id, &root).await?;
                ctx.success(format!("Bound project {} to {}", args.project_id, root));
                Ok(())
            }
        },
        AgentCommand::Start(args) => {
            let config = load_agent_config()?;
            let _child = maybe_spawn_opcode(
                ctx,
                &config,
                args.opcode_bin.as_deref(),
                &base_url(ctx),
            )
            .await?;
            let token = ctx
                .tokens
                .lookup()?
                .ok_or_else(|| anyhow!("missing auth token"))?;
            let mut url = Url::parse(
                &base_url(ctx)
                    .replace("http://", "ws://")
                    .replace("https://", "wss://"),
            )
            .map_err(|e| anyhow!(e))?;
            url.set_path("/api/agent/connect");
            url.query_pairs_mut()
                .append_pair("token", token.expose_secret())
                .append_pair("device_id", &config.device_id);
            let (ws, _) = connect_async(url.as_str())
                .await
                .map_err(|e| anyhow!(e))?;
            let (mut sink, mut stream) = ws.split();
            let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Message>();

            let writer = tokio::spawn(async move {
                while let Some(message) = out_rx.recv().await {
                    if sink.send(message).await.is_err() {
                        break;
                    }
                }
            });

            while let Some(message) = stream.next().await {
                let message = message.map_err(|e| anyhow!(e))?;
                if !message.is_text() {
                    continue;
                }
                let payload: serde_json::Value = serde_json::from_str(
                    message.to_text().map_err(|e| anyhow!(e))?,
                )
                .map_err(|e| anyhow!(e))?;
                match payload.get("type").and_then(|v| v.as_str()).unwrap_or("") {
                    "request" => {
                        let response = relay_local_http(&config.opcode_base_url, &payload).await;
                        let envelope = json!({
                            "type": "response",
                            "request_id": payload.get("request_id").cloned().unwrap_or(serde_json::Value::Null),
                            "status_code": response.get("status_code").cloned().unwrap_or(json!(500)),
                            "message": response.get("message").cloned().unwrap_or(json!("error")),
                            "body": response.get("body").cloned().unwrap_or(serde_json::Value::Null),
                        });
                        out_tx
                            .send(Message::Text(envelope.to_string()))
                            .map_err(|e| anyhow!(e))?;
                    }
                    "ws_open" => {
                        let tunnel_id = payload
                            .get("tunnel_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let session_id = payload
                            .get("session_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let sender = out_tx.clone();
                        let opcode_base = config.opcode_base_url.clone();
                        tokio::spawn(async move {
                            open_local_ws_tunnel(opcode_base, tunnel_id, session_id, sender).await;
                        });
                    }
                    "ws_send" => {
                        let tunnel_id = payload
                            .get("tunnel_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let writer = TUNNELS.lock().await.get(&tunnel_id).cloned();
                        if let Some(writer) = writer {
                            let mut writer = writer.lock().await;
                            if let Some(text) = payload.get("text").and_then(|v| v.as_str()) {
                                writer
                                    .send(Message::Text(text.to_string()))
                                    .await
                                    .map_err(|e| anyhow!(e))?;
                            } else if let Some(encoded) =
                                payload.get("bytes_base64").and_then(|v| v.as_str())
                            {
                                writer
                                    .send(Message::Binary(STANDARD.decode(encoded).map_err(|e| anyhow!(e))?.into()))
                                    .await
                                    .map_err(|e| anyhow!(e))?;
                            }
                        }
                    }
                    "ws_close" => {
                        let tunnel_id = payload
                            .get("tunnel_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let writer = TUNNELS.lock().await.remove(&tunnel_id);
                        if let Some(writer) = writer {
                            let mut writer = writer.lock().await;
                            let _ = writer.send(Message::Close(None)).await;
                        }
                    }
                    _ => {}
                }
            }
            writer.abort();
            Ok(())
        }
        AgentCommand::Status => {
            let config = load_agent_config()?;
            let home_root = config.home_root.clone().unwrap_or_else(|| "-".to_string());
            let bindings = if config.project_bindings.is_empty() {
                "-".to_string()
            } else {
                config
                    .project_bindings
                    .iter()
                    .map(|item| format!("{} => {}", item.project_id, item.workspace_root))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            ctx.present(
                Text::new()
                    .line(Line::styled("Agent configuration".to_string(), theme::ansi::success()))
                    .line(Line::raw(format!("  Device ID: {}", config.device_id)))
                    .line(Line::raw(format!("  Device Name: {}", config.device_name)))
                    .line(Line::raw(format!("  Home Root: {}", home_root)))
                    .line(Line::raw(format!("  Opcode Base URL: {}", config.opcode_base_url)))
                    .line(Line::raw(format!("  Project Bindings: {}", bindings))),
                &config,
            )?;
            Ok(())
        }
        AgentCommand::InitRuntime(args) => {
            let mut config = load_agent_config()?;
            let home_path = args
                .path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from(DEFAULT_HOME_DIR));
            ensure_home_layout(&home_path)?;
            config.home_root = Some(canonical_string(&home_path));
            if let Some(name) = args.name {
                config.device_name = name;
            }
            if let Some(device_id) = args.device_id {
                config.device_id = device_id;
            }
            let root = canonical_string(&args.path);
            init_workspace_binding(ctx, &args.project_id, &args.path, None).await?;
            upsert_binding(&mut config, &args.project_id, &root);
            save_agent_config(&config)?;
            put_project_runtime(ctx, &args.project_id, &config.device_id).await?;
            put_project_binding(ctx, &args.project_id, &root).await?;
            ctx.success(format!(
                "Initialized compatibility runtime binding for project {} at {}",
                args.project_id, root
            ));
            Ok(())
        }
    }
}
