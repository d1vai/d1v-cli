use clap::{Args, Subcommand};
use d1v_api::api::projects::{CreateProjectResponse, Project, Template};
use serde::Serialize;

use crate::Context;
use crate::error::Result;
use crate::text::{Field, Fields, Line, Span, Table, TableRow, Text};
use crate::theme;

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
    /// Delete a project
    Delete(DeleteArgs),
}

#[derive(Args)]
pub struct GetArgs {
    /// Project ID
    pub project_id: String,
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
}

#[derive(Args)]
pub struct UpdateArgs {
    /// Project ID
    pub project_id: String,
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
    pub project_id: String,
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
                    .repository_full_name
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
            field_opt("Repository", self.project.repository_full_name.as_deref()),
            field_opt(
                "Repo branch",
                self.project.repository_current_branch.as_deref(),
            ),
            field_opt(
                "Workspace branch",
                self.project.workspace_current_branch.as_deref(),
            ),
            field_opt("Preview URL", self.project.latest_preview_url.as_deref()),
            field_opt("Dev URL", self.project.latest_dev_deployment_url.as_deref()),
            field_opt(
                "Prod URL",
                self.project.latest_prod_deployment_url.as_deref(),
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
            field_opt_bool("Analytics", self.project.analytics_enabled),
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
            let project = ctx
                .client
                .project(&args.project_id)
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
            if args.is_empty() {
                ctx.message("Nothing to update. Pass at least one field flag.");
                return Ok(());
            }

            let project = ctx
                .client
                .project(&args.project_id)
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
        ProjectCommand::Delete(args) => {
            ctx.client.project(&args.project_id).delete().await?;
            ctx.success(format!("Deleted project {}", args.project_id));
            Ok(())
        }
    }
}
