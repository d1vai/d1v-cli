use clap::{Args, Subcommand};
use d1v_api::{
    GitHubAppInstallation, GitHubAppRepository, GitHubAppStatus, GitHubImportAutoDeploy,
    GitHubImportRequest, GitHubImportResponse,
};
use serde::Serialize;

use crate::Context;
use crate::error::Result;
use crate::text::{Field, Fields, Line, Span, Table, TableRow, Text};
use crate::theme;

const DEFAULT_SETTINGS_URL: &str = "https://d1v.ai/setting?tab=github";

#[derive(Subcommand)]
pub enum GitHubCommand {
    /// Show GitHub App connection status
    Status,
    /// List GitHub App installations available to the current account
    Installations,
    /// Open the GitHub connect/install flow in browser
    Bind(BindArgs),
    /// List repositories for a GitHub App installation
    Repos(ReposArgs),
    /// Import a repository into d1v as a project
    Import(ImportArgs),
}

#[derive(Args)]
pub struct BindArgs {
    /// Open the GitHub App installation page instead of OAuth connect
    #[arg(long)]
    pub install: bool,
    /// Print the resolved URL without opening a browser
    #[arg(long)]
    pub print_only: bool,
    /// Optional redirect target for OAuth connect URL generation
    #[arg(long)]
    pub redirect_to: Option<String>,
}

#[derive(Args)]
pub struct ReposArgs {
    /// GitHub App installation ID
    #[arg(long)]
    pub installation_id: u64,
}

#[derive(Args)]
pub struct ImportArgs {
    /// GitHub App installation ID
    #[arg(long)]
    pub installation_id: u64,
    /// GitHub repository ID
    #[arg(long)]
    pub repository_id: u64,
    /// Optional d1v project name override
    #[arg(long)]
    pub project_name: Option<String>,
    /// Optional d1v project description override
    #[arg(long)]
    pub project_description: Option<String>,
}

#[derive(Debug, Serialize)]
struct GitHubStatusJson<'a> {
    status: &'a GitHubAppStatus,
}

#[derive(Debug, Serialize)]
struct GitHubInstallationsJson<'a> {
    installations: &'a [GitHubAppInstallation],
}

#[derive(Debug, Serialize)]
struct GitHubRepositoriesJson<'a> {
    repositories: &'a [GitHubAppRepository],
}

#[derive(Debug, Serialize)]
struct GitHubImportJson<'a> {
    result: &'a GitHubImportResponse,
}

struct GitHubStatusView<'a> {
    status: &'a GitHubAppStatus,
}

impl crate::text::Render for GitHubStatusView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        let headline = match (
            self.status.connected,
            self.status.token_valid,
            self.status.configured,
        ) {
            (true, true, _) => Line::styled("GitHub connected", theme::ansi::success()),
            (true, false, _) => {
                Line::styled("GitHub connected, token invalid", theme::ansi::warning())
            }
            (false, _, true) => Line::styled("GitHub not connected", theme::ansi::warning()),
            (false, _, false) => Line::styled("GitHub App not configured", theme::ansi::error()),
        };

        let fields = vec![
            bool_field("Configured", self.status.configured),
            bool_field("Connected", self.status.connected),
            bool_field("Token valid", self.status.token_valid),
            opt_field("GitHub login", self.status.github_login.as_deref()),
            opt_field("GitHub name", self.status.github_name.as_deref()),
            opt_field("App slug", self.status.app_slug.as_deref()),
            opt_field("Install URL", self.status.app_install_url.as_deref()),
            opt_field("Settings URL", Some(DEFAULT_SETTINGS_URL)),
        ];

        Text::new().line(headline).render(ctx)?;
        Fields::new(fields).indent(2).render(ctx)
    }
}

struct GitHubInstallationsView<'a> {
    installations: &'a [GitHubAppInstallation],
}

impl crate::text::Render for GitHubInstallationsView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        if self.installations.is_empty() {
            return Text::new()
                .line(Line::styled(
                    "No GitHub installations found.",
                    theme::ansi::dim(),
                ))
                .render(ctx);
        }

        let rows = self.installations.iter().map(|item| {
            TableRow::new([
                item.id.to_string(),
                item.account_login
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
                item.target_type.clone().unwrap_or_else(|| "-".to_string()),
                item.repository_selection
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
                item.html_url.clone().unwrap_or_else(|| "-".to_string()),
            ])
        });

        Table::new(rows)
            .header(TableRow::new([
                "id",
                "account",
                "target_type",
                "repo_selection",
                "html_url",
            ]))
            .border_style(theme::ansi::border())
            .render(ctx)
    }
}

struct GitHubRepositoriesView<'a> {
    repositories: &'a [GitHubAppRepository],
}

impl crate::text::Render for GitHubRepositoriesView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        if self.repositories.is_empty() {
            return Text::new()
                .line(Line::styled("No repositories found.", theme::ansi::dim()))
                .render(ctx);
        }

        let rows = self.repositories.iter().map(|item| {
            TableRow::new([
                item.id.to_string(),
                item.full_name.clone(),
                item.default_branch.clone(),
                if item.is_private { "private" } else { "public" }.to_string(),
                item.language.clone().unwrap_or_else(|| "-".to_string()),
            ])
        });

        Table::new(rows)
            .header(TableRow::new([
                "id",
                "full_name",
                "default_branch",
                "visibility",
                "language",
            ]))
            .border_style(theme::ansi::border())
            .render(ctx)
    }
}

struct GitHubImportView<'a> {
    result: &'a GitHubImportResponse,
}

impl crate::text::Render for GitHubImportView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        let project = &self.result.project;
        let headline = Line::styled("Imported repository".to_string(), theme::ansi::success())
            .push_plain(" ")
            .push_styled(
                format!("{} ({})", project.project_name, project.id),
                theme::ansi::plain(),
            );

        Text::new().line(headline).render(ctx)?;
        Fields::new([
            opt_field("Description", Some(project.project_description.as_str())),
            opt_field("Repository", project.repository_full_name.as_deref()),
            opt_field("Repo branch", project.repository_current_branch.as_deref()),
            opt_field("Preview URL", project.latest_preview_url.as_deref()),
            opt_field("Prod URL", project.latest_prod_deployment_url.as_deref()),
            bool_opt_field("Auto deploy", project.auto_deploy_on_execute),
        ])
        .indent(2)
        .render(ctx)?;

        if let Some(auto_deploy) = &self.result.import_auto_deploy {
            writeln!(ctx.writer)?;
            Text::new()
                .line(Line::styled(
                    "Import deployability".to_string(),
                    theme::ansi::success(),
                ))
                .render(ctx)?;
            Fields::new(render_auto_deploy_fields(auto_deploy))
                .indent(2)
                .render(ctx)?;
        }

        Ok(())
    }
}

fn bool_field(label: &'static str, value: bool) -> Field {
    Field::new(
        Span::styled(label, theme::ansi::label()),
        Line::styled(if value { "true" } else { "false" }, theme::ansi::value()),
    )
}

fn bool_opt_field(label: &'static str, value: Option<bool>) -> Field {
    let rendered = match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "-",
    };
    opt_field(label, Some(rendered))
}

fn opt_field(label: &'static str, value: Option<&str>) -> Field {
    Field::new(
        Span::styled(label, theme::ansi::label()),
        Line::styled(value.unwrap_or("-").to_string(), theme::ansi::value()),
    )
}

fn render_auto_deploy_fields(auto_deploy: &GitHubImportAutoDeploy) -> Vec<Field> {
    let monorepo_candidates = auto_deploy.monorepo_candidates.as_ref().map(|items| {
        if items.is_empty() {
            "-".to_string()
        } else {
            items.join(", ")
        }
    });

    vec![
        bool_opt_field("Deployable", auto_deploy.is_deployable),
        opt_field("Framework", auto_deploy.framework.as_deref()),
        bool_opt_field("Auto deploy queued", auto_deploy.auto_deploy_queued),
        opt_field("Reason", auto_deploy.reason.as_deref()),
        opt_field("Monorepo candidates", monorepo_candidates.as_deref()),
    ]
}

pub async fn run(ctx: &Context, command: GitHubCommand) -> Result<()> {
    match command {
        GitHubCommand::Status => {
            let status = ctx.client.github_app().status().await?;
            ctx.present(
                GitHubStatusView { status: &status },
                &GitHubStatusJson { status: &status },
            )
        }
        GitHubCommand::Installations => {
            let installations = ctx.client.github_app().list_installations().await?;
            ctx.present(
                GitHubInstallationsView {
                    installations: &installations,
                },
                &GitHubInstallationsJson {
                    installations: &installations,
                },
            )
        }
        GitHubCommand::Bind(args) => {
            let status = ctx.client.github_app().status().await?;
            let url = if args.install {
                status
                    .app_install_url
                    .clone()
                    .unwrap_or_else(|| DEFAULT_SETTINGS_URL.to_string())
            } else if status.connected && status.token_valid {
                status
                    .app_install_url
                    .clone()
                    .unwrap_or_else(|| DEFAULT_SETTINGS_URL.to_string())
            } else {
                ctx.client
                    .github_app()
                    .connect_url(args.redirect_to.as_deref())
                    .await
                    .map(|item| item.url)
                    .unwrap_or_else(|_| DEFAULT_SETTINGS_URL.to_string())
            };

            if args.print_only {
                ctx.message(&url);
                return Ok(());
            }

            open::that(&url)?;
            ctx.success(format!("Opened {url}"));
            Ok(())
        }
        GitHubCommand::Repos(args) => {
            let repositories = ctx
                .client
                .github_app()
                .list_repositories(args.installation_id)
                .await?;
            ctx.present(
                GitHubRepositoriesView {
                    repositories: &repositories,
                },
                &GitHubRepositoriesJson {
                    repositories: &repositories,
                },
            )
        }
        GitHubCommand::Import(args) => {
            let result = ctx
                .client
                .github_app()
                .import(&GitHubImportRequest {
                    installation_id: args.installation_id,
                    repository_id: args.repository_id,
                    project_name: args.project_name,
                    project_description: args.project_description,
                })
                .await?;
            ctx.success(format!("Imported repository into {}", result.project.id));
            ctx.present(
                GitHubImportView { result: &result },
                &GitHubImportJson { result: &result },
            )
        }
    }
}
