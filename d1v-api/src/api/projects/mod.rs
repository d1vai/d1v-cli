mod db;
mod env;
mod integrations;
mod pay;
mod project;
mod session;
mod storage;
mod types;

pub use db::{
    ColumnIdentity, ColumnSchema, DatabaseSchema, DbBranch, DbColumn, DbData, DbRow, DeleteDbRows,
    DropDbTableOptions, ExecuteSqlResponse, ForeignKeySchema, Granularity, InsertDbRow, NeonUsage,
    ProjectsDb, TableSchema, UpdateDbRows,
};
pub use env::{
    EnvVar, ExportEnvVarsResponse, ImportEnvVarsResponse, ProjectEnv, SyncEnvVarsResponse,
    UpdateEnvVar,
};
pub use integrations::{IntegrationResponse, ProjectIntegrations};
pub use pay::{
    CreatePayBankAccount, DeletePayBankAccountResponse, DeletePayTokenResponse,
    DeletePayWebhookResponse, PayBankAccount, PayBankAccounts, PayDashboardMetrics,
    PayPaginatedTransactionsOptions, PayPaymentIntent, PayPaymentLink, PayProduct, PayProducts,
    PayRevenue, PayToken, PayTokens, PayTransactionStats, PayTransactions, PayWebhook, PayWebhooks,
    PayWithdrawal, PayWithdrawals, ProjectPay, RegeneratePayWebhookSecretResponse,
    UpdatePayBankAccount,
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
    CreateProjectResponse, CreateProjectWithIntegrations, Database, Deployment,
    DeploymentEnvironment, DeploymentOptions, GenerateEmojisResponse, GenerateMeta,
    GitMigrationStatus, ImportFromGithub, ImportLocal, LocalImportFile, Meta, Project,
    PublishResponse, RepositoryMode, Template, Token, TokenRequest, TransferResponse,
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

    #[builder]
    pub async fn create(
        &self,
        #[builder(start_fn)] project_name: &str,
        #[builder(start_fn)] project_description: &str,
        enable_pay: Option<bool>,
        enable_database: Option<bool>,
        enable_resend: Option<bool>,
    ) -> Result<CreateProjectResponse, Error> {
        #[skip_serializing_none]
        #[derive(Serialize)]
        struct Payload<'a> {
            project_name: &'a str,
            project_description: &'a str,
            enable_pay: Option<bool>,
            enable_database: Option<bool>,
            enable_resend: Option<bool>,
        }

        self.client
            .post("/api/projects/")
            .json(&Payload {
                project_name,
                project_description,
                enable_pay,
                enable_database,
                enable_resend,
            })
            .ok()
            .await
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

    #[builder]
    pub async fn import_public_to_org(
        &self,
        #[builder(start_fn)] source_url: &str,
        #[builder(start_fn)] project_name: &str,
        project_description: Option<&str>,
        private: Option<bool>,
    ) -> Result<CreateProjectResponse, Error> {
        #[skip_serializing_none]
        #[derive(Serialize)]
        struct Payload<'a> {
            source_url: &'a str,
            project_name: &'a str,
            project_description: Option<&'a str>,
            private: Option<bool>,
        }

        self.client
            .post("/api/projects/import-public-to-org")
            .json(&Payload {
                source_url,
                project_name,
                project_description,
                private,
            })
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
