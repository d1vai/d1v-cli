use anyhow::anyhow;
use clap::{Args, Subcommand};
use d1v_api::api::migrations::Msg;
use d1v_api::api::projects::{DatabaseSchema, DbBranch, DbColumn, Token, TokenScope};
use d1v_api::{
    ApprovalRequest, ApprovalResponse, AutoReviewResponse, ExecuteRequest, ExecuteResponse,
    HistoryResponse, PlanRequest, PlanResponse, ValidateRequest, ValidateResponse,
};
use itertools::Itertools;
use serde::Serialize;

use crate::Context;
use crate::error::Result;
use crate::text::{Field, Fields, Line, Span, Table, TableRow, Text};
use crate::theme;

#[derive(Subcommand)]
pub enum DbCommand {
    /// Inspect database schema
    Schema(SchemaArgs),
    /// Inspect sampled database data
    Data(DataArgs),
    /// List available Neon branches
    Branches(ProjectArgs),
    /// Manage tables
    Tables {
        #[command(subcommand)]
        command: TableCommand,
    },
    /// Manage rows
    Rows {
        #[command(subcommand)]
        command: RowCommand,
    },
    /// Issue or refresh project-scoped DB/migration tokens
    Token {
        #[command(subcommand)]
        command: TokenCommand,
    },
    /// Manage migrations
    Migrate {
        #[command(subcommand)]
        command: MigrateCommand,
    },
}

#[derive(Subcommand)]
pub enum TableCommand {
    /// Create a table
    Create(TableCreateArgs),
    /// Drop a table
    Drop(TableDropArgs),
}

#[derive(Subcommand)]
pub enum RowCommand {
    /// List rows from a table
    List(RowListArgs),
    /// Insert one row
    Insert(RowInsertArgs),
    /// Update rows by equality match
    Update(RowUpdateArgs),
    /// Delete rows by equality match
    Delete(RowDeleteArgs),
}

#[derive(Subcommand)]
pub enum TokenCommand {
    /// Issue a new project token
    Issue(TokenArgs),
    /// Refresh a project token
    Refresh(TokenArgs),
}

#[derive(Subcommand)]
pub enum MigrateCommand {
    /// Create a migration plan from SQL
    Plan(MigratePlanArgs),
    /// Validate a migration plan
    Validate(MigrateValidateArgs),
    /// Create an approval request
    Approve(MigrateApproveArgs),
    /// Auto-review an approval request
    AutoReview(MigrateApprovalIdArgs),
    /// Manually approve an approval request
    ManualApprove(MigrateApprovalIdArgs),
    /// Execute an approved migration
    Execute(MigrateExecuteArgs),
    /// Show migration history
    History(MigrateHistoryArgs),
    /// Show migration plan detail
    Detail(MigrateDetailArgs),
}

#[derive(Args)]
pub struct ProjectArgs {
    pub project_id: String,
}

#[derive(Args)]
pub struct SchemaArgs {
    pub project_id: String,
    #[arg(long)]
    pub branch: Option<String>,
    #[arg(long)]
    pub include_views: bool,
    #[arg(long)]
    pub with_row_counts: bool,
    #[arg(long)]
    pub include_system_schemas: bool,
}

#[derive(Args)]
pub struct DataArgs {
    pub project_id: String,
    #[arg(long)]
    pub branch: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit_per_table: u32,
    #[arg(long)]
    pub include_views: bool,
    #[arg(long)]
    pub include_system_schemas: bool,
}

#[derive(Args)]
pub struct TableCreateArgs {
    pub project_id: String,
    #[arg(long, default_value = "public")]
    pub schema: String,
    #[arg(long)]
    pub table: String,
    /// JSON array of column definitions
    #[arg(long)]
    pub columns: String,
    #[arg(long, value_delimiter = ',')]
    pub primary_key: Vec<String>,
    #[arg(long)]
    pub branch: Option<String>,
    #[arg(long, default_value_t = true)]
    pub create_schema_if_missing: bool,
}

#[derive(Args)]
pub struct TableDropArgs {
    pub project_id: String,
    #[arg(long, default_value = "public")]
    pub schema: String,
    #[arg(long)]
    pub table: String,
    #[arg(long)]
    pub branch: Option<String>,
    #[arg(long)]
    pub cascade: bool,
}

#[derive(Args)]
pub struct RowListArgs {
    pub project_id: String,
    #[arg(long, default_value = "public")]
    pub schema: String,
    #[arg(long)]
    pub table: String,
    #[arg(long)]
    pub branch: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub limit: u32,
    #[arg(long, default_value_t = 0)]
    pub offset: u32,
}

#[derive(Args)]
pub struct RowInsertArgs {
    pub project_id: String,
    #[arg(long, default_value = "public")]
    pub schema: String,
    #[arg(long)]
    pub table: String,
    /// JSON object payload for inserted values
    #[arg(long)]
    pub values: String,
    #[arg(long)]
    pub branch: Option<String>,
}

#[derive(Args)]
pub struct RowUpdateArgs {
    pub project_id: String,
    #[arg(long, default_value = "public")]
    pub schema: String,
    #[arg(long)]
    pub table: String,
    /// JSON object payload used as equality filter
    #[arg(long)]
    pub where_json: String,
    /// JSON object payload for updated values
    #[arg(long)]
    pub values: String,
    #[arg(long)]
    pub branch: Option<String>,
}

#[derive(Args)]
pub struct RowDeleteArgs {
    pub project_id: String,
    #[arg(long, default_value = "public")]
    pub schema: String,
    #[arg(long)]
    pub table: String,
    /// JSON object payload used as equality filter
    #[arg(long)]
    pub where_json: String,
    #[arg(long)]
    pub branch: Option<String>,
}

#[derive(Args)]
pub struct TokenArgs {
    /// Project ID
    pub project_id: String,
    /// Optional comma-separated scopes like db:read,db:write,migrate
    #[arg(long, value_delimiter = ',')]
    pub scopes: Vec<String>,
    /// Token TTL in seconds
    #[arg(long, default_value_t = 1800)]
    pub ttl_seconds: u32,
}

#[derive(Args)]
pub struct MigratePlanArgs {
    pub project_id: String,
    #[arg(long)]
    pub sql: String,
    #[arg(long)]
    pub intent: Option<String>,
}

#[derive(Args)]
pub struct MigrateValidateArgs {
    pub plan_id: String,
    #[arg(long)]
    pub sql: Option<String>,
}

#[derive(Args)]
pub struct MigrateApproveArgs {
    pub plan_id: String,
    #[arg(long)]
    pub risk_summary: Option<String>,
    #[arg(long)]
    pub expires_in_minutes: Option<u32>,
}

#[derive(Args)]
pub struct MigrateApprovalIdArgs {
    pub approval_id: String,
}

#[derive(Args)]
pub struct MigrateExecuteArgs {
    pub plan_id: String,
    #[arg(long)]
    pub approval_token: String,
    #[arg(long)]
    pub strategy: Option<String>,
}

#[derive(Args)]
pub struct MigrateHistoryArgs {
    pub project_id: String,
    #[arg(long, default_value_t = 20)]
    pub limit: u32,
    #[arg(long, default_value_t = 0)]
    pub offset: u32,
}

#[derive(Args)]
pub struct MigrateDetailArgs {
    pub plan_id: String,
}

#[derive(Debug, Serialize)]
struct Affected {
    affected: i64,
}

#[derive(Debug, Serialize)]
struct DbSchemaJson<'a> {
    schema: &'a DatabaseSchema,
}

#[derive(Debug, Serialize)]
struct DbDataJson<'a> {
    data: &'a serde_json::Value,
}

#[derive(Debug, Serialize)]
struct DbBranchesJson<'a> {
    branches: &'a [DbBranch],
}

#[derive(Debug, Serialize)]
struct DbRowsJson<'a> {
    rows: &'a [serde_json::Map<String, serde_json::Value>],
}

#[derive(Debug, Serialize)]
struct DbMessageJson<'a> {
    result: &'a Msg,
}

#[derive(Debug, Serialize)]
struct PlanJson<'a> {
    plan: &'a PlanResponse,
}

#[derive(Debug, Serialize)]
struct ValidateJson<'a> {
    validation: &'a ValidateResponse,
}

#[derive(Debug, Serialize)]
struct ApprovalJson<'a> {
    approval: &'a ApprovalResponse,
}

#[derive(Debug, Serialize)]
struct AutoReviewJson<'a> {
    review: &'a AutoReviewResponse,
}

#[derive(Debug, Serialize)]
struct ExecuteJson<'a> {
    execution: &'a ExecuteResponse,
}

#[derive(Debug, Serialize)]
struct HistoryJson<'a> {
    history: &'a HistoryResponse,
}

#[derive(Debug, Serialize)]
struct ProjectTokenJson<'a> {
    token: &'a Token,
}

fn field(label: &'static str, value: impl Into<String>) -> Field {
    Field::new(
        Span::styled(label, theme::ansi::label()),
        Line::styled(value.into(), theme::ansi::value()),
    )
}

fn field_opt(label: &'static str, value: Option<&str>) -> Field {
    field(label, value.unwrap_or("-"))
}

fn field_bool(label: &'static str, value: Option<bool>) -> Field {
    let rendered = match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "-",
    };
    field(label, rendered)
}

struct DbSchemaView<'a> {
    schema: &'a DatabaseSchema,
}

impl crate::text::Render for DbSchemaView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        if self.schema.tables.is_empty() {
            return Text::new()
                .line(Line::styled("No tables found.", theme::ansi::dim()))
                .render(ctx);
        }

        let rows = self.schema.tables.iter().map(|table| {
            TableRow::new([
                table.schema.clone(),
                table.name.clone(),
                table.kind.clone(),
                table.columns.len().to_string(),
                table
                    .row_count
                    .map(|count| count.to_string())
                    .unwrap_or_else(|| "-".to_string()),
            ])
        });

        Text::new()
            .line(Line::styled(
                format!(
                    "Database schema{}",
                    self.schema
                        .default_schema
                        .as_deref()
                        .map(|v| format!(" (default: {v})"))
                        .unwrap_or_default()
                ),
                theme::ansi::success(),
            ))
            .render(ctx)?;

        Table::new(rows)
            .header(TableRow::new([
                "schema", "table", "kind", "columns", "rows",
            ]))
            .border_style(theme::ansi::border())
            .render(ctx)
    }
}

struct DbBranchesView<'a> {
    branches: &'a [DbBranch],
}

impl crate::text::Render for DbBranchesView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        if self.branches.is_empty() {
            return Text::new()
                .line(Line::styled("No branches found.", theme::ansi::dim()))
                .render(ctx);
        }

        let rows = self.branches.iter().map(|branch| {
            TableRow::new([
                branch.id.clone(),
                branch.name.clone().unwrap_or_default(),
                if branch.primary.unwrap_or(false) {
                    "true"
                } else {
                    "false"
                }
                .to_string(),
            ])
        });

        Table::new(rows)
            .header(TableRow::new(["id", "name", "primary"]))
            .border_style(theme::ansi::border())
            .render(ctx)
    }
}

struct DbValueView<'a> {
    title: &'a str,
    value: &'a serde_json::Value,
}

impl crate::text::Render for DbValueView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        let rendered =
            serde_json::to_string_pretty(self.value).unwrap_or_else(|_| self.value.to_string());
        Text::new()
            .line(Line::styled(self.title.to_string(), theme::ansi::success()))
            .line("")
            .line(rendered)
            .render(ctx)
    }
}

struct DbRowsView<'a> {
    rows: &'a [serde_json::Map<String, serde_json::Value>],
}

impl crate::text::Render for DbRowsView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        if self.rows.is_empty() {
            return Text::new()
                .line(Line::styled("No rows found.", theme::ansi::dim()))
                .render(ctx);
        }

        let mut text = Text::new().line(Line::styled(
            format!("Rows ({})", self.rows.len()),
            theme::ansi::success(),
        ));

        for row in self.rows {
            let rendered = serde_json::to_string(row).unwrap_or_default();
            text = text.line(rendered);
        }

        text.render(ctx)
    }
}

struct StrView<'a> {
    title: &'a str,
    value: &'a str,
}

impl crate::text::Render for StrView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        Text::new()
            .line(Line::styled(self.title.to_string(), theme::ansi::success()))
            .render(ctx)?;
        Fields::new([field("Message", self.value)])
            .indent(2)
            .render(ctx)
    }
}

struct DbMessageView<'a> {
    title: &'a str,
    result: &'a Msg,
}

impl crate::text::Render for DbMessageView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        Text::new()
            .line(Line::styled(self.title.to_string(), theme::ansi::success()))
            .render(ctx)?;
        Fields::new([field("Message", &self.result.message)])
            .indent(2)
            .render(ctx)
    }
}

struct PlanView<'a> {
    result: &'a PlanResponse,
}

impl crate::text::Render for PlanView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        Text::new()
            .line(
                Line::styled("Migration plan".to_string(), theme::ansi::success())
                    .push_plain(" ")
                    .push_styled(self.result.plan_id.clone(), theme::ansi::plain()),
            )
            .render(ctx)?;
        Fields::new([
            field("Project", &self.result.project_id),
            field("Intent", &self.result.intent),
            field("Created", &self.result.created_at),
        ])
        .indent(2)
        .render(ctx)
    }
}

struct ValidateView<'a> {
    result: &'a ValidateResponse,
}

impl crate::text::Render for ValidateView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        Text::new()
            .line(
                Line::styled("Migration validation".to_string(), theme::ansi::success())
                    .push_plain(" ")
                    .push_styled(self.result.job_id.clone(), theme::ansi::plain()),
            )
            .render(ctx)?;
        let warnings = if self.result.warnings.is_empty() {
            "-".to_string()
        } else {
            self.result.warnings.join("; ")
        };
        Fields::new([
            field("Plan", &self.result.plan_id),
            field("Stage", &self.result.stage),
            field("Status", &self.result.status),
            field("Statements", self.result.statement_count.to_string()),
            field_bool("Dry-run fallback", self.result.fallback_to_dry_run),
            field("Warnings", warnings),
        ])
        .indent(2)
        .render(ctx)
    }
}

struct ApprovalView<'a> {
    title: &'a str,
    result: &'a ApprovalResponse,
}

impl crate::text::Render for ApprovalView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        Text::new()
            .line(
                Line::styled(self.title.to_string(), theme::ansi::success())
                    .push_plain(" ")
                    .push_styled(self.result.approval_id.clone(), theme::ansi::plain()),
            )
            .render(ctx)?;
        Fields::new([
            field("Plan", &self.result.plan_id),
            field("Status", &self.result.status),
            field_opt("Approval token", self.result.approval_token.as_deref()),
            field("Created", &self.result.created_at),
            field("Expires", &self.result.expires_at),
        ])
        .indent(2)
        .render(ctx)
    }
}

struct AutoReviewView<'a> {
    result: &'a AutoReviewResponse,
}

impl crate::text::Render for AutoReviewView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        let reasons = self
            .result
            .reasons
            .as_ref()
            .map(|items| items.join("; "))
            .unwrap_or_else(|| "-".to_string());
        Text::new()
            .line(Line::styled(
                "Migration auto review".to_string(),
                theme::ansi::success(),
            ))
            .render(ctx)?;
        Fields::new([
            field("Status", &self.result.status),
            field_opt("Approval token", self.result.approval_token.as_deref()),
            field(
                "Risk score",
                self.result
                    .risk_score
                    .map(|score| score.to_string())
                    .unwrap_or_else(|| "-".to_string()),
            ),
            field("Reasons", reasons),
        ])
        .indent(2)
        .render(ctx)
    }
}

struct ExecuteView<'a> {
    result: &'a ExecuteResponse,
}

impl crate::text::Render for ExecuteView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        Text::new()
            .line(
                Line::styled("Migration execution".to_string(), theme::ansi::success())
                    .push_plain(" ")
                    .push_styled(self.result.job_id.clone(), theme::ansi::plain()),
            )
            .render(ctx)?;
        Fields::new([
            field("Plan", &self.result.plan_id),
            field("Stage", &self.result.stage),
            field("Status", &self.result.status),
            field("Created", &self.result.created_at),
            field_bool("Dry-run fallback", self.result.fallback_to_dry_run),
        ])
        .indent(2)
        .render(ctx)
    }
}

struct HistoryView<'a> {
    result: &'a HistoryResponse,
}

impl crate::text::Render for HistoryView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        if self.result.plans.is_empty() {
            return Text::new()
                .line(Line::styled(
                    "No migration history found.",
                    theme::ansi::dim(),
                ))
                .render(ctx);
        }

        let rows = self.result.plans.iter().map(|plan| {
            TableRow::new([
                json_string(plan.get("id").or_else(|| plan.get("plan_id"))),
                json_string(plan.get("intent")),
                json_string(plan.get("created_at")),
                json_string(plan.get("status")),
            ])
        });

        Table::new(rows)
            .header(TableRow::new(["plan_id", "intent", "created_at", "status"]))
            .border_style(theme::ansi::border())
            .render(ctx)
    }
}

struct ProjectTokenView<'a> {
    title: &'a str,
    token: &'a Token,
}

impl crate::text::Render for ProjectTokenView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        Text::new()
            .line(Line::styled(self.title.to_string(), theme::ansi::success()))
            .render(ctx)?;
        Fields::new([
            field(
                "Expires",
                self.token
                    .expires_at
                    .strftime("%Y-%m-%d %H:%M:%S")
                    .to_string(),
            ),
            field(
                "Scopes",
                if self.token.scopes.is_empty() {
                    "-".to_string()
                } else {
                    self.token.scopes.iter().map(|s| s.to_string()).join(",")
                },
            ),
            field("Token", &self.token.project_token),
        ])
        .indent(2)
        .render(ctx)
    }
}

fn json_string(value: Option<&serde_json::Value>) -> String {
    match value {
        Some(serde_json::Value::String(text)) => text.clone(),
        Some(serde_json::Value::Null) | None => "-".to_string(),
        Some(other) => other.to_string(),
    }
}

fn parse_json_map(
    input: &str,
    flag_name: &str,
) -> Result<serde_json::Map<String, serde_json::Value>> {
    let value: serde_json::Value = serde_json::from_str(input)
        .map_err(|err| anyhow!("invalid JSON for {flag_name}: {err}"))?;
    match value {
        serde_json::Value::Object(map) => Ok(map),
        _ => Err(anyhow!("{flag_name} must be a JSON object").into()),
    }
}

fn parse_columns(input: &str) -> Result<Vec<DbColumn>> {
    serde_json::from_str(input).map_err(|err| anyhow!("invalid JSON for --columns: {err}").into())
}

pub async fn run(ctx: &Context, command: DbCommand) -> Result<()> {
    match command {
        DbCommand::Schema(args) => {
            let schema = ctx
                .client
                .project(&args.project_id)
                .db()
                .schema()
                .maybe_branch(args.branch.as_deref())
                .include_views(args.include_views)
                .with_row_counts(args.with_row_counts)
                .include_system_schemas(args.include_system_schemas)
                .call()
                .await?;
            ctx.present(
                DbSchemaView { schema: &schema },
                &DbSchemaJson { schema: &schema },
            )
        }
        DbCommand::Data(args) => {
            let data = ctx
                .client
                .project(&args.project_id)
                .db()
                .data()
                .maybe_branch(args.branch.as_deref())
                .limit_per_table(args.limit_per_table)
                .include_views(args.include_views)
                .include_system_schemas(args.include_system_schemas)
                .call()
                .await?;
            ctx.present(
                DbValueView {
                    title: "Database data",
                    value: &data,
                },
                &DbDataJson { data: &data },
            )
        }
        DbCommand::Branches(args) => {
            let branches = ctx.client.project(&args.project_id).db().branches().await?;
            ctx.present(
                DbBranchesView {
                    branches: &branches,
                },
                &DbBranchesJson {
                    branches: &branches,
                },
            )
        }
        DbCommand::Tables { command } => match command {
            TableCommand::Create(args) => {
                let columns = parse_columns(&args.columns)?;
                let msg = ctx
                    .client
                    .projects()
                    .project(&args.project_id)
                    .db()
                    .create_table(&args.table)
                    .columns(columns)
                    .maybe_schema_name(Some(args.schema.as_str()))
                    .maybe_primary_key((!args.primary_key.is_empty()).then_some(args.primary_key))
                    .maybe_branch(args.branch.as_deref())
                    .create_schema_if_missing(args.create_schema_if_missing)
                    .call()
                    .await?;
                ctx.success("Table created");
                ctx.present(
                    StrView {
                        title: "Create table",
                        value: &msg,
                    },
                    &msg,
                )
            }
            TableCommand::Drop(args) => {
                let msg = ctx
                    .client
                    .projects()
                    .project(&args.project_id)
                    .db()
                    .drop_table(&args.schema, &args.table)
                    .maybe_branch(args.branch.as_deref())
                    .maybe_cascade(args.cascade.then_some(true))
                    .call()
                    .await?;
                ctx.success("Table dropped");
                ctx.present(
                    StrView {
                        title: "Drop table",
                        value: &msg,
                    },
                    &msg,
                )
            }
        },
        DbCommand::Rows { command } => match command {
            RowCommand::List(args) => {
                let rows = ctx
                    .client
                    .projects()
                    .project(&args.project_id)
                    .db()
                    .list_table_rows(&args.schema, &args.table)
                    .maybe_branch(args.branch.as_deref())
                    .limit(args.limit)
                    .offset(args.offset)
                    .call()
                    .await?;
                ctx.present(DbRowsView { rows: &rows }, &DbRowsJson { rows: &rows })
            }
            RowCommand::Insert(args) => {
                let values = parse_json_map(&args.values, "--values")?;
                let affected = ctx
                    .client
                    .projects()
                    .project(&args.project_id)
                    .db()
                    .insert_table_row(&args.schema, &args.table)
                    .values(values)
                    .maybe_branch(args.branch.as_deref())
                    .call()
                    .await?;
                let msg = format!("Affected rows: {affected}");
                ctx.success("Row inserted");
                ctx.present(
                    StrView {
                        title: "Insert row",
                        value: &msg,
                    },
                    &Affected { affected },
                )
            }
            RowCommand::Update(args) => {
                let where_ = parse_json_map(&args.where_json, "--where-json")?;
                let values = parse_json_map(&args.values, "--values")?;
                let affected = ctx
                    .client
                    .projects()
                    .project(&args.project_id)
                    .db()
                    .update_table_rows(&args.schema, &args.table)
                    .where_(where_)
                    .values(values)
                    .maybe_branch(args.branch.as_deref())
                    .call()
                    .await?;
                let msg = format!("Affected rows: {affected}");
                ctx.success("Rows updated");
                ctx.present(
                    StrView {
                        title: "Update rows",
                        value: &msg,
                    },
                    &Affected { affected },
                )
            }
            RowCommand::Delete(args) => {
                let where_ = parse_json_map(&args.where_json, "--where-json")?;
                let affected = ctx
                    .client
                    .projects()
                    .project(&args.project_id)
                    .db()
                    .delete_table_rows(&args.schema, &args.table)
                    .where_(where_)
                    .maybe_branch(args.branch.as_deref())
                    .call()
                    .await?;
                let msg = format!("Affected rows: {affected}");
                ctx.success("Rows deleted");
                ctx.present(
                    StrView {
                        title: "Delete rows",
                        value: &msg,
                    },
                    &Affected { affected },
                )
            }
        },
        DbCommand::Token { command } => match command {
            TokenCommand::Issue(args) => {
                let scopes: Vec<TokenScope> =
                    args.scopes.iter().filter_map(|s| s.parse().ok()).collect();
                let token = ctx
                    .client
                    .projects()
                    .project(&args.project_id)
                    .issue_token()
                    .maybe_scopes((!scopes.is_empty()).then_some(scopes))
                    .ttl_seconds(args.ttl_seconds)
                    .call()
                    .await?;
                ctx.success(format!("Issued project token for {}", args.project_id));
                ctx.present(
                    ProjectTokenView {
                        title: "Project token",
                        token: &token,
                    },
                    &ProjectTokenJson { token: &token },
                )
            }
            TokenCommand::Refresh(args) => {
                let scopes: Vec<TokenScope> =
                    args.scopes.iter().filter_map(|s| s.parse().ok()).collect();
                let token = ctx
                    .client
                    .projects()
                    .project(&args.project_id)
                    .refresh_token()
                    .maybe_scopes((!scopes.is_empty()).then_some(scopes))
                    .ttl_seconds(args.ttl_seconds)
                    .call()
                    .await?;
                ctx.success(format!("Refreshed project token for {}", args.project_id));
                ctx.present(
                    ProjectTokenView {
                        title: "Project token",
                        token: &token,
                    },
                    &ProjectTokenJson { token: &token },
                )
            }
        },
        DbCommand::Migrate { command } => match command {
            MigrateCommand::Plan(args) => {
                let result = ctx
                    .client
                    .migrations()
                    .plan(&PlanRequest {
                        project_id: args.project_id,
                        intent: args.intent,
                        proposed_sql: args.sql,
                    })
                    .await?;
                ctx.success(format!("Created plan {}", result.plan_id));
                ctx.present(PlanView { result: &result }, &PlanJson { plan: &result })
            }
            MigrateCommand::Validate(args) => {
                let result = ctx
                    .client
                    .migrations()
                    .validate(&ValidateRequest {
                        plan_id: args.plan_id,
                        sql: args.sql,
                    })
                    .await?;
                ctx.present(
                    ValidateView { result: &result },
                    &ValidateJson {
                        validation: &result,
                    },
                )
            }
            MigrateCommand::Approve(args) => {
                let result = ctx
                    .client
                    .migrations()
                    .create_approval(&ApprovalRequest {
                        plan_id: args.plan_id,
                        risk_summary: args.risk_summary,
                        expires_in_minutes: args.expires_in_minutes,
                    })
                    .await?;
                ctx.success(format!("Created approval {}", result.approval_id));
                ctx.present(
                    ApprovalView {
                        title: "Migration approval",
                        result: &result,
                    },
                    &ApprovalJson { approval: &result },
                )
            }
            MigrateCommand::AutoReview(args) => {
                let result = ctx
                    .client
                    .migrations()
                    .auto_review(&args.approval_id)
                    .await?;
                ctx.present(
                    AutoReviewView { result: &result },
                    &AutoReviewJson { review: &result },
                )
            }
            MigrateCommand::ManualApprove(args) => {
                let result = ctx.client.migrations().approve(&args.approval_id).await?;
                ctx.success(format!("Approved {}", args.approval_id));
                ctx.present(
                    DbMessageView {
                        title: "Manual approval",
                        result: &result,
                    },
                    &DbMessageJson { result: &result },
                )
            }
            MigrateCommand::Execute(args) => {
                let result = ctx
                    .client
                    .migrations()
                    .execute(&ExecuteRequest {
                        plan_id: args.plan_id,
                        strategy: args.strategy,
                        approval_token: args.approval_token,
                    })
                    .await?;
                ctx.success(format!("Started execution {}", result.job_id));
                ctx.present(
                    ExecuteView { result: &result },
                    &ExecuteJson { execution: &result },
                )
            }
            MigrateCommand::History(args) => {
                let result = ctx
                    .client
                    .migrations()
                    .history(&args.project_id, Some(args.limit), Some(args.offset))
                    .await?;
                ctx.present(
                    HistoryView { result: &result },
                    &HistoryJson { history: &result },
                )
            }
            MigrateCommand::Detail(args) => {
                let result = ctx.client.migrations().detail(&args.plan_id).await?;
                ctx.present(
                    DbValueView {
                        title: "Migration detail",
                        value: &result,
                    },
                    &result,
                )
            }
        },
    }
}
