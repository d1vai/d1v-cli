use d1v_api::Client;
use d1v_api::api::projects::{
    AssetFile, ColumnIdentity, CreatePayBankAccount, CreateProjectWithIntegrations, DbColumn,
    DeleteDbRows, DeploymentEnvironment, Direction, Engine, ExecuteSession, GenerateMeta,
    Granularity, ImportFromGithub, ImportLocal, InsertDbRow, LocalImportFile, MessageType,
    SessionType, TokenRequest, UpdateDbRows, UpdateEnvVar, UpdatePayBankAccount, UpdateProject,
    UploadAsset,
};
use httpmock::prelude::*;
use jiff::Timestamp;
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
        .create("demo", "Demo project")
        .enable_database(true)
        .enable_pay(false)
        .call()
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
        .import_public_to_org("https://github.com/d1v/public-demo.git", "public-demo")
        .private(true)
        .call()
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
        .generate_meta(&GenerateMeta {
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
        .project("proj_123")
        .get(Some(false))
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
        .project("proj_123")
        .update(&UpdateProject {
            project_name: Some("renamed".to_string()),
            auto_deploy_on_execute: Some(false),
            ..UpdateProject::default()
        })
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
        .project("proj_123")
        .delete()
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
        .project("proj_123")
        .database()
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
        .project("proj_123")
        .github_migration_status()
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
        .project("proj_123")
        .migrate_github_to_platform()
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
        .project("proj_123")
        .publish()
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
        .project("proj_123")
        .deployments()
        .environment(DeploymentEnvironment::Prod)
        .limit(10)
        .call()
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
        .project("proj_123")
        .transfer("target@example.com")
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

#[tokio::test]
async fn project_db_schema() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123/db/schema")
            .query_param("branch", "main")
            .query_param("include_views", "true")
            .query_param("with_row_counts", "true")
            .query_param("include_system_schemas", "false")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "tables": [{
                        "schema": "public",
                        "name": "users",
                        "kind": "BASE TABLE",
                        "columns": [],
                        "primary_key": [],
                        "foreign_keys": [],
                        "row_count": 2
                    }]
                }
            }));
    });

    let schema = authed_client(&server)
        .projects()
        .project("proj_123")
        .db()
        .schema()
        .branch("main")
        .include_views(true)
        .with_row_counts(true)
        .include_system_schemas(false)
        .call()
        .await
        .unwrap();

    assert_eq!(schema.tables[0].name, "users");
    mock.assert();
}

#[tokio::test]
async fn project_db_data() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123/db/data")
            .query_param("branch", "dev")
            .query_param("limit_per_table", "5")
            .query_param("include_views", "false")
            .query_param("include_system_schemas", "false")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "tables": [{
                        "schema_name": "public",
                        "table_name": "users",
                        "rows": [{"id": 1, "email": "user@example.com"}]
                    }]
                }
            }));
    });

    let data = authed_client(&server)
        .projects()
        .project("proj_123")
        .db()
        .data()
        .branch("dev")
        .limit_per_table(5)
        .include_views(false)
        .include_system_schemas(false)
        .call()
        .await
        .unwrap();

    assert_eq!(data["tables"][0]["rows"][0]["email"], "user@example.com");
    mock.assert();
}

#[tokio::test]
async fn project_db_branches() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123/db/branches")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": [{
                    "id": "br-main",
                    "name": "main",
                    "current": true
                }]
            }));
    });

    let branches = authed_client(&server)
        .projects()
        .project("proj_123")
        .db()
        .branches()
        .await
        .unwrap();

    assert_eq!(branches[0]["name"], "main");
    mock.assert();
}

#[tokio::test]
async fn neon_usage() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/db/neon-usage")
            .query_param("from_iso", "2026-05-01T00:00:00Z")
            .query_param("to_iso", "2026-05-02T00:00:00Z")
            .query_param("granularity", "hourly")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "granularity": "hourly",
                    "projects": []
                }
            }));
    });

    let usage = authed_client(&server)
        .projects()
        .neon_usage()
        .from_iso("2026-05-01T00:00:00Z".parse().unwrap())
        .to_iso("2026-05-02T00:00:00Z".parse().unwrap())
        .granularity(Granularity::Hourly)
        .call()
        .await
        .unwrap();

    assert_eq!(usage["granularity"], "hourly");
    mock.assert();
}

#[tokio::test]
async fn create_db_table() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/proj_123/db/tables")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "schema_name": "public",
                "table_name": "users",
                "columns": [{
                    "name": "id",
                    "data_type": "INTEGER",
                    "is_nullable": false,
                    "identity": "by_default"
                }],
                "primary_key": ["id"],
                "branch": "main",
                "create_schema_if_missing": true
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": { "message": "table created" }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .db()
        .create_table("users")
        .columns(vec![DbColumn {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            is_nullable: Some(false),
            default_expr: None,
            identity: Some(ColumnIdentity::ByDefault),
        }])
        .schema_name("public")
        .primary_key(vec!["id".to_string()])
        .branch("main")
        .create_schema_if_missing(true)
        .call()
        .await
        .unwrap();

    assert_eq!(response, "table created");
    mock.assert();
}

#[tokio::test]
async fn drop_db_table() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(DELETE)
            .path("/api/projects/proj_123/db/tables/public/users")
            .query_param("branch", "main")
            .query_param("cascade", "true")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": { "message": "table dropped" }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .db()
        .drop_table("public", "users")
        .branch("main")
        .cascade(true)
        .call()
        .await
        .unwrap();

    assert_eq!(response, "table dropped");
    mock.assert();
}

#[tokio::test]
async fn list_db_table_rows() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123/db/tables/public/users/rows")
            .query_param("branch", "dev")
            .query_param("limit", "10")
            .query_param("offset", "20")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": [{ "id": 1, "email": "user@example.com" }]
            }));
    });

    let rows = authed_client(&server)
        .projects()
        .project("proj_123")
        .db()
        .list_table_rows("public", "users")
        .branch("dev")
        .limit(10)
        .offset(20)
        .call()
        .await
        .unwrap();

    assert_eq!(rows[0]["email"], "user@example.com");
    mock.assert();
}

#[tokio::test]
async fn insert_db_table_row() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/proj_123/db/tables/public/users/rows")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "values": { "email": "user@example.com" },
                "branch": "main"
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": { "affected": 1 }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .db()
        .insert_table_row(
            "public",
            "users",
            &InsertDbRow {
                values: serde_json::Map::from_iter([(
                    "email".to_string(),
                    json!("user@example.com"),
                )]),
                branch: Some("main".to_string()),
            },
        )
        .await
        .unwrap();

    assert_eq!(response, 1);
    mock.assert();
}

#[tokio::test]
async fn update_db_table_rows() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(PATCH)
            .path("/api/projects/proj_123/db/tables/public/users/rows")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "where": { "id": 1 },
                "values": { "email": "updated@example.com" },
                "branch": "main"
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": { "affected": 1 }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .db()
        .update_table_rows(
            "public",
            "users",
            &UpdateDbRows {
                where_: serde_json::Map::from_iter([("id".to_string(), json!(1))]),
                values: serde_json::Map::from_iter([(
                    "email".to_string(),
                    json!("updated@example.com"),
                )]),
                branch: Some("main".to_string()),
            },
        )
        .await
        .unwrap();

    assert_eq!(response, 1);
    mock.assert();
}

#[tokio::test]
async fn delete_db_table_rows() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/proj_123/db/tables/public/users/rows/delete")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "where": { "id": 1 },
                "branch": "main"
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": { "affected": 1 }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .db()
        .delete_table_rows(
            "public",
            "users",
            &DeleteDbRows {
                where_: serde_json::Map::from_iter([("id".to_string(), json!(1))]),
                branch: Some("main".to_string()),
            },
        )
        .await
        .unwrap();

    assert_eq!(response, 1);
    mock.assert();
}

#[tokio::test]
async fn execute_db_sql() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/proj_123/db/sql")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "sql": "select * from users",
                "branch": "main",
                "dry_run": false,
                "read_only": true,
                "max_rows": 50
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "status": "success",
                    "statement_count": 1,
                    "results": []
                }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .db()
        .execute_sql("select * from users")
        .branch("main")
        .dry_run(false)
        .read_only(true)
        .max_rows(50)
        .call()
        .await
        .unwrap();

    assert_eq!(response["status"], "success");
    mock.assert();
}

#[tokio::test]
async fn issue_project_token() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/proj_123/project-token/issue")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "scopes": ["db:read", "migrate"],
                "ttl_seconds": 900
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "project_token": "project.jwt",
                    "expires_at": "2026-05-01T00:15:00+00:00",
                    "scopes": ["db:read", "migrate"]
                }
            }));
    });

    let token = authed_client(&server)
        .projects()
        .project("proj_123")
        .issue_token(&TokenRequest {
            scopes: Some(vec!["db:read".to_string(), "migrate".to_string()]),
            ttl_seconds: Some(900),
        })
        .await
        .unwrap();

    assert_eq!(token.project_token, "project.jwt");
    assert_eq!(token.scopes, ["db:read", "migrate"]);
    mock.assert();
}

#[tokio::test]
async fn refresh_project_token() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/proj_123/project-token/refresh")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "ttl_seconds": 1800
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "project_token": "refreshed.jwt",
                    "expires_at": "2026-05-01T00:30:00+00:00",
                    "scopes": ["db:read", "migrate"]
                }
            }));
    });

    let token = authed_client(&server)
        .projects()
        .project("proj_123")
        .refresh_token(&TokenRequest {
            scopes: None,
            ttl_seconds: Some(1800),
        })
        .await
        .unwrap();

    assert_eq!(token.project_token, "refreshed.jwt");
    mock.assert();
}

#[tokio::test]
async fn execute_project_session() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/proj_123/sessions/execute")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "prompt": "Build a todo app",
                "session_type": "new",
                "model": "gpt-5.4",
                "engine": "codex",
                "auto_deploy": false
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "session_id": "sess_123",
                    "websocket_url": "wss://example.com/ws/sess_123",
                    "session": {
                        "project_id": "proj_123",
                        "session_id": "sess_123",
                        "model": "gpt-5.4",
                        "status": "running",
                        "websocket_url": "wss://example.com/ws/sess_123"
                    }
                }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .execute_session(
            &ExecuteSession::builder()
                .prompt("Build a todo app")
                .session_type(SessionType::New)
                .model("gpt-5.4")
                .engine(Engine::Codex)
                .auto_deploy(false)
                .build(),
        )
        .await
        .unwrap();

    assert_eq!(response.session_id, "sess_123");
    assert_eq!(response.session.status.as_deref(), Some("running"));
    mock.assert();
}

#[tokio::test]
async fn project_history() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123/history")
            .query_param("limit", "10")
            .query_param("before_ts", "2026-05-02T00:00:00Z")
            .query_param("before_id", "99")
            .query_param("direction", "user")
            .query_param("message_type", "prompt,result")
            .query_param("include_payload", "false")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": [{
                    "id": 1,
                    "project_id": "proj_123",
                    "direction": "user",
                    "message_type": "prompt",
                    "message_text": "Build a todo app",
                    "payload": {},
                    "created_at": "2026-05-01T00:00:00Z"
                }]
            }));
    });

    let history = authed_client(&server)
        .projects()
        .project("proj_123")
        .history()
        .limit(10)
        .before_ts("2026-05-02T00:00:00Z".parse().unwrap())
        .before_id(99)
        .direction(Direction::User)
        .message_type(vec![MessageType::Prompt, MessageType::Result])
        .include_payload(false)
        .call()
        .await
        .unwrap();

    assert_eq!(history[0].message_text.as_deref(), Some("Build a todo app"));
    mock.assert();
}

#[tokio::test]
async fn active_project_session() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123/sessions/active")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "project_id": "proj_123",
                    "session_id": "sess_123",
                    "model": "gpt-5.4",
                    "status": "running"
                }
            }));
    });

    let session = authed_client(&server)
        .projects()
        .project("proj_123")
        .active_session()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(session.session_id, "sess_123");
    mock.assert();
}

#[tokio::test]
async fn no_active_project_session() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123/sessions/active")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({ "code": 0, "msg": "success", "data": null }));
    });

    let session = authed_client(&server)
        .projects()
        .project("proj_123")
        .active_session()
        .await
        .unwrap();

    assert!(session.is_none());
    mock.assert();
}

#[tokio::test]
async fn project_history_detail() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123/history/1")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "id": 1,
                    "project_id": "proj_123",
                    "direction": "assistant",
                    "message_type": "result",
                    "message_text": "Done",
                    "payload": { "session_id": "sess_123" },
                    "created_at": "2026-05-01T00:00:00Z"
                }
            }));
    });

    let detail = authed_client(&server)
        .projects()
        .project("proj_123")
        .history_detail(1)
        .await
        .unwrap();

    assert_eq!(detail.payload["session_id"], "sess_123");
    mock.assert();
}

#[tokio::test]
async fn cancel_project_session() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/sessions/sess_123/cancel")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "session_id": "sess_123",
                    "cancelled": true
                }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .cancel_session("sess_123")
        .await
        .unwrap();

    assert_eq!(response["cancelled"], true);
    mock.assert();
}

#[tokio::test]
async fn execute_claude_session() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/claude/execute")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "prompt": "Continue the task",
                "session_type": "continue",
                "session_id": "sess_123",
                "project_path": "/users/demo/projects/app",
                "model": "claude-sonnet-4.6",
                "engine": "claude"
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "session_id": "sess_123",
                    "websocket_url": "wss://example.com/ws/sess_123",
                    "session": {
                        "project_id": "proj_123",
                        "opcode_project_path": "/users/demo/projects/app",
                        "session_id": "sess_123",
                        "model": "claude-sonnet-4.6",
                        "status": "running"
                    }
                }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .execute_claude_session(
            &ExecuteSession::builder()
                .prompt("Continue the task")
                .session_type(SessionType::Continue)
                .session_id("sess_123")
                .model("claude-sonnet-4.6")
                .engine(Engine::Claude)
                .project_path("/users/demo/projects/app")
                .build(),
        )
        .await
        .unwrap();

    assert_eq!(response.session_id, "sess_123");
    assert_eq!(
        response.session.opcode_project_path.as_deref(),
        Some("/users/demo/projects/app")
    );
    mock.assert();
}

#[tokio::test]
async fn claude_user_projects() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/api/claude/users/default/projects")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": [{
                    "id": "opcode_123",
                    "name": "demo",
                    "path": "/users/demo/projects/app",
                    "username": "demo"
                }]
            }));
    });

    let projects = authed_client(&server)
        .projects()
        .claude_user_projects("default")
        .await
        .unwrap();

    assert_eq!(projects[0].id, "opcode_123");
    assert_eq!(
        projects[0].path.as_deref(),
        Some("/users/demo/projects/app")
    );
    mock.assert();
}

#[tokio::test]
async fn pay_products() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123/pay/products")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "items": [{
                        "id": "prod_123",
                        "name": "Pro Plan",
                        "active": true
                    }]
                }
            }));
    });

    let products = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .products()
        .await
        .unwrap();

    assert_eq!(products["items"][0]["id"], "prod_123");
    mock.assert();
}

#[tokio::test]
async fn create_pay_product() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/proj_123/pay/products")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "userId": "pay_user_123",
                "name": "Pro Plan",
                "description": "Monthly plan",
                "category": "subscription",
                "active": true,
                "platformFeePercentage": 2.5,
                "price": {
                    "amount": 9900,
                    "currency": "usd"
                }
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "id": "prod_123",
                    "name": "Pro Plan"
                }
            }));
    });

    let product = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .create_product("pay_user_123", "Pro Plan")
        .description("Monthly plan")
        .category("subscription")
        .active(true)
        .platform_fee_percentage(2.5)
        .price(&json!({"amount": 9900, "currency": "usd"}))
        .call()
        .await
        .unwrap();

    assert_eq!(product["id"], "prod_123");
    mock.assert();
}

#[tokio::test]
async fn pay_product_payment_link() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123/pay/products/prod_123/payment-link")
            .query_param("prefilled_email", "buyer@example.com")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "url": "https://pay.example.com/link"
                }
            }));
    });

    let link = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .product_payment_link("prod_123", Some("buyer@example.com"))
        .await
        .unwrap();

    assert_eq!(link["url"], "https://pay.example.com/link");
    mock.assert();
}

#[tokio::test]
async fn create_pay_payment_link() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/proj_123/pay/create-payment-link")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "productId": "prod_123",
                "userId": "pay_user_123",
                "successUrl": "https://example.com/success",
                "cancelUrl": "https://example.com/cancel",
                "customFields": {
                    "source": "cli"
                }
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "url": "https://pay.example.com/checkout"
                }
            }));
    });

    let link = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .create_payment_link(
            "prod_123",
            "pay_user_123",
            "https://example.com/success",
            "https://example.com/cancel",
            Some(&json!({ "source": "cli" })),
        )
        .await
        .unwrap();

    assert_eq!(link["url"], "https://pay.example.com/checkout");
    mock.assert();
}

#[tokio::test]
async fn create_pay_payment_intent() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/proj_123/pay/create-payment-intent")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "priceId": "price_123",
                "customerEmail": "buyer@example.com"
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "client_secret": "pi_secret"
                }
            }));
    });

    let intent = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .create_payment_intent("price_123", Some("buyer@example.com"))
        .await
        .unwrap();

    assert_eq!(intent["client_secret"], "pi_secret");
    mock.assert();
}

#[tokio::test]
async fn pay_transactions() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123/pay/transactions")
            .query_param("created_after", "1700000000")
            .query_param("status", "succeeded")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "items": [{
                        "id": "txn_123",
                        "status": "succeeded"
                    }]
                }
            }));
    });

    let transactions = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .transactions(
            Some(Timestamp::from_second(1_700_000_000).unwrap()),
            Some("succeeded"),
        )
        .await
        .unwrap();

    assert_eq!(transactions["items"][0]["id"], "txn_123");
    mock.assert();
}

#[tokio::test]
async fn pay_transactions_paginated() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123/pay/transactions/paginated")
            .query_param("page", "2")
            .query_param("pageSize", "25")
            .query_param("created_after", "1700000000")
            .query_param("status", "pending")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "page": 2,
                    "pageSize": 25,
                    "items": []
                }
            }));
    });

    let transactions = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .transactions_paginated(2, 25)
        .created_after(Timestamp::from_second(1_700_000_000).unwrap())
        .status("pending")
        .call()
        .await
        .unwrap();

    assert_eq!(transactions["page"], 2);
    mock.assert();
}

#[tokio::test]
async fn pay_transaction_stats() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123/pay/transactions/stats")
            .query_param("created_after", "1700000000")
            .query_param("status", "succeeded")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "count": 3,
                    "amount": 9900
                }
            }));
    });

    let stats = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .transaction_stats(
            Some(Timestamp::from_second(1_700_000_000).unwrap()),
            Some("succeeded"),
        )
        .await
        .unwrap();

    assert_eq!(stats["count"], 3);
    mock.assert();
}

#[tokio::test]
async fn pay_dashboard_metrics() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123/pay/dashboard/metrics")
            .query_param("days", "30")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "totalRevenue": 9900
                }
            }));
    });

    let metrics = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .dashboard_metrics(Some(30))
        .await
        .unwrap();

    assert_eq!(metrics["totalRevenue"], 9900);
    mock.assert();
}

#[tokio::test]
async fn pay_revenue() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123/pay/revenue")
            .query_param("days", "7")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "revenue": 1200
                }
            }));
    });

    let revenue = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .revenue(Some(7))
        .await
        .unwrap();

    assert_eq!(revenue["revenue"], 1200);
    mock.assert();
}

#[tokio::test]
async fn pay_dashboard_revenue() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123/pay/dashboard/revenue")
            .query_param("days", "7")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "revenue": 1200
                }
            }));
    });

    let revenue = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .dashboard_revenue(Some(7))
        .await
        .unwrap();

    assert_eq!(revenue["revenue"], 1200);
    mock.assert();
}

#[tokio::test]
async fn pay_webhooks() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123/pay/webhooks")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "items": [{
                        "id": "wh_123",
                        "url": "https://example.com/webhook"
                    }]
                }
            }));
    });

    let webhooks = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .webhooks()
        .await
        .unwrap();

    assert_eq!(webhooks["items"][0]["id"], "wh_123");
    mock.assert();
}

#[tokio::test]
async fn create_pay_webhook() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/proj_123/pay/webhooks")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "name": "payments",
                "url": "https://example.com/webhook",
                "events": ["payment.succeeded"],
                "isActive": true
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "id": "wh_123",
                    "name": "payments"
                }
            }));
    });

    let webhook = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .create_webhook("payments", "https://example.com/webhook")
        .events(vec!["payment.succeeded".to_string()])
        .is_active(true)
        .call()
        .await
        .unwrap();

    assert_eq!(webhook["id"], "wh_123");
    mock.assert();
}

#[tokio::test]
async fn update_pay_webhook() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(PATCH)
            .path("/api/projects/proj_123/pay/webhooks/wh_123")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "name": "updated",
                "isActive": false
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "id": "wh_123",
                    "name": "updated",
                    "isActive": false
                }
            }));
    });

    let webhook = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .update_webhook("wh_123")
        .name("updated")
        .is_active(false)
        .call()
        .await
        .unwrap();

    assert_eq!(webhook["name"], "updated");
    mock.assert();
}

#[tokio::test]
async fn regenerate_pay_webhook_secret() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/proj_123/pay/webhooks/wh_123/regenerate-secret")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "secret": "whsec_new"
                }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .regenerate_webhook_secret("wh_123")
        .await
        .unwrap();

    assert_eq!(response["secret"], "whsec_new");
    mock.assert();
}

#[tokio::test]
async fn delete_pay_webhook() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(DELETE)
            .path("/api/projects/proj_123/pay/webhooks/wh_123")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "deleted": true
                }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .delete_webhook("wh_123")
        .await
        .unwrap();

    assert_eq!(response["deleted"], true);
    mock.assert();
}

#[tokio::test]
async fn pay_bank_accounts() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123/pay/bank-accounts")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": { "items": [{ "id": "bank_123" }] }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .bank_accounts()
        .await
        .unwrap();

    assert_eq!(response["items"][0]["id"], "bank_123");
    mock.assert();
}

#[tokio::test]
async fn create_pay_bank_account() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/proj_123/pay/bank-accounts")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "accountHolderName": "D1V",
                "bankName": "Test Bank",
                "accountNumber": "123456789",
                "routingNumber": "021000021",
                "accountType": "checking",
                "currency": "USD",
                "country": "US",
                "isDefault": true
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": { "id": "bank_123" }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .create_bank_account(
            &CreatePayBankAccount::builder()
                .account_holder_name("D1V")
                .bank_name("Test Bank")
                .account_number("123456789")
                .routing_number("021000021")
                .account_type("checking")
                .currency("USD")
                .country("US")
                .is_default(true)
                .build(),
        )
        .await
        .unwrap();

    assert_eq!(response["id"], "bank_123");
    mock.assert();
}

#[tokio::test]
async fn pay_bank_account() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123/pay/bank-accounts/bank_123")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": { "id": "bank_123" }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .bank_account("bank_123")
        .await
        .unwrap();

    assert_eq!(response["id"], "bank_123");
    mock.assert();
}

#[tokio::test]
async fn update_pay_bank_account() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(PUT)
            .path("/api/projects/proj_123/pay/bank-accounts/bank_123")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "bankName": "Updated Bank",
                "isDefault": false
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": { "id": "bank_123", "bankName": "Updated Bank" }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .update_bank_account(
            "bank_123",
            &UpdatePayBankAccount::builder()
                .bank_name("Updated Bank")
                .is_default(false)
                .build(),
        )
        .await
        .unwrap();

    assert_eq!(response["bankName"], "Updated Bank");
    mock.assert();
}

#[tokio::test]
async fn set_default_pay_bank_account() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(PUT)
            .path("/api/projects/proj_123/pay/bank-accounts/bank_123/set-default")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": { "id": "bank_123", "isDefault": true }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .set_default_bank_account("bank_123")
        .await
        .unwrap();

    assert_eq!(response["isDefault"], true);
    mock.assert();
}

#[tokio::test]
async fn delete_pay_bank_account() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(DELETE)
            .path("/api/projects/proj_123/pay/bank-accounts/bank_123")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": { "deleted": true }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .delete_bank_account("bank_123")
        .await
        .unwrap();

    assert_eq!(response["deleted"], true);
    mock.assert();
}

#[tokio::test]
async fn pay_withdrawal_requests() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123/pay/withdrawal-requests")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": { "items": [{ "id": "wd_123" }] }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .withdrawal_requests()
        .await
        .unwrap();

    assert_eq!(response["items"][0]["id"], "wd_123");
    mock.assert();
}

#[tokio::test]
async fn create_pay_withdrawal_request() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/proj_123/pay/withdrawal-requests")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "amount": 100.5,
                "currency": "USD",
                "bankAccountId": "bank_123",
                "note": "monthly"
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": { "id": "wd_123" }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .create_withdrawal_request(100.5, "USD", "bank_123", Some("monthly"))
        .await
        .unwrap();

    assert_eq!(response["id"], "wd_123");
    mock.assert();
}

#[tokio::test]
async fn pay_withdrawals_alias() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123/pay/withdrawals")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": { "items": [{ "id": "wd_123" }] }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .withdrawals()
        .await
        .unwrap();

    assert_eq!(response["items"][0]["id"], "wd_123");
    mock.assert();
}

#[tokio::test]
async fn create_pay_withdrawal_alias() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/proj_123/pay/withdrawals")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "amount": 100.5,
                "currency": "USD",
                "bankAccountId": "bank_123"
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": { "id": "wd_123" }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .create_withdrawal(100.5, "USD", "bank_123", None)
        .await
        .unwrap();

    assert_eq!(response["id"], "wd_123");
    mock.assert();
}

#[tokio::test]
async fn pay_tokens() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123/pay/tokens")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": { "items": [{ "id": "tok_123" }] }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .tokens()
        .await
        .unwrap();

    assert_eq!(response["items"][0]["id"], "tok_123");
    mock.assert();
}

#[tokio::test]
async fn create_pay_token() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/proj_123/pay/tokens")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "name": "cli",
                "permissions": ["products:read"],
                "isActive": true,
                "expiresAt": "2026-05-01T00:00:00Z"
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": { "id": "tok_123", "token": "pay_secret" }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .create_token("cli")
        .permissions(vec!["products:read".to_string()])
        .is_active(true)
        .expires_at("2026-05-01T00:00:00Z".parse().unwrap())
        .call()
        .await
        .unwrap();

    assert_eq!(response["id"], "tok_123");
    mock.assert();
}

#[tokio::test]
async fn delete_pay_token() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(DELETE)
            .path("/api/projects/proj_123/pay/tokens/tok_123")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": { "deleted": true }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .pay()
        .delete_token("tok_123")
        .await
        .unwrap();

    assert_eq!(response["deleted"], true);
    mock.assert();
}

fn env_var_json() -> serde_json::Value {
    json!({
        "id": 1,
        "key": "API_KEY",
        "value": "***",
        "value_preview": "sk-***123",
        "description": "API key",
        "is_sensitive": true,
        "created_at": "2026-05-01T00:00:00Z",
        "updated_at": "2026-05-02T00:00:00Z"
    })
}

#[tokio::test]
async fn project_env_vars() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123/env-vars")
            .query_param("show_values", "true")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({ "code": 0, "msg": "success", "data": [env_var_json()] }));
    });

    let vars = authed_client(&server)
        .projects()
        .project("proj_123")
        .env()
        .vars(true)
        .await
        .unwrap();

    assert_eq!(vars[0].key, "API_KEY");
    mock.assert();
}

#[tokio::test]
async fn create_project_env_var() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/proj_123/env-vars")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "key": "API_KEY",
                "value": "sk-secret",
                "description": "API key",
                "is_sensitive": true
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({ "code": 0, "msg": "success", "data": env_var_json() }));
    });

    let var = authed_client(&server)
        .projects()
        .project("proj_123")
        .env()
        .create_var("API_KEY", "sk-secret")
        .description("API key")
        .is_sensitive(true)
        .call()
        .await
        .unwrap();

    assert_eq!(var.value_preview, "sk-***123");
    mock.assert();
}

#[tokio::test]
async fn update_project_env_var() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(PATCH)
            .path("/api/projects/proj_123/env-vars/1")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "value": "sk-updated",
                "is_sensitive": false
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({ "code": 0, "msg": "success", "data": env_var_json() }));
    });

    let var = authed_client(&server)
        .projects()
        .project("proj_123")
        .env()
        .update_var(
            1,
            &UpdateEnvVar::builder()
                .value("sk-updated")
                .is_sensitive(false)
                .build(),
        )
        .await
        .unwrap();

    assert_eq!(var.id, 1);
    mock.assert();
}

#[tokio::test]
async fn delete_project_env_var() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(DELETE)
            .path("/api/projects/proj_123/env-vars/1")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": { "message": "Environment variable 'API_KEY' deleted" }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .env()
        .delete_var(1)
        .await
        .unwrap();

    assert_eq!(response, "Environment variable 'API_KEY' deleted");
    mock.assert();
}

#[tokio::test]
async fn import_project_env_vars() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/proj_123/env-vars/batch-import")
            .header("authorization", "Bearer token123")
            .header("content-type", "application/json")
            .json_body(json!({
                "env_content": "API_KEY=sk-secret",
                "overwrite": true
            }));
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": { "message": "Import completed", "created": 1, "updated": 0, "skipped": 0, "total": 1 }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .env()
        .import_vars("API_KEY=sk-secret", true)
        .await
        .unwrap();

    assert_eq!(response.total, 1);
    mock.assert();
}

#[tokio::test]
async fn export_project_env_vars() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123/env-vars/export")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": { "content": "API_KEY=sk-secret", "filename": "proj_123.env" }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .env()
        .export_vars()
        .await
        .unwrap();

    assert_eq!(response.filename, "proj_123.env");
    mock.assert();
}

#[tokio::test]
async fn sync_project_env_vars_to_vercel() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/proj_123/env-vars/sync-vercel")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "message": "Vercel environment sync completed",
                    "vercel_dev_project_id": "vercel_dev_1",
                    "vercel_prod_project_id": "vercel_prod_1",
                    "dev_local_env_count": 3,
                    "prod_local_env_count": 2,
                    "dev_up_to_date": true,
                    "prod_up_to_date": true
                }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .env()
        .sync_vercel()
        .await
        .unwrap();

    assert_eq!(response.dev_up_to_date, true);
    mock.assert();
}

#[tokio::test]
async fn activate_project_pay_integration() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/proj_123/integrations/activate-pay")
            .header("authorization", "Bearer token123");
        let mut project = project_json();
        project["project_pay_id"] = json!("pay_user_123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "project": project,
                    "message": "Pay activated successfully"
                }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .integrations()
        .activate_pay()
        .await
        .unwrap();

    assert_eq!(
        response.project.project_pay_id.as_deref(),
        Some("pay_user_123")
    );
    assert_eq!(response.message, "Pay activated successfully");
    mock.assert();
}

#[tokio::test]
async fn refresh_project_pay_token() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/proj_123/integrations/refresh-pay-token")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "project": project_json(),
                    "message": "Pay token refreshed"
                }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .integrations()
        .refresh_pay_token()
        .await
        .unwrap();

    assert_eq!(response.message, "Pay token refreshed");
    mock.assert();
}

#[tokio::test]
async fn activate_project_database_integration() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/proj_123/integrations/activate-database")
            .header("authorization", "Bearer token123");
        let mut project = project_json();
        project["project_database_id"] = json!("db_456");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "project": project,
                    "message": "Database activated successfully"
                }
            }));
    });

    let response = authed_client(&server)
        .projects()
        .project("proj_123")
        .integrations()
        .activate_database()
        .await
        .unwrap();

    assert_eq!(
        response.project.project_database_id.as_deref(),
        Some("db_456")
    );
    assert_eq!(response.message, "Database activated successfully");
    mock.assert();
}

fn asset_json() -> serde_json::Value {
    json!({
        "provider": "s3",
        "bucket_or_container": "assets",
        "key": "proj_123/images/logo.png",
        "path": "images/logo.png",
        "url": "https://cdn.example.com/images/logo.png",
        "etag": "etag123",
        "size": 4,
        "content_type": "image/png"
    })
}

#[tokio::test]
async fn project_storage_structure() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/storage/proj_123/structure")
            .query_param("sub_path", "src")
            .query_param("ext", "tsx")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": { "children": [{ "path": "src/app.tsx" }] }
            }));
    });

    let structure = authed_client(&server)
        .projects()
        .project("proj_123")
        .storage()
        .structure()
        .sub_path("src")
        .ext("tsx")
        .call()
        .await
        .unwrap();

    assert_eq!(structure["children"][0]["path"], "src/app.tsx");
    mock.assert();
}

#[tokio::test]
async fn project_storage_file() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/storage/proj_123/files/src/app.tsx")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": { "path": "src/app.tsx", "content": "export default App" }
            }));
    });

    let file = authed_client(&server)
        .projects()
        .project("proj_123")
        .storage()
        .file("src/app.tsx")
        .await
        .unwrap();

    assert_eq!(file["path"], "src/app.tsx");
    mock.assert();
}

#[tokio::test]
async fn upload_project_asset() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/projects/proj_123/assets")
            .header("authorization", "Bearer token123")
            .header_exists("content-type")
            .body_includes(r#"name="path""#)
            .body_includes("images/logo.png")
            .body_includes(r#"name="file""#)
            .body_includes(r#"filename="logo.png""#);
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({ "code": 0, "msg": "success", "data": asset_json() }));
    });

    let asset = authed_client(&server)
        .projects()
        .project("proj_123")
        .storage()
        .upload_asset(UploadAsset {
            path: "images/logo.png".to_string(),
            file: AssetFile {
                path: "logo.png".to_string(),
                bytes: b"logo".to_vec(),
            },
        })
        .await
        .unwrap();

    assert_eq!(asset.path, "images/logo.png");
    mock.assert();
}

#[tokio::test]
async fn replace_project_asset() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(PUT)
            .path("/api/projects/proj_123/assets/images/logo.png")
            .header("authorization", "Bearer token123")
            .body_includes(r#"name="file""#)
            .body_includes(r#"filename="logo.png""#);
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({ "code": 0, "msg": "success", "data": asset_json() }));
    });

    let asset = authed_client(&server)
        .projects()
        .project("proj_123")
        .storage()
        .replace_asset(
            "images/logo.png",
            AssetFile {
                path: "logo.png".to_string(),
                bytes: b"logo".to_vec(),
            },
        )
        .await
        .unwrap();

    assert_eq!(asset.key, "proj_123/images/logo.png");
    mock.assert();
}

#[tokio::test]
async fn get_project_asset() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/projects/proj_123/assets/images/logo.png")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({ "code": 0, "msg": "success", "data": asset_json() }));
    });

    let asset = authed_client(&server)
        .projects()
        .project("proj_123")
        .storage()
        .asset("images/logo.png")
        .await
        .unwrap();

    assert_eq!(asset.content_type.as_deref(), Some("image/png"));
    mock.assert();
}

#[tokio::test]
async fn delete_project_asset() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(DELETE)
            .path("/api/projects/proj_123/assets/images/logo.png")
            .header("authorization", "Bearer token123");
        let mut asset = asset_json();
        asset["deleted"] = json!(true);
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({ "code": 0, "msg": "success", "data": asset }));
    });

    let asset = authed_client(&server)
        .projects()
        .project("proj_123")
        .storage()
        .delete_asset("images/logo.png")
        .await
        .unwrap();

    assert_eq!(asset.deleted, Some(true));
    mock.assert();
}
