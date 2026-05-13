use crate::{Client, Error};

use super::db::ProjectsDb;
use super::session::{
    ExecuteProjectSession, ExecuteProjectSessionResponse, ProjectChatHistory,
    ProjectHistoryOptions, ProjectRuntimeSession,
};
use super::types::{
    Project, ProjectDatabase, ProjectDeployment, ProjectDeploymentOptions,
    ProjectGitMigrationStatus, ProjectToken, ProjectTokenRequest, PublishProjectResponse,
    TransferProject, TransferProjectResponse, UpdateProject,
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

    pub async fn database(&self) -> Result<ProjectDatabase, Error> {
        self.client
            .get(format!("/api/projects/database/{}", self.project_id))
            .ok()
            .await
    }

    pub async fn github_migration_status(&self) -> Result<ProjectGitMigrationStatus, Error> {
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

    pub async fn publish(&self) -> Result<PublishProjectResponse, Error> {
        self.client
            .post(format!("/api/projects/{}/publish", self.project_id))
            .ok()
            .await
    }

    pub async fn deployments(
        &self,
        options: &ProjectDeploymentOptions,
    ) -> Result<Vec<ProjectDeployment>, Error> {
        self.client
            .get(format!("/api/projects/{}/deployments", self.project_id))
            .query(options)
            .ok()
            .await
    }

    pub async fn transfer(
        &self,
        payload: &TransferProject,
    ) -> Result<TransferProjectResponse, Error> {
        self.client
            .post(format!("/api/projects/{}/transfer", self.project_id))
            .json(payload)
            .ok()
            .await
    }

    pub async fn issue_token(&self, payload: &ProjectTokenRequest) -> Result<ProjectToken, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/project-token/issue",
                self.project_id
            ))
            .json(payload)
            .ok()
            .await
    }

    pub async fn refresh_token(
        &self,
        payload: &ProjectTokenRequest,
    ) -> Result<ProjectToken, Error> {
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
        payload: &ExecuteProjectSession,
    ) -> Result<ExecuteProjectSessionResponse, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/sessions/execute",
                self.project_id
            ))
            .json(payload)
            .ok()
            .await
    }

    pub async fn history(
        &self,
        options: &ProjectHistoryOptions,
    ) -> Result<Vec<ProjectChatHistory>, Error> {
        self.client
            .get(format!("/api/projects/{}/history", self.project_id))
            .query(options)
            .ok()
            .await
    }

    pub async fn active_session(&self) -> Result<Option<ProjectRuntimeSession>, Error> {
        self.client
            .get(format!("/api/projects/{}/sessions/active", self.project_id))
            .ok()
            .await
    }

    pub async fn history_detail(&self, history_id: i64) -> Result<ProjectChatHistory, Error> {
        self.client
            .get(format!(
                "/api/projects/{}/history/{}",
                self.project_id, history_id
            ))
            .ok()
            .await
    }
}
