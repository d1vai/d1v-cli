use bon::Builder;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{Client, Error};

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnvVar {
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
#[derive(Debug, Clone, Default, Serialize, Builder)]
pub struct CreateEnvVar {
    #[builder(into)]
    pub key: String,
    #[builder(into)]
    pub value: String,
    #[builder(into)]
    pub description: Option<String>,
    pub is_sensitive: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Builder)]
pub struct UpdateEnvVar {
    #[builder(into)]
    pub value: Option<String>,
    #[builder(into)]
    pub description: Option<String>,
    pub is_sensitive: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportEnvVarsResponse {
    pub message: String,
    pub created: i64,
    pub updated: i64,
    pub skipped: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportEnvVarsResponse {
    pub content: String,
    pub filename: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEnvVarsResponse {
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

    pub async fn vars(&self, show_values: bool) -> Result<Vec<EnvVar>, Error> {
        #[derive(Serialize)]
        struct Query {
            show_values: bool,
        }

        self.client
            .get(format!("/api/projects/{}/env-vars", self.project_id))
            .query(&Query { show_values })
            .ok()
            .await
    }

    pub async fn create_var(&self, payload: &CreateEnvVar) -> Result<EnvVar, Error> {
        self.client
            .post(format!("/api/projects/{}/env-vars", self.project_id))
            .json(payload)
            .ok()
            .await
    }

    pub async fn update_var(
        &self,
        env_var_id: i64,
        payload: &UpdateEnvVar,
    ) -> Result<EnvVar, Error> {
        self.client
            .patch(format!(
                "/api/projects/{}/env-vars/{}",
                self.project_id, env_var_id
            ))
            .json(payload)
            .ok()
            .await
    }

    /// Deletes an environment variable. Returns the server message.
    pub async fn delete_var(&self, env_var_id: i64) -> Result<String, Error> {
        #[derive(Deserialize)]
        struct MsgResult {
            message: String,
        }

        self.client
            .delete(format!(
                "/api/projects/{}/env-vars/{}",
                self.project_id, env_var_id
            ))
            .ok::<MsgResult>()
            .await
            .map(|r| r.message)
    }

    pub async fn import_vars(
        &self,
        env_content: impl AsRef<str>,
        overwrite: bool,
    ) -> Result<ImportEnvVarsResponse, Error> {
        #[derive(Serialize)]
        struct Payload<'a> {
            env_content: &'a str,
            overwrite: bool,
        }

        self.client
            .post(format!(
                "/api/projects/{}/env-vars/batch-import",
                self.project_id
            ))
            .json(&Payload {
                env_content: env_content.as_ref(),
                overwrite,
            })
            .ok()
            .await
    }

    pub async fn export_vars(&self) -> Result<ExportEnvVarsResponse, Error> {
        self.client
            .get(format!("/api/projects/{}/env-vars/export", self.project_id))
            .ok()
            .await
    }

    pub async fn sync_vercel(&self) -> Result<SyncEnvVarsResponse, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/env-vars/sync-vercel",
                self.project_id
            ))
            .ok()
            .await
    }
}
