use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{Client, Error};

pub struct SessionApi {
    client: Client,
}

impl Client {
    #[must_use]
    pub fn session(&self) -> SessionApi {
        SessionApi {
            client: self.clone(),
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectSession {
    pub id: Option<i64>,
    pub project_id: String,
    pub opcode_project_id: Option<String>,
    pub opcode_project_path: Option<String>,
    pub session_id: String,
    pub model: Option<String>,
    pub status: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub websocket_url: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChatHistoryEntry {
    pub id: i64,
    pub project_id: String,
    pub direction: String,
    pub message_type: Option<String>,
    pub message_text: Option<String>,
    pub payload: Option<serde_json::Value>,
    pub created_at: String,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecuteSessionRequest {
    pub prompt: String,
    pub session_type: Option<String>,
    pub session_id: Option<String>,
    pub model: Option<String>,
    pub engine: Option<String>,
    pub system_prompt: Option<String>,
    pub auto_deploy: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecuteSessionResponse {
    pub session_id: String,
    pub websocket_url: String,
    pub session: ProjectSession,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize)]
pub struct HistoryOptions {
    pub limit: Option<u32>,
    pub before_ts: Option<String>,
    pub before_id: Option<i64>,
    pub direction: Option<String>,
    pub message_type: Option<String>,
    pub include_payload: Option<bool>,
}

impl SessionApi {
    pub async fn execute(
        &self,
        project_id: impl AsRef<str>,
        payload: &ExecuteSessionRequest,
    ) -> Result<ExecuteSessionResponse, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/sessions/execute",
                project_id.as_ref()
            ))
            .json(payload)
            .ok()
            .await
    }

    pub async fn active(
        &self,
        project_id: impl AsRef<str>,
    ) -> Result<Option<ProjectSession>, Error> {
        self.client
            .get(format!(
                "/api/projects/{}/sessions/active",
                project_id.as_ref()
            ))
            .ok()
            .await
    }

    pub async fn history(
        &self,
        project_id: impl AsRef<str>,
        options: &HistoryOptions,
    ) -> Result<Vec<ChatHistoryEntry>, Error> {
        self.client
            .get(format!("/api/projects/{}/history", project_id.as_ref()))
            .query(options)
            .ok()
            .await
    }

    pub async fn cancel(&self, session_id: impl AsRef<str>) -> Result<serde_json::Value, Error> {
        self.client
            .post(format!(
                "/api/projects/sessions/{}/cancel",
                session_id.as_ref()
            ))
            .ok()
            .await
    }
}
