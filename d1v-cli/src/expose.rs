use anyhow::anyhow;
use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};

use crate::Context;
use crate::error::Result;
use crate::text::{Field, Fields, Line, Render, RenderContext, Span, Table, TableRow, Text};
use crate::theme;

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
    let binding = request_open(ExposePortArgs {
        port,
        project_id: args.project_id,
        hostname: args.hostname,
        node_id: args.node_id,
        host_port: args.host_port,
    })
    .await?;
    ctx.present(
        ExposeBindingView { binding: &binding },
        &ExposeEnvelope {
            accepted: true,
            binding: &binding,
        },
    )
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
