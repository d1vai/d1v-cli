mod db;
mod types;

pub use db::{
    CreateProjectDbTable, DeleteProjectDbRows, DropProjectDbTableOptions, ExecuteProjectSql,
    ExecuteProjectSqlResponse, InsertProjectDbRow, ListProjectDbRowsOptions, NeonUsage,
    NeonUsageOptions, ProjectDbBranch, ProjectDbColumn, ProjectDbData, ProjectDbDataOptions,
    ProjectDbMutation, ProjectDbRow, ProjectDbSchema, ProjectDbSchemaOptions, ProjectsDb,
    UpdateProjectDbRows,
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

    pub async fn get(
        &self,
        project_id: impl AsRef<str>,
        sync: Option<bool>,
    ) -> Result<Project, Error> {
        self.client
            .get(format!("/api/projects/{}", project_id.as_ref()))
            .query_if_some("sync", sync)
            .ok()
            .await
    }

    pub async fn update(
        &self,
        project_id: impl AsRef<str>,
        payload: &UpdateProject,
    ) -> Result<Project, Error> {
        self.client
            .put(format!("/api/projects/{}", project_id.as_ref()))
            .json(payload)
            .ok()
            .await
    }

    pub async fn delete(&self, project_id: impl AsRef<str>) -> Result<(), Error> {
        self.client
            .delete(format!("/api/projects/{}", project_id.as_ref()))
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

    pub async fn database(&self, project_id: impl AsRef<str>) -> Result<ProjectDatabase, Error> {
        self.client
            .get(format!("/api/projects/database/{}", project_id.as_ref()))
            .ok()
            .await
    }

    pub async fn github_migration_status(
        &self,
        project_id: impl AsRef<str>,
    ) -> Result<ProjectGitMigrationStatus, Error> {
        self.client
            .get(format!(
                "/api/projects/{}/github-migration-status",
                project_id.as_ref()
            ))
            .ok()
            .await
    }

    pub async fn migrate_github_to_platform(
        &self,
        project_id: impl AsRef<str>,
    ) -> Result<Project, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/github-migrate-platform",
                project_id.as_ref()
            ))
            .ok()
            .await
    }

    pub async fn publish(
        &self,
        project_id: impl AsRef<str>,
    ) -> Result<PublishProjectResponse, Error> {
        self.client
            .post(format!("/api/projects/{}/publish", project_id.as_ref()))
            .ok()
            .await
    }

    pub async fn deployments(
        &self,
        project_id: impl AsRef<str>,
        options: &ProjectDeploymentOptions,
    ) -> Result<Vec<ProjectDeployment>, Error> {
        self.client
            .get(format!("/api/projects/{}/deployments", project_id.as_ref()))
            .query(options)
            .ok()
            .await
    }

    pub async fn transfer(
        &self,
        project_id: impl AsRef<str>,
        payload: &TransferProject,
    ) -> Result<TransferProjectResponse, Error> {
        self.client
            .post(format!("/api/projects/{}/transfer", project_id.as_ref()))
            .json(payload)
            .ok()
            .await
    }

    pub async fn generate_emojis(&self) -> Result<GenerateProjectEmojisResponse, Error> {
        self.client
            .post("/api/projects/admin/generate-emojis")
            .ok()
            .await
    }

    pub fn db(&self, project_id: impl Into<String>) -> ProjectsDb {
        ProjectsDb::new(self.client.clone(), project_id.into())
    }

    pub async fn neon_usage(&self, options: &NeonUsageOptions) -> Result<NeonUsage, Error> {
        self.client
            .get("/api/projects/db/neon-usage")
            .query(options)
            .ok()
            .await
    }

    pub async fn issue_project_token(
        &self,
        project_id: impl AsRef<str>,
        payload: &ProjectTokenRequest,
    ) -> Result<ProjectToken, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/project-token/issue",
                project_id.as_ref()
            ))
            .json(payload)
            .ok()
            .await
    }

    pub async fn refresh_project_token(
        &self,
        project_id: impl AsRef<str>,
        payload: &ProjectTokenRequest,
    ) -> Result<ProjectToken, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/project-token/refresh",
                project_id.as_ref()
            ))
            .json(payload)
            .ok()
            .await
    }
}
