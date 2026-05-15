use itertools::Itertools;
use jiff::Timestamp;
use serde::{Deserialize, Serialize, Serializer};
use serde_with::{SerializeAs, serde_as, skip_serializing_none};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionType {
    #[default]
    New,
    Continue,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Engine {
    #[default]
    Claude,
    Codex,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Prompt,
    #[serde(rename = "git_commit")]
    GitCommit,
    Result,
    Complete,
    Cancelled,
    Error,
}

impl MessageType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Prompt => "prompt",
            Self::GitCommit => "git_commit",
            Self::Result => "result",
            Self::Complete => "complete",
            Self::Cancelled => "cancelled",
            Self::Error => "error",
        }
    }
}

struct CommaSeparated;

impl SerializeAs<Vec<MessageType>> for CommaSeparated {
    fn serialize_as<S: Serializer>(
        source: &Vec<MessageType>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let s = source.iter().map(MessageType::as_str).join(",");
        serializer.serialize_str(&s)
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecuteSession {
    pub prompt: String,
    pub session_type: Option<SessionType>,
    pub session_id: Option<String>,
    pub model: Option<String>,
    pub engine: Option<Engine>,
    pub system_prompt: Option<String>,
    pub project_path: Option<String>,
    pub auto_deploy: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuntimeSession {
    pub id: Option<i64>,
    pub project_id: String,
    pub opcode_project_id: Option<String>,
    pub opcode_project_path: Option<String>,
    pub session_id: String,
    pub model: Option<String>,
    pub status: Option<String>,
    pub created_at: Option<Timestamp>,
    pub updated_at: Option<Timestamp>,
    pub websocket_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteSessionResponse {
    pub session_id: String,
    pub websocket_url: String,
    pub session: RuntimeSession,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize)]
pub struct HistoryOptions {
    pub limit: Option<u32>,
    pub before_ts: Option<Timestamp>,
    pub before_id: Option<i64>,
    pub direction: Option<Direction>,
    #[serde_as(as = "Option<CommaSeparated>")]
    pub message_type: Option<Vec<MessageType>>,
    pub include_payload: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatHistory {
    pub id: i64,
    pub project_id: String,
    pub direction: Direction,
    pub message_type: Option<String>,
    pub message_text: Option<String>,
    #[serde(default)]
    pub payload: serde_json::Value,
    pub created_at: Timestamp,
}

pub type CancelSessionResponse = serde_json::Value;

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClaudeProject {
    pub id: String,
    pub name: String,
    pub path: Option<String>,
    pub username: String,
}
