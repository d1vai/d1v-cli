use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{Client, Error};

pub struct DbApi {
    client: Client,
}

impl Client {
    #[must_use]
    pub fn db(&self) -> DbApi {
        DbApi {
            client: self.clone(),
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbSchemaOptions {
    pub branch: Option<String>,
    pub include_views: Option<bool>,
    pub with_row_counts: Option<bool>,
    pub include_system_schemas: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbDataOptions {
    pub branch: Option<String>,
    pub limit_per_table: Option<u32>,
    pub include_views: Option<bool>,
    pub include_system_schemas: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbRowsOptions {
    pub branch: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbColumnSchema {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub default: Option<String>,
    pub ordinal_position: Option<i64>,
    pub is_primary_key: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbForeignKeySchema {
    pub constrained_columns: Vec<String>,
    pub referred_schema: String,
    pub referred_table: String,
    pub referred_columns: Vec<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbTableSchema {
    pub schema: String,
    pub name: String,
    pub kind: String,
    pub columns: Vec<DbColumnSchema>,
    pub primary_key: Vec<String>,
    pub foreign_keys: Vec<DbForeignKeySchema>,
    pub row_count: Option<i64>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbSchemaResponse {
    pub tables: Vec<DbTableSchema>,
    pub default_schema: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbBranch {
    pub id: String,
    pub name: String,
    pub primary: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbTableColumnInput {
    pub name: String,
    pub data_type: String,
    pub is_nullable: Option<bool>,
    pub default_expr: Option<String>,
    pub identity: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbCreateTableRequest {
    pub schema_name: Option<String>,
    pub table_name: String,
    pub columns: Vec<DbTableColumnInput>,
    pub primary_key: Option<Vec<String>>,
    pub branch: Option<String>,
    pub create_schema_if_missing: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbRenameTableRequest {
    pub new_table_name: String,
    pub branch: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbValuesRequest {
    pub values: BTreeMap<String, serde_json::Value>,
    pub branch: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbUpdateRowsRequest {
    pub where_: BTreeMap<String, serde_json::Value>,
    pub values: BTreeMap<String, serde_json::Value>,
    pub branch: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbDeleteRowsRequest {
    pub where_: BTreeMap<String, serde_json::Value>,
    pub branch: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbMessageResponse {
    pub message: String,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbAffectedResponse {
    pub affected: i64,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectTokenRequest {
    pub scopes: Option<Vec<String>>,
    pub ttl_seconds: Option<u32>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectTokenResponse {
    pub project_token: String,
    pub expires_at: String,
    pub scopes: Vec<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MigrationPlanRequest {
    pub project_id: String,
    pub intent: Option<String>,
    pub proposed_sql: String,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MigrationPlanResponse {
    pub plan_id: String,
    pub project_id: String,
    pub intent: String,
    pub created_at: String,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MigrationValidateRequest {
    pub plan_id: String,
    pub sql: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MigrationValidateResponse {
    pub job_id: String,
    pub plan_id: String,
    pub stage: String,
    pub status: String,
    pub statement_count: i64,
    pub warnings: Vec<String>,
    pub created_at: String,
    pub fallback_to_dry_run: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MigrationApprovalRequest {
    pub plan_id: String,
    pub risk_summary: Option<String>,
    pub expires_in_minutes: Option<u32>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MigrationApprovalResponse {
    pub approval_id: String,
    pub plan_id: String,
    pub status: String,
    pub approval_token: Option<String>,
    pub created_at: String,
    pub expires_at: String,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MigrationAutoReviewResponse {
    pub status: String,
    pub approval_token: Option<String>,
    pub risk_score: Option<f64>,
    pub reasons: Option<Vec<String>>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MigrationExecuteRequest {
    pub plan_id: String,
    pub strategy: Option<String>,
    pub approval_token: String,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MigrationExecuteResponse {
    pub job_id: String,
    pub plan_id: String,
    pub stage: String,
    pub status: String,
    pub created_at: String,
    pub fallback_to_dry_run: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MigrationHistoryResponse {
    pub plans: Vec<serde_json::Value>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

impl DbApi {
    pub async fn schema(
        &self,
        project_id: impl AsRef<str>,
        options: &DbSchemaOptions,
    ) -> Result<DbSchemaResponse, Error> {
        self.client
            .get(format!("/api/projects/{}/db/schema", project_id.as_ref()))
            .query(options)
            .ok()
            .await
    }

    pub async fn data(
        &self,
        project_id: impl AsRef<str>,
        options: &DbDataOptions,
    ) -> Result<serde_json::Value, Error> {
        self.client
            .get(format!("/api/projects/{}/db/data", project_id.as_ref()))
            .query(options)
            .ok()
            .await
    }

    pub async fn branches(&self, project_id: impl AsRef<str>) -> Result<Vec<DbBranch>, Error> {
        self.client
            .get(format!("/api/projects/{}/db/branches", project_id.as_ref()))
            .ok()
            .await
    }

    pub async fn list_rows(
        &self,
        project_id: impl AsRef<str>,
        schema: impl AsRef<str>,
        table: impl AsRef<str>,
        options: &DbRowsOptions,
    ) -> Result<Vec<serde_json::Value>, Error> {
        self.client
            .get(format!(
                "/api/projects/{}/db/tables/{}/{}/rows",
                project_id.as_ref(),
                urlencoding::encode(schema.as_ref()),
                urlencoding::encode(table.as_ref())
            ))
            .query(options)
            .ok()
            .await
    }

    pub async fn create_table(
        &self,
        project_id: impl AsRef<str>,
        payload: &DbCreateTableRequest,
    ) -> Result<DbMessageResponse, Error> {
        self.client
            .post(format!("/api/projects/{}/db/tables", project_id.as_ref()))
            .json(payload)
            .ok()
            .await
    }

    pub async fn drop_table(
        &self,
        project_id: impl AsRef<str>,
        schema: impl AsRef<str>,
        table: impl AsRef<str>,
        branch: Option<&str>,
        cascade: Option<bool>,
    ) -> Result<DbMessageResponse, Error> {
        self.client
            .delete(format!(
                "/api/projects/{}/db/tables/{}/{}",
                project_id.as_ref(),
                urlencoding::encode(schema.as_ref()),
                urlencoding::encode(table.as_ref())
            ))
            .query_if_some("branch", branch)
            .query_if_some("cascade", cascade)
            .ok()
            .await
    }

    pub async fn rename_table(
        &self,
        project_id: impl AsRef<str>,
        schema: impl AsRef<str>,
        table: impl AsRef<str>,
        payload: &DbRenameTableRequest,
    ) -> Result<DbMessageResponse, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/db/tables/{}/{}/rename",
                project_id.as_ref(),
                urlencoding::encode(schema.as_ref()),
                urlencoding::encode(table.as_ref())
            ))
            .json(payload)
            .ok()
            .await
    }

    pub async fn insert_row(
        &self,
        project_id: impl AsRef<str>,
        schema: impl AsRef<str>,
        table: impl AsRef<str>,
        payload: &DbValuesRequest,
    ) -> Result<DbAffectedResponse, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/db/tables/{}/{}/rows",
                project_id.as_ref(),
                urlencoding::encode(schema.as_ref()),
                urlencoding::encode(table.as_ref())
            ))
            .json(payload)
            .ok()
            .await
    }

    pub async fn update_rows(
        &self,
        project_id: impl AsRef<str>,
        schema: impl AsRef<str>,
        table: impl AsRef<str>,
        payload: &DbUpdateRowsRequest,
    ) -> Result<DbAffectedResponse, Error> {
        #[derive(Serialize)]
        struct Payload<'a> {
            #[serde(rename = "where")]
            where_: &'a BTreeMap<String, serde_json::Value>,
            values: &'a BTreeMap<String, serde_json::Value>,
            branch: Option<&'a str>,
        }

        self.client
            .patch(format!(
                "/api/projects/{}/db/tables/{}/{}/rows",
                project_id.as_ref(),
                urlencoding::encode(schema.as_ref()),
                urlencoding::encode(table.as_ref())
            ))
            .json(&Payload {
                where_: &payload.where_,
                values: &payload.values,
                branch: payload.branch.as_deref(),
            })
            .ok()
            .await
    }

    pub async fn delete_rows(
        &self,
        project_id: impl AsRef<str>,
        schema: impl AsRef<str>,
        table: impl AsRef<str>,
        payload: &DbDeleteRowsRequest,
    ) -> Result<DbAffectedResponse, Error> {
        #[derive(Serialize)]
        struct Payload<'a> {
            #[serde(rename = "where")]
            where_: &'a BTreeMap<String, serde_json::Value>,
            branch: Option<&'a str>,
        }

        self.client
            .post(format!(
                "/api/projects/{}/db/tables/{}/{}/rows/delete",
                project_id.as_ref(),
                urlencoding::encode(schema.as_ref()),
                urlencoding::encode(table.as_ref())
            ))
            .json(&Payload {
                where_: &payload.where_,
                branch: payload.branch.as_deref(),
            })
            .ok()
            .await
    }

    pub async fn migration_plan(
        &self,
        payload: &MigrationPlanRequest,
    ) -> Result<MigrationPlanResponse, Error> {
        self.client
            .post("/api/migrations/plan")
            .json(&serde_json::json!({
                "project_id": payload.project_id,
                "intent": payload.intent.clone().unwrap_or_else(|| "schema_change".to_string()),
                "proposed_sql": payload.proposed_sql,
            }))
            .ok()
            .await
    }

    pub async fn migration_validate(
        &self,
        payload: &MigrationValidateRequest,
    ) -> Result<MigrationValidateResponse, Error> {
        self.client
            .post("/api/migrations/validate")
            .json(payload)
            .ok()
            .await
    }

    pub async fn migration_create_approval(
        &self,
        payload: &MigrationApprovalRequest,
    ) -> Result<MigrationApprovalResponse, Error> {
        self.client
            .post("/api/migrations/approvals")
            .json(payload)
            .ok()
            .await
    }

    pub async fn migration_auto_review(
        &self,
        approval_id: impl AsRef<str>,
    ) -> Result<MigrationAutoReviewResponse, Error> {
        self.client
            .post(format!(
                "/api/migrations/approvals/{}/auto-review",
                approval_id.as_ref()
            ))
            .json(&serde_json::json!({}))
            .ok()
            .await
    }

    pub async fn migration_approve(
        &self,
        approval_id: impl AsRef<str>,
    ) -> Result<DbMessageResponse, Error> {
        self.client
            .post(format!(
                "/api/migrations/approvals/{}/approve",
                approval_id.as_ref()
            ))
            .json(&serde_json::json!({}))
            .ok()
            .await
    }

    pub async fn migration_execute(
        &self,
        payload: &MigrationExecuteRequest,
    ) -> Result<MigrationExecuteResponse, Error> {
        self.client
            .post("/api/migrations/execute")
            .json(payload)
            .ok()
            .await
    }

    pub async fn migration_history(
        &self,
        project_id: impl AsRef<str>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<MigrationHistoryResponse, Error> {
        self.client
            .get(format!("/api/migrations/history/{}", project_id.as_ref()))
            .query_if_some("limit", limit)
            .query_if_some("offset", offset)
            .ok()
            .await
    }

    pub async fn migration_detail(
        &self,
        plan_id: impl AsRef<str>,
    ) -> Result<serde_json::Value, Error> {
        self.client
            .get(format!("/api/migrations/plans/{}/detail", plan_id.as_ref()))
            .ok()
            .await
    }

    pub async fn issue_project_token(
        &self,
        project_id: impl AsRef<str>,
        payload: &ProjectTokenRequest,
    ) -> Result<ProjectTokenResponse, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/project-token/issue",
                project_id.as_ref()
            ))
            .json(payload)
            .ok()
            .await
    }

    pub async fn refresh_project_token(
        &self,
        project_id: impl AsRef<str>,
        payload: &ProjectTokenRequest,
    ) -> Result<ProjectTokenResponse, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/project-token/refresh",
                project_id.as_ref()
            ))
            .json(payload)
            .ok()
            .await
    }
}
