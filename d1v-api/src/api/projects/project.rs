use serde::Serialize;

use crate::{Client, Error};

use super::db::ProjectsDb;
use super::env::ProjectEnv;
use super::integrations::ProjectIntegrations;
use super::pay::ProjectPay;
use super::session::{
    ChatHistory, ExecuteSession, ExecuteSessionResponse, HistoryOptions, RuntimeSession,
};
use super::storage::ProjectStorage;
use super::types::{
    Database, Deployment, DeploymentOptions, GitMigrationStatus, Project, PublishResponse, Token,
    TokenRequest, TransferResponse, UpdateProject,
};

pub struct ProjectApi {
    client: Client,
    project_id: String,
}

impl ProjectApi {
    pub fn new(client: Client, project_id: String) -> Self {
        Self { client, project_id }
    }

    pub fn db(&self) -> ProjectsDb {
        ProjectsDb::new(self.client.clone(), self.project_id.clone())
    }

    pub fn pay(&self) -> ProjectPay {
        ProjectPay::new(self.client.clone(), self.project_id.clone())
    }

    pub fn env(&self) -> ProjectEnv {
        ProjectEnv::new(self.client.clone(), self.project_id.clone())
    }

    pub fn integrations(&self) -> ProjectIntegrations {
        ProjectIntegrations::new(self.client.clone(), self.project_id.clone())
    }

    pub fn storage(&self) -> ProjectStorage {
        ProjectStorage::new(self.client.clone(), self.project_id.clone())
    }

    pub async fn get(&self, sync: Option<bool>) -> Result<Project, Error> {
        self.client
            .get(format!("/api/projects/{}", self.project_id))
            .query_if_some("sync", sync)
            .ok()
            .await
    }

    pub async fn update(&self, payload: &UpdateProject) -> Result<Project, Error> {
        self.client
            .put(format!("/api/projects/{}", self.project_id))
            .json(payload)
            .ok()
            .await
    }

    pub async fn delete(&self) -> Result<(), Error> {
        self.client
            .delete(format!("/api/projects/{}", self.project_id))
            .ok()
            .await
    }

    pub async fn database(&self) -> Result<Database, Error> {
        self.client
            .get(format!("/api/projects/database/{}", self.project_id))
            .ok()
            .await
    }

    pub async fn github_migration_status(&self) -> Result<GitMigrationStatus, Error> {
        self.client
            .get(format!(
                "/api/projects/{}/github-migration-status",
                self.project_id
            ))
            .ok()
            .await
    }

    pub async fn migrate_github_to_platform(&self) -> Result<Project, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/github-migrate-platform",
                self.project_id
            ))
            .ok()
            .await
    }

    pub async fn publish(&self) -> Result<PublishResponse, Error> {
        self.client
            .post(format!("/api/projects/{}/publish", self.project_id))
            .ok()
            .await
    }

    pub async fn deployments(&self, options: &DeploymentOptions) -> Result<Vec<Deployment>, Error> {
        self.client
            .get(format!("/api/projects/{}/deployments", self.project_id))
            .query(options)
            .ok()
            .await
    }

    pub async fn transfer(&self, target_email: impl AsRef<str>) -> Result<TransferResponse, Error> {
        #[derive(Serialize)]
        struct Payload<'a> {
            target_email: &'a str,
        }

        self.client
            .post(format!("/api/projects/{}/transfer", self.project_id))
            .json(&Payload {
                target_email: target_email.as_ref(),
            })
            .ok()
            .await
    }

    pub async fn issue_token(&self, payload: &TokenRequest) -> Result<Token, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/project-token/issue",
                self.project_id
            ))
            .json(payload)
            .ok()
            .await
    }

    pub async fn refresh_token(&self, payload: &TokenRequest) -> Result<Token, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/project-token/refresh",
                self.project_id
            ))
            .json(payload)
            .ok()
            .await
    }

    pub async fn execute_session(
        &self,
        payload: &ExecuteSession,
    ) -> Result<ExecuteSessionResponse, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/sessions/execute",
                self.project_id
            ))
            .json(payload)
            .ok()
            .await
    }

    pub async fn history(&self, options: &HistoryOptions) -> Result<Vec<ChatHistory>, Error> {
        self.client
            .get(format!("/api/projects/{}/history", self.project_id))
            .query(options)
            .ok()
            .await
    }

    pub async fn active_session(&self) -> Result<Option<RuntimeSession>, Error> {
        self.client
            .get(format!("/api/projects/{}/sessions/active", self.project_id))
            .ok()
            .await
    }

    pub async fn history_detail(&self, history_id: i64) -> Result<ChatHistory, Error> {
        self.client
            .get(format!(
                "/api/projects/{}/history/{}",
                self.project_id, history_id
            ))
            .ok()
            .await
    }
}
