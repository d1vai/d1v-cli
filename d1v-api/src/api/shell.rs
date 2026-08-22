use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::encode::encode_segment;
use crate::{Client, Error};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShellTarget {
    Workspace,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShellName {
    Auto,
    Bash,
    Zsh,
    Sh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShellMode {
    Pty,
    Exec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShellSessionStatus {
    Creating,
    Ready,
    Detached,
    Exited,
    Failed,
    Terminated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShellTransport {
    Direct,
    Relay,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateShellSessionRequest {
    pub target: ShellTarget,
    pub cols: u16,
    pub rows: u16,
    pub shell: ShellName,
    pub resume_session_id: Option<String>,
    pub mode: ShellMode,
    pub argv: Option<Vec<String>>,
}

impl CreateShellSessionRequest {
    #[must_use]
    pub fn workspace(cols: u16, rows: u16) -> Self {
        Self {
            target: ShellTarget::Workspace,
            cols,
            rows,
            shell: ShellName::Auto,
            resume_session_id: None,
            mode: ShellMode::Pty,
            argv: None,
        }
    }

    #[must_use]
    pub fn project(cols: u16, rows: u16) -> Self {
        Self {
            target: ShellTarget::Project,
            ..Self::workspace(cols, rows)
        }
    }

    #[must_use]
    pub fn exec(argv: Vec<String>) -> Self {
        Self {
            target: ShellTarget::Workspace,
            mode: ShellMode::Exec,
            argv: Some(argv),
            ..Self::workspace(120, 40)
        }
    }
}

// Deliberately does not implement Debug or Serialize: connection_ticket is a secret.
#[derive(Clone, Deserialize)]
pub struct ShellConnection {
    pub session_id: String,
    pub workspace_scope: String,
    pub project_id: Option<String>,
    pub runtime_provider: String,
    pub node_id: String,
    pub cwd: String,
    pub transport: ShellTransport,
    pub websocket_url: String,
    pub connection_ticket: String,
    pub ticket_expires_at: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellSession {
    pub session_id: String,
    pub workspace_scope: String,
    pub project_id: Option<String>,
    pub runtime_provider: String,
    pub node_id: String,
    pub cwd: String,
    pub mode: ShellMode,
    pub status: ShellSessionStatus,
    pub created_at: Option<Timestamp>,
    pub connected_at: Option<Timestamp>,
    pub last_seen_at: Option<Timestamp>,
    pub ended_at: Option<Timestamp>,
    pub exit_code: Option<i32>,
    pub termination_reason: Option<String>,
    pub bytes_in: u64,
    pub bytes_out: u64,
}

pub struct ShellApi {
    client: Client,
}

impl Client {
    #[must_use]
    pub fn shell(&self) -> ShellApi {
        ShellApi {
            client: self.clone(),
        }
    }
}

impl ShellApi {
    pub async fn create_workspace(
        &self,
        organization_id: Option<u64>,
        request: &CreateShellSessionRequest,
    ) -> Result<ShellConnection, Error> {
        self.client
            .post("/api/workspace/shell-sessions")
            .query_if_some("organization_id", organization_id)
            .json(request)
            .ok()
            .await
    }

    pub async fn create_project(
        &self,
        project_id: impl AsRef<str>,
        request: &CreateShellSessionRequest,
    ) -> Result<ShellConnection, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/shell-sessions",
                encode_segment(project_id)
            ))
            .json(request)
            .ok()
            .await
    }

    pub async fn get(&self, session_id: impl AsRef<str>) -> Result<ShellSession, Error> {
        self.client
            .get(format!(
                "/api/shell-sessions/{}",
                encode_segment(session_id)
            ))
            .ok()
            .await
    }

    pub async fn refresh_ticket(
        &self,
        session_id: impl AsRef<str>,
    ) -> Result<ShellConnection, Error> {
        self.client
            .post(format!(
                "/api/shell-sessions/{}/ticket",
                encode_segment(session_id)
            ))
            .ok()
            .await
    }

    pub async fn close(&self, session_id: impl AsRef<str>) -> Result<ShellSession, Error> {
        self.client
            .delete(format!(
                "/api/shell-sessions/{}",
                encode_segment(session_id)
            ))
            .ok()
            .await
    }
}

#[cfg(test)]
mod tests {
    use httpmock::prelude::*;
    use serde_json::json;

    use super::*;

    fn connection_response() -> serde_json::Value {
        json!({
            "code": 0,
            "msg": "success",
            "data": {
                "session_id": "sh_123",
                "workspace_scope": "organization:42",
                "project_id": null,
                "runtime_provider": "fabric",
                "node_id": "node_1",
                "cwd": "/workspace-root",
                "transport": "direct",
                "websocket_url": "wss://node.example/ws/terminal/sh_123",
                "connection_ticket": "one-time-secret",
                "ticket_expires_at": "2026-08-22T12:00:30Z"
            }
        })
    }

    fn session_response() -> serde_json::Value {
        json!({
            "code": 0,
            "msg": "success",
            "data": {
                "session_id": "sh/unsafe",
                "workspace_scope": "user:7",
                "project_id": null,
                "runtime_provider": "fabric",
                "node_id": "node_1",
                "cwd": "/workspace-root",
                "mode": "pty",
                "status": "terminated",
                "created_at": "2026-08-22T12:00:00Z",
                "connected_at": null,
                "last_seen_at": null,
                "ended_at": "2026-08-22T12:00:05Z",
                "exit_code": 0,
                "termination_reason": "client_close",
                "bytes_in": 4,
                "bytes_out": 12
            }
        })
    }

    fn client(server: &MockServer) -> Client {
        Client::builder()
            .base_url(server.base_url())
            .token("test-token")
            .client_name("d1v-cli")
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn creates_organization_workspace_session() {
        let server = MockServer::start();
        let request = CreateShellSessionRequest::workspace(120, 40);
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/api/workspace/shell-sessions")
                .query_param("organization_id", "42")
                .header("authorization", "Bearer test-token")
                .header("x-d1v-client", "d1v-cli")
                .json_body(json!({
                    "target": "workspace",
                    "cols": 120,
                    "rows": 40,
                    "shell": "auto",
                    "resume_session_id": null,
                    "mode": "pty",
                    "argv": null
                }));
            then.status(200).json_body(connection_response());
        });

        let connection = client(&server)
            .shell()
            .create_workspace(Some(42), &request)
            .await
            .unwrap();

        mock.assert();
        assert_eq!(connection.session_id, "sh_123");
        assert_eq!(connection.connection_ticket, "one-time-secret");
        assert_eq!(connection.transport, ShellTransport::Direct);
    }

    #[tokio::test]
    async fn encodes_project_and_session_path_segments() {
        let server = MockServer::start();
        let project_mock = server.mock(|when, then| {
            when.method(POST)
                .path("/api/projects/project%2Funsafe/shell-sessions");
            then.status(200).json_body(connection_response());
        });
        let close_mock = server.mock(|when, then| {
            when.method(DELETE).path("/api/shell-sessions/sh%2Funsafe");
            then.status(200).json_body(session_response());
        });
        let api = client(&server).shell();

        api.create_project(
            "project/unsafe",
            &CreateShellSessionRequest::project(80, 24),
        )
        .await
        .unwrap();
        let closed = api.close("sh/unsafe").await.unwrap();

        project_mock.assert();
        close_mock.assert();
        assert_eq!(closed.status, ShellSessionStatus::Terminated);
        assert_eq!(closed.ended_at.unwrap().to_string(), "2026-08-22T12:00:05Z");
    }

    #[tokio::test]
    async fn creates_exec_session_with_argv() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/api/workspace/shell-sessions")
                .json_body(json!({
                    "target": "workspace",
                    "cols": 120,
                    "rows": 40,
                    "shell": "auto",
                    "resume_session_id": null,
                    "mode": "exec",
                    "argv": ["git", "status", "--short"]
                }));
            then.status(200).json_body(connection_response());
        });

        client(&server)
            .shell()
            .create_workspace(
                None,
                &CreateShellSessionRequest::exec(vec![
                    "git".into(),
                    "status".into(),
                    "--short".into(),
                ]),
            )
            .await
            .unwrap();

        mock.assert();
    }
}
