use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{Client, Error};

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize)]
pub struct ProjectStorageStructureOptions {
    pub sub_path: Option<String>,
    pub ext: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProjectAssetFile {
    pub path: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct UploadProjectAsset {
    pub path: String,
    pub file: ProjectAssetFile,
}

impl From<UploadProjectAsset> for Form {
    fn from(payload: UploadProjectAsset) -> Self {
        Form::new().text("path", payload.path).part(
            "file",
            Part::bytes(payload.file.bytes).file_name(payload.file.path),
        )
    }
}

impl From<ProjectAssetFile> for Form {
    fn from(file: ProjectAssetFile) -> Self {
        Form::new().part("file", Part::bytes(file.bytes).file_name(file.path))
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAsset {
    pub provider: String,
    pub bucket_or_container: String,
    pub key: String,
    pub path: String,
    pub url: Option<String>,
    pub etag: Option<String>,
    pub size: Option<u64>,
    pub content_type: Option<String>,
    pub deleted: Option<bool>,
}

pub type ProjectStorageStructure = serde_json::Value;
pub type ProjectStorageFile = serde_json::Value;

pub struct ProjectStorage {
    client: Client,
    project_id: String,
}

impl ProjectStorage {
    pub fn new(client: Client, project_id: String) -> Self {
        Self { client, project_id }
    }

    pub async fn structure(
        &self,
        options: &ProjectStorageStructureOptions,
    ) -> Result<ProjectStorageStructure, Error> {
        self.client
            .get(format!(
                "/api/projects/storage/{}/structure",
                self.project_id
            ))
            .query(options)
            .ok()
            .await
    }

    pub async fn file(&self, file_path: impl AsRef<str>) -> Result<ProjectStorageFile, Error> {
        self.client
            .get(format!(
                "/api/projects/storage/{}/files/{}",
                self.project_id,
                encode_path(file_path.as_ref())
            ))
            .ok()
            .await
    }

    pub async fn upload_asset(&self, payload: UploadProjectAsset) -> Result<ProjectAsset, Error> {
        self.client
            .post(format!("/api/projects/{}/assets", self.project_id))
            .multipart(payload.into())
            .ok()
            .await
    }

    pub async fn replace_asset(
        &self,
        object_path: impl AsRef<str>,
        file: ProjectAssetFile,
    ) -> Result<ProjectAsset, Error> {
        self.client
            .put(format!(
                "/api/projects/{}/assets/{}",
                self.project_id,
                encode_path(object_path.as_ref())
            ))
            .multipart(file.into())
            .ok()
            .await
    }

    pub async fn asset(&self, object_path: impl AsRef<str>) -> Result<ProjectAsset, Error> {
        self.client
            .get(format!(
                "/api/projects/{}/assets/{}",
                self.project_id,
                encode_path(object_path.as_ref())
            ))
            .ok()
            .await
    }

    pub async fn delete_asset(&self, object_path: impl AsRef<str>) -> Result<ProjectAsset, Error> {
        self.client
            .delete(format!(
                "/api/projects/{}/assets/{}",
                self.project_id,
                encode_path(object_path.as_ref())
            ))
            .ok()
            .await
    }
}

fn encode_path(path: &str) -> String {
    path.split('/')
        .map(urlencoding::encode)
        .collect::<Vec<_>>()
        .join("/")
}
