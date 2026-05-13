mod db;
mod pay;
mod project;
mod session;
mod types;

pub use db::{
    CreateProjectDbTable, DeleteProjectDbRows, DropProjectDbTableOptions, ExecuteProjectSql,
    ExecuteProjectSqlResponse, InsertProjectDbRow, ListProjectDbRowsOptions, NeonUsage,
    NeonUsageOptions, ProjectDbBranch, ProjectDbColumn, ProjectDbData, ProjectDbDataOptions,
    ProjectDbMutation, ProjectDbRow, ProjectDbSchema, ProjectDbSchemaOptions, ProjectsDb,
    UpdateProjectDbRows,
};
pub use pay::{
    CreatePayPaymentIntent, CreatePayPaymentLink, CreatePayProduct, CreatePayWebhook,
    DeletePayWebhookResponse, PayAnalyticsOptions, PayDashboardMetrics,
    PayPaginatedTransactionsOptions, PayPaymentIntent, PayPaymentLink, PayProduct,
    PayProductPaymentLinkOptions, PayProducts, PayRevenue, PayTransactionStats, PayTransactions,
    PayTransactionsOptions, PayWebhook, PayWebhooks, ProjectPay,
    RegeneratePayWebhookSecretResponse, UpdatePayWebhook,
};
pub use project::ProjectApi;
pub use session::{
    CancelProjectSessionResponse, ClaudeUserProject, ExecuteProjectSession,
    ExecuteProjectSessionResponse, ProjectChatHistory, ProjectHistoryOptions,
    ProjectRuntimeSession,
};
pub use types::{
    CreateProject, CreateProjectResponse, CreateProjectWithIntegrations,
    GenerateProjectEmojisResponse, GenerateProjectMeta, ImportFromGithub, ImportLocal,
    ImportPublic, LocalImportFile, Project, ProjectDatabase, ProjectDeployment,
    ProjectDeploymentOptions, ProjectGitMigrationStatus, ProjectMeta, ProjectTemplate,
    ProjectToken, ProjectTokenRequest, PublishProjectResponse, TransferProject,
    TransferProjectResponse, UpdateProject,
};

use serde::Serialize;

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

impl ProjectsApi {
    pub async fn list(&self) -> Result<Vec<Project>, Error> {
        self.client.get("/api/projects/").ok().await
    }

    pub async fn create(&self, payload: &CreateProject) -> Result<CreateProjectResponse, Error> {
        self.client.post("/api/projects/").json(payload).ok().await
    }

    pub async fn templates(&self) -> Result<Vec<ProjectTemplate>, Error> {
        self.client.get("/api/projects/templates").ok().await
    }

    pub async fn generate_meta(&self, payload: &GenerateProjectMeta) -> Result<ProjectMeta, Error> {
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

    pub async fn generate_emojis(&self) -> Result<GenerateProjectEmojisResponse, Error> {
        self.client
            .post("/api/projects/admin/generate-emojis")
            .ok()
            .await
    }

    pub fn project(&self, project_id: impl Into<String>) -> ProjectApi {
        ProjectApi::new(self.client.clone(), project_id.into())
    }

    pub async fn neon_usage(&self, options: &NeonUsageOptions) -> Result<NeonUsage, Error> {
        self.client
            .get("/api/projects/db/neon-usage")
            .query(options)
            .ok()
            .await
    }

    pub async fn cancel_session(
        &self,
        session_id: impl AsRef<str>,
    ) -> Result<CancelProjectSessionResponse, Error> {
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
        payload: &ExecuteProjectSession,
    ) -> Result<ExecuteProjectSessionResponse, Error> {
        self.client
            .post("/api/projects/claude/execute")
            .json(payload)
            .ok()
            .await
    }

    pub async fn claude_user_projects(
        &self,
        username: impl AsRef<str>,
    ) -> Result<Vec<ClaudeUserProject>, Error> {
        self.client
            .get(format!(
                "/api/projects/api/claude/users/{}/projects",
                username.as_ref()
            ))
            .ok()
            .await
    }
}
