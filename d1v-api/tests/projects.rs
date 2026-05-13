use d1v_api::Client;
use d1v_api::api::projects::{
    CreateProject, CreateProjectDbTable, CreateProjectWithIntegrations, DeleteProjectDbRows,
    DropProjectDbTableOptions, ExecuteProjectSql, GenerateProjectMeta, ImportFromGithub,
    ImportLocal, ImportPublic, InsertProjectDbRow, ListProjectDbRowsOptions, LocalImportFile,
    NeonUsageOptions, ProjectDbColumn, ProjectDbDataOptions, ProjectDbSchemaOptions,
    ProjectDeploymentOptions, ProjectTokenRequest, TransferProject, UpdateProject,
    UpdateProjectDbRows,
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
        .db("proj_123")
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
        .db("proj_123")
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
        .db("proj_123")
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
        .db("proj_123")
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
        .db("proj_123")
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
        .db("proj_123")
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
        .db("proj_123")
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
        .db("proj_123")
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
        .db("proj_123")
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
        .db("proj_123")
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
        .issue_project_token(
            "proj_123",
            &ProjectTokenRequest {
                scopes: Some(vec!["db:read".to_string(), "migrate".to_string()]),
                ttl_seconds: Some(900),
            },
        )
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
        .refresh_project_token(
            "proj_123",
            &ProjectTokenRequest {
                scopes: None,
                ttl_seconds: Some(1800),
            },
        )
        .await
        .unwrap();

    assert_eq!(token.project_token, "refreshed.jwt");
    mock.assert();
}
