mod db;
mod env;
mod integrations;
mod pay;
mod project;
mod session;
mod storage;
mod types;

pub use db::{
    ColumnIdentity, ColumnSchema, CreateDbTable, DatabaseSchema, DbBranch, DbColumn, DbData, DbRow,
    DeleteDbRows, DropDbTableOptions, ExecuteSql, ExecuteSqlResponse, ForeignKeySchema,
    Granularity, InsertDbRow, NeonUsage, ProjectsDb, TableSchema, UpdateDbRows,
};
pub use env::{
    CreateEnvVar, EnvVar, ExportEnvVarsResponse, ImportEnvVarsResponse, ProjectEnv,
    SyncEnvVarsResponse, UpdateEnvVar,
};
pub use integrations::{IntegrationResponse, ProjectIntegrations};
pub use pay::{
    CreatePayBankAccount, CreatePayPaymentLink, CreatePayProduct, CreatePayToken, CreatePayWebhook,
    CreatePayWithdrawal, DeletePayBankAccountResponse, DeletePayTokenResponse,
    DeletePayWebhookResponse, PayBankAccount, PayBankAccounts, PayDashboardMetrics,
    PayPaginatedTransactionsOptions, PayPaymentIntent, PayPaymentLink, PayProduct, PayProducts,
    PayRevenue, PayToken, PayTokens, PayTransactionStats, PayTransactions, PayWebhook, PayWebhooks,
    PayWithdrawal, PayWithdrawals, ProjectPay, RegeneratePayWebhookSecretResponse,
    UpdatePayBankAccount, UpdatePayWebhook,
};
pub use project::ProjectApi;
pub use session::{
    CancelSessionResponse, ChatHistory, ClaudeProject, Direction, Engine, ExecuteSession,
    ExecuteSessionResponse, HistoryOptions, MessageType, RuntimeSession, SessionType,
};
pub use storage::{
    Asset, AssetFile, ProjectStorage, StorageFile, StorageStructure, StorageStructureOptions,
    UploadAsset,
};
pub use types::{
    CreateProject, CreateProjectResponse, CreateProjectWithIntegrations, Database, Deployment,
    DeploymentEnvironment, DeploymentOptions, GenerateEmojisResponse, GenerateMeta,
    GitMigrationStatus, ImportFromGithub, ImportLocal, ImportPublic, LocalImportFile, Meta,
    Project, PublishResponse, RepositoryMode, Template, Token, TokenRequest, TransferResponse,
    UpdateProject,
};

use bon::bon;
use jiff::Timestamp;
use serde::Serialize;
use serde_with::skip_serializing_none;

use crate::{Client, Error};

pub struct ProjectsApi {
    client: Client,
}

impl Client {
    #[must_use]
    pub fn projects(&self) -> ProjectsApi {
        ProjectsApi {
            client: self.clone(),
        }
    }
}

#[bon]
impl ProjectsApi {
    pub async fn list(&self) -> Result<Vec<Project>, Error> {
        self.client.get("/api/projects/").ok().await
    }

    pub async fn create(&self, payload: &CreateProject) -> Result<CreateProjectResponse, Error> {
        self.client.post("/api/projects/").json(payload).ok().await
    }

    pub async fn templates(&self) -> Result<Vec<Template>, Error> {
        self.client.get("/api/projects/templates").ok().await
    }

    pub async fn generate_meta(&self, payload: &GenerateMeta) -> Result<Meta, Error> {
        self.client
            .post("/api/projects/ai/generate-meta")
            .json(payload)
            .ok()
            .await
    }

    pub async fn search(&self, keyword: impl AsRef<str>) -> Result<Vec<Project>, Error> {
        #[derive(Serialize)]
        struct Query<'a> {
            keyword: &'a str,
        }

        self.client
            .get("/api/projects/search")
            .query(&Query {
                keyword: keyword.as_ref(),
            })
            .ok()
            .await
    }

    pub async fn create_with_integrations(
        &self,
        payload: &CreateProjectWithIntegrations,
    ) -> Result<CreateProjectResponse, Error> {
        self.client
            .post("/api/projects/create-with-integrations")
            .json(payload)
            .ok()
            .await
    }

    pub async fn import_from_github(
        &self,
        payload: &ImportFromGithub,
        schedule_auto_deploy: Option<bool>,
    ) -> Result<CreateProjectResponse, Error> {
        self.client
            .post("/api/projects/import-from-github")
            .query_if_some("schedule_auto_deploy", schedule_auto_deploy)
            .json(payload)
            .ok()
            .await
    }

    pub async fn import_public_to_org(
        &self,
        payload: &ImportPublic,
    ) -> Result<CreateProjectResponse, Error> {
        self.client
            .post("/api/projects/import-public-to-org")
            .json(payload)
            .ok()
            .await
    }

    pub async fn import_from_local(
        &self,
        payload: ImportLocal,
    ) -> Result<CreateProjectResponse, Error> {
        self.client
            .post("/api/projects/import-from-local")
            .multipart(payload.into())
            .ok()
            .await
    }

    pub async fn cli_import_local(
        &self,
        payload: ImportLocal,
    ) -> Result<CreateProjectResponse, Error> {
        self.client
            .post("/api/projects/cli-import-local")
            .multipart(payload.into())
            .ok()
            .await
    }

    pub async fn generate_emojis(&self) -> Result<GenerateEmojisResponse, Error> {
        self.client
            .post("/api/projects/admin/generate-emojis")
            .ok()
            .await
    }

    pub fn project(&self, project_id: impl Into<String>) -> ProjectApi {
        ProjectApi::new(self.client.clone(), project_id.into())
    }

    #[builder]
    pub async fn neon_usage(
        &self,
        from_iso: Option<Timestamp>,
        to_iso: Option<Timestamp>,
        granularity: Option<Granularity>,
    ) -> Result<NeonUsage, Error> {
        #[skip_serializing_none]
        #[derive(Serialize)]
        struct Query {
            from_iso: Option<Timestamp>,
            to_iso: Option<Timestamp>,
            granularity: Option<Granularity>,
        }

        self.client
            .get("/api/projects/db/neon-usage")
            .query(&Query {
                from_iso,
                to_iso,
                granularity,
            })
            .ok()
            .await
    }

    pub async fn cancel_session(
        &self,
        session_id: impl AsRef<str>,
    ) -> Result<CancelSessionResponse, Error> {
        self.client
            .post(format!(
                "/api/projects/sessions/{}/cancel",
                session_id.as_ref()
            ))
            .ok()
            .await
    }

    pub async fn execute_claude_session(
        &self,
        payload: &ExecuteSession,
    ) -> Result<ExecuteSessionResponse, Error> {
        self.client
            .post("/api/projects/claude/execute")
            .json(payload)
            .ok()
            .await
    }

    pub async fn claude_user_projects(
        &self,
        username: impl AsRef<str>,
    ) -> Result<Vec<ClaudeProject>, Error> {
        self.client
            .get(format!(
                "/api/projects/api/claude/users/{}/projects",
                username.as_ref()
            ))
            .ok()
            .await
    }
}
