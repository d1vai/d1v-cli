use clap::{Args, Subcommand};
use d1v_api::{
    DeploymentInfo, DeploymentListOptions, DeploymentListResponse, DeploymentLogsResponse,
    DeploymentResponse,
};
use serde::Serialize;

use crate::Context;
use crate::error::Result;
use crate::text::{Field, Fields, Line, Span, Table, TableRow, Text};
use crate::theme;

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
            let deployment = ctx.client.deployment().preview(&args.project_id).await?;
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
            let deployment = ctx.client.deployment().production(&args.project_id).await?;
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
