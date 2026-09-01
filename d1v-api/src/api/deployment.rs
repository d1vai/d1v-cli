use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{Client, Error};
use serde_json::Value;

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReleasePreflight {
    pub first_release: Option<bool>,
    pub database_summary: Option<String>,
    pub environment_variables: Vec<ReleaseEnvironmentVariable>,
    pub recommended_action: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReleaseEnvironmentVariable {
    pub key: String,
    pub has_development_value: Option<bool>,
    pub has_production_value: Option<bool>,
    pub needs_value: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProductionRelease {
    pub id: Option<String>,
    pub status: String,
    pub phase: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub production_url: Option<String>,
    pub deployment_id: Option<String>,
    pub details: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateReleaseRequest {
    pub idempotency_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment_decisions: Option<Value>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize)]
pub struct DeploymentListOptions {
    pub environment: Option<String>,
    pub limit: Option<u32>,
}

impl DeploymentApi {
    pub async fn get_release_preflight(
        &self,
        project_id: impl AsRef<str>,
    ) -> Result<ReleasePreflight, Error> {
        self.client
            .get(format!(
                "/api/deployment/{}/release-preflight",
                project_id.as_ref()
            ))
            .ok()
            .await
    }

    pub async fn create_production_release(
        &self,
        project_id: impl AsRef<str>,
        request: &CreateReleaseRequest,
    ) -> Result<ProductionRelease, Error> {
        self.client
            .post(format!("/api/deployment/{}/releases", project_id.as_ref()))
            .json(request)
            .ok()
            .await
    }

    pub async fn get_production_release(
        &self,
        project_id: impl AsRef<str>,
        release_id: impl AsRef<str>,
    ) -> Result<ProductionRelease, Error> {
        self.client
            .get(format!(
                "/api/deployment/{}/releases/{}",
                project_id.as_ref(),
                release_id.as_ref()
            ))
            .ok()
            .await
    }

    pub async fn retry_production_release(
        &self,
        project_id: impl AsRef<str>,
        release_id: impl AsRef<str>,
        idempotency_key: impl AsRef<str>,
    ) -> Result<ProductionRelease, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/releases/{}/retry",
                project_id.as_ref(),
                release_id.as_ref()
            ))
            .json(&serde_json::json!({"idempotency_key": idempotency_key.as_ref()}))
            .ok()
            .await
    }

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
