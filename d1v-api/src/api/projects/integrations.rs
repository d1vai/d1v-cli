use bon::bon;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{Client, Error};

use super::types::Project;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationResponse {
    pub project: Project,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize)]
struct EnsureProjectIntegrationsRequest {
    database: bool,
    pay: bool,
    analytics: bool,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnsureProjectIntegrationStatus {
    pub requested: bool,
    pub status: String,
    pub changed: bool,
    pub message: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnsureProjectIntegrationsResponse {
    pub project: Project,
    pub database: EnsureProjectIntegrationStatus,
    pub pay: EnsureProjectIntegrationStatus,
    pub analytics: EnsureProjectIntegrationStatus,
    #[serde(default)]
    pub errors: Vec<String>,
}

pub struct ProjectIntegrations {
    client: Client,
    project_id: String,
}

#[bon]
impl ProjectIntegrations {
    pub fn new(client: Client, project_id: String) -> Self {
        Self { client, project_id }
    }

    pub async fn activate_pay(&self) -> Result<IntegrationResponse, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/integrations/activate-pay",
                self.project_id
            ))
            .ok()
            .await
    }

    pub async fn refresh_pay_token(&self) -> Result<IntegrationResponse, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/integrations/refresh-pay-token",
                self.project_id
            ))
            .ok()
            .await
    }

    pub async fn activate_database(&self) -> Result<IntegrationResponse, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/integrations/activate-database",
                self.project_id
            ))
            .ok()
            .await
    }

    #[builder]
    pub async fn ensure(
        &self,
        database: bool,
        pay: bool,
        analytics: bool,
    ) -> Result<EnsureProjectIntegrationsResponse, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/integrations/ensure",
                self.project_id
            ))
            .json(&EnsureProjectIntegrationsRequest {
                database,
                pay,
                analytics,
            })
            .ok()
            .await
    }
}
