use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context as AnyhowContext, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use clap::{Args, Subcommand};
use futures_util::{SinkExt, StreamExt};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::{Mutex, mpsc};
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::header::{ORIGIN, SEC_WEBSOCKET_PROTOCOL};
use tokio_tungstenite::tungstenite::http::{HeaderName, HeaderValue};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
use url::{Host, Url};

use crate::Context;
use crate::config::Config;
use crate::error::Result;
use crate::runtime_install;
use crate::text::{Line, Text};
use crate::theme;
use crate::token::TokenSource;
use crate::workspace;

const DEFAULT_HOME_DIR: &str = ".d1v/agent/home";
const DEFAULT_OPCODE_HEALTH: &str = "http://127.0.0.1:9191/health";
const DEFAULT_OPCODE_BASE: &str = "http://127.0.0.1:9191";
const DEFAULT_AUTO_EXPOSE_CANDIDATE_PORTS: &[u16] = &[3000, 4173, 5173, 8000, 8787, 9000];

type TunnelWriter = futures_util::stream::SplitSink<
    WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
    Message,
>;

static TUNNELS: std::sync::LazyLock<Mutex<HashMap<String, Arc<Mutex<TunnelWriter>>>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

fn append_debug_log(message: &str) {
    let path = std::env::temp_dir().join("d1v-expose-debug.log");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{message}");
    }
}

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
    pub code: Option<String>,
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
pub(crate) struct AgentConfig {
    pub(crate) device_id: String,
    pub(crate) device_name: String,
    pub(crate) home_root: Option<String>,
    pub(crate) opcode_base_url: String,
    pub(crate) project_bindings: Vec<ProjectBinding>,
}

pub(crate) struct RuntimeDoctorConfig {
    pub device_id: String,
    pub home_root: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct RuntimeTerminalBootstrap {
    enabled: bool,
    public_key: Option<String>,
    #[serde(default)]
    issuer: String,
    #[serde(default)]
    audience: String,
}

impl RuntimeTerminalBootstrap {
    fn validated(mut self) -> Self {
        let public_key = self
            .public_key
            .take()
            .map(|value| value.replace("\\n", "\n").trim().to_string())
            .filter(|value| !value.is_empty());
        if !self.enabled
            || public_key.is_none()
            || self.issuer.trim().is_empty()
            || self.audience.trim().is_empty()
        {
            return Self::default();
        }
        self.public_key = public_key;
        self
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
struct OpcodeRuntimeCapabilities {
    #[serde(default)]
    supports_terminal: bool,
    #[serde(default)]
    terminal_base_url: Option<String>,
}

pub(crate) fn load_runtime_doctor_config() -> Result<RuntimeDoctorConfig> {
    let config = load_agent_config()?;
    Ok(RuntimeDoctorConfig {
        device_id: config.device_id,
        home_root: config.home_root,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ProjectBinding {
    project_id: String,
    workspace_root: String,
}

fn agent_dir() -> Result<PathBuf> {
    Ok(Config::dir()?.join("agent"))
}

fn agent_config_path() -> Result<PathBuf> {
    Ok(agent_dir()?.join("config.json"))
}

pub(crate) fn load_agent_config() -> Result<AgentConfig> {
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

pub(crate) fn save_agent_config(config: &AgentConfig) -> Result {
    let path = agent_config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(config).map_err(|e| anyhow!(e))?,
    )?;
    Ok(())
}

pub(crate) fn uuid_like() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    format!("{nanos:x}")
}

pub(crate) fn system_hostname() -> Option<String> {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("HOST"))
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .or_else(|| {
            Command::new("hostname")
                .output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        })
}

fn default_device_name() -> String {
    let host = system_hostname().unwrap_or_else(|| "local-device".to_string());
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

pub(crate) async fn authed_http_client(ctx: &Context) -> Result<reqwest::Client> {
    let token = ctx
        .tokens
        .lookup()?
        .ok_or_else(|| anyhow!("missing auth token"))?;
    let mut builder = build_authed_client_builder(token.expose_secret())?;
    if is_loopback_base_url(&base_url(ctx)) {
        builder = builder.no_proxy();
    }
    Ok(builder.build().map_err(|e| anyhow!(e))?)
}

pub(crate) fn build_authed_client(token: &str) -> Result<reqwest::Client> {
    build_authed_client_builder(token)?
        .no_proxy()
        .build()
        .map_err(|e| anyhow!(e).into())
}

fn build_authed_client_builder(token: &str) -> Result<reqwest::ClientBuilder> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        format!("Bearer {}", token)
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
        .timeout(Duration::from_secs(60)))
}

pub(crate) fn base_url(ctx: &Context) -> String {
    ctx.client.base_url().trim_end_matches('/').to_string()
}

fn is_loopback_base_url(raw: &str) -> bool {
    let Ok(url) = Url::parse(raw) else {
        return false;
    };
    match url.host() {
        Some(Host::Domain(host)) => host.eq_ignore_ascii_case("localhost"),
        Some(Host::Ipv4(ip)) => ip.is_loopback(),
        Some(Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    }
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

async fn put_project_runtime(ctx: &Context, project_id: &str, device_id: &str) -> Result {
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

async fn put_project_binding(ctx: &Context, project_id: &str, workspace_root: &str) -> Result {
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

fn runtime_node_capabilities(
    config: &AgentConfig,
    runtime: &OpcodeRuntimeCapabilities,
) -> serde_json::Value {
    let mut capabilities = serde_json::Map::from_iter([
        ("runtime".to_string(), json!("opcode-api")),
        ("transport".to_string(), json!("relay")),
        ("device_id".to_string(), json!(config.device_id)),
    ]);
    if let Some(terminal_base_url) = verified_terminal_base_url(runtime) {
        capabilities.insert("supports_terminal".to_string(), json!(true));
        capabilities.insert("terminal_base_url".to_string(), json!(terminal_base_url));
    }
    serde_json::Value::Object(capabilities)
}

fn verified_terminal_base_url(runtime: &OpcodeRuntimeCapabilities) -> Option<String> {
    if !runtime.supports_terminal {
        return None;
    }
    let value = runtime
        .terminal_base_url
        .as_deref()?
        .trim()
        .trim_end_matches('/');
    if !is_loopback_base_url(value) {
        return None;
    }
    Some(value.to_string())
}

pub(crate) async fn register_customer_runtime_node(ctx: &Context, config: &AgentConfig) -> Result {
    let client = authed_http_client(ctx).await?;
    let runtime = fetch_opcode_runtime_capabilities(&config.opcode_base_url).await;
    let capabilities = runtime_node_capabilities(config, &runtime);
    let resp = client
        .post(format!(
            "{}/api/devices/runtime-node/register",
            base_url(ctx)
        ))
        .json(&json!({
            "node_id": format!("customer-{}", config.device_id),
            "region": "local",
            "version": "dev",
            "total_slots": 1,
            "cpu_cores": 0,
            "memory_mb": 0,
            "disk_gb": 0,
            "capabilities": capabilities,
            "ingress_mode": "reverse_relay",
        }))
        .send()
        .await
        .map_err(|e| anyhow!(e))?;
    if !resp.status().is_success() {
        return Err(anyhow!(
            "failed to register customer runtime node: {}",
            resp.status()
        )
        .into());
    }
    Ok(())
}

async fn fetch_runtime_terminal_bootstrap(ctx: &Context) -> RuntimeTerminalBootstrap {
    let Ok(client) = authed_http_client(ctx).await else {
        return RuntimeTerminalBootstrap::default();
    };
    let Ok(response) = client
        .get(format!("{}/api/devices/runtime/bootstrap", base_url(ctx)))
        .send()
        .await
    else {
        return RuntimeTerminalBootstrap::default();
    };
    if !response.status().is_success() {
        return RuntimeTerminalBootstrap::default();
    }
    let Ok(response) = response.json::<d1v_api::response::Response>().await else {
        return RuntimeTerminalBootstrap::default();
    };
    let Ok(bootstrap) = response.ok::<RuntimeTerminalBootstrap>() else {
        return RuntimeTerminalBootstrap::default();
    };
    bootstrap.validated()
}

async fn fetch_opcode_runtime_capabilities(base: &str) -> OpcodeRuntimeCapabilities {
    let Ok(client) = reqwest::Client::builder().no_proxy().build() else {
        return OpcodeRuntimeCapabilities::default();
    };
    let base = base.trim().trim_end_matches('/');
    let Ok(response) = client
        .get(format!("{base}/api/d1v/runtime/capabilities"))
        .send()
        .await
    else {
        return OpcodeRuntimeCapabilities::default();
    };
    if !response.status().is_success() {
        return OpcodeRuntimeCapabilities::default();
    }
    response.json().await.unwrap_or_default()
}

fn opcode_port_from_base_url(base: &str) -> u16 {
    let Ok(url) = Url::parse(base) else {
        return 9191;
    };
    if let Some(port) = url.port_or_known_default() {
        return port;
    }
    9191
}

fn parse_listening_ports(output: &str) -> Vec<u16> {
    let mut ports = Vec::new();
    for raw in output.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let candidate = line
            .rsplit(':')
            .next()
            .and_then(|value| value.parse::<u16>().ok());
        if let Some(port) = candidate {
            if !ports.contains(&port) {
                ports.push(port);
            }
        }
    }
    ports
}

fn configured_auto_expose_candidate_ports() -> Vec<u16> {
    if let Ok(raw) = env::var("D1V_RUNTIME_AUTO_EXPOSE_CANDIDATE_PORTS") {
        let mut ports = Vec::new();
        for part in raw.split(',') {
            let value = part.trim();
            if value.is_empty() {
                continue;
            }
            if let Ok(port) = value.parse::<u16>() {
                if !ports.contains(&port) {
                    ports.push(port);
                }
            }
        }
        if !ports.is_empty() {
            return ports;
        }
    }
    DEFAULT_AUTO_EXPOSE_CANDIDATE_PORTS.to_vec()
}

fn detect_local_expose_ports(config: &AgentConfig) -> Vec<u16> {
    let mut ports = Vec::new();
    let opcode_port = opcode_port_from_base_url(&config.opcode_base_url);
    ports.push(opcode_port);

    let candidates = configured_auto_expose_candidate_ports();
    let result = Command::new("sh")
        .arg("-lc")
        .arg("lsof -nP -iTCP -sTCP:LISTEN 2>/dev/null | awk 'NR>1 {print $9}'")
        .output();
    if let Ok(output) = result {
        if output.status.success() {
            let discovered = parse_listening_ports(&String::from_utf8_lossy(&output.stdout));
            for port in discovered {
                if candidates.contains(&port) && !ports.contains(&port) {
                    ports.push(port);
                }
            }
        }
    }
    ports
}

async fn ensure_customer_auto_expose(ctx: &Context, config: &AgentConfig) -> Result {
    let client = authed_http_client(ctx).await?;
    for port in detect_local_expose_ports(config) {
        let resp = client
            .post(format!(
                "{}/api/devices/runtime-node/exposes",
                base_url(ctx)
            ))
            .json(&json!({
                "node_id": format!("customer-{}", config.device_id),
                "container_port": port,
                "host_port": port,
                "protocol": "http",
                "container_id": config.device_id,
            }))
            .send()
            .await
            .map_err(|e| anyhow!(e))?;
        if !resp.status().is_success() {
            return Err(anyhow!(
                "failed to create customer expose binding: {}",
                resp.status()
            )
            .into());
        }
    }
    Ok(())
}

async fn ensure_customer_auto_expose_once(
    api_base_url: String,
    auth_token: String,
    device_id: String,
    opcode_base_url: String,
) -> Result {
    let client = build_authed_client(&auth_token)?;

    let list_resp = client
        .get(format!(
            "{}/api/devices/runtime-node/exposes?node_id=customer-{}",
            api_base_url, device_id
        ))
        .send()
        .await
        .map_err(|e| anyhow!(e))?;
    if !list_resp.status().is_success() {
        return Err(anyhow!(
            "failed to list customer expose bindings: {}",
            list_resp.status()
        )
        .into());
    }
    let value: serde_json::Value = list_resp.json().await.map_err(|e| anyhow!(e))?;
    let rows = value
        .get("data")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let config = AgentConfig {
        device_id: device_id.clone(),
        device_name: String::new(),
        home_root: None,
        opcode_base_url,
        project_bindings: Vec::new(),
    };
    for port in detect_local_expose_ports(&config) {
        let already_present = rows.iter().any(|item| {
            let container_port = item
                .get("container_port")
                .and_then(|v| v.as_u64())
                .unwrap_or_default() as u16;
            let status = item
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            container_port == port && matches!(status, "pending" | "active")
        });
        if already_present {
            continue;
        }

        let resp = client
            .post(format!("{}/api/devices/runtime-node/exposes", api_base_url))
            .json(&json!({
                "node_id": format!("customer-{}", device_id),
                "container_port": port,
                "host_port": port,
                "protocol": "http",
                "container_id": device_id,
            }))
            .send()
            .await
            .map_err(|e| anyhow!(e))?;
        if !resp.status().is_success() {
            return Err(anyhow!(
                "failed to create customer expose binding: {}",
                resp.status()
            )
            .into());
        }
    }
    Ok(())
}

pub(crate) async fn heartbeat_customer_runtime_node_loop(
    api_base_url: String,
    auth_token: String,
    device_id: String,
) -> Result {
    let client = build_authed_client(&auth_token)?;
    loop {
        let resp = client
            .post(format!(
                "{}/api/devices/runtime-node/heartbeat",
                api_base_url
            ))
            .json(&json!({
                "node_id": format!("customer-{}", device_id),
                "status": "online",
                "used_slots": 0,
            }))
            .send()
            .await
            .map_err(|e| anyhow!(e))?;
        if !resp.status().is_success() {
            return Err(anyhow!(
                "failed to heartbeat customer runtime node: {}",
                resp.status()
            )
            .into());
        }
        sleep(Duration::from_secs(15)).await;
    }
}

async fn ensure_customer_auto_expose_loop(
    api_base_url: String,
    auth_token: String,
    device_id: String,
    opcode_base_url: String,
) -> Result {
    loop {
        let _ = ensure_customer_auto_expose_once(
            api_base_url.clone(),
            auth_token.clone(),
            device_id.clone(),
            opcode_base_url.clone(),
        )
        .await;
        sleep(Duration::from_secs(15)).await;
    }
}

pub(crate) async fn complete_pairing(ctx: &Context, code: &str, config: &AgentConfig) -> Result {
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

#[derive(Debug, Deserialize)]
struct PairStartResponse {
    pairing_code: String,
}

pub(crate) async fn start_pairing(ctx: &Context) -> Result<String> {
    let client = authed_http_client(ctx).await?;
    let response = client
        .post(format!("{}/api/devices/pair/start", base_url(ctx)))
        .send()
        .await
        .map_err(|e| anyhow!(e))?
        .json::<d1v_api::response::Response>()
        .await
        .map_err(|e| anyhow!(e))?;
    let payload: PairStartResponse = response.ok().map_err(|e| anyhow!(e))?;
    Ok(payload.pairing_code)
}

fn upsert_binding(config: &mut AgentConfig, project_id: &str, workspace_root: &str) {
    config
        .project_bindings
        .retain(|item| item.project_id != project_id);
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
    terminal_bootstrap: &RuntimeTerminalBootstrap,
) -> Result<Option<Child>> {
    if opcode_is_healthy().await {
        if let Ok(Some(version)) = runtime_install::available_runtime_update(None).await {
            ctx.info(format!(
                "opcode-api runtime update available: {version} (run `d1v runtime upgrade`)"
            ));
        }
        return Ok(None);
    }
    if wait_for_existing_opcode_health().await {
        if let Ok(Some(version)) = runtime_install::available_runtime_update(None).await {
            ctx.info(format!(
                "opcode-api runtime update available: {version} (run `d1v runtime upgrade`)"
            ));
        }
        return Ok(None);
    }
    if opcode_port_is_occupied() {
        return Err(anyhow!(
            "port 9191 is already in use but is not serving opcode-api; stop the conflicting process and retry"
        )
        .into());
    }

    let bin = if let Some(path) = opcode_bin {
        path.to_path_buf()
    } else if let Ok(raw) = std::env::var("D1V_OPCODE_API_BIN") {
        PathBuf::from(raw)
    } else {
        runtime_install::ensure_runtime_installed(ctx, None).await?
    };

    ensure_system_cli_tools(ctx)?;

    let workspace_root = config.home_root.clone().unwrap_or_else(|| {
        Config::dir()
            .map(|p| p.join("agent/home").display().to_string())
            .unwrap_or_else(|_| DEFAULT_HOME_DIR.to_string())
    });
    let log_path = runtime_log_path(config)?;
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    let log_file_err = log_file.try_clone()?;
    ctx.info(format!(
        "starting opcode-api runtime with logs at {}",
        log_path.display()
    ));

    let mut command = Command::new(bin);
    command
        .env("WORKSPACE_ROOT", workspace_root)
        .env("OPCODE_RUNTIME_MODE", "cloud-managed")
        .env("OPCODE_DEVICE_ID", &config.device_id)
        .env("OPCODE_CLOUD_CONTROL_URL", cloud_control_url)
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(log_file_err));
    configure_terminal_command(&mut command, config, terminal_bootstrap);
    let child = command.spawn()?;

    for _ in 0..20 {
        if opcode_is_healthy().await {
            if let Ok(Some(version)) = runtime_install::available_runtime_update(None).await {
                ctx.info(format!(
                    "opcode-api runtime update available: {version} (run `d1v runtime upgrade`)"
                ));
            }
            return Ok(Some(child));
        }
        sleep(Duration::from_secs(1)).await;
    }

    if opcode_port_is_occupied() {
        return Err(anyhow!(
            "opcode-api failed to become healthy on {}. Check logs at {}",
            DEFAULT_OPCODE_HEALTH,
            log_path.display()
        )
        .into());
    }

    Err(anyhow!(
        "opcode-api did not become healthy on {}. Check logs at {}",
        DEFAULT_OPCODE_HEALTH,
        log_path.display()
    )
    .into())
}

fn configure_terminal_command(
    command: &mut Command,
    config: &AgentConfig,
    terminal_bootstrap: &RuntimeTerminalBootstrap,
) {
    command
        .env(
            "OPCODE_RUNTIME_NODE_ID",
            format!("customer-{}", config.device_id),
        )
        .env(
            "D1V_SHELL_ENABLED",
            if terminal_bootstrap.enabled { "1" } else { "0" },
        );
    if terminal_bootstrap.enabled {
        command
            .env(
                "D1V_SHELL_TICKET_PUBLIC_KEY",
                terminal_bootstrap.public_key.as_deref().unwrap_or_default(),
            )
            .env("D1V_SHELL_TICKET_ISSUER", &terminal_bootstrap.issuer)
            .env("D1V_SHELL_TICKET_AUDIENCE", &terminal_bootstrap.audience);
    }
}

async fn opcode_is_healthy() -> bool {
    let client = match reqwest::Client::builder().no_proxy().build() {
        Ok(client) => client,
        Err(_) => return false,
    };
    let health = client.get(DEFAULT_OPCODE_HEALTH).send().await;
    matches!(health, Ok(resp) if resp.status().is_success())
}

async fn wait_for_existing_opcode_health() -> bool {
    if !opcode_port_is_occupied() {
        return false;
    }
    for _ in 0..10 {
        if opcode_is_healthy().await {
            return true;
        }
        sleep(Duration::from_millis(300)).await;
    }
    false
}

fn opcode_port_is_occupied() -> bool {
    let addr: SocketAddr = "127.0.0.1:9191".parse().expect("valid socket addr");
    TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok()
}

fn runtime_log_path(config: &AgentConfig) -> Result<PathBuf> {
    let base = config
        .home_root
        .as_deref()
        .map(PathBuf::from)
        .or_else(|| Config::dir().ok().map(|dir| dir.join("agent/home")))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_HOME_DIR));
    Ok(base.join("logs").join("opcode-api.log"))
}

struct RequiredCliTool {
    label: &'static str,
    binary: &'static str,
    env_override: Option<&'static str>,
    common_paths: &'static [&'static str],
}

const REQUIRED_CLI_TOOLS: &[RequiredCliTool] = &[
    RequiredCliTool {
        label: "Claude Code",
        binary: "claude",
        env_override: None,
        common_paths: &[
            "/usr/local/bin/claude",
            "/opt/homebrew/bin/claude",
            "~/.local/bin/claude",
        ],
    },
    RequiredCliTool {
        label: "Codex",
        binary: "codex",
        env_override: Some("CODEX_BIN"),
        common_paths: &[
            "/usr/local/bin/codex",
            "/opt/homebrew/bin/codex",
            "~/.local/bin/codex",
        ],
    },
];

enum InstallAction {
    Shell {
        program: &'static str,
        args: Vec<&'static str>,
        display: &'static str,
    },
    Unsupported {
        reason: String,
    },
}

fn find_binary_on_path(binary: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|dir| dir.join(binary))
        .find(|candidate| candidate.is_file())
}

fn expand_home_path(path: &str) -> Option<PathBuf> {
    if let Some(suffix) = path.strip_prefix("~/") {
        return dirs::home_dir().map(|home| home.join(suffix));
    }
    Some(PathBuf::from(path))
}

fn resolve_tool_binary(tool: &RequiredCliTool) -> Option<PathBuf> {
    if let Some(env_name) = tool.env_override {
        if let Ok(value) = env::var(env_name) {
            let candidate = PathBuf::from(value);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    if let Some(path) = find_binary_on_path(tool.binary) {
        return Some(path);
    }

    tool.common_paths
        .iter()
        .filter_map(|path| expand_home_path(path))
        .find(|candidate| candidate.is_file())
}

fn install_action_for_tool(tool: &RequiredCliTool) -> InstallAction {
    match tool.binary {
        "claude" => InstallAction::Shell {
            program: "sh",
            args: vec!["-lc", "curl -fsSL https://claude.ai/install.sh | bash"],
            display: "curl -fsSL https://claude.ai/install.sh | bash",
        },
        "codex" => match std::env::consts::OS {
            "macos" | "linux" => InstallAction::Shell {
                program: "sh",
                args: vec!["-lc", "curl -fsSL https://chatgpt.com/codex/install.sh | sh"],
                display: "curl -fsSL https://chatgpt.com/codex/install.sh | sh",
            },
            "windows" => InstallAction::Unsupported {
                reason: "Codex on Windows should follow the official Windows guide or WSL flow: https://developers.openai.com/codex/windows".to_string(),
            },
            other => InstallAction::Unsupported {
                reason: format!(
                    "Codex automatic install is not configured for OS `{other}`. Follow https://developers.openai.com/codex/cli"
                ),
            },
        },
        _ => InstallAction::Unsupported {
            reason: format!("no installer configured for {}", tool.label),
        },
    }
}

fn run_install_action(action: &InstallAction) -> Result {
    match action {
        InstallAction::Shell {
            program,
            args,
            display,
        } => {
            let status = Command::new(program)
                .args(args)
                .status()
                .with_context(|| format!("failed to run installer command: {display}"))?;
            if status.success() {
                Ok(())
            } else {
                Err(anyhow!("installer command exited with status {status}: {display}").into())
            }
        }
        InstallAction::Unsupported { reason } => Err(anyhow!(reason.clone()).into()),
    }
}

fn ensure_system_cli_tools(ctx: &Context) -> Result {
    for tool in REQUIRED_CLI_TOOLS {
        if let Some(path) = resolve_tool_binary(tool) {
            ctx.info(format!("found {} at {}", tool.label, path.display()));
            continue;
        }

        let action = install_action_for_tool(tool);
        if matches!(action, InstallAction::Shell { display, .. } if display.contains("curl"))
            && find_binary_on_path("curl").is_none()
        {
            return Err(anyhow!(
                "{} is not installed and curl is missing. Install curl first, then rerun `d1v agent start`.",
                tool.label
            )
            .into());
        }

        match &action {
            InstallAction::Shell { display, .. } => {
                ctx.info(format!(
                    "{} not found; installing with `{}`",
                    tool.label, display
                ));
            }
            InstallAction::Unsupported { reason } => {
                return Err(anyhow!("{} not found. {}", tool.label, reason).into());
            }
        }

        run_install_action(&action)?;

        if let Some(path) = resolve_tool_binary(tool) {
            ctx.success(format!("installed {} at {}", tool.label, path.display()));
            continue;
        }

        return Err(anyhow!(
            "{} installer completed but `{}` is still not available on PATH/common locations.",
            tool.label,
            tool.binary
        )
        .into());
    }

    Ok(())
}

async fn relay_local_http(base: &str, payload: &serde_json::Value) -> serde_json::Value {
    let target_base = payload
        .get("target_base_url")
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(base);
    let method = payload
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("GET");
    let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or("/");
    let query = payload.get("query").cloned().unwrap_or_else(|| json!({}));
    let headers = payload.get("headers").cloned().unwrap_or_else(|| json!({}));
    let json_body = payload.get("json").cloned();
    let body_base64 = payload.get("body_base64").and_then(|v| v.as_str());
    let timeout = payload
        .get("timeout")
        .and_then(|v| v.as_f64())
        .unwrap_or(90.0);

    let client = match reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs_f64(timeout))
        .build()
    {
        Ok(client) => client,
        Err(err) => return json!({"status_code": 500, "message": err.to_string(), "body": null}),
    };

    let mut req = client.request(
        reqwest::Method::from_bytes(method.as_bytes()).unwrap_or(reqwest::Method::GET),
        format!("{}{}", target_base.trim_end_matches('/'), path),
    );
    if let Some(map) = query.as_object() {
        req = req.query(map);
    }
    if let Some(map) = headers.as_object() {
        for (key, value) in map {
            let Some(value) = value.as_str() else {
                continue;
            };
            req = req.header(key, value);
        }
    }
    if let Some(body) = json_body {
        req = req.json(&body);
    } else if let Some(encoded) = body_base64 {
        match STANDARD.decode(encoded) {
            Ok(bytes) => {
                req = req.body(bytes);
            }
            Err(err) => {
                return json!({"status_code": 400, "message": err.to_string(), "body": null});
            }
        }
    }

    match req.send().await {
        Ok(resp) => {
            let status = resp.status();
            let headers: serde_json::Map<String, serde_json::Value> = resp
                .headers()
                .iter()
                .filter_map(|(key, value)| {
                    value
                        .to_str()
                        .ok()
                        .map(|value| (key.as_str().to_string(), json!(value)))
                })
                .collect();
            let bytes = match resp.bytes().await {
                Ok(bytes) => bytes,
                Err(err) => {
                    return json!({"status_code": 502, "message": err.to_string(), "body": null});
                }
            };
            eprintln!(
                "d1v-agent relay_local_http upstream status={} bytes={}",
                status.as_u16(),
                bytes.len()
            );
            let body = if bytes.is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::from_slice::<serde_json::Value>(&bytes)
                    .unwrap_or(serde_json::Value::Null)
            };
            json!({
                "status_code": status.as_u16(),
                "message": status.canonical_reason().unwrap_or("ok"),
                "headers": headers,
                "body_base64": STANDARD.encode(&bytes),
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
    target_base_url: Option<String>,
    websocket_path: Option<String>,
    headers: HashMap<String, String>,
    subprotocols: Vec<String>,
    sender: mpsc::UnboundedSender<Message>,
) {
    let expects_terminal_protocol = subprotocols.iter().any(|value| value == "d1v-terminal.v1");
    let target_base = target_base_url.unwrap_or(opcode_base);
    if expects_terminal_protocol && !is_loopback_base_url(&target_base) {
        let _ = sender.send(Message::Text(
            json!({"type":"ws_event","tunnel_id":tunnel_id,"event":"error","detail":"terminal_target_must_be_loopback"}).to_string(),
        ));
        return;
    }
    let ws_base = target_base
        .replace("http://", "ws://")
        .replace("https://", "wss://")
        .trim_end_matches('/')
        .to_string();
    let ws_path = websocket_path
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("/ws/claude/{}", session_id));
    let ws_url = format!("{}{}", ws_base, ws_path);
    let request = match local_ws_tunnel_request(&ws_url, &headers, &subprotocols) {
        Ok(request) => request,
        Err(err) => {
            let _ = sender.send(Message::Text(
                json!({"type":"ws_event","tunnel_id":tunnel_id,"event":"error","detail":err.to_string()}).to_string(),
            ));
            return;
        }
    };
    match connect_async(request).await {
        Ok((socket, response)) => {
            if expects_terminal_protocol
                && response
                    .headers()
                    .get(SEC_WEBSOCKET_PROTOCOL)
                    .and_then(|value| value.to_str().ok())
                    != Some("d1v-terminal.v1")
            {
                let _ = sender.send(Message::Text(
                    json!({"type":"ws_event","tunnel_id":tunnel_id,"event":"error","detail":"upstream_websocket_subprotocol_mismatch"}).to_string(),
                ));
                return;
            }
            let (write, mut read) = socket.split();
            let write = Arc::new(Mutex::new(write));
            let _ = sender.send(Message::Text(
                json!({"type":"ws_event","tunnel_id":tunnel_id,"event":"open"}).to_string(),
            ));
            TUNNELS
                .lock()
                .await
                .insert(tunnel_id.clone(), write.clone());

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

fn local_ws_tunnel_request(
    ws_url: &str,
    headers: &HashMap<String, String>,
    subprotocols: &[String],
) -> anyhow::Result<tokio_tungstenite::tungstenite::http::Request<()>> {
    let mut request = ws_url
        .into_client_request()
        .map_err(|error| anyhow!("invalid relay WebSocket URL: {error}"))?;
    for (name, value) in headers {
        let header = match name.trim().to_ascii_lowercase().as_str() {
            "origin" => ORIGIN,
            "x-d1v-shell-ticket" => HeaderName::from_static("x-d1v-shell-ticket"),
            _ => continue,
        };
        request.headers_mut().insert(
            header,
            HeaderValue::from_str(value)
                .map_err(|_| anyhow!("invalid relay WebSocket handshake header"))?,
        );
    }
    if subprotocols.iter().any(|value| value == "d1v-terminal.v1") {
        request.headers_mut().insert(
            SEC_WEBSOCKET_PROTOCOL,
            HeaderValue::from_static("d1v-terminal.v1"),
        );
    }
    Ok(request)
}

pub(crate) async fn run_agent_relay_forever(
    ctx: &Context,
    config: AgentConfig,
    auth_token: String,
) -> Result {
    append_debug_log(&format!(
        "run_agent_relay_forever start device_id={} base_url={}",
        config.device_id,
        base_url(ctx)
    ));
    let mut url = Url::parse(
        &base_url(ctx)
            .replace("http://", "ws://")
            .replace("https://", "wss://"),
    )
    .map_err(|e| anyhow!(e))?;
    url.set_path("/api/agent/connect");
    url.query_pairs_mut()
        .append_pair("token", &auth_token)
        .append_pair("device_id", &config.device_id);

    loop {
        append_debug_log(&format!("connect_async begin url={}", url));
        let (ws, _) = connect_async(url.as_str()).await.map_err(|e| anyhow!(e))?;
        append_debug_log("connect_async connected");
        let (mut sink, mut stream) = ws.split();
        let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Message>();

        let writer = tokio::spawn(async move {
            while let Some(message) = out_rx.recv().await {
                if sink.send(message).await.is_err() {
                    break;
                }
            }
        });

        let mut reconnect = false;
        while let Some(message) = stream.next().await {
            let message = match message {
                Ok(message) => message,
                Err(err) => {
                    append_debug_log(&format!("stream error err={err}"));
                    ctx.info(format!("agent relay disconnected: {err}; reconnecting"));
                    reconnect = true;
                    break;
                }
            };
            if !message.is_text() {
                append_debug_log("received non-text websocket frame");
                continue;
            }
            let payload: serde_json::Value =
                serde_json::from_str(message.to_text().map_err(|e| anyhow!(e))?)
                    .map_err(|e| anyhow!(e))?;
            if let Some(kind) = payload.get("type").and_then(|v| v.as_str()) {
                append_debug_log(&format!("received frame type={kind}"));
                ctx.info(format!("agent relay received frame type={kind}"));
            }
            match payload.get("type").and_then(|v| v.as_str()).unwrap_or("") {
                "request" => {
                    ctx.info(format!(
                        "agent relay forwarding http request path={} target_base_url={}",
                        payload.get("path").and_then(|v| v.as_str()).unwrap_or("/"),
                        payload
                            .get("target_base_url")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&config.opcode_base_url)
                    ));
                    let response = relay_local_http(&config.opcode_base_url, &payload).await;
                    append_debug_log(&format!(
                        "http response status={} body_base64_len={}",
                        response
                            .get("status_code")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(500),
                        response
                            .get("body_base64")
                            .and_then(|v| v.as_str())
                            .map(|v| v.len())
                            .unwrap_or(0)
                    ));
                    let envelope = json!({
                        "type": "response",
                        "request_id": payload.get("request_id").cloned().unwrap_or(serde_json::Value::Null),
                        "status_code": response.get("status_code").cloned().unwrap_or(json!(500)),
                        "message": response.get("message").cloned().unwrap_or(json!("error")),
                        "headers": response.get("headers").cloned().unwrap_or_else(|| json!({})),
                        "body_base64": response.get("body_base64").cloned().unwrap_or(serde_json::Value::Null),
                        "body": response.get("body").cloned().unwrap_or(serde_json::Value::Null),
                    });
                    ctx.info(format!(
                        "agent relay sending http response status={} body_base64_len={}",
                        response
                            .get("status_code")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(500),
                        response
                            .get("body_base64")
                            .and_then(|v| v.as_str())
                            .map(|v| v.len())
                            .unwrap_or(0)
                    ));
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
                    let target_base_url = payload
                        .get("target_base_url")
                        .and_then(|v| v.as_str())
                        .map(str::to_string);
                    let websocket_path = payload
                        .get("websocket_path")
                        .and_then(|v| v.as_str())
                        .map(str::to_string);
                    let headers = payload
                        .get("headers")
                        .and_then(|value| value.as_object())
                        .map(|values| {
                            values
                                .iter()
                                .filter_map(|(key, value)| {
                                    value.as_str().map(|value| (key.clone(), value.to_string()))
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    let subprotocols = payload
                        .get("subprotocols")
                        .and_then(|value| value.as_array())
                        .map(|values| {
                            values
                                .iter()
                                .filter_map(|value| value.as_str().map(str::to_string))
                                .collect()
                        })
                        .unwrap_or_default();
                    let sender = out_tx.clone();
                    let opcode_base = config.opcode_base_url.clone();
                    tokio::spawn(async move {
                        open_local_ws_tunnel(
                            opcode_base,
                            tunnel_id,
                            session_id,
                            target_base_url,
                            websocket_path,
                            headers,
                            subprotocols,
                            sender,
                        )
                        .await;
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
                                .send(Message::Binary(
                                    STANDARD.decode(encoded).map_err(|e| anyhow!(e))?.into(),
                                ))
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
        append_debug_log(&format!("loop end reconnect={reconnect}"));
        if !reconnect {
            ctx.info("agent relay connection closed; reconnecting");
        }
        sleep(Duration::from_secs(2)).await;
    }
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
            let pairing_code = match args.code {
                Some(code) => code,
                None => start_pairing(ctx).await?,
            };
            complete_pairing(ctx, &pairing_code, &config).await?;
            save_agent_config(&config)?;
            ctx.success(format!("Paired device {}", config.device_id));
            Ok(())
        }
        AgentCommand::Project { command } => match command {
            AgentProjectCommand::Create(args) => {
                let mut config = load_agent_config()?;
                let home_root = config.home_root.clone().ok_or_else(|| {
                    anyhow!("agent home is not initialized; run `d1v agent init-home` first")
                })?;
                let path = args
                    .path
                    .unwrap_or_else(|| PathBuf::from(&home_root).join("projects").join(&args.name));
                fs::create_dir_all(&path)?;
                init_workspace_binding(ctx, &args.project_id, &path, Some(args.name.clone()))
                    .await?;
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
            let terminal_bootstrap = fetch_runtime_terminal_bootstrap(ctx).await;
            let _child = maybe_spawn_opcode(
                ctx,
                &config,
                args.opcode_bin.as_deref(),
                &base_url(ctx),
                &terminal_bootstrap,
            )
            .await?;
            let token = ctx
                .tokens
                .lookup()?
                .ok_or_else(|| anyhow!("missing auth token"))?;
            let _ = register_customer_runtime_node(ctx, &config).await;
            let _ = ensure_customer_auto_expose(ctx, &config).await;
            let heartbeat_device_id = config.device_id.clone();
            let heartbeat_base_url = base_url(ctx);
            let heartbeat_token = token.expose_secret().to_string();
            let expose_device_id = config.device_id.clone();
            let expose_base_url = heartbeat_base_url.clone();
            let expose_token = heartbeat_token.clone();
            let expose_opcode_base_url = config.opcode_base_url.clone();
            tokio::spawn(async move {
                let _ = heartbeat_customer_runtime_node_loop(
                    heartbeat_base_url,
                    heartbeat_token,
                    heartbeat_device_id,
                )
                .await;
            });
            tokio::spawn(async move {
                let _ = ensure_customer_auto_expose_loop(
                    expose_base_url,
                    expose_token,
                    expose_device_id,
                    expose_opcode_base_url,
                )
                .await;
            });
            run_agent_relay_forever(ctx, config, token.expose_secret().to_string()).await
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
                    .line(Line::styled(
                        "Agent configuration".to_string(),
                        theme::ansi::success(),
                    ))
                    .line(Line::raw(format!("  Device ID: {}", config.device_id)))
                    .line(Line::raw(format!("  Device Name: {}", config.device_name)))
                    .line(Line::raw(format!("  Home Root: {}", home_root)))
                    .line(Line::raw(format!(
                        "  Opcode Base URL: {}",
                        config.opcode_base_url
                    )))
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

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::sync::{Arc, Mutex as StdMutex};

    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_hdr_async;
    use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};

    use super::*;

    fn test_agent_config() -> AgentConfig {
        AgentConfig {
            device_id: "device-1".to_string(),
            device_name: "Test".to_string(),
            home_root: Some("/tmp/d1v".to_string()),
            opcode_base_url: DEFAULT_OPCODE_BASE.to_string(),
            project_bindings: vec![],
        }
    }

    fn command_env(command: &Command, name: &str) -> Option<String> {
        command
            .get_envs()
            .find(|(key, _)| *key == OsStr::new(name))
            .and_then(|(_, value)| value)
            .map(|value| value.to_string_lossy().to_string())
    }

    #[test]
    fn parse_listening_ports_extracts_unique_ports() {
        let input = "\
127.0.0.1:3000\n\
*:5173\n\
localhost:3000\n\
[::1]:8787\n";
        assert_eq!(parse_listening_ports(input), vec![3000, 5173, 8787]);
    }

    #[test]
    fn configured_auto_expose_candidate_ports_uses_env_override() {
        unsafe {
            env::set_var("D1V_RUNTIME_AUTO_EXPOSE_CANDIDATE_PORTS", "3000,5173,3000");
        }
        assert_eq!(configured_auto_expose_candidate_ports(), vec![3000, 5173]);
        unsafe {
            env::remove_var("D1V_RUNTIME_AUTO_EXPOSE_CANDIDATE_PORTS");
        }
    }

    #[test]
    fn terminal_bootstrap_requires_complete_public_configuration() {
        let valid = RuntimeTerminalBootstrap {
            enabled: true,
            public_key: Some(" public\\nkey ".to_string()),
            issuer: "issuer".to_string(),
            audience: "audience".to_string(),
        }
        .validated();
        assert!(valid.enabled);
        assert_eq!(valid.public_key.as_deref(), Some("public\nkey"));

        let incomplete = RuntimeTerminalBootstrap {
            enabled: true,
            public_key: None,
            issuer: "issuer".to_string(),
            audience: "audience".to_string(),
        }
        .validated();
        assert!(!incomplete.enabled);
    }

    #[test]
    fn terminal_bootstrap_configures_managed_runtime_environment() {
        let config = test_agent_config();
        let bootstrap = RuntimeTerminalBootstrap {
            enabled: true,
            public_key: Some("public-key".to_string()),
            issuer: "issuer".to_string(),
            audience: "audience".to_string(),
        };
        let mut command = Command::new("opcode-api");

        configure_terminal_command(&mut command, &config, &bootstrap);

        assert_eq!(
            command_env(&command, "OPCODE_RUNTIME_NODE_ID").as_deref(),
            Some("customer-device-1")
        );
        assert_eq!(
            command_env(&command, "D1V_SHELL_ENABLED").as_deref(),
            Some("1")
        );
        assert_eq!(
            command_env(&command, "D1V_SHELL_TICKET_PUBLIC_KEY").as_deref(),
            Some("public-key")
        );
        assert_eq!(
            command_env(&command, "D1V_SHELL_TICKET_ISSUER").as_deref(),
            Some("issuer")
        );
        assert_eq!(
            command_env(&command, "D1V_SHELL_TICKET_AUDIENCE").as_deref(),
            Some("audience")
        );
    }

    #[test]
    fn runtime_registration_only_advertises_verified_terminal_capability() {
        let config = test_agent_config();
        let disabled = runtime_node_capabilities(&config, &OpcodeRuntimeCapabilities::default());
        assert_eq!(disabled["device_id"], "device-1");
        assert!(disabled.get("supports_terminal").is_none());
        assert!(disabled.get("terminal_base_url").is_none());

        let external = runtime_node_capabilities(
            &config,
            &OpcodeRuntimeCapabilities {
                supports_terminal: true,
                terminal_base_url: Some("https://example.com".to_string()),
            },
        );
        assert!(external.get("supports_terminal").is_none());

        let enabled = runtime_node_capabilities(
            &config,
            &OpcodeRuntimeCapabilities {
                supports_terminal: true,
                terminal_base_url: Some("http://127.0.0.1:9191/".to_string()),
            },
        );
        assert_eq!(enabled["supports_terminal"], true);
        assert_eq!(enabled["terminal_base_url"], "http://127.0.0.1:9191");
    }

    #[tokio::test]
    async fn local_tunnel_forwards_restricted_terminal_handshake() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let saw_handshake = Arc::new(StdMutex::new(false));
        let server_handshake = saw_handshake.clone();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket =
                accept_hdr_async(stream, move |request: &Request, mut response: Response| {
                    assert_eq!(request.uri().path(), "/ws/terminal/sh-local");
                    assert!(request.uri().query().is_none());
                    assert_eq!(request.headers().get(ORIGIN).unwrap(), "https://www.d1v.ai");
                    assert_eq!(
                        request.headers().get("x-d1v-shell-ticket").unwrap(),
                        "secret-ticket"
                    );
                    assert!(request.headers().get("authorization").is_none());
                    assert_eq!(
                        request.headers().get(SEC_WEBSOCKET_PROTOCOL).unwrap(),
                        "d1v-terminal.v1"
                    );
                    response.headers_mut().insert(
                        SEC_WEBSOCKET_PROTOCOL,
                        HeaderValue::from_static("d1v-terminal.v1"),
                    );
                    *server_handshake.lock().unwrap() = true;
                    Ok(response)
                })
                .await
                .unwrap();
            socket.close(None).await.unwrap();
        });
        let (sender, mut receiver) = mpsc::unbounded_channel();

        open_local_ws_tunnel(
            "http://127.0.0.1:9191".into(),
            "tunnel-local".into(),
            "sh-local".into(),
            Some(format!("http://{address}")),
            Some("/ws/terminal/sh-local".into()),
            HashMap::from([
                ("Origin".into(), "https://www.d1v.ai".into()),
                ("X-D1V-Shell-Ticket".into(), "secret-ticket".into()),
                ("Authorization".into(), "must-not-forward".into()),
            ]),
            vec!["d1v-terminal.v1".into(), "must-not-forward".into()],
            sender,
        )
        .await;
        server.await.unwrap();

        let mut events = Vec::new();
        while let Ok(message) = receiver.try_recv() {
            if let Message::Text(text) = message {
                events.push(serde_json::from_str::<serde_json::Value>(&text).unwrap());
            }
        }
        assert!(*saw_handshake.lock().unwrap());
        assert_eq!(events[0]["event"], "open");
        assert_eq!(events.last().unwrap()["event"], "close");
        assert!(!TUNNELS.lock().await.contains_key("tunnel-local"));
    }

    #[tokio::test]
    async fn local_tunnel_rejects_non_loopback_terminal_target() {
        let (sender, mut receiver) = mpsc::unbounded_channel();

        open_local_ws_tunnel(
            "http://127.0.0.1:9191".into(),
            "tunnel-external".into(),
            "sh-external".into(),
            Some("https://example.com".into()),
            Some("/ws/terminal/sh-external".into()),
            HashMap::new(),
            vec!["d1v-terminal.v1".into()],
            sender,
        )
        .await;

        let Message::Text(event) = receiver.try_recv().unwrap() else {
            panic!("expected text error event");
        };
        let payload: serde_json::Value = serde_json::from_str(&event).unwrap();
        assert_eq!(payload["event"], "error");
        assert_eq!(payload["detail"], "terminal_target_must_be_loopback");
        assert!(!TUNNELS.lock().await.contains_key("tunnel-external"));
    }
}
