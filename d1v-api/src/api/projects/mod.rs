mod types;

pub use types::{
    CreateProject, CreateProjectResponse, GenerateProjectMeta, Project, ProjectMeta,
    ProjectTemplate, UpdateProject,
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
}
