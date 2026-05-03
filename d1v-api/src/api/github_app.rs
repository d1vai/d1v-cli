use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{Client, Error};

use super::project::UserProject;

pub struct GitHubAppApi {
    client: Client,
}

impl Client {
    #[must_use]
    pub fn github_app(&self) -> GitHubAppApi {
        GitHubAppApi {
            client: self.clone(),
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubAppStatus {
    pub configured: bool,
    pub connected: bool,
    pub token_valid: bool,
    pub github_login: Option<String>,
    pub github_name: Option<String>,
    pub github_avatar_url: Option<String>,
    pub app_slug: Option<String>,
    pub app_install_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubAppConnectUrl {
    pub url: String,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubAppInstallation {
    pub id: u64,
    pub account_login: Option<String>,
    pub account_type: Option<String>,
    pub target_type: Option<String>,
    pub repository_selection: Option<String>,
    pub html_url: Option<String>,
    pub permissions: Option<BTreeMap<String, String>>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubAppRepository {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub owner: Option<String>,
    pub clone_url: String,
    pub ssh_url: Option<String>,
    pub default_branch: String,
    pub is_private: bool,
    pub description: Option<String>,
    pub language: Option<String>,
    pub permissions: Option<BTreeMap<String, bool>>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubImportAutoDeploy {
    pub is_deployable: Option<bool>,
    pub framework: Option<String>,
    pub auto_deploy_queued: Option<bool>,
    pub reason: Option<String>,
    pub monorepo_candidates: Option<Vec<String>>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubImportResponse {
    pub project: UserProject,
    pub import_auto_deploy: Option<GitHubImportAutoDeploy>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitHubImportRequest {
    pub installation_id: u64,
    pub repository_id: u64,
    pub project_name: Option<String>,
    pub project_description: Option<String>,
}

impl GitHubAppApi {
    pub async fn status(&self) -> Result<GitHubAppStatus, Error> {
        self.client.get("/api/github-app/status").ok().await
    }

    pub async fn connect_url(
        &self,
        redirect_to: Option<&str>,
    ) -> Result<GitHubAppConnectUrl, Error> {
        self.client
            .get("/api/github-app/connect-url")
            .query_if_some("redirect_to", redirect_to)
            .ok()
            .await
    }

    pub async fn list_installations(&self) -> Result<Vec<GitHubAppInstallation>, Error> {
        self.client.get("/api/github-app/installations").ok().await
    }

    pub async fn list_repositories(
        &self,
        installation_id: u64,
    ) -> Result<Vec<GitHubAppRepository>, Error> {
        self.client
            .get("/api/github-app/repositories")
            .query_if_some("installation_id", Some(installation_id))
            .ok()
            .await
    }

    pub async fn import(
        &self,
        payload: &GitHubImportRequest,
    ) -> Result<GitHubImportResponse, Error> {
        self.client
            .post("/api/github-app/import")
            .json(payload)
            .ok()
            .await
    }
}
