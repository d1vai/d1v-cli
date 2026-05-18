use bon::bon;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::skip_serializing_none;

use crate::{Client, Error};

use super::db::ProjectsDb;
use super::env::ProjectEnv;
use super::integrations::ProjectIntegrations;
use super::pay::ProjectPay;
use super::session::{
    ChatHistory, CommaSeparated, Direction, Engine, ExecuteSessionResponse, MessageType,
    RuntimeSession, SessionType, TokenScope,
};
use super::storage::ProjectStorage;
use super::types::{
    Database, Deployment, DeploymentEnvironment, GitMigrationStatus, Project, PublishResponse,
    RepositoryInfo, Token,
};

#[skip_serializing_none]
#[derive(Serialize)]
struct TokenPayload<'a> {
    scopes: Option<&'a [TokenScope]>,
    ttl_seconds: Option<u32>,
}

#[skip_serializing_none]
#[derive(Serialize)]
pub(crate) struct ExecuteSessionPayload<'a> {
    pub prompt: &'a str,
    pub session_type: Option<SessionType>,
    pub session_id: Option<&'a str>,
    pub model: Option<&'a str>,
    pub engine: Option<Engine>,
    pub system_prompt: Option<&'a str>,
    pub project_path: Option<&'a str>,
    pub auto_deploy: Option<bool>,
}

pub struct ProjectApi {
    client: Client,
    project_id: String,
}

#[bon]
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

    #[builder]
    pub async fn update(
        &self,
        project_name: Option<&str>,
        project_description: Option<&str>,
        emoji: Option<&str>,
        auto_deploy_on_execute: Option<bool>,
        super_admin_email: Option<&str>,
        project_secret: Option<&str>,
        repository: Option<&RepositoryInfo>,
    ) -> Result<Project, Error> {
        #[skip_serializing_none]
        #[derive(Serialize)]
        struct Payload<'a> {
            project_name: Option<&'a str>,
            project_description: Option<&'a str>,
            emoji: Option<&'a str>,
            auto_deploy_on_execute: Option<bool>,
            super_admin_email: Option<&'a str>,
            project_secret: Option<&'a str>,
            #[serde(flatten)]
            repository: Option<&'a RepositoryInfo>,
        }

        self.client
            .put(format!("/api/projects/{}", self.project_id))
            .json(&Payload {
                project_name,
                project_description,
                emoji,
                auto_deploy_on_execute,
                super_admin_email,
                project_secret,
                repository,
            })
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

    #[builder]
    pub async fn deployments(
        &self,
        environment: Option<DeploymentEnvironment>,
        limit: Option<u32>,
    ) -> Result<Vec<Deployment>, Error> {
        #[skip_serializing_none]
        #[derive(Serialize)]
        struct Query {
            environment: Option<DeploymentEnvironment>,
            limit: Option<u32>,
        }

        self.client
            .get(format!("/api/projects/{}/deployments", self.project_id))
            .query(&Query { environment, limit })
            .ok()
            .await
    }

    pub async fn transfer(&self, target_email: impl AsRef<str>) -> Result<Project, Error> {
        #[derive(Deserialize)]
        struct TransferResult {
            project: Project,
        }

        #[derive(Serialize)]
        struct Payload<'a> {
            target_email: &'a str,
        }

        self.client
            .post(format!("/api/projects/{}/transfer", self.project_id))
            .json(&Payload {
                target_email: target_email.as_ref(),
            })
            .ok::<TransferResult>()
            .await
            .map(|r| r.project)
    }

    #[builder]
    pub async fn issue_token(
        &self,
        scopes: Option<Vec<TokenScope>>,
        ttl_seconds: Option<u32>,
    ) -> Result<Token, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/project-token/issue",
                self.project_id
            ))
            .json(&TokenPayload {
                scopes: scopes.as_deref(),
                ttl_seconds,
            })
            .ok()
            .await
    }

    #[builder]
    pub async fn refresh_token(
        &self,
        scopes: Option<Vec<TokenScope>>,
        ttl_seconds: Option<u32>,
    ) -> Result<Token, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/project-token/refresh",
                self.project_id
            ))
            .json(&TokenPayload {
                scopes: scopes.as_deref(),
                ttl_seconds,
            })
            .ok()
            .await
    }

    #[builder]
    pub async fn execute_session(
        &self,
        #[builder(start_fn)] prompt: impl AsRef<str>,
        session_type: Option<SessionType>,
        session_id: Option<&str>,
        model: Option<&str>,
        engine: Option<Engine>,
        system_prompt: Option<&str>,
        project_path: Option<&str>,
        auto_deploy: Option<bool>,
    ) -> Result<ExecuteSessionResponse, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/sessions/execute",
                self.project_id
            ))
            .json(&ExecuteSessionPayload {
                prompt: prompt.as_ref(),
                session_type,
                session_id,
                model,
                engine,
                system_prompt,
                project_path,
                auto_deploy,
            })
            .ok()
            .await
    }

    #[builder]
    pub async fn history(
        &self,
        limit: Option<u32>,
        before_ts: Option<Timestamp>,
        before_id: Option<i64>,
        direction: Option<Direction>,
        message_type: Option<Vec<MessageType>>,
        include_payload: Option<bool>,
    ) -> Result<Vec<ChatHistory>, Error> {
        #[serde_as]
        #[skip_serializing_none]
        #[derive(Serialize)]
        struct Query {
            limit: Option<u32>,
            before_ts: Option<Timestamp>,
            before_id: Option<i64>,
            direction: Option<Direction>,
            #[serde_as(as = "Option<CommaSeparated>")]
            message_type: Option<Vec<MessageType>>,
            include_payload: Option<bool>,
        }

        self.client
            .get(format!("/api/projects/{}/history", self.project_id))
            .query(&Query {
                limit,
                before_ts,
                before_id,
                direction,
                message_type,
                include_payload,
            })
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
