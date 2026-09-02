use std::path::PathBuf;

use anyhow::anyhow;
use clap::{Args, Subcommand, ValueEnum};
use d1v_api::api::projects::{
    CreateProjectResponse, EnsureProjectIntegrationStatus, EnsureProjectIntegrationsResponse,
    Project, Template,
};
use serde::Serialize;

use crate::Context;
use crate::error::Result;
use crate::text::{Field, Fields, Line, Span, Table, TableRow, Text};
use crate::theme;
use crate::workspace;

#[derive(Subcommand)]
pub enum ProjectCommand {
    /// List projects
    List,
    /// List supported project templates
    Templates,
    /// Get project details
    Get(GetArgs),
    /// Create a project
    Create(CreateArgs),
    /// Update a project
    Update(UpdateArgs),
    /// Ensure project integrations are enabled
    Ensure(EnsureArgs),
    /// Delete a project
    Delete(DeleteArgs),
}

#[derive(Args)]
pub struct GetArgs {
    /// Project ID
    pub project_id: Option<String>,
    /// Ask backend to refresh synced project state
    #[arg(long)]
    pub sync: bool,
}

#[derive(Args)]
pub struct CreateArgs {
    /// Project name for direct project creation
    #[arg(long)]
    pub name: Option<String>,
    /// Project description for direct project creation
    #[arg(long)]
    pub description: Option<String>,
    /// One-step AI project creation prompt
    #[arg(long)]
    pub prompt: Option<String>,
    /// Template repo for AI project creation
    #[arg(long)]
    pub template_repo: Option<String>,
    /// Auto deploy after execution
    #[arg(long)]
    pub auto_deploy: Option<bool>,
    /// Enable database integration in one-step creation
    #[arg(long)]
    pub enable_database: Option<bool>,
    /// Enable payment integration in one-step creation
    #[arg(long)]
    pub enable_pay: Option<bool>,
    /// Target organization ID; omit for a personal project
    #[arg(long)]
    pub organization_id: Option<u64>,
}

#[derive(Args)]
pub struct UpdateArgs {
    /// Project ID
    pub project_id: Option<String>,
    /// New project name
    #[arg(long)]
    pub name: Option<String>,
    /// New project description
    #[arg(long)]
    pub description: Option<String>,
    /// Project emoji
    #[arg(long)]
    pub emoji: Option<String>,
    /// Override auto-deploy-on-execute
    #[arg(long)]
    pub auto_deploy: Option<bool>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum EnsureTarget {
    #[value(alias = "db")]
    Database,
    #[value(alias = "payments", alias = "payment")]
    Pay,
    Analytics,
}

#[derive(Args)]
pub struct EnsureArgs {
    /// Integration targets to ensure
    #[arg(value_enum, num_args = 1..)]
    pub targets: Vec<EnsureTarget>,
    /// Project ID override (defaults to `.env`, D1V_PROJECT_ID, or workspace binding)
    #[arg(long)]
    pub project_id: Option<String>,
    /// Resolve workspace metadata from this path instead of the current directory
    #[arg(long)]
    pub path: Option<PathBuf>,
}

impl UpdateArgs {
    fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.description.is_none()
            && self.emoji.is_none()
            && self.auto_deploy.is_none()
    }
}

#[derive(Args)]
pub struct DeleteArgs {
    /// Project ID
    pub project_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct ProjectListJson<'a> {
    projects: &'a [Project],
}

struct ProjectListView<'a> {
    projects: &'a [Project],
}

impl crate::text::Render for ProjectListView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        if self.projects.is_empty() {
            return Text::new()
                .line(Line::styled("No projects found.", theme::ansi::dim()))
                .render(ctx);
        }

        let rows = self.projects.iter().map(|project| {
            TableRow::new([
                project.id.clone(),
                project.project_name.clone(),
                project
                    .repository
                    .full_name
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
                project
                    .updated_at
                    .map(|t| t.strftime("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_default(),
            ])
        });

        Table::new(rows)
            .header(TableRow::new(["id", "name", "repo", "updated_at"]))
            .border_style(theme::ansi::border())
            .render(ctx)
    }
}

#[derive(Debug, Serialize)]
struct ProjectTemplatesJson<'a> {
    templates: &'a [Template],
}

struct ProjectTemplatesView<'a> {
    templates: &'a [Template],
}

impl crate::text::Render for ProjectTemplatesView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        if self.templates.is_empty() {
            return Text::new()
                .line(Line::styled("No templates found.", theme::ansi::dim()))
                .render(ctx);
        }

        let rows = self.templates.iter().map(|template| {
            TableRow::new([
                template.template_repo.clone(),
                template.name.clone(),
                template.category.clone().unwrap_or_else(|| "-".to_string()),
                template.kind.clone().unwrap_or_else(|| "-".to_string()),
            ])
        });

        Table::new(rows)
            .header(TableRow::new(["template_repo", "name", "category", "kind"]))
            .border_style(theme::ansi::border())
            .render(ctx)
    }
}

#[derive(Debug, Serialize)]
struct ProjectDetailJson<'a> {
    project: &'a Project,
}

struct ProjectDetailView<'a> {
    title: &'a str,
    project: &'a Project,
}

impl crate::text::Render for ProjectDetailView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        let heading = Line::styled(self.title.to_string(), theme::ansi::success())
            .push_plain(" ")
            .push_styled(
                format!("{} ({})", self.project.project_name, self.project.id),
                theme::ansi::plain(),
            );

        let fields = vec![
            field_opt(
                "Description",
                Some(self.project.project_description.as_str()),
            ),
            field_opt("Emoji", self.project.emoji.as_deref()),
            field_opt("Repository", self.project.repository.full_name.as_deref()),
            field_opt(
                "Repo branch",
                self.project.repository.current_branch.as_deref(),
            ),
            field_opt(
                "Workspace branch",
                self.project.workspace_current_branch.as_deref(),
            ),
            field_opt(
                "Preview URL",
                self.project.vercel.latest_preview_url.as_deref(),
            ),
            field_opt(
                "Dev URL",
                self.project.vercel.latest_dev_deployment_url.as_deref(),
            ),
            field_opt(
                "Prod URL",
                self.project.vercel.latest_prod_deployment_url.as_deref(),
            ),
            field_opt(
                "Created",
                self.project
                    .created_at
                    .map(|t| t.strftime("%Y-%m-%d %H:%M:%S").to_string())
                    .as_deref(),
            ),
            field_opt(
                "Updated",
                self.project
                    .updated_at
                    .map(|t| t.strftime("%Y-%m-%d %H:%M:%S").to_string())
                    .as_deref(),
            ),
            field_opt_bool("Auto deploy", self.project.auto_deploy_on_execute),
            field_opt_bool("Analytics", self.project.analytics.enabled),
            field_opt("Database", self.project.project_database_id.as_deref()),
            field_opt("Payments", self.project.project_pay_id.as_deref()),
        ];

        Text::new().line(heading).render(ctx)?;
        Fields::new(fields).indent(2).render(ctx)
    }
}

#[derive(Debug, Serialize)]
struct ProjectCreateJson<'a> {
    result: &'a CreateProjectResponse,
}

struct ProjectCreateView<'a> {
    result: &'a CreateProjectResponse,
}

#[derive(Debug, Serialize)]
struct ProjectEnsureJson<'a> {
    result: &'a EnsureProjectIntegrationsResponse,
}

struct ProjectEnsureView<'a> {
    result: &'a EnsureProjectIntegrationsResponse,
}

impl crate::text::Render for ProjectCreateView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        ProjectDetailView {
            title: "Created",
            project: &self.result.project,
        }
        .render(ctx)?;

        if let Some(session) = &self.result.session {
            writeln!(ctx.writer)?;
            Text::new()
                .line(
                    Line::styled("Initial session".to_string(), theme::ansi::success())
                        .push_plain(" ")
                        .push_styled(session.session_id.clone(), theme::ansi::plain()),
                )
                .render(ctx)?;
            Fields::new([
                field_opt("Model", session.model.as_deref()),
                field_opt("Status", session.status.as_deref()),
                field_opt("WebSocket", session.websocket_url.as_deref()),
            ])
            .indent(2)
            .render(ctx)?;
        }

        Ok(())
    }
}

impl crate::text::Render for ProjectEnsureView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        let project = &self.result.project;
        let heading = Line::styled("Ensured".to_string(), theme::ansi::success())
            .push_plain(" ")
            .push_styled(
                format!("{} ({})", project.project_name, project.id),
                theme::ansi::plain(),
            );

        Text::new().line(heading).render(ctx)?;
        Fields::new([
            field_opt(
                "Database",
                Some(&render_ensure_status(&self.result.database)),
            ),
            field_opt("Pay", Some(&render_ensure_status(&self.result.pay))),
            field_opt(
                "Analytics",
                Some(&render_ensure_status(&self.result.analytics)),
            ),
        ])
        .indent(2)
        .render(ctx)?;

        if !self.result.errors.is_empty() {
            let joined = self.result.errors.join("; ");
            Fields::new([field_opt("Errors", Some(&joined))])
                .indent(2)
                .render(ctx)?;
        }

        Ok(())
    }
}

fn field_opt(label: &'static str, value: Option<&str>) -> Field {
    Field::new(
        Span::styled(label, theme::ansi::label()),
        Line::styled(value.unwrap_or("-").to_string(), theme::ansi::value()),
    )
}

fn field_opt_bool(label: &'static str, value: Option<bool>) -> Field {
    let rendered = match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "-",
    };
    field_opt(label, Some(rendered))
}

fn render_ensure_status(status: &EnsureProjectIntegrationStatus) -> String {
    let mut rendered = format!("{}: {}", status.status, status.message);
    if let Some(error) = status.error.as_deref() {
        rendered.push_str(" (");
        rendered.push_str(error);
        rendered.push(')');
    }
    rendered
}

fn resolve_ensure_project_id(args: &EnsureArgs) -> Result<String> {
    if let Some(project_id) = args
        .project_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(project_id.to_string());
    }

    if let Some(project_id) = workspace::resolve_env_project_id(args.path.as_deref())? {
        return Ok(project_id);
    }

    if let Some(project_id) = std::env::var("D1V_PROJECT_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        return Ok(project_id);
    }

    if let Some(project_id) = workspace::resolve_bound_project_id(args.path.as_deref())? {
        return Ok(project_id);
    }

    Err(anyhow!(
        "project id is required. Set D1V_PROJECT_ID, pass --project-id, or run inside a bound d1v workspace."
    )
    .into())
}

fn resolve_project_id(explicit: Option<String>) -> Result<String> {
    if let Some(project_id) = explicit.filter(|value| !value.trim().is_empty()) {
        return Ok(project_id);
    }
    if let Some(project_id) = workspace::resolve_env_project_id(None)? {
        return Ok(project_id);
    }
    if let Some(project_id) = std::env::var("D1V_PROJECT_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())
    {
        return Ok(project_id);
    }
    if let Some(project_id) = workspace::resolve_bound_project_id(None)? {
        return Ok(project_id);
    }
    Err(anyhow!("project id is required; pass PROJECT_ID or add D1V_PROJECT_ID to .env").into())
}

pub async fn run(ctx: &Context, command: ProjectCommand) -> Result<()> {
    match command {
        ProjectCommand::List => {
            let projects = ctx.client.projects().list().await?;
            ctx.present(
                ProjectListView {
                    projects: &projects,
                },
                &ProjectListJson {
                    projects: &projects,
                },
            )
        }
        ProjectCommand::Templates => {
            let templates = ctx.client.projects().templates().await?;
            ctx.present(
                ProjectTemplatesView {
                    templates: &templates,
                },
                &ProjectTemplatesJson {
                    templates: &templates,
                },
            )
        }
        ProjectCommand::Get(args) => {
            let project_id = resolve_project_id(args.project_id.clone())?;
            let project = ctx
                .client
                .project(&project_id)
                .get(args.sync.then_some(true))
                .await?;
            ctx.present(
                ProjectDetailView {
                    title: "Project",
                    project: &project,
                },
                &ProjectDetailJson { project: &project },
            )
        }
        ProjectCommand::Create(args) => {
            if let Some(prompt) = args.prompt {
                let result = ctx
                    .client
                    .projects()
                    .create_with_integrations(&prompt)
                    .max_desc_len(120)
                    .maybe_template_repo(args.template_repo.as_deref())
                    .maybe_auto_deploy_on_execute(args.auto_deploy)
                    .maybe_enable_pay(args.enable_pay)
                    .maybe_enable_database(args.enable_database)
                    .maybe_organization_id(args.organization_id)
                    .call()
                    .await?;
                ctx.success(format!("Created project {}", result.project.id));
                ctx.present(
                    ProjectCreateView { result: &result },
                    &ProjectCreateJson { result: &result },
                )
            } else {
                let name = args.name.unwrap_or_default();
                let description = args.description.unwrap_or_default();
                if name.trim().is_empty() || description.trim().is_empty() {
                    ctx.message(
                        "Direct create requires --name and --description, or use --prompt for one-step AI creation.",
                    );
                    return Ok(());
                }
                let result = ctx
                    .client
                    .projects()
                    .create(&name, &description)
                    .maybe_enable_database(args.enable_database)
                    .maybe_enable_pay(args.enable_pay)
                    .maybe_organization_id(args.organization_id)
                    .call()
                    .await?;
                ctx.success(format!("Created project {}", result.project.id));
                ctx.present(
                    ProjectCreateView { result: &result },
                    &ProjectCreateJson { result: &result },
                )
            }
        }
        ProjectCommand::Update(args) => {
            let project_id = resolve_project_id(args.project_id.clone())?;
            if args.is_empty() {
                ctx.message("Nothing to update. Pass at least one field flag.");
                return Ok(());
            }

            let project = ctx
                .client
                .project(&project_id)
                .update()
                .maybe_project_name(args.name.as_deref())
                .maybe_project_description(args.description.as_deref())
                .maybe_emoji(args.emoji.as_deref())
                .maybe_auto_deploy_on_execute(args.auto_deploy)
                .call()
                .await?;
            ctx.success(format!("Updated project {}", project.id));
            ctx.present(
                ProjectDetailView {
                    title: "Updated",
                    project: &project,
                },
                &ProjectDetailJson { project: &project },
            )
        }
        ProjectCommand::Ensure(args) => {
            let project_id = resolve_ensure_project_id(&args)?;
            let mut database = false;
            let mut pay = false;
            let mut analytics = false;
            for target in args.targets {
                match target {
                    EnsureTarget::Database => database = true,
                    EnsureTarget::Pay => pay = true,
                    EnsureTarget::Analytics => analytics = true,
                }
            }

            let result = ctx
                .client
                .project(&project_id)
                .integrations()
                .ensure()
                .database(database)
                .pay(pay)
                .analytics(analytics)
                .call()
                .await?;

            if result.errors.is_empty() {
                ctx.success(format!(
                    "Ensured integrations for project {}",
                    result.project.id
                ));
            } else {
                ctx.message(format!(
                    "Integration ensure completed with {} error(s)",
                    result.errors.len()
                ));
            }

            ctx.present(
                ProjectEnsureView { result: &result },
                &ProjectEnsureJson { result: &result },
            )
        }
        ProjectCommand::Delete(args) => {
            let project_id = resolve_project_id(args.project_id)?;
            ctx.client.project(&project_id).delete().await?;
            ctx.success(format!("Deleted project {}", project_id));
            Ok(())
        }
    }
}
