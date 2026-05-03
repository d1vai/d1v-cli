use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{Client, Error};

pub struct DeploymentApi {
    client: Client,
}

impl Client {
    #[must_use]
    pub fn deployment(&self) -> DeploymentApi {
        DeploymentApi {
            client: self.clone(),
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeploymentResponse {
    pub success: bool,
    pub message: String,
    pub commit_hash: Option<String>,
    pub production_url: Option<String>,
    pub vercel_url: Option<String>,
    pub deployment_id: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeploymentInfo {
    pub id: String,
    pub url: String,
    pub status: String,
    pub environment: String,
    pub created_at: String,
    pub commit_hash: Option<String>,
    pub vercel_deployment_id: Option<String>,
    pub vercel_deployment_url: Option<String>,
    pub vercel_project_id: Option<String>,
    pub vercel_domain: Option<String>,
    pub vercel_framework: Option<String>,
    pub git_branch: Option<String>,
    pub git_commit_message: Option<String>,
    pub git_commit_author: Option<String>,
    pub deployed_by: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub deployment_duration_seconds: Option<i64>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeploymentListResponse {
    pub deployments: Vec<DeploymentInfo>,
    pub total: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeploymentLogsResponse {
    pub build_log: String,
    pub from_cache: bool,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize)]
pub struct DeploymentListOptions {
    pub environment: Option<String>,
    pub limit: Option<u32>,
}

impl DeploymentApi {
    pub async fn preview(&self, project_id: impl AsRef<str>) -> Result<DeploymentResponse, Error> {
        self.client
            .post(format!("/api/deployment/{}/preview", project_id.as_ref()))
            .ok()
            .await
    }

    pub async fn production(
        &self,
        project_id: impl AsRef<str>,
    ) -> Result<DeploymentResponse, Error> {
        self.client
            .post(format!(
                "/api/deployment/{}/production",
                project_id.as_ref()
            ))
            .ok()
            .await
    }

    pub async fn preview_status(
        &self,
        project_id: impl AsRef<str>,
    ) -> Result<DeploymentResponse, Error> {
        self.client
            .get(format!(
                "/api/deployment/{}/preview/status",
                project_id.as_ref()
            ))
            .ok()
            .await
    }

    pub async fn history(
        &self,
        project_id: impl AsRef<str>,
        options: &DeploymentListOptions,
    ) -> Result<DeploymentListResponse, Error> {
        self.client
            .get(format!(
                "/api/deployment/{}/deployments",
                project_id.as_ref()
            ))
            .query(options)
            .ok()
            .await
    }

    pub async fn logs(
        &self,
        vercel_deployment_id: impl AsRef<str>,
    ) -> Result<DeploymentLogsResponse, Error> {
        self.client
            .get(format!(
                "/api/deployment/logs/{}",
                vercel_deployment_id.as_ref()
            ))
            .ok()
            .await
    }
}
