mod db;
mod env;
mod integrations;
mod pay;
mod project;
mod session;
mod storage;
mod types;

pub use db::{
    ColumnIdentity, ColumnSchema, DatabaseSchema, DbBranch, DbColumn, DbData, DbRow,
    ExecuteSqlResponse, ForeignKeySchema, Granularity, NeonUsage, ProjectsDb, TableSchema,
};
pub use env::{
    EnvVar, ExportEnvVarsResponse, ImportEnvVarsResponse, ProjectEnv, SyncEnvVarsResponse,
};
pub use integrations::{IntegrationResponse, ProjectIntegrations};
pub use pay::{
    CreatePayBankAccount, DeletePayBankAccountResponse, DeletePayTokenResponse,
    DeletePayWebhookResponse, PayBankAccount, PayBankAccounts, PayDashboardMetrics,
    PayPaymentIntent, PayPaymentLink, PayPermission, PayProduct, PayProducts, PayRevenue, PayToken,
    PayTokens, PayTransactionStats, PayTransactions, PayWebhook, PayWebhooks, PayWithdrawal,
    PayWithdrawals, ProjectPay, RegeneratePayWebhookSecretResponse,
};
pub use project::ProjectApi;
pub use session::{
    CancelSessionResponse, ChatHistory, ClaudeProject, Direction, Engine, ExecuteSessionResponse,
    MessageType, Session, SessionType, TokenScope,
};
pub use storage::{Asset, AssetFile, ProjectStorage, StorageFile, StorageStructure, UploadAsset};
pub use types::{
    AnalyticsInfo, CreateProjectResponse, Database, Deployment, DeploymentEnvironment,
    GenerateEmojisProject, GenerateEmojisResponse, GenerateMetaResponse, GitCommitInfo,
    GitMigrationStatus, ImportLocal, LocalImportFile, OpcodeInfo, Project, PublishResponse,
    Repository, RepositoryInfo, RepositoryMode, Template, Token, VercelDeploymentInfo, VercelInfo,
};

use bon::bon;
use jiff::Timestamp;
use serde::Serialize;
use serde_with::skip_serializing_none;

use crate::{Client, Error};
use project::ExecuteSessionPayload;

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

    #[must_use]
    pub fn project(&self, project_id: impl Into<String>) -> ProjectApi {
        ProjectApi::new(self.clone(), project_id.into())
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
        #[builder(start_fn)] project_name: impl AsRef<str>,
        #[builder(start_fn)] project_description: impl AsRef<str>,
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
                project_name: project_name.as_ref(),
                project_description: project_description.as_ref(),
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

    pub async fn generate_meta(
        &self,
        prompt: impl AsRef<str>,
        max_desc_len: Option<u32>,
    ) -> Result<GenerateMetaResponse, Error> {
        #[skip_serializing_none]
        #[derive(Serialize)]
        struct Payload<'a> {
            prompt: &'a str,
            max_desc_len: Option<u32>,
        }

        self.client
            .post("/api/projects/ai/generate-meta")
            .json(&Payload {
                prompt: prompt.as_ref(),
                max_desc_len,
            })
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

    #[builder]
    pub async fn create_with_integrations(
        &self,
        #[builder(start_fn)] prompt: impl AsRef<str>,
        max_desc_len: Option<u32>,
        template_repo: Option<&str>,
        auto_deploy_on_execute: Option<bool>,
        enable_pay: Option<bool>,
        enable_database: Option<bool>,
        enable_resend: Option<bool>,
        repository: Option<&Repository>,
    ) -> Result<CreateProjectResponse, Error> {
        #[skip_serializing_none]
        #[derive(Serialize)]
        struct Payload<'a> {
            prompt: &'a str,
            max_desc_len: Option<u32>,
            template_repo: Option<&'a str>,
            auto_deploy_on_execute: Option<bool>,
            enable_pay: Option<bool>,
            enable_database: Option<bool>,
            enable_resend: Option<bool>,
            #[serde(flatten)]
            repository: Option<&'a Repository>,
        }

        self.client
            .post("/api/projects/create-with-integrations")
            .json(&Payload {
                prompt: prompt.as_ref(),
                max_desc_len,
                template_repo,
                auto_deploy_on_execute,
                enable_pay,
                enable_database,
                enable_resend,
                repository,
            })
            .ok()
            .await
    }

    #[builder]
    pub async fn import_from_github(
        &self,
        #[builder(start_fn)] repository_full_name: impl AsRef<str>,
        project_name: Option<&str>,
        project_description: Option<&str>,
        repository_url: Option<&str>,
        repository_ssh_url: Option<&str>,
        default_branch: Option<&str>,
        is_private: Option<bool>,
        primary_language: Option<&str>,
        repository_mode: Option<RepositoryMode>,
        source_repository_full_name: Option<&str>,
        source_repository_url: Option<&str>,
        platform_managed_repository: Option<bool>,
        schedule_auto_deploy: Option<bool>,
    ) -> Result<CreateProjectResponse, Error> {
        #[skip_serializing_none]
        #[derive(Serialize)]
        struct Payload<'a> {
            repository_full_name: &'a str,
            project_name: Option<&'a str>,
            project_description: Option<&'a str>,
            repository_url: Option<&'a str>,
            repository_ssh_url: Option<&'a str>,
            default_branch: Option<&'a str>,
            is_private: Option<bool>,
            primary_language: Option<&'a str>,
            repository_mode: Option<RepositoryMode>,
            source_repository_full_name: Option<&'a str>,
            source_repository_url: Option<&'a str>,
            platform_managed_repository: Option<bool>,
        }

        self.client
            .post("/api/projects/import-from-github")
            .query_if_some("schedule_auto_deploy", schedule_auto_deploy)
            .json(&Payload {
                repository_full_name: repository_full_name.as_ref(),
                project_name,
                project_description,
                repository_url,
                repository_ssh_url,
                default_branch,
                is_private,
                primary_language,
                repository_mode,
                source_repository_full_name,
                source_repository_url,
                platform_managed_repository,
            })
            .ok()
            .await
    }

    #[builder]
    pub async fn import_public_to_org(
        &self,
        #[builder(start_fn)] source_url: impl AsRef<str>,
        #[builder(start_fn)] project_name: impl AsRef<str>,
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
                source_url: source_url.as_ref(),
                project_name: project_name.as_ref(),
                project_description,
                private,
            })
            .ok()
            .await
    }

    #[builder(on(String, into))]
    pub async fn import_from_local(
        &self,
        project_name: Option<String>,
        project_description: Option<String>,
        private: Option<bool>,
        archive: Option<LocalImportFile>,
        #[builder(default)] files: Vec<LocalImportFile>,
        single_file_name: Option<String>,
        single_file_type: Option<String>,
        single_file_content: Option<String>,
        wait_for_deploy: Option<bool>,
        wait_deploy_seconds: Option<u32>,
    ) -> Result<CreateProjectResponse, Error> {
        self.client
            .post("/api/projects/import-from-local")
            .multipart(
                ImportLocal {
                    project_name,
                    project_description,
                    private,
                    archive,
                    files,
                    single_file_name,
                    single_file_type,
                    single_file_content,
                    wait_for_deploy,
                    wait_deploy_seconds,
                }
                .into(),
            )
            .ok()
            .await
    }

    #[builder(on(String, into))]
    pub async fn cli_import_local(
        &self,
        project_name: Option<String>,
        project_description: Option<String>,
        private: Option<bool>,
        archive: Option<LocalImportFile>,
        #[builder(default)] files: Vec<LocalImportFile>,
        single_file_name: Option<String>,
        single_file_type: Option<String>,
        single_file_content: Option<String>,
        wait_for_deploy: Option<bool>,
        wait_deploy_seconds: Option<u32>,
    ) -> Result<CreateProjectResponse, Error> {
        self.client
            .post("/api/projects/cli-import-local")
            .multipart(
                ImportLocal {
                    project_name,
                    project_description,
                    private,
                    archive,
                    files,
                    single_file_name,
                    single_file_type,
                    single_file_content,
                    wait_for_deploy,
                    wait_deploy_seconds,
                }
                .into(),
            )
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

    #[builder]
    pub async fn execute_claude_session(
        &self,
        #[builder(start_fn)] prompt: impl AsRef<str>,
        #[builder(start_fn)] project_path: impl AsRef<str>,
        session_type: Option<SessionType>,
        session_id: Option<&str>,
        model: Option<&str>,
        engine: Option<Engine>,
        system_prompt: Option<&str>,
        auto_deploy: Option<bool>,
    ) -> Result<ExecuteSessionResponse, Error> {
        self.client
            .post("/api/projects/claude/execute")
            .json(&ExecuteSessionPayload {
                prompt: prompt.as_ref(),
                project_path: Some(project_path.as_ref()),
                session_type,
                session_id,
                model,
                engine,
                system_prompt,
                auto_deploy,
            })
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
