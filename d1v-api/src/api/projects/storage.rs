use bon::bon;
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::encode::encode_path;
use crate::{Client, Error};

#[derive(Debug, Clone)]
pub struct AssetFile {
    pub path: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct UploadAsset {
    pub path: String,
    pub file: AssetFile,
}

impl From<UploadAsset> for Form {
    fn from(payload: UploadAsset) -> Self {
        Form::new().text("path", payload.path).part(
            "file",
            Part::bytes(payload.file.bytes).file_name(payload.file.path),
        )
    }
}

impl From<AssetFile> for Form {
    fn from(file: AssetFile) -> Self {
        Form::new().part("file", Part::bytes(file.bytes).file_name(file.path))
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStructure {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size: Option<u64>,
    pub children: Option<Vec<StorageStructure>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageFile {
    pub path: String,
    pub content: String,
    pub size: u64,
    pub is_binary: bool,
}

pub struct ProjectStorage {
    client: Client,
    project_id: String,
}

#[bon]
impl ProjectStorage {
    pub fn new(client: Client, project_id: String) -> Self {
        Self { client, project_id }
    }

    #[builder]
    pub async fn structure(
        &self,
        sub_path: Option<&str>,
        ext: Option<&str>,
    ) -> Result<StorageStructure, Error> {
        #[skip_serializing_none]
        #[derive(Serialize)]
        struct Query<'a> {
            sub_path: Option<&'a str>,
            ext: Option<&'a str>,
        }

        self.client
            .get(format!(
                "/api/projects/storage/{}/structure",
                self.project_id
            ))
            .query(&Query { sub_path, ext })
            .ok()
            .await
    }

    pub async fn file(&self, file_path: impl AsRef<str>) -> Result<StorageFile, Error> {
        self.client
            .get(format!(
                "/api/projects/storage/{}/files/{}",
                self.project_id,
                encode_path(file_path)
            ))
            .ok()
            .await
    }

    pub async fn upload_asset(&self, payload: UploadAsset) -> Result<Asset, Error> {
        self.client
            .post(format!("/api/projects/{}/assets", self.project_id))
            .multipart(payload.into())
            .ok()
            .await
    }

    pub async fn replace_asset(
        &self,
        object_path: impl AsRef<str>,
        file: AssetFile,
    ) -> Result<Asset, Error> {
        self.client
            .put(format!(
                "/api/projects/{}/assets/{}",
                self.project_id,
                encode_path(object_path)
            ))
            .multipart(file.into())
            .ok()
            .await
    }

    pub async fn asset(&self, object_path: impl AsRef<str>) -> Result<Asset, Error> {
        self.client
            .get(format!(
                "/api/projects/{}/assets/{}",
                self.project_id,
                encode_path(object_path)
            ))
            .ok()
            .await
    }

    pub async fn delete_asset(&self, object_path: impl AsRef<str>) -> Result<Asset, Error> {
        self.client
            .delete(format!(
                "/api/projects/{}/assets/{}",
                self.project_id,
                encode_path(object_path)
            ))
            .ok()
            .await
    }
}
