use anyhow::anyhow;
use clap::{Args, Subcommand};
use d1v_api::{CreateReleaseRequest, ProductionRelease, ReleaseEnvironmentDecision};
use d1v_api::{
    DeploymentInfo, DeploymentListOptions, DeploymentListResponse, DeploymentLogsResponse,
    DeploymentResponse,
};
use serde::Serialize;
use std::io::{self, IsTerminal};
use std::time::Duration;

use crate::Context;
use crate::error::Result;
use crate::text::{Field, Fields, Line, Span, Table, TableRow, Text};
use crate::theme;
use crate::ui::{Confirm, Select, SelectOption};

pub async fn wait_for_preview(ctx: &Context, project_id: &str) -> Result<DeploymentResponse> {
    let started = ctx.client.deployment().preview(project_id).await?;
    let deployment_id = started.deployment_id.clone();
    for _ in 0..300 {
        let status = ctx.client.deployment().preview_status(project_id).await?;
        let message = status.message.to_ascii_uppercase();
        if message.contains("READY") && !message.contains("NOT_READY") {
            let url = status
                .production_url
                .clone()
                .or(status.vercel_url.clone())
                .or(started.production_url.clone())
                .or(started.vercel_url.clone());
            let mut result = status;
            result.deployment_id = result.deployment_id.or(deployment_id.clone());
            result.production_url = url;
            return Ok(result);
        }
        if ["ERROR", "FAILED", "CANCELED"]
            .iter()
            .any(|s| message.contains(s))
        {
            return Err(anyhow!("preview deployment failed: {}", status.message).into());
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    Err(anyhow!(
        "preview deployment timed out after 10 minutes (deployment ID: {})",
        deployment_id.as_deref().unwrap_or("unknown")
    )
    .into())
}

pub async fn production_release(ctx: &Context, project_id: &str) -> Result<ProductionRelease> {
    if !io::stdin().is_terminal() {
        return Err(anyhow!(
            "production release requires an interactive TTY; rerun d1v deploy prod in a terminal"
        )
        .into());
    }
    let preflight = ctx
        .client
        .deployment()
        .get_release_preflight(project_id)
        .await?;
    ctx.info(format!(
        "Release preflight: first_release={} env_vars={} {}",
        preflight.first_release.unwrap_or(false),
        preflight.environment_variables.len(),
        preflight.mode.as_deref().unwrap_or("")
    ));
    let mut decisions = Vec::new();
    for variable in &preflight.environment_variables {
        if variable.needs_value.unwrap_or(false)
            || (variable.has_development_value == Some(true)
                && variable.has_production_value != Some(true))
        {
            let choice = Select::new(format!("Production value for {}", variable.key))
                .options([
                    SelectOption::new("development", "Reuse development value"),
                    SelectOption::new("production", "Use existing production value"),
                    SelectOption::new("skip", "Do not copy"),
                ])
                .prompt()?;
            let action = match choice.to_string().as_str() {
                "development" => "reuse_dev",
                "production" => "use_prod_existing",
                _ => "omit",
            };
            decisions.push(ReleaseEnvironmentDecision {
                key: variable.key.clone(),
                action: action.to_string(),
                value: None,
            });
        }
    }
    if !Confirm::new(format!("Confirm production release for {project_id}?"))
        .default(false)
        .prompt()?
    {
        return Err(anyhow!("production release canceled").into());
    }
    let key = format!("d1v-{}-{}", std::process::id(), jiff::Timestamp::now());
    let release = ctx
        .client
        .deployment()
        .create_production_release(
            project_id,
            &CreateReleaseRequest {
                idempotency_key: key,
                expected_dev_commit_sha: None,
                confirm_managed_reuse: true,
                copy_development_data: false,
                environment_decisions: decisions,
            },
        )
        .await?;
    let release_id = release
        .id
        .clone()
        .ok_or_else(|| anyhow!("release response did not include an id"))?;
    for _ in 0..300 {
        let status = match ctx
            .client
            .deployment()
            .get_production_release(project_id, &release_id)
            .await
        {
            Ok(status) => status,
            Err(error) if error.is_timeout() || error.is_network() => {
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        match status.status.to_ascii_lowercase().as_str() {
            "succeeded" | "success" | "ready" => return Ok(status),
            "failed" | "error" | "canceled" | "cancelled" => {
                return Err(anyhow!(
                    "production release failed (phase={}, code={}): {}",
                    status.phase.as_deref().unwrap_or("-"),
                    status.error_code.as_deref().unwrap_or("-"),
                    status.error_message.as_deref().unwrap_or("unknown error")
                )
                .into());
            }
            _ => tokio::time::sleep(Duration::from_secs(2)).await,
        }
    }
    Err(anyhow!("production release timed out after 10 minutes (release ID: {release_id})").into())
}

#[derive(Subcommand)]
pub enum DeployCommand {
    /// Trigger a preview deployment
    Preview(ProjectDeployArgs),
    /// Trigger a production deployment
    Prod(ProjectDeployArgs),
    /// Show current deployment status summary
    Status(StatusArgs),
    /// Show deployment history
    History(HistoryArgs),
    /// Show build logs for a Vercel deployment ID
    Logs(LogsArgs),
}

#[derive(Args)]
pub struct ProjectDeployArgs {
    pub project_id: String,
}

#[derive(Args)]
pub struct StatusArgs {
    pub project_id: String,
}

#[derive(Args)]
pub struct HistoryArgs {
    pub project_id: String,
    #[arg(long)]
    pub environment: Option<String>,
    #[arg(long, default_value_t = 20)]
    pub limit: u32,
}

#[derive(Args)]
pub struct LogsArgs {
    pub vercel_deployment_id: String,
}

#[derive(Debug, Serialize)]
struct DeployResponseJson<'a> {
    deployment: &'a DeploymentResponse,
}

#[derive(Debug, Serialize)]
struct DeployHistoryJson<'a> {
    history: &'a DeploymentListResponse,
}

#[derive(Debug, Serialize)]
struct DeployLogsJson<'a> {
    logs: &'a DeploymentLogsResponse,
}

struct DeployResponseView<'a> {
    title: &'a str,
    deployment: &'a DeploymentResponse,
}

impl crate::text::Render for DeployResponseView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        Text::new()
            .line(Line::styled(self.title.to_string(), theme::ansi::success()))
            .render(ctx)?;
        Fields::new([
            field(
                "Success",
                if self.deployment.success {
                    "true"
                } else {
                    "false"
                },
            ),
            field("Message", &self.deployment.message),
            field_opt("Production URL", self.deployment.production_url.as_deref()),
            field_opt("Vercel URL", self.deployment.vercel_url.as_deref()),
            field_opt("Commit", self.deployment.commit_hash.as_deref()),
            field_opt("Deployment ID", self.deployment.deployment_id.as_deref()),
        ])
        .indent(2)
        .render(ctx)
    }
}

struct DeployHistoryView<'a> {
    deployments: &'a [DeploymentInfo],
}

impl crate::text::Render for DeployHistoryView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        if self.deployments.is_empty() {
            return Text::new()
                .line(Line::styled("No deployments found.", theme::ansi::dim()))
                .render(ctx);
        }

        let rows = self.deployments.iter().map(|item| {
            TableRow::new([
                item.id.clone(),
                item.environment.clone(),
                item.status.clone(),
                item.git_branch.clone().unwrap_or_else(|| "-".to_string()),
                item.created_at.clone(),
                item.url.clone(),
            ])
        });

        Table::new(rows)
            .header(TableRow::new([
                "id",
                "env",
                "status",
                "branch",
                "created_at",
                "url",
            ]))
            .border_style(theme::ansi::border())
            .render(ctx)
    }
}

struct DeployLogsView<'a> {
    logs: &'a DeploymentLogsResponse,
}

impl crate::text::Render for DeployLogsView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        Text::new()
            .line(Line::styled(
                "Build logs".to_string(),
                theme::ansi::success(),
            ))
            .line(Line::styled(
                format!("from_cache={}", self.logs.from_cache),
                theme::ansi::dim(),
            ))
            .line("")
            .line(self.logs.build_log.clone())
            .render(ctx)
    }
}

fn field(label: &'static str, value: &str) -> Field {
    Field::new(
        Span::styled(label, theme::ansi::label()),
        Line::styled(value.to_string(), theme::ansi::value()),
    )
}

fn field_opt(label: &'static str, value: Option<&str>) -> Field {
    field(label, value.unwrap_or("-"))
}

pub async fn run(ctx: &Context, command: DeployCommand) -> Result<()> {
    match command {
        DeployCommand::Preview(args) => {
            let deployment = wait_for_preview(ctx, &args.project_id).await?;
            ctx.success(format!(
                "Preview deployment requested for {}",
                args.project_id
            ));
            ctx.present(
                DeployResponseView {
                    title: "Preview deployment",
                    deployment: &deployment,
                },
                &DeployResponseJson {
                    deployment: &deployment,
                },
            )
        }
        DeployCommand::Prod(args) => {
            let release = production_release(ctx, &args.project_id).await?;
            let deployment = DeploymentResponse {
                success: true,
                message: release.status.clone(),
                commit_hash: None,
                production_url: release.production_url.clone(),
                vercel_url: None,
                deployment_id: release.deployment_id.clone().or(release.id.clone()),
            };
            ctx.success(format!(
                "Production deployment requested for {}",
                args.project_id
            ));
            ctx.present(
                DeployResponseView {
                    title: "Production deployment",
                    deployment: &deployment,
                },
                &DeployResponseJson {
                    deployment: &deployment,
                },
            )
        }
        DeployCommand::Status(args) => {
            let preview = ctx
                .client
                .deployment()
                .preview_status(&args.project_id)
                .await?;
            let history = ctx
                .client
                .deployment()
                .history(
                    &args.project_id,
                    &DeploymentListOptions {
                        environment: None,
                        limit: Some(1),
                    },
                )
                .await?;

            ctx.present(
                Text::new()
                    .line(Line::styled(
                        "Deployment status".to_string(),
                        theme::ansi::success(),
                    ))
                    .line("")
                    .line("Preview")
                    .line(format!(
                        "  success={} message={} url={}",
                        preview.success,
                        preview.message,
                        preview
                            .production_url
                            .as_deref()
                            .or(preview.vercel_url.as_deref())
                            .unwrap_or("-")
                    ))
                    .line("")
                    .line("Latest history")
                    .lines(if history.deployments.is_empty() {
                        vec!["  -".to_string()]
                    } else {
                        history
                            .deployments
                            .iter()
                            .take(1)
                            .map(|item| {
                                format!(
                                    "  {} {} {} {}",
                                    item.environment, item.status, item.created_at, item.url
                                )
                            })
                            .collect()
                    }),
                &serde_json::json!({
                    "preview": preview,
                    "latest": history.deployments.first(),
                }),
            )
        }
        DeployCommand::History(args) => {
            let history = ctx
                .client
                .deployment()
                .history(
                    &args.project_id,
                    &DeploymentListOptions {
                        environment: args.environment,
                        limit: Some(args.limit),
                    },
                )
                .await?;
            ctx.present(
                DeployHistoryView {
                    deployments: &history.deployments,
                },
                &DeployHistoryJson { history: &history },
            )
        }
        DeployCommand::Logs(args) => {
            let logs = ctx
                .client
                .deployment()
                .logs(&args.vercel_deployment_id)
                .await?;
            ctx.present(
                DeployLogsView { logs: &logs },
                &DeployLogsJson { logs: &logs },
            )
        }
    }
}
