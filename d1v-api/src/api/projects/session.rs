use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecuteProjectSession {
    pub prompt: String,
    pub session_type: Option<String>,
    pub session_id: Option<String>,
    pub model: Option<String>,
    pub engine: Option<String>,
    pub system_prompt: Option<String>,
    pub project_path: Option<String>,
    pub auto_deploy: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectRuntimeSession {
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
pub struct ExecuteProjectSessionResponse {
    pub session_id: String,
    pub websocket_url: String,
    pub session: ProjectRuntimeSession,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize)]
pub struct ProjectHistoryOptions {
    pub limit: Option<u32>,
    pub before_ts: Option<Timestamp>,
    pub before_id: Option<i64>,
    pub direction: Option<String>,
    pub message_type: Option<String>,
    pub include_payload: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectChatHistory {
    pub id: i64,
    pub project_id: String,
    pub direction: String,
    pub message_type: Option<String>,
    pub message_text: Option<String>,
    #[serde(default)]
    pub payload: serde_json::Value,
    pub created_at: Timestamp,
}

pub type CancelProjectSessionResponse = serde_json::Value;

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClaudeUserProject {
    pub id: String,
    pub name: String,
    pub path: Option<String>,
    pub username: String,
}
