use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{Client, Error};

use super::session::ProjectSession;
use crate::multipart::FormExt;

pub struct ProjectApi {
    client: Client,
}

impl Client {
    #[must_use]
    pub fn project(&self) -> ProjectApi {
        ProjectApi {
            client: self.clone(),
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateProject {
    pub project_name: String,
    pub project_description: String,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateProject {
    pub project_name: Option<String>,
    pub project_description: Option<String>,
    pub emoji: Option<String>,
    pub auto_deploy_on_execute: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectTemplateInfo {
    pub template_repo: String,
    pub name: String,
    pub description: String,
    pub category: Option<String>,
    pub kind: Option<String>,
    pub featured: Option<bool>,
    pub rank: Option<i64>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateProjectWithIntegrations {
    pub prompt: String,
    pub max_desc_len: Option<u32>,
    pub template_repo: Option<String>,
    pub auto_deploy_on_execute: Option<bool>,
    pub enable_pay: Option<bool>,
    pub enable_database: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateProjectResponse {
    pub project: UserProject,
    pub session: Option<ProjectSession>,
    pub import_auto_deploy: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct LocalImportUploadFile {
    pub path: String,
    pub bytes: Vec<u8>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserProject {
    pub id: String,
    #[serde(default)]
    pub project_name: String,
    #[serde(default)]
    pub project_description: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    pub project_port: Option<u16>,
    pub emoji: Option<String>,
    pub repository_full_name: Option<String>,
    pub repository_current_branch: Option<String>,
    pub workspace_current_branch: Option<String>,
    pub latest_preview_url: Option<String>,
    pub latest_dev_deployment_url: Option<String>,
    pub latest_prod_deployment_url: Option<String>,
    pub analytics_enabled: Option<bool>,
    pub project_database_id: Option<String>,
    pub project_pay_id: Option<String>,
    pub auto_deploy_on_execute: Option<bool>,
}

impl ProjectApi {
    pub async fn templates(&self) -> Result<Vec<ProjectTemplateInfo>, Error> {
        self.client.get("/api/projects/templates").ok().await
    }

    pub async fn list(&self) -> Result<Vec<UserProject>, Error> {
        self.client.get("/api/projects").ok().await
    }

    pub async fn get(
        &self,
        project_id: impl AsRef<str>,
        sync: Option<bool>,
    ) -> Result<UserProject, Error> {
        self.client
            .get(format!("/api/projects/{}", project_id.as_ref()))
            .query_if_some("sync", sync)
            .ok()
            .await
    }

    pub async fn create(&self, payload: &CreateProject) -> Result<UserProject, Error> {
        self.client.post("/api/projects").json(payload).ok().await
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

    pub async fn cli_import_local(
        &self,
        project_name: Option<&str>,
        project_description: Option<&str>,
        files: &[LocalImportUploadFile],
    ) -> Result<CreateProjectResponse, Error> {
        let mut form = Form::new()
            .text("private", "true")
            .text_if("project_name", project_name.map(str::to_string))
            .text_if(
                "project_description",
                project_description.map(str::to_string),
            )
            .text("wait_for_deploy", "false");

        for file in files {
            form = form.part(
                "files",
                Part::bytes(file.bytes.clone()).file_name(file.path.clone()),
            );
        }

        self.client
            .post("/api/projects/cli-import-local")
            .multipart(form)
            .ok()
            .await
    }

    pub async fn update(
        &self,
        project_id: impl AsRef<str>,
        payload: &UpdateProject,
    ) -> Result<UserProject, Error> {
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
}
