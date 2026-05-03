use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{Client, Error};

pub struct GitHubOpsApi {
    client: Client,
}

impl Client {
    #[must_use]
    pub fn github_ops(&self) -> GitHubOpsApi {
        GitHubOpsApi {
            client: self.clone(),
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PullWorkspaceRequest {
    pub branch: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PullWorkspaceResponse {
    pub success: Option<bool>,
    pub message: Option<String>,
    pub commit_hash: Option<String>,
    pub branch: Option<String>,
}

impl GitHubOpsApi {
    pub async fn pull_workspace(
        &self,
        project_id: impl AsRef<str>,
        payload: &PullWorkspaceRequest,
    ) -> Result<PullWorkspaceResponse, Error> {
        self.client
            .post(format!("/api/github-ops/{}/pull", project_id.as_ref()))
            .json(payload)
            .ok()
            .await
    }
}
