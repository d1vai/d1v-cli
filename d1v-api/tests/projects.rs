use d1v_api::Client;
use d1v_api::api::projects::{
    CreatePayPaymentIntent, CreatePayPaymentLink, CreatePayProduct, CreateProject,
    CreateProjectDbTable, CreateProjectWithIntegrations, DeleteProjectDbRows,
    DropProjectDbTableOptions, ExecuteProjectSession, ExecuteProjectSql, GenerateProjectMeta,
    ImportFromGithub, ImportLocal, ImportPublic, InsertProjectDbRow, ListProjectDbRowsOptions,
    LocalImportFile, NeonUsageOptions, PayProductPaymentLinkOptions, ProjectDbColumn,
    ProjectDbDataOptions, ProjectDbSchemaOptions, ProjectDeploymentOptions, ProjectHistoryOptions,
    ProjectTokenRequest, TransferProject, UpdateProject, UpdateProjectDbRows,
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
        .deployments(&ProjectDeploymentOptions {
            environment: Some("prod".to_string()),
            limit: Some(10),
        })
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
        .transfer(&TransferProject {
            target_email: "target@example.com".to_string(),
        })
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
                        "schema_name": "public",
                        "table_name": "users",
                        "row_count": 2
                    }]
                }
            }));
    });

    let schema = authed_client(&server)
        .projects()
        .project("proj_123")
        .db()
        .schema(&ProjectDbSchemaOptions {
            branch: Some("main".to_string()),
            include_views: Some(true),
            with_row_counts: Some(true),
            include_system_schemas: Some(false),
        })
        .await
        .unwrap();

    assert_eq!(schema["tables"][0]["table_name"], "users");
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
        .data(&ProjectDbDataOptions {
            branch: Some("dev".to_string()),
            limit_per_table: Some(5),
            include_views: Some(false),
            include_system_schemas: Some(false),
        })
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
            .query_param("granularity", "hour")
            .header("authorization", "Bearer token123");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "code": 0,
                "msg": "success",
                "data": {
                    "granularity": "hour",
                    "projects": []
                }
            }));
    });

    let usage = authed_client(&server)
        .projects()
        .neon_usage(&NeonUsageOptions {
            from_iso: Some("2026-05-01T00:00:00Z".parse().unwrap()),
            to_iso: Some("2026-05-02T00:00:00Z".parse().unwrap()),
            granularity: Some("hour".to_string()),
        })
        .await
        .unwrap();

    assert_eq!(usage["granularity"], "hour");
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
        .create_table(&CreateProjectDbTable {
            schema_name: Some("public".to_string()),
            table_name: "users".to_string(),
            columns: vec![ProjectDbColumn {
                name: "id".to_string(),
                data_type: "INTEGER".to_string(),
                is_nullable: Some(false),
                default_expr: None,
                identity: Some("by_default".to_string()),
            }],
            primary_key: Some(vec!["id".to_string()]),
            branch: Some("main".to_string()),
            create_schema_if_missing: Some(true),
        })
        .await
        .unwrap();

    assert_eq!(response["message"], "table created");
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
        .drop_table(
            "public",
            "users",
            &DropProjectDbTableOptions {
                branch: Some("main".to_string()),
                cascade: Some(true),
            },
        )
        .await
        .unwrap();

    assert_eq!(response["message"], "table dropped");
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
        .list_table_rows(
            "public",
            "users",
            &ListProjectDbRowsOptions {
                branch: Some("dev".to_string()),
                limit: Some(10),
                offset: Some(20),
            },
        )
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
            &InsertProjectDbRow {
                values: serde_json::Map::from_iter([(
                    "email".to_string(),
                    json!("user@example.com"),
                )]),
                branch: Some("main".to_string()),
            },
        )
        .await
        .unwrap();

    assert_eq!(response["affected"], 1);
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
            &UpdateProjectDbRows {
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

    assert_eq!(response["affected"], 1);
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
            &DeleteProjectDbRows {
                where_: serde_json::Map::from_iter([("id".to_string(), json!(1))]),
                branch: Some("main".to_string()),
            },
        )
        .await
        .unwrap();

    assert_eq!(response["affected"], 1);
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
        .execute_sql(&ExecuteProjectSql {
            sql: "select * from users".to_string(),
            branch: Some("main".to_string()),
            dry_run: Some(false),
            read_only: Some(true),
            approval_token: None,
            max_rows: Some(50),
        })
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
        .issue_token(&ProjectTokenRequest {
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
        .refresh_token(&ProjectTokenRequest {
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
        .execute_session(&ExecuteProjectSession {
            prompt: "Build a todo app".to_string(),
            session_type: Some("new".to_string()),
            session_id: None,
            model: Some("gpt-5.4".to_string()),
            engine: Some("codex".to_string()),
            system_prompt: None,
            project_path: None,
            auto_deploy: Some(false),
        })
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
        .history(&ProjectHistoryOptions {
            limit: Some(10),
            before_ts: Some("2026-05-02T00:00:00Z".parse().unwrap()),
            before_id: Some(99),
            direction: Some("user".to_string()),
            message_type: Some("prompt,result".to_string()),
            include_payload: Some(false),
        })
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
        .execute_claude_session(&ExecuteProjectSession {
            prompt: "Continue the task".to_string(),
            session_type: Some("continue".to_string()),
            session_id: Some("sess_123".to_string()),
            model: Some("claude-sonnet-4.6".to_string()),
            engine: Some("claude".to_string()),
            system_prompt: None,
            project_path: Some("/users/demo/projects/app".to_string()),
            auto_deploy: None,
        })
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
        .create_product(&CreatePayProduct {
            user_id: Some("pay_user_123".to_string()),
            name: Some("Pro Plan".to_string()),
            description: Some("Monthly plan".to_string()),
            category: Some("subscription".to_string()),
            active: Some(true),
            platform_fee_percentage: Some(2.5),
            price: Some(json!({
                "amount": 9900,
                "currency": "usd"
            })),
        })
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
        .product_payment_link(
            "prod_123",
            &PayProductPaymentLinkOptions {
                prefilled_email: Some("buyer@example.com".to_string()),
            },
        )
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
        .create_payment_link(&CreatePayPaymentLink {
            product_id: "prod_123".to_string(),
            user_id: "pay_user_123".to_string(),
            success_url: "https://example.com/success".to_string(),
            cancel_url: "https://example.com/cancel".to_string(),
            custom_fields: Some(json!({ "source": "cli" })),
        })
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
        .create_payment_intent(&CreatePayPaymentIntent {
            price_id: "price_123".to_string(),
            customer_email: Some("buyer@example.com".to_string()),
        })
        .await
        .unwrap();

    assert_eq!(intent["client_secret"], "pi_secret");
    mock.assert();
}
