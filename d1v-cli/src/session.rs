use clap::{Args, Subcommand};
use d1v_api::{
    ChatHistoryEntry, ExecuteSessionRequest, ExecuteSessionResponse, HistoryOptions, ProjectSession,
};
use serde::Serialize;

use crate::Context;
use crate::error::Result;
use crate::text::{Field, Fields, Line, Span, Table, TableRow, Text};
use crate::theme;

#[derive(Subcommand)]
pub enum SessionCommand {
    /// Start a new AI development session
    Run(RunArgs),
    /// Continue an existing or active session
    Continue(ContinueArgs),
    /// Show the latest active session for a project
    Status(StatusArgs),
    /// Show project chat/session history
    History(HistoryArgs),
    /// Cancel a session by session ID
    Cancel(CancelArgs),
}

#[derive(Args)]
pub struct RunArgs {
    pub project_id: String,
    #[arg(long)]
    pub prompt: String,
    #[arg(long)]
    pub model: Option<String>,
    #[arg(long)]
    pub engine: Option<String>,
    #[arg(long)]
    pub auto_deploy: Option<bool>,
}

#[derive(Args)]
pub struct ContinueArgs {
    pub project_id: String,
    #[arg(long)]
    pub prompt: String,
    #[arg(long)]
    pub session_id: Option<String>,
    #[arg(long)]
    pub model: Option<String>,
    #[arg(long)]
    pub engine: Option<String>,
    #[arg(long)]
    pub auto_deploy: Option<bool>,
}

#[derive(Args)]
pub struct StatusArgs {
    pub project_id: String,
}

#[derive(Args)]
pub struct HistoryArgs {
    pub project_id: String,
    #[arg(long, default_value_t = 20)]
    pub limit: u32,
    #[arg(long)]
    pub include_payload: bool,
    #[arg(long)]
    pub direction: Option<String>,
    #[arg(long)]
    pub message_type: Option<String>,
}

#[derive(Args)]
pub struct CancelArgs {
    pub session_id: String,
}

#[derive(Debug, Serialize)]
struct SessionResponseJson<'a> {
    response: &'a ExecuteSessionResponse,
}

#[derive(Debug, Serialize)]
struct SessionStatusJson<'a> {
    session: &'a Option<ProjectSession>,
}

#[derive(Debug, Serialize)]
struct SessionHistoryJson<'a> {
    history: &'a [ChatHistoryEntry],
}

struct SessionResponseView<'a> {
    title: &'a str,
    response: &'a ExecuteSessionResponse,
}

impl crate::text::Render for SessionResponseView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        Text::new()
            .line(
                Line::styled(self.title.to_string(), theme::ansi::success())
                    .push_plain(" ")
                    .push_styled(self.response.session_id.clone(), theme::ansi::plain()),
            )
            .render(ctx)?;

        Fields::new([
            field("Project", &self.response.session.project_id),
            field_opt("Model", self.response.session.model.as_deref()),
            field_opt("Status", self.response.session.status.as_deref()),
            field("WebSocket", &self.response.websocket_url),
        ])
        .indent(2)
        .render(ctx)
    }
}

struct SessionStatusView<'a> {
    session: &'a Option<ProjectSession>,
}

impl crate::text::Render for SessionStatusView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        match self.session {
            Some(session) => {
                Text::new()
                    .line(
                        Line::styled("Active session".to_string(), theme::ansi::success())
                            .push_plain(" ")
                            .push_styled(session.session_id.clone(), theme::ansi::plain()),
                    )
                    .render(ctx)?;
                Fields::new([
                    field("Project", &session.project_id),
                    field_opt("Model", session.model.as_deref()),
                    field_opt("Status", session.status.as_deref()),
                    field_opt("Created", session.created_at.as_deref()),
                    field_opt("Updated", session.updated_at.as_deref()),
                ])
                .indent(2)
                .render(ctx)
            }
            None => Text::new()
                .line(Line::styled("No active session.", theme::ansi::dim()))
                .render(ctx),
        }
    }
}

struct SessionHistoryView<'a> {
    entries: &'a [ChatHistoryEntry],
}

impl crate::text::Render for SessionHistoryView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        if self.entries.is_empty() {
            return Text::new()
                .line(Line::styled("No history found.", theme::ansi::dim()))
                .render(ctx);
        }

        let rows = self.entries.iter().map(|entry| {
            let text = entry
                .message_text
                .as_deref()
                .unwrap_or("")
                .chars()
                .take(72)
                .collect::<String>();
            TableRow::new([
                entry.id.to_string(),
                entry.direction.clone(),
                entry
                    .message_type
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
                entry.created_at.clone(),
                if text.is_empty() {
                    "-".to_string()
                } else {
                    text
                },
            ])
        });

        Table::new(rows)
            .header(TableRow::new([
                "id",
                "direction",
                "type",
                "created_at",
                "text",
            ]))
            .border_style(theme::ansi::border())
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

pub async fn run(ctx: &Context, command: SessionCommand) -> Result<()> {
    match command {
        SessionCommand::Run(args) => {
            let response = ctx
                .client
                .session()
                .execute(
                    &args.project_id,
                    &ExecuteSessionRequest {
                        prompt: args.prompt,
                        session_type: Some("new".to_string()),
                        session_id: None,
                        model: args.model,
                        engine: args.engine,
                        system_prompt: None,
                        auto_deploy: args.auto_deploy,
                    },
                )
                .await?;
            ctx.success(format!("Started session {}", response.session_id));
            ctx.present(
                SessionResponseView {
                    title: "Started session",
                    response: &response,
                },
                &SessionResponseJson {
                    response: &response,
                },
            )
        }
        SessionCommand::Continue(args) => {
            let response = ctx
                .client
                .session()
                .execute(
                    &args.project_id,
                    &ExecuteSessionRequest {
                        prompt: args.prompt,
                        session_type: Some("continue".to_string()),
                        session_id: args.session_id,
                        model: args.model,
                        engine: args.engine,
                        system_prompt: None,
                        auto_deploy: args.auto_deploy,
                    },
                )
                .await?;
            ctx.success(format!("Continued session {}", response.session_id));
            ctx.present(
                SessionResponseView {
                    title: "Continued session",
                    response: &response,
                },
                &SessionResponseJson {
                    response: &response,
                },
            )
        }
        SessionCommand::Status(args) => {
            let session = ctx.client.session().active(&args.project_id).await?;
            ctx.present(
                SessionStatusView { session: &session },
                &SessionStatusJson { session: &session },
            )
        }
        SessionCommand::History(args) => {
            let history = ctx
                .client
                .session()
                .history(
                    &args.project_id,
                    &HistoryOptions {
                        limit: Some(args.limit),
                        before_ts: None,
                        before_id: None,
                        direction: args.direction,
                        message_type: args.message_type,
                        include_payload: Some(args.include_payload),
                    },
                )
                .await?;
            ctx.present(
                SessionHistoryView { entries: &history },
                &SessionHistoryJson { history: &history },
            )
        }
        SessionCommand::Cancel(args) => {
            let result = ctx.client.session().cancel(&args.session_id).await?;
            ctx.success(format!("Cancel requested for {}", args.session_id));
            ctx.output.present(Text::from("cancel requested"), &result)
        }
    }
}
