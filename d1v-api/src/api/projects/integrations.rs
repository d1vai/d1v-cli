use serde::{Deserialize, Serialize};

use crate::{Client, Error};

use super::types::Project;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationResponse {
    pub project: Project,
    pub message: String,
}

pub struct ProjectIntegrations {
    client: Client,
    project_id: String,
}

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
}
