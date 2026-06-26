use anyhow::anyhow;
use clap::{Args, Subcommand};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::task::JoinHandle;

use crate::Context;
use crate::agent;
use crate::error::Result;
use crate::text::{Field, Fields, Line, Render, RenderContext, Span, Table, TableRow, Text};
use crate::theme;
use crate::token::TokenSource;

const DEFAULT_RUNTIME_AGENT_BASE: &str = "http://127.0.0.1:8080";

#[derive(Subcommand)]
pub enum ExposeCommand {
    /// List active expose bindings
    List(ExposeListArgs),
    /// Close an expose binding
    Close(ExposeCloseArgs),
}

#[derive(Args)]
pub struct ExposeArgs {
    /// Expose a local port through the active runtime agent
    pub port: Option<u16>,
    #[command(subcommand)]
    pub command: Option<ExposeCommand>,
    #[arg(long)]
    pub project_id: Option<String>,
    #[arg(long)]
    pub hostname: Option<String>,
    #[arg(long)]
    pub node_id: Option<String>,
    #[arg(long)]
    pub host_port: Option<u16>,
}

#[derive(Args)]
pub struct ExposePortArgs {
    pub port: u16,
    #[arg(long)]
    pub project_id: Option<String>,
    #[arg(long)]
    pub hostname: Option<String>,
    #[arg(long)]
    pub node_id: Option<String>,
    #[arg(long)]
    pub host_port: Option<u16>,
}

#[derive(Args)]
pub struct ExposeListArgs {
    #[arg(long)]
    pub node_id: Option<String>,
    #[arg(long)]
    pub project_id: Option<String>,
    #[arg(long)]
    pub status: Option<String>,
}

#[derive(Args)]
pub struct ExposeCloseArgs {
    pub binding_id: String,
    #[arg(long)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposeBinding {
    pub binding_id: String,
    pub node_id: String,
    pub lease_id: Option<String>,
    pub project_id: Option<String>,
    pub container_id: Option<String>,
    pub container_port: u16,
    pub host_port: Option<u16>,
    pub protocol: String,
    pub hostname: String,
    pub public_url: String,
    pub status: String,
    pub origin_mode: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Serialize)]
struct ExposeEnvelope<'a> {
    accepted: bool,
    binding: &'a ExposeBinding,
}

#[derive(Debug, Serialize)]
struct ExposeBindingsEnvelope<'a> {
    bindings: &'a [ExposeBinding],
}

struct ExposeBindingView<'a> {
    binding: &'a ExposeBinding,
}

impl Render for ExposeBindingView<'_> {
    fn render(&self, ctx: &mut RenderContext<'_>) -> std::io::Result<()> {
        Text::new()
            .line(
                Line::styled("Expose binding".to_string(), theme::ansi::success())
                    .push_plain(" ")
                    .push_styled(self.binding.binding_id.clone(), theme::ansi::plain()),
            )
            .render(ctx)?;
        Fields::new([
            field("Node", &self.binding.node_id),
            field_opt("Project", self.binding.project_id.as_deref()),
            field_opt("Origin Mode", self.binding.origin_mode.as_deref()),
            field("Hostname", &self.binding.hostname),
            field("URL", &self.binding.public_url),
            field("Target Port", &self.binding.container_port.to_string()),
            field_opt(
                "Host Port",
                self.binding
                    .host_port
                    .as_ref()
                    .map(ToString::to_string)
                    .as_deref(),
            ),
            field("Status", &self.binding.status),
        ])
        .indent(2)
        .render(ctx)
    }
}

struct ExposeBindingsView<'a> {
    bindings: &'a [ExposeBinding],
}

impl Render for ExposeBindingsView<'_> {
    fn render(&self, ctx: &mut RenderContext<'_>) -> std::io::Result<()> {
        if self.bindings.is_empty() {
            return Text::new()
                .line(Line::styled(
                    "No expose bindings found.",
                    theme::ansi::dim(),
                ))
                .render(ctx);
        }

        Table::new(self.bindings.iter().map(|binding| {
            TableRow::new([
                binding.binding_id.clone(),
                binding.node_id.clone(),
                binding.hostname.clone(),
                binding.public_url.clone(),
                binding.status.clone(),
            ])
        }))
        .header(TableRow::new([
            "binding_id",
            "node",
            "hostname",
            "url",
            "status",
        ]))
        .border_style(theme::ansi::border())
        .render(ctx)
    }
}

pub async fn run(ctx: &Context, args: ExposeArgs) -> Result<()> {
    if let Some(command) = args.command {
        return match command {
            ExposeCommand::List(list_args) => {
                let bindings = request_list_customer(ctx, list_args).await?;
                ctx.present(
                    ExposeBindingsView {
                        bindings: &bindings,
                    },
                    &ExposeBindingsEnvelope {
                        bindings: &bindings,
                    },
                )
            }
            ExposeCommand::Close(close_args) => {
                let binding = close_customer_expose(ctx, &close_args.binding_id).await?;
                ctx.present(
                    ExposeBindingView { binding: &binding },
                    &ExposeEnvelope {
                        accepted: true,
                        binding: &binding,
                    },
                )
            }
        };
    }

    let port = args.port.ok_or_else(|| anyhow!("missing expose port"))?;
    request_open_cli_relay(ctx, port, args.project_id, args.hostname).await
}

pub async fn run_node_mode(ctx: &Context, args: ExposeArgs) -> Result<()> {
    if let Some(command) = args.command {
        return match command {
            ExposeCommand::List(list_args) => {
                let bindings = request_list(list_args).await?;
                ctx.present(
                    ExposeBindingsView {
                        bindings: &bindings,
                    },
                    &ExposeBindingsEnvelope {
                        bindings: &bindings,
                    },
                )
            }
            ExposeCommand::Close(close_args) => {
                let binding = request_close(close_args).await?;
                ctx.present(
                    ExposeBindingView { binding: &binding },
                    &ExposeEnvelope {
                        accepted: true,
                        binding: &binding,
                    },
                )
            }
        };
    }

    let port = args.port.ok_or_else(|| anyhow!("missing expose port"))?;
    let fallback_project_id = args.project_id.clone();
    let fallback_hostname = args.hostname.clone();
    let open_args = ExposePortArgs {
        port,
        project_id: args.project_id,
        hostname: args.hostname,
        node_id: args.node_id,
        host_port: args.host_port,
    };
    let binding = match request_open(open_args).await {
        Ok(binding) => binding,
        Err(err) if should_fallback_to_cli_relay(&err) => {
            return request_open_cli_relay(ctx, port, fallback_project_id, fallback_hostname).await;
        }
        Err(err) => return Err(err),
    };
    ctx.present(
        ExposeBindingView { binding: &binding },
        &ExposeEnvelope {
            accepted: true,
            binding: &binding,
        },
    )
}

async fn request_open_cli_relay(
    ctx: &Context,
    port: u16,
    project_id: Option<String>,
    hostname: Option<String>,
) -> Result<()> {
    let token = ctx
        .tokens
        .lookup()?
        .ok_or_else(|| anyhow!("missing auth token; run `d1v login` first"))?;
    let token = token.expose_secret().to_string();
    let base_config = agent::load_agent_config()?;
    let mut config = base_config.clone();
    config.device_id = cli_free_device_id(&base_config.device_id);
    config.device_name = default_cli_free_device_name();
    config.opcode_base_url = format!("http://127.0.0.1:{port}");

    ensure_cli_free_device(ctx, &config).await?;
    agent::register_customer_runtime_node(ctx, &config).await?;
    let binding = create_customer_expose(ctx, &config, port, project_id, hostname).await?;
    ctx.present(
        ExposeBindingView { binding: &binding },
        &ExposeEnvelope {
            accepted: true,
            binding: &binding,
        },
    )?;
    ctx.info("CLI relay is running in the foreground. Press Ctrl-C to close this expose binding.");

    let heartbeat = spawn_customer_heartbeat(ctx, &config, token.clone());
    tokio::select! {
        result = agent::run_agent_relay_forever(ctx, config, token) => {
            heartbeat.abort();
            result?;
        }
        _ = tokio::signal::ctrl_c() => {
            heartbeat.abort();
        }
    }
    let _ = close_customer_expose(ctx, &binding.binding_id).await;
    ctx.info(format!("Closed expose binding {}", binding.binding_id));
    Ok(())
}

async fn request_open(args: ExposePortArgs) -> Result<ExposeBinding> {
    let client = local_client()?;
    let mut payload = serde_json::json!({
        "container_port": args.port,
        "protocol": "http",
        "created_by": "d1v-cli",
    });
    if let Some(project_id) = args.project_id {
        payload["project_id"] = serde_json::Value::String(project_id);
    }
    if let Some(hostname) = args.hostname {
        payload["hostname"] = serde_json::Value::String(hostname);
    }
    if let Some(node_id) = args.node_id {
        payload["node_id"] = serde_json::Value::String(node_id);
    }
    if let Some(host_port) = args.host_port {
        payload["host_port"] = serde_json::Value::Number(host_port.into());
    }
    let value: serde_json::Value = client
        .post(control_url("/control/runtime/exposes"))
        .json(&payload)
        .send()
        .await
        .map_err(anyhow::Error::from)?
        .json()
        .await
        .map_err(anyhow::Error::from)?;
    parse_binding(value)
}

async fn create_customer_expose(
    ctx: &Context,
    config: &agent::AgentConfig,
    port: u16,
    project_id: Option<String>,
    hostname: Option<String>,
) -> Result<ExposeBinding> {
    let client = agent::authed_http_client(ctx).await?;
    let mut payload = json!({
        "node_id": format!("customer-{}", config.device_id),
        "container_port": port,
        "host_port": port,
        "protocol": "http",
        "container_id": config.device_id,
    });
    if let Some(project_id) = project_id {
        payload["project_id"] = serde_json::Value::String(project_id);
    }
    if let Some(hostname) = hostname {
        payload["hostname"] = serde_json::Value::String(hostname);
    }
    let value: serde_json::Value = client
        .post(format!(
            "{}/api/devices/runtime-node/exposes",
            agent::base_url(ctx)
        ))
        .json(&payload)
        .send()
        .await
        .map_err(anyhow::Error::from)?
        .json()
        .await
        .map_err(anyhow::Error::from)?;
    parse_binding(value)
}

async fn request_list_customer(ctx: &Context, args: ExposeListArgs) -> Result<Vec<ExposeBinding>> {
    let client = agent::authed_http_client(ctx).await?;
    let mut request = client.get(format!(
        "{}/api/devices/runtime-node/exposes",
        agent::base_url(ctx)
    ));
    if let Some(node_id) = args.node_id {
        request = request.query(&[("node_id", node_id)]);
    }
    let value: serde_json::Value = request
        .send()
        .await
        .map_err(anyhow::Error::from)?
        .json()
        .await
        .map_err(anyhow::Error::from)?;
    parse_bindings(value)
}

async fn close_customer_expose(ctx: &Context, binding_id: &str) -> Result<ExposeBinding> {
    let client = agent::authed_http_client(ctx).await?;
    let value: serde_json::Value = client
        .post(format!(
            "{}/api/devices/runtime-node/exposes/{}/close",
            agent::base_url(ctx),
            binding_id
        ))
        .send()
        .await
        .map_err(anyhow::Error::from)?
        .json()
        .await
        .map_err(anyhow::Error::from)?;
    parse_binding(value)
}

async fn ensure_cli_free_device(ctx: &Context, config: &agent::AgentConfig) -> Result<()> {
    let client = agent::authed_http_client(ctx).await?;
    let response = client
        .get(format!("{}/api/devices", agent::base_url(ctx)))
        .send()
        .await
        .map_err(anyhow::Error::from)?;
    if response.status().is_success() {
        let value: serde_json::Value = response.json().await.map_err(anyhow::Error::from)?;
        let exists = value
            .get("data")
            .and_then(|value| value.as_array())
            .map(|items| {
                items.iter().any(|item| {
                    item.get("device_id").and_then(|value| value.as_str())
                        == Some(config.device_id.as_str())
                })
            })
            .unwrap_or(false);
        if exists {
            return Ok(());
        }
    }

    let pairing_code = agent::start_pairing(ctx).await?;
    agent::complete_pairing(ctx, &pairing_code, config).await
}

fn spawn_customer_heartbeat(
    ctx: &Context,
    config: &agent::AgentConfig,
    token: String,
) -> JoinHandle<()> {
    let base_url = agent::base_url(ctx);
    let device_id = config.device_id.clone();
    tokio::spawn(async move {
        let _ = agent::heartbeat_customer_runtime_node_loop(base_url, token, device_id).await;
    })
}

fn should_fallback_to_cli_relay(err: &crate::error::Error) -> bool {
    let message = err.to_string();
    message.contains("Connection refused")
        || message.contains("error sending request")
        || message.contains("tcp connect error")
        || message.contains("operation timed out")
}

async fn request_list(args: ExposeListArgs) -> Result<Vec<ExposeBinding>> {
    let client = local_client()?;
    let mut request = client.get(control_url("/control/runtime/exposes"));
    if let Some(node_id) = args.node_id {
        request = request.query(&[("node_id", node_id)]);
    }
    if let Some(project_id) = args.project_id {
        request = request.query(&[("project_id", project_id)]);
    }
    if let Some(status) = args.status {
        request = request.query(&[("status", status)]);
    }
    let value: serde_json::Value = request
        .send()
        .await
        .map_err(anyhow::Error::from)?
        .json()
        .await
        .map_err(anyhow::Error::from)?;
    parse_bindings(value)
}

async fn request_close(args: ExposeCloseArgs) -> Result<ExposeBinding> {
    let client = local_client()?;
    let value: serde_json::Value = client
        .post(control_url(&format!(
            "/control/runtime/exposes/{}/close",
            args.binding_id
        )))
        .json(&serde_json::json!({ "reason": args.reason }))
        .send()
        .await
        .map_err(anyhow::Error::from)?
        .json()
        .await
        .map_err(anyhow::Error::from)?;
    parse_binding(value)
}

fn local_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(anyhow::Error::from)
        .map_err(Into::into)
}

fn runtime_agent_base_url() -> String {
    std::env::var("D1V_RUNTIME_AGENT_BASE")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_RUNTIME_AGENT_BASE.to_string())
        .trim_end_matches('/')
        .to_string()
}

fn control_url(path: &str) -> String {
    format!("{}{}", runtime_agent_base_url(), path)
}

fn parse_binding(value: serde_json::Value) -> Result<ExposeBinding> {
    let data = value
        .get("data")
        .ok_or_else(|| anyhow!("missing expose response data"))?;
    serde_json::from_value(data.clone())
        .map_err(anyhow::Error::from)
        .map_err(Into::into)
}

fn uuid_like() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("{:x}", nanos)
}

fn default_cli_free_device_name() -> String {
    std::env::var("HOSTNAME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!("CLI free expose ({value})"))
        .unwrap_or_else(|| "CLI free expose".to_string())
}

fn cli_free_device_id(base_device_id: &str) -> String {
    let raw = str::trim(base_device_id);
    if raw.starts_with("cli-free-") {
        return raw.to_string();
    }
    let suffix = raw
        .strip_prefix("dev-")
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(raw);
    let sanitized: String = suffix
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' {
                ch
            } else {
                '-'
            }
        })
        .collect();
    let sanitized = sanitized.trim_matches('-');
    if sanitized.is_empty() {
        return format!("cli-free-{}", uuid_like());
    }
    format!("cli-free-{sanitized}")
}

fn parse_bindings(value: serde_json::Value) -> Result<Vec<ExposeBinding>> {
    let data = value
        .get("data")
        .ok_or_else(|| anyhow!("missing expose response data"))?;
    serde_json::from_value(data.clone())
        .map_err(anyhow::Error::from)
        .map_err(Into::into)
}

fn field(label: &'static str, value: &str) -> Field {
    Field::new(
        Span::styled(label.to_string(), theme::ansi::label()),
        Line::styled(value.to_string(), theme::ansi::value()),
    )
}

fn field_opt(label: &'static str, value: Option<&str>) -> Field {
    field(label, value.unwrap_or("-"))
}
