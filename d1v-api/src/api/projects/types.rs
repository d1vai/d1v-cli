use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use super::super::session::ProjectSession;
use crate::multipart::FormExt;

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateProject {
    pub project_name: String,
    pub project_description: String,
    pub enable_pay: Option<bool>,
    pub enable_database: Option<bool>,
    pub enable_resend: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateProject {
    pub project_name: Option<String>,
    pub project_description: Option<String>,
    pub emoji: Option<String>,
    pub auto_deploy_on_execute: Option<bool>,
    pub super_admin_email: Option<String>,
    pub project_secret: Option<String>,
    pub repository_platform: Option<String>,
    pub repository_full_name: Option<String>,
    pub repository_id: Option<String>,
    pub repository_owner: Option<String>,
    pub repository_name: Option<String>,
    pub repository_clone_url: Option<String>,
    pub repository_ssh_url: Option<String>,
    pub repository_default_branch: Option<String>,
    pub repository_is_private: Option<bool>,
    pub repository_description: Option<String>,
    pub repository_language: Option<String>,
    pub repository_metadata: Option<serde_json::Value>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GenerateProjectMeta {
    pub prompt: String,
    pub max_desc_len: Option<u32>,
}

pub type ProjectMeta = serde_json::Value;

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectTemplate {
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
pub struct CreateProjectResponse {
    pub project: Project,
    pub session: Option<ProjectSession>,
    pub import_auto_deploy: Option<serde_json::Value>,
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
    pub enable_resend: Option<bool>,
    pub repository_platform: Option<String>,
    pub repository_full_name: Option<String>,
    pub repository_id: Option<String>,
    pub repository_owner: Option<String>,
    pub repository_name: Option<String>,
    pub repository_clone_url: Option<String>,
    pub repository_ssh_url: Option<String>,
    pub repository_default_branch: Option<String>,
    pub repository_is_private: Option<bool>,
    pub repository_description: Option<String>,
    pub repository_language: Option<String>,
    pub repository_metadata: Option<serde_json::Value>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportFromGithub {
    pub project_name: Option<String>,
    pub project_description: Option<String>,
    pub repository_full_name: String,
    pub repository_url: Option<String>,
    pub repository_ssh_url: Option<String>,
    pub default_branch: Option<String>,
    pub is_private: Option<bool>,
    pub primary_language: Option<String>,
    pub repository_mode: Option<String>,
    pub source_repository_full_name: Option<String>,
    pub source_repository_url: Option<String>,
    pub platform_managed_repository: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportPublic {
    pub source_url: String,
    pub project_name: String,
    pub project_description: Option<String>,
    pub private: Option<bool>,
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
    pub repository_mode: Option<String>,
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
    pub opcode_last_accessed_at: Option<String>,
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
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    #[serde(default)]
    pub sessions: Vec<ProjectSession>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn create_omits_unset_flags() {
        let payload = CreateProject {
            project_name: "demo".to_string(),
            project_description: "Demo project".to_string(),
            ..CreateProject::default()
        };

        assert_eq!(
            serde_json::to_value(payload).unwrap(),
            json!({
                "project_name": "demo",
                "project_description": "Demo project"
            })
        );
    }
}
