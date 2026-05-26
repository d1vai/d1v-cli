use itertools::Itertools;
use jiff::Timestamp;
use serde::{Deserialize, Serialize, Serializer};
use serde_with::{SerializeAs, skip_serializing_none};
use strum::{Display, EnumString};

use crate::time::{deserialize_optional_timestamp, deserialize_timestamp};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, EnumString, Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum SessionType {
    #[default]
    New,
    Continue,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, EnumString, Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Engine {
    #[default]
    Claude,
    Codex,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, EnumString, Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Direction {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, EnumString, Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum MessageType {
    Prompt,
    #[serde(rename = "git_commit")]
    #[strum(serialize = "git_commit")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumString, Display)]
pub enum TokenScope {
    #[serde(rename = "db:read")]
    #[strum(serialize = "db:read")]
    DbRead,
    #[serde(rename = "db:write")]
    #[strum(serialize = "db:write")]
    DbWrite,
    #[serde(rename = "migrate")]
    #[strum(to_string = "migrate")]
    Migrate,
}

pub(crate) struct CommaSeparated;

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
pub struct Session {
    pub id: Option<i64>,
    pub project_id: String,
    pub opcode_project_id: Option<String>,
    pub opcode_project_path: Option<String>,
    pub session_id: String,
    pub model: Option<String>,
    pub status: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_timestamp")]
    pub created_at: Option<Timestamp>,
    #[serde(default, deserialize_with = "deserialize_optional_timestamp")]
    pub updated_at: Option<Timestamp>,
    pub websocket_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteSessionResponse {
    pub session_id: String,
    pub websocket_url: String,
    pub session: Session,
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
    #[serde(deserialize_with = "deserialize_timestamp")]
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelSessionResponse {
    pub session_id: String,
    pub cancelled: bool,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClaudeProject {
    pub id: String,
    pub name: String,
    pub path: Option<String>,
    pub username: String,
}
