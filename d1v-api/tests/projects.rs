use d1v_api::Client;
use d1v_api::api::projects::{
    CreateProject, CreateProjectWithIntegrations, GenerateProjectMeta, ImportFromGithub,
    ImportLocal, ImportPublic, LocalImportFile, ProjectDeploymentOptions, TransferProject,
    UpdateProject,
};
use httpmock::prelude::*;
use serde_json::json;

fn authed_client(server: &MockServer) -> Client {
    Client::builder()
        .base_url(server.base_url())
        .token("token123")
        .build()
        .unwrap()
}

fn project_json() -> serde_json::Value {
    json!({
        "id": "proj_123",
        "user_id": 42,
        "project_name": "demo",
        "project_description": "Demo project",
        "project_port": 3000,
        "repository_full_name": "d1v/demo",
        "repository_current_branch": "main",
        "workspace_current_branch": "dev",
        "latest_preview_url": "https://demo-preview.example.com",
        "latest_dev_deployment_url": "https://demo-dev.example.com",
        "latest_prod_deployment_url": "https://demo.example.com",
        "analytics_enabled": true,
        "project_database_id": "db_123",
        "project_pay_id": "pay_123",
        "emoji": "🚀",
        "auto_deploy_on_execute": true,
        "created_at": "2026-05-01T00:00:00Z",
        "updated_at": "2026-05-02T00:00:00Z",
        "sessions": []
    })
}

fn create_response_json() -> serde_json::Value {
    json!({
        "code": 0,
        "msg": "success",
        "data": {
            "project": project_json(),
            "session": null,
            "import_auto_deploy": null
        }
    })
}

#[tokio::test]
async fn list_projects() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({ "code": 0, "msg": "success", "data": [project_json()] }));
    });

    let projects = authed_client(&server).projects().list().await.unwrap();

    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].id, "proj_123");
    assert_eq!(
        projects[0].repository_full_name.as_deref(),
        Some("d1v/demo")
    );
    mock.assert();
}

#[tokio::test]
async fn create_project() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "project_name": "demo",
                "project_description": "Demo project",
                "enable_database": true,
                "enable_pay": false,
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(create_response_json());
    });

    let response = authed_client(&server)
        .projects()
        .create(&CreateProject {
            project_name: "demo".to_string(),
            project_description: "Demo project".to_string(),
            enable_database: Some(true),
            enable_pay: Some(false),
            enable_resend: None,
        })
        .await
        .unwrap();

    assert_eq!(response.project.id, "proj_123");
    assert!(response.session.is_none());
    mock.assert();
}

#[tokio::test]
async fn create_with_integrations() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/create-with-integrations")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "prompt": "Build a CRM",
                "max_desc_len": 120,
                "enable_database": true
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(create_response_json());
    });

    let response = authed_client(&server)
        .projects()
        .create_with_integrations(&CreateProjectWithIntegrations {
            prompt: "Build a CRM".to_string(),
            max_desc_len: Some(120),
            enable_database: Some(true),
            ..CreateProjectWithIntegrations::default()
        })
        .await
        .unwrap();

    assert_eq!(response.project.id, "proj_123");
    mock.assert();
}

#[tokio::test]
async fn import_from_github() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/import-from-github")
            .query_param("schedule_auto_deploy", "false")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "repository_full_name": "d1v/demo",
                "repository_url": "https://github.com/d1v/demo.git",
                "default_branch": "main",
                "is_private": false
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(create_response_json());
    });

    let response = authed_client(&server)
        .projects()
        .import_from_github(
            &ImportFromGithub {
                repository_full_name: "d1v/demo".to_string(),
                repository_url: Some("https://github.com/d1v/demo.git".to_string()),
                default_branch: Some("main".to_string()),
                is_private: Some(false),
                ..ImportFromGithub::default()
            },
            Some(false),
        )
        .await
        .unwrap();

    assert_eq!(
        response.project.repository_full_name.as_deref(),
        Some("d1v/demo")
    );
    mock.assert();
}

#[tokio::test]
async fn import_public_to_org() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/import-public-to-org")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "source_url": "https://github.com/d1v/public-demo.git",
                "project_name": "public-demo",
                "private": true
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(create_response_json());
    });

    let response = authed_client(&server)
        .projects()
        .import_public_to_org(&ImportPublic {
            source_url: "https://github.com/d1v/public-demo.git".to_string(),
            project_name: "public-demo".to_string(),
            project_description: None,
            private: Some(true),
        })
        .await
        .unwrap();

    assert_eq!(response.project.id, "proj_123");
    mock.assert();
}

#[tokio::test]
async fn import_from_local() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/import-from-local")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(create_response_json());
    });

    let response = authed_client(&server)
        .projects()
        .import_from_local(ImportLocal {
            project_name: Some("local-demo".to_string()),
            private: Some(true),
            files: vec![LocalImportFile {
                path: "index.html".to_string(),
                bytes: b"<h1>Hello</h1>".to_vec(),
            }],
            wait_for_deploy: Some(false),
            ..ImportLocal::default()
        })
        .await
        .unwrap();

    assert_eq!(response.project.id, "proj_123");
    mock.assert();
}

#[tokio::test]
async fn cli_import_local() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/cli-import-local")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(create_response_json());
    });

    let response = authed_client(&server)
        .projects()
        .cli_import_local(ImportLocal {
            project_name: Some("cli-demo".to_string()),
            single_file_name: Some("index.html".to_string()),
            single_file_type: Some("html".to_string()),
            single_file_content: Some("<h1>Hello</h1>".to_string()),
            wait_for_deploy: Some(false),
            ..ImportLocal::default()
        })
        .await
        .unwrap();

    assert_eq!(response.project.id, "proj_123");
    mock.assert();
}

#[tokio::test]
async fn templates() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/templates")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": [{
                    "template_repo": "d1v/remix-template",
                    "name": "Remix",
                    "description": "Remix starter",
                    "category": "foundation",
                    "kind": "foundation",
                    "featured": true,
                    "rank": 1
                }]
            }));
    });

    let templates = authed_client(&server).projects().templates().await.unwrap();

    assert_eq!(templates[0].template_repo, "d1v/remix-template");
    assert_eq!(templates[0].featured, Some(true));
    mock.assert();
}

#[tokio::test]
async fn generate_meta() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/ai/generate-meta")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "prompt": "Build a todo app",
                "max_desc_len": 120
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "project_name": "todo-app",
                    "project_description": "A focused todo app",
                    "emoji": "✅"
                }
            }));
    });

    let meta = authed_client(&server)
        .projects()
        .generate_meta(&GenerateProjectMeta {
            prompt: "Build a todo app".to_string(),
            max_desc_len: Some(120),
        })
        .await
        .unwrap();

    assert_eq!(meta["project_name"], "todo-app");
    mock.assert();
}

#[tokio::test]
async fn search_projects() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/search")
            .query_param("keyword", "demo")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({ "code": 0, "msg": "success", "data": [project_json()] }));
    });

    let projects = authed_client(&server)
        .projects()
        .search("demo")
        .await
        .unwrap();

    assert_eq!(projects[0].project_name, "demo");
    mock.assert();
}

#[tokio::test]
async fn get_project_with_sync() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123")
            .query_param("sync", "false")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({ "code": 0, "msg": "success", "data": project_json() }));
    });

    let project = authed_client(&server)
        .projects()
        .get("proj_123", Some(false))
        .await
        .unwrap();

    assert_eq!(project.id, "proj_123");
    mock.assert();
}

#[tokio::test]
async fn update_project() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(PUT)
            .path("/api/projects/proj_123")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "project_name": "renamed",
                "auto_deploy_on_execute": false
            }));
        let mut project = project_json();
        project["project_name"] = json!("renamed");
        project["auto_deploy_on_execute"] = json!(false);
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({ "code": 0, "msg": "success", "data": project }));
    });

    let project = authed_client(&server)
        .projects()
        .update(
            "proj_123",
            &UpdateProject {
                project_name: Some("renamed".to_string()),
                auto_deploy_on_execute: Some(false),
                ..UpdateProject::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(project.project_name, "renamed");
    assert_eq!(project.auto_deploy_on_execute, Some(false));
    mock.assert();
}

#[tokio::test]
async fn delete_project() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(DELETE)
            .path("/api/projects/proj_123")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({ "code": 0, "msg": "success", "data": null }));
    });

    authed_client(&server)
        .projects()
        .delete("proj_123")
        .await
        .unwrap();

    mock.assert();
}

#[tokio::test]
async fn project_database() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/database/proj_123")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": [{"table_name": "users"}]
            }));
    });

    let database = authed_client(&server)
        .projects()
        .database("proj_123")
        .await
        .unwrap();

    assert_eq!(database[0]["table_name"], "users");
    mock.assert();
}

#[tokio::test]
async fn github_migration_status() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123/github-migration-status")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "required": true,
                    "reason": "legacy",
                    "source_repository_full_name": "source/demo",
                    "target_repository_full_name": "d1v/demo",
                    "repository_mode": "mirrored",
                    "platform_managed_repository": false,
                    "has_direct_write_access": false,
                    "can_migrate_to_platform": true
                }
            }));
    });

    let status = authed_client(&server)
        .projects()
        .github_migration_status("proj_123")
        .await
        .unwrap();

    assert!(status.required);
    assert!(status.can_migrate_to_platform);
    mock.assert();
}

#[tokio::test]
async fn migrate_github_to_platform() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/proj_123/github-migrate-platform")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({ "code": 0, "msg": "success", "data": project_json() }));
    });

    let project = authed_client(&server)
        .projects()
        .migrate_github_to_platform("proj_123")
        .await
        .unwrap();

    assert_eq!(project.id, "proj_123");
    mock.assert();
}

#[tokio::test]
async fn publish_project() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/proj_123/publish")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "success": true,
                    "commit_hash": "abc123",
                    "message": "published",
                    "production_url": "https://demo.example.com"
                }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .publish("proj_123")
        .await
        .unwrap();

    assert!(response.success);
    assert_eq!(response.commit_hash.as_deref(), Some("abc123"));
    mock.assert();
}

#[tokio::test]
async fn project_deployments() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123/deployments")
            .query_param("environment", "prod")
            .query_param("limit", "10")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": [{
                    "id": 1,
                    "project_id": "proj_123",
                    "environment": "prod",
                    "status": "success",
                    "git_commit_sha": "abc123",
                    "created_at": "2026-05-01T00:00:00Z"
                }]
            }));
    });

    let deployments = authed_client(&server)
        .projects()
        .deployments(
            "proj_123",
            &ProjectDeploymentOptions {
                environment: Some("prod".to_string()),
                limit: Some(10),
            },
        )
        .await
        .unwrap();

    assert_eq!(deployments[0].status, "success");
    mock.assert();
}

#[tokio::test]
async fn transfer_project() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/proj_123/transfer")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({ "target_email": "target@example.com" }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": { "project": project_json() }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .transfer(
            "proj_123",
            &TransferProject {
                target_email: "target@example.com".to_string(),
            },
        )
        .await
        .unwrap();

    assert_eq!(response["project"]["id"], "proj_123");
    mock.assert();
}

#[tokio::test]
async fn generate_project_emojis() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/admin/generate-emojis")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": { "updated": 3 }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .generate_emojis()
        .await
        .unwrap();

    assert_eq!(response["updated"], 3);
    mock.assert();
}
