use std::collections::BTreeMap;

use anyhow::anyhow;
use clap::{Args, Subcommand};
use d1v_api::{
    ApprovalRequest, ApprovalResponse, AutoReviewResponse, DbAffectedResponse, DbBranch,
    DbCreateTableRequest, DbDataOptions, DbDeleteRowsRequest, DbMessageResponse,
    DbRenameTableRequest, DbRowsOptions, DbSchemaOptions, DbSchemaResponse, DbTableColumnInput,
    DbUpdateRowsRequest, DbValuesRequest, ExecuteRequest, ExecuteResponse, HistoryResponse,
    PlanRequest, PlanResponse, ProjectTokenRequest, ProjectTokenResponse, ValidateRequest,
    ValidateResponse,
};
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
    /// Rename a table
    Rename(TableRenameArgs),
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
pub struct TableRenameArgs {
    pub project_id: String,
    #[arg(long, default_value = "public")]
    pub schema: String,
    #[arg(long)]
    pub table: String,
    #[arg(long)]
    pub new_table_name: String,
    #[arg(long)]
    pub branch: Option<String>,
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
struct DbSchemaJson<'a> {
    schema: &'a DbSchemaResponse,
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
    rows: &'a [serde_json::Value],
}

#[derive(Debug, Serialize)]
struct DbMessageJson<'a> {
    result: &'a DbMessageResponse,
}

#[derive(Debug, Serialize)]
struct DbAffectedJson<'a> {
    result: &'a DbAffectedResponse,
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
    token: &'a ProjectTokenResponse,
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
    schema: &'a DbSchemaResponse,
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
                branch.name.clone(),
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
    rows: &'a [serde_json::Value],
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
            let rendered = serde_json::to_string(row).unwrap_or_else(|_| row.to_string());
            text = text.line(rendered);
        }

        text.render(ctx)
    }
}

struct DbMessageView<'a> {
    title: &'a str,
    result: &'a DbMessageResponse,
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

struct DbAffectedView<'a> {
    title: &'a str,
    result: &'a DbAffectedResponse,
}

impl crate::text::Render for DbAffectedView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        Text::new()
            .line(Line::styled(self.title.to_string(), theme::ansi::success()))
            .render(ctx)?;
        Fields::new([field("Affected", self.result.affected.to_string())])
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
    result: &'a ProjectTokenResponse,
}

impl crate::text::Render for ProjectTokenView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        Text::new()
            .line(Line::styled(self.title.to_string(), theme::ansi::success()))
            .render(ctx)?;
        Fields::new([
            field("Expires", &self.result.expires_at),
            field(
                "Scopes",
                if self.result.scopes.is_empty() {
                    "-".to_string()
                } else {
                    self.result.scopes.join(",")
                },
            ),
            field("Token", &self.result.project_token),
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

fn parse_json_map(input: &str, flag_name: &str) -> Result<BTreeMap<String, serde_json::Value>> {
    let value: serde_json::Value = serde_json::from_str(input)
        .map_err(|err| anyhow!("invalid JSON for {flag_name}: {err}"))?;
    match value {
        serde_json::Value::Object(map) => Ok(map.into_iter().collect()),
        _ => Err(anyhow!("{flag_name} must be a JSON object").into()),
    }
}

fn parse_columns(input: &str) -> Result<Vec<DbTableColumnInput>> {
    serde_json::from_str(input).map_err(|err| anyhow!("invalid JSON for --columns: {err}").into())
}

pub async fn run(ctx: &Context, command: DbCommand) -> Result<()> {
    match command {
        DbCommand::Schema(args) => {
            let schema = ctx
                .client
                .db()
                .schema(
                    &args.project_id,
                    &DbSchemaOptions {
                        branch: args.branch,
                        include_views: Some(args.include_views),
                        with_row_counts: Some(args.with_row_counts),
                        include_system_schemas: Some(args.include_system_schemas),
                    },
                )
                .await?;
            ctx.present(
                DbSchemaView { schema: &schema },
                &DbSchemaJson { schema: &schema },
            )
        }
        DbCommand::Data(args) => {
            let data = ctx
                .client
                .db()
                .data(
                    &args.project_id,
                    &DbDataOptions {
                        branch: args.branch,
                        limit_per_table: Some(args.limit_per_table),
                        include_views: Some(args.include_views),
                        include_system_schemas: Some(args.include_system_schemas),
                    },
                )
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
            let branches = ctx.client.db().branches(&args.project_id).await?;
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
                let result = ctx
                    .client
                    .db()
                    .create_table(
                        &args.project_id,
                        &DbCreateTableRequest {
                            schema_name: Some(args.schema),
                            table_name: args.table,
                            columns,
                            primary_key: if args.primary_key.is_empty() {
                                None
                            } else {
                                Some(args.primary_key)
                            },
                            branch: args.branch,
                            create_schema_if_missing: Some(args.create_schema_if_missing),
                        },
                    )
                    .await?;
                ctx.success("Table created");
                ctx.present(
                    DbMessageView {
                        title: "Create table",
                        result: &result,
                    },
                    &DbMessageJson { result: &result },
                )
            }
            TableCommand::Rename(args) => {
                let result = ctx
                    .client
                    .db()
                    .rename_table(
                        &args.project_id,
                        &args.schema,
                        &args.table,
                        &DbRenameTableRequest {
                            new_table_name: args.new_table_name,
                            branch: args.branch,
                        },
                    )
                    .await?;
                ctx.success("Table renamed");
                ctx.present(
                    DbMessageView {
                        title: "Rename table",
                        result: &result,
                    },
                    &DbMessageJson { result: &result },
                )
            }
            TableCommand::Drop(args) => {
                let result = ctx
                    .client
                    .db()
                    .drop_table(
                        &args.project_id,
                        &args.schema,
                        &args.table,
                        args.branch.as_deref(),
                        Some(args.cascade),
                    )
                    .await?;
                ctx.success("Table dropped");
                ctx.present(
                    DbMessageView {
                        title: "Drop table",
                        result: &result,
                    },
                    &DbMessageJson { result: &result },
                )
            }
        },
        DbCommand::Rows { command } => match command {
            RowCommand::List(args) => {
                let rows = ctx
                    .client
                    .db()
                    .list_rows(
                        &args.project_id,
                        &args.schema,
                        &args.table,
                        &DbRowsOptions {
                            branch: args.branch,
                            limit: Some(args.limit),
                            offset: Some(args.offset),
                        },
                    )
                    .await?;
                ctx.present(DbRowsView { rows: &rows }, &DbRowsJson { rows: &rows })
            }
            RowCommand::Insert(args) => {
                let values = parse_json_map(&args.values, "--values")?;
                let result = ctx
                    .client
                    .db()
                    .insert_row(
                        &args.project_id,
                        &args.schema,
                        &args.table,
                        &DbValuesRequest {
                            values,
                            branch: args.branch,
                        },
                    )
                    .await?;
                ctx.success("Row insert requested");
                ctx.present(
                    DbAffectedView {
                        title: "Insert row",
                        result: &result,
                    },
                    &DbAffectedJson { result: &result },
                )
            }
            RowCommand::Update(args) => {
                let where_ = parse_json_map(&args.where_json, "--where-json")?;
                let values = parse_json_map(&args.values, "--values")?;
                let result = ctx
                    .client
                    .db()
                    .update_rows(
                        &args.project_id,
                        &args.schema,
                        &args.table,
                        &DbUpdateRowsRequest {
                            where_,
                            values,
                            branch: args.branch,
                        },
                    )
                    .await?;
                ctx.success("Row update requested");
                ctx.present(
                    DbAffectedView {
                        title: "Update rows",
                        result: &result,
                    },
                    &DbAffectedJson { result: &result },
                )
            }
            RowCommand::Delete(args) => {
                let where_ = parse_json_map(&args.where_json, "--where-json")?;
                let result = ctx
                    .client
                    .db()
                    .delete_rows(
                        &args.project_id,
                        &args.schema,
                        &args.table,
                        &DbDeleteRowsRequest {
                            where_,
                            branch: args.branch,
                        },
                    )
                    .await?;
                ctx.success("Row delete requested");
                ctx.present(
                    DbAffectedView {
                        title: "Delete rows",
                        result: &result,
                    },
                    &DbAffectedJson { result: &result },
                )
            }
        },
        DbCommand::Token { command } => match command {
            TokenCommand::Issue(args) => {
                let result = ctx
                    .client
                    .db()
                    .issue_project_token(
                        &args.project_id,
                        &ProjectTokenRequest {
                            scopes: if args.scopes.is_empty() {
                                None
                            } else {
                                Some(args.scopes)
                            },
                            ttl_seconds: Some(args.ttl_seconds),
                        },
                    )
                    .await?;
                ctx.success(format!("Issued project token for {}", args.project_id));
                ctx.present(
                    ProjectTokenView {
                        title: "Project token",
                        result: &result,
                    },
                    &ProjectTokenJson { token: &result },
                )
            }
            TokenCommand::Refresh(args) => {
                let result = ctx
                    .client
                    .db()
                    .refresh_project_token(
                        &args.project_id,
                        &ProjectTokenRequest {
                            scopes: if args.scopes.is_empty() {
                                None
                            } else {
                                Some(args.scopes)
                            },
                            ttl_seconds: Some(args.ttl_seconds),
                        },
                    )
                    .await?;
                ctx.success(format!("Refreshed project token for {}", args.project_id));
                ctx.present(
                    ProjectTokenView {
                        title: "Project token",
                        result: &result,
                    },
                    &ProjectTokenJson { token: &result },
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
