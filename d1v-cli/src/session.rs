use clap::{Args, Subcommand, ValueEnum};
use d1v_api::api::projects::{
    ChatHistory, Direction, Engine, ExecuteSessionResponse, MessageType, RuntimeSession,
    SessionType,
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
    pub engine: Option<EngineArg>,
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
    pub engine: Option<EngineArg>,
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
    pub direction: Option<DirectionArg>,
    #[arg(long, value_delimiter = ',')]
    pub message_type: Vec<MessageTypeArg>,
}

#[derive(Args)]
pub struct CancelArgs {
    pub session_id: String,
}

#[derive(ValueEnum, Clone, Copy)]
pub enum DirectionArg {
    User,
    Assistant,
    System,
}

impl From<DirectionArg> for Direction {
    fn from(v: DirectionArg) -> Self {
        match v {
            DirectionArg::User => Self::User,
            DirectionArg::Assistant => Self::Assistant,
            DirectionArg::System => Self::System,
        }
    }
}

#[derive(ValueEnum, Clone, Copy)]
pub enum MessageTypeArg {
    Prompt,
    #[value(name = "git_commit")]
    GitCommit,
    Result,
    Complete,
    Cancelled,
    Error,
}

impl From<MessageTypeArg> for MessageType {
    fn from(v: MessageTypeArg) -> Self {
        match v {
            MessageTypeArg::Prompt => Self::Prompt,
            MessageTypeArg::GitCommit => Self::GitCommit,
            MessageTypeArg::Result => Self::Result,
            MessageTypeArg::Complete => Self::Complete,
            MessageTypeArg::Cancelled => Self::Cancelled,
            MessageTypeArg::Error => Self::Error,
        }
    }
}

#[derive(ValueEnum, Clone, Copy)]
pub enum EngineArg {
    Claude,
    Codex,
}

impl From<EngineArg> for Engine {
    fn from(v: EngineArg) -> Self {
        match v {
            EngineArg::Claude => Self::Claude,
            EngineArg::Codex => Self::Codex,
        }
    }
}

#[derive(Debug, Serialize)]
struct SessionResponseJson<'a> {
    response: &'a ExecuteSessionResponse,
}

#[derive(Debug, Serialize)]
struct SessionStatusJson<'a> {
    session: &'a Option<RuntimeSession>,
}

#[derive(Debug, Serialize)]
struct SessionHistoryJson<'a> {
    history: &'a [ChatHistory],
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
            field_opt(
                "Created",
                self.response
                    .session
                    .created_at
                    .map(|t| t.strftime("%Y-%m-%d %H:%M:%S").to_string())
                    .as_deref(),
            ),
            field("WebSocket", &self.response.websocket_url),
        ])
        .indent(2)
        .render(ctx)
    }
}

struct SessionStatusView<'a> {
    session: &'a Option<RuntimeSession>,
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
                    field_opt(
                        "Created",
                        session
                            .created_at
                            .map(|t| t.strftime("%Y-%m-%d %H:%M:%S").to_string())
                            .as_deref(),
                    ),
                    field_opt(
                        "Updated",
                        session
                            .updated_at
                            .map(|t| t.strftime("%Y-%m-%d %H:%M:%S").to_string())
                            .as_deref(),
                    ),
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
    entries: &'a [ChatHistory],
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
                entry.direction.to_string(),
                entry
                    .message_type
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
                entry.created_at.strftime("%Y-%m-%d %H:%M:%S").to_string(),
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
                .projects()
                .project(&args.project_id)
                .execute_session(&args.prompt)
                .session_type(SessionType::New)
                .maybe_model(args.model.as_deref())
                .maybe_engine(args.engine.map(Into::into))
                .maybe_auto_deploy(args.auto_deploy)
                .call()
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
                .projects()
                .project(&args.project_id)
                .execute_session(&args.prompt)
                .session_type(SessionType::Continue)
                .maybe_session_id(args.session_id.as_deref())
                .maybe_model(args.model.as_deref())
                .maybe_engine(args.engine.map(Into::into))
                .maybe_auto_deploy(args.auto_deploy)
                .call()
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
            let session = ctx
                .client
                .projects()
                .project(&args.project_id)
                .active_session()
                .await?;
            ctx.present(
                SessionStatusView { session: &session },
                &SessionStatusJson { session: &session },
            )
        }
        SessionCommand::History(args) => {
            let history = ctx
                .client
                .projects()
                .project(&args.project_id)
                .history()
                .limit(args.limit)
                .maybe_direction(args.direction.map(Into::into))
                .maybe_message_type(
                    (!args.message_type.is_empty())
                        .then(|| args.message_type.into_iter().map(Into::into).collect()),
                )
                .include_payload(args.include_payload)
                .call()
                .await?;
            ctx.present(
                SessionHistoryView { entries: &history },
                &SessionHistoryJson { history: &history },
            )
        }
        SessionCommand::Cancel(args) => {
            let result = ctx
                .client
                .projects()
                .cancel_session(&args.session_id)
                .await?;
            ctx.success(format!("Cancel requested for {}", args.session_id));
            ctx.output.present(Text::from("cancel requested"), &result)
        }
    }
}
