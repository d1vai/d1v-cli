use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{Client, Error};

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize)]
pub struct ProjectEnvVarsOptions {
    pub show_values: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectEnvVar {
    pub id: i64,
    pub key: String,
    pub value: Option<String>,
    pub value_preview: String,
    pub description: Option<String>,
    pub is_sensitive: bool,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize)]
pub struct CreateProjectEnvVar {
    pub key: String,
    pub value: String,
    pub description: Option<String>,
    pub is_sensitive: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize)]
pub struct UpdateProjectEnvVar {
    pub value: Option<String>,
    pub description: Option<String>,
    pub is_sensitive: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImportProjectEnvVars {
    pub env_content: String,
    pub overwrite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteProjectEnvVarResponse {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProjectEnvVarsResponse {
    pub message: String,
    pub created: i64,
    pub updated: i64,
    pub skipped: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportProjectEnvVarsResponse {
    pub content: String,
    pub filename: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncProjectEnvVarsResponse {
    pub message: String,
    pub vercel_dev_project_id: String,
    pub vercel_prod_project_id: String,
    pub dev_local_env_count: i64,
    pub prod_local_env_count: i64,
    pub dev_up_to_date: bool,
    pub prod_up_to_date: bool,
}

pub struct ProjectEnv {
    client: Client,
    project_id: String,
}

impl ProjectEnv {
    pub fn new(client: Client, project_id: String) -> Self {
        Self { client, project_id }
    }

    pub async fn vars(&self, options: &ProjectEnvVarsOptions) -> Result<Vec<ProjectEnvVar>, Error> {
        self.client
            .get(format!("/api/projects/{}/env-vars", self.project_id))
            .query(options)
            .ok()
            .await
    }

    pub async fn create_var(&self, payload: &CreateProjectEnvVar) -> Result<ProjectEnvVar, Error> {
        self.client
            .post(format!("/api/projects/{}/env-vars", self.project_id))
            .json(payload)
            .ok()
            .await
    }

    pub async fn update_var(
        &self,
        env_var_id: i64,
        payload: &UpdateProjectEnvVar,
    ) -> Result<ProjectEnvVar, Error> {
        self.client
            .patch(format!(
                "/api/projects/{}/env-vars/{}",
                self.project_id, env_var_id
            ))
            .json(payload)
            .ok()
            .await
    }

    pub async fn delete_var(&self, env_var_id: i64) -> Result<DeleteProjectEnvVarResponse, Error> {
        self.client
            .delete(format!(
                "/api/projects/{}/env-vars/{}",
                self.project_id, env_var_id
            ))
            .ok()
            .await
    }

    pub async fn import_vars(
        &self,
        payload: &ImportProjectEnvVars,
    ) -> Result<ImportProjectEnvVarsResponse, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/env-vars/batch-import",
                self.project_id
            ))
            .json(payload)
            .ok()
            .await
    }

    pub async fn export_vars(&self) -> Result<ExportProjectEnvVarsResponse, Error> {
        self.client
            .get(format!("/api/projects/{}/env-vars/export", self.project_id))
            .ok()
            .await
    }

    pub async fn sync_vercel(&self) -> Result<SyncProjectEnvVarsResponse, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/env-vars/sync-vercel",
                self.project_id
            ))
            .ok()
            .await
    }
}
