use bon::Builder;
use jiff::Timestamp;
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use super::super::session::ProjectSession;
use super::session::TokenScope;
use crate::multipart::FormExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateMetaResponse {
    pub project_name: String,
    pub project_description: String,
    pub emoji: String,
    pub template_repo: String,
    pub template_reason: String,
    pub template_confidence: i64,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryMode {
    #[default]
    Direct,
    Forked,
    Mirrored,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentEnvironment {
    #[default]
    Dev,
    Prod,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Template {
    pub template_repo: String,
    pub name: String,
    pub description: String,
    pub category: Option<String>,
    pub kind: Option<String>,
    pub featured: Option<bool>,
    pub rank: Option<i64>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Builder)]
pub struct RepositoryInfo {
    #[builder(into)]
    #[serde(rename = "repository_platform")]
    pub platform: Option<String>,
    #[builder(into)]
    #[serde(rename = "repository_full_name")]
    pub full_name: Option<String>,
    #[builder(into)]
    #[serde(rename = "repository_id")]
    pub id: Option<String>,
    #[builder(into)]
    #[serde(rename = "repository_owner")]
    pub owner: Option<String>,
    #[builder(into)]
    #[serde(rename = "repository_name")]
    pub name: Option<String>,
    #[builder(into)]
    #[serde(rename = "repository_clone_url")]
    pub clone_url: Option<String>,
    #[builder(into)]
    #[serde(rename = "repository_ssh_url")]
    pub ssh_url: Option<String>,
    #[builder(into)]
    #[serde(rename = "repository_default_branch")]
    pub default_branch: Option<String>,
    #[serde(rename = "repository_is_private")]
    pub is_private: Option<bool>,
    #[builder(into)]
    #[serde(rename = "repository_description")]
    pub description: Option<String>,
    #[builder(into)]
    #[serde(rename = "repository_language")]
    pub language: Option<String>,
    #[serde(rename = "repository_metadata")]
    pub metadata: Option<serde_json::Value>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateProjectResponse {
    pub project: Project,
    pub session: Option<ProjectSession>,
    pub import_auto_deploy: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct LocalImportFile {
    pub path: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
pub struct ImportLocal {
    pub project_name: Option<String>,
    pub project_description: Option<String>,
    pub private: Option<bool>,
    pub archive: Option<LocalImportFile>,
    pub files: Vec<LocalImportFile>,
    pub single_file_name: Option<String>,
    pub single_file_type: Option<String>,
    pub single_file_content: Option<String>,
    pub wait_for_deploy: Option<bool>,
    pub wait_deploy_seconds: Option<u32>,
}

impl From<ImportLocal> for Form {
    fn from(payload: ImportLocal) -> Self {
        let ImportLocal {
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
        } = payload;

        let mut form = Form::new()
            .text_if("project_name", project_name)
            .text_if("project_description", project_description)
            .text_if("single_file_name", single_file_name)
            .text_if("single_file_type", single_file_type)
            .text_if("single_file_content", single_file_content)
            .text_if_display("private", private)
            .text_if_display("wait_for_deploy", wait_for_deploy)
            .text_if_display("wait_deploy_seconds", wait_deploy_seconds);

        if let Some(archive) = archive {
            form = form.part(
                "archive",
                Part::bytes(archive.bytes).file_name(archive.path),
            );
        }
        for file in files {
            form = form.part("files", Part::bytes(file.bytes).file_name(file.path));
        }

        form
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitMigrationStatus {
    pub required: bool,
    pub reason: Option<String>,
    pub source_repository_full_name: Option<String>,
    pub source_repository_url: Option<String>,
    pub target_repository_full_name: Option<String>,
    pub repository_mode: Option<RepositoryMode>,
    pub platform_managed_repository: bool,
    pub has_direct_write_access: bool,
    pub can_migrate_to_platform: bool,
    pub connect_settings_path: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PublishResponse {
    pub success: bool,
    pub commit_hash: Option<String>,
    pub message: String,
    pub production_url: Option<String>,
    pub vercel_url: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Deployment {
    pub id: i64,
    pub project_id: String,
    pub environment: DeploymentEnvironment,
    pub status: String,
    pub vercel_deployment_id: Option<String>,
    pub vercel_deployment_url: Option<String>,
    pub vercel_project_id: Option<String>,
    pub vercel_domain: Option<String>,
    pub vercel_framework: Option<String>,
    pub vercel_build_command: Option<String>,
    pub vercel_output_dir: Option<String>,
    pub git_branch: Option<String>,
    pub git_commit_sha: Option<String>,
    pub git_commit_message: Option<String>,
    pub git_commit_author: Option<String>,
    pub deployed_by: Option<String>,
    pub started_at: Option<Timestamp>,
    pub completed_at: Option<Timestamp>,
    pub deployment_duration_seconds: Option<i64>,
    pub error_message: Option<String>,
    pub created_at: Option<Timestamp>,
    pub updated_at: Option<Timestamp>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub project_token: String,
    pub expires_at: Timestamp,
    pub scopes: Vec<TokenScope>,
}

pub type Database = Vec<serde_json::Value>;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateEmojisProject {
    pub project_id: String,
    pub project_name: String,
    pub emoji: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateEmojisResponse {
    pub message: String,
    pub updated_count: i64,
    #[serde(default)]
    pub projects: Option<Vec<GenerateEmojisProject>>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub user_id: Option<i64>,
    #[serde(default)]
    pub project_name: String,
    #[serde(default)]
    pub project_description: String,
    pub project_port: Option<u16>,
    pub project_pay_id: Option<String>,
    pub project_database_id: Option<String>,
    pub repository_platform: Option<String>,
    pub repository_mode: Option<RepositoryMode>,
    pub repository_full_name: Option<String>,
    pub repository_id: Option<String>,
    pub repository_owner: Option<String>,
    pub repository_name: Option<String>,
    pub repository_clone_url: Option<String>,
    pub repository_ssh_url: Option<String>,
    pub repository_default_branch: Option<String>,
    pub repository_current_branch: Option<String>,
    pub workspace_current_branch: Option<String>,
    pub repository_is_private: Option<bool>,
    pub repository_description: Option<String>,
    pub repository_language: Option<String>,
    pub source_repository_full_name: Option<String>,
    pub source_repository_url: Option<String>,
    pub platform_managed_repository: Option<bool>,
    pub repository_metadata: Option<serde_json::Value>,
    pub opcode_project_id: Option<String>,
    pub opcode_project_path: Option<String>,
    pub opcode_username: Option<String>,
    pub opcode_last_accessed_at: Option<Timestamp>,
    pub vercel_dev_project_id: Option<String>,
    pub vercel_dev_domain: Option<String>,
    pub latest_dev_deployment_url: Option<String>,
    pub vercel_prod_project_id: Option<String>,
    pub vercel_prod_domain: Option<String>,
    pub latest_prod_deployment_url: Option<String>,
    pub latest_preview_url: Option<String>,
    pub vercel_framework: Option<String>,
    pub vercel_build_command: Option<String>,
    pub vercel_output_dir: Option<String>,
    pub umami_website_id: Option<String>,
    pub analytics_enabled: Option<bool>,
    pub analytics_team_code: Option<String>,
    pub analytics_team_id: Option<String>,
    pub analytics_id: Option<String>,
    pub emoji: Option<String>,
    pub auto_deploy_on_execute: Option<bool>,
    pub created_at: Option<Timestamp>,
    pub updated_at: Option<Timestamp>,
    #[serde(default)]
    pub sessions: Vec<ProjectSession>,
}
