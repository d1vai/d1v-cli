use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{Client, Error};

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize)]
pub struct ProjectDbSchemaOptions {
    pub branch: Option<String>,
    pub include_views: Option<bool>,
    pub with_row_counts: Option<bool>,
    pub include_system_schemas: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize)]
pub struct ProjectDbDataOptions {
    pub branch: Option<String>,
    pub limit_per_table: Option<u32>,
    pub include_views: Option<bool>,
    pub include_system_schemas: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize)]
pub struct NeonUsageOptions {
    pub from_iso: Option<Timestamp>,
    pub to_iso: Option<Timestamp>,
    pub granularity: Option<String>,
}

pub type ProjectDbSchema = serde_json::Value;
pub type ProjectDbData = serde_json::Value;
pub type ProjectDbBranch = serde_json::Value;
pub type NeonUsage = serde_json::Value;

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectDbColumn {
    pub name: String,
    pub data_type: String,
    pub is_nullable: Option<bool>,
    pub default_expr: Option<String>,
    pub identity: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateProjectDbTable {
    pub schema_name: Option<String>,
    pub table_name: String,
    pub columns: Vec<ProjectDbColumn>,
    pub primary_key: Option<Vec<String>>,
    pub branch: Option<String>,
    pub create_schema_if_missing: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize)]
pub struct DropProjectDbTableOptions {
    pub branch: Option<String>,
    pub cascade: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize)]
pub struct ListProjectDbRowsOptions {
    pub branch: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InsertProjectDbRow {
    pub values: serde_json::Map<String, serde_json::Value>,
    pub branch: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateProjectDbRows {
    #[serde(rename = "where")]
    pub where_: serde_json::Map<String, serde_json::Value>,
    pub values: serde_json::Map<String, serde_json::Value>,
    pub branch: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeleteProjectDbRows {
    #[serde(rename = "where")]
    pub where_: serde_json::Map<String, serde_json::Value>,
    pub branch: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecuteProjectSql {
    pub sql: String,
    pub branch: Option<String>,
    pub dry_run: Option<bool>,
    pub read_only: Option<bool>,
    pub approval_token: Option<String>,
    pub max_rows: Option<u32>,
}

pub type ProjectDbMutation = serde_json::Value;
pub type ProjectDbRow = serde_json::Map<String, serde_json::Value>;
pub type ExecuteProjectSqlResponse = serde_json::Value;

pub struct ProjectsDb {
    client: Client,
    project_id: String,
}

impl ProjectsDb {
    pub fn new(client: Client, project_id: String) -> Self {
        Self { client, project_id }
    }

    pub async fn schema(&self, options: &ProjectDbSchemaOptions) -> Result<ProjectDbSchema, Error> {
        self.client
            .get(format!("/api/projects/{}/db/schema", self.project_id))
            .query(options)
            .ok()
            .await
    }

    pub async fn data(&self, options: &ProjectDbDataOptions) -> Result<ProjectDbData, Error> {
        self.client
            .get(format!("/api/projects/{}/db/data", self.project_id))
            .query(options)
            .ok()
            .await
    }

    pub async fn branches(&self) -> Result<Vec<ProjectDbBranch>, Error> {
        self.client
            .get(format!("/api/projects/{}/db/branches", self.project_id))
            .ok()
            .await
    }

    pub async fn create_table(
        &self,
        payload: &CreateProjectDbTable,
    ) -> Result<ProjectDbMutation, Error> {
        self.client
            .post(format!("/api/projects/{}/db/tables", self.project_id))
            .json(payload)
            .ok()
            .await
    }

    pub async fn drop_table(
        &self,
        schema_name: impl AsRef<str>,
        table_name: impl AsRef<str>,
        options: &DropProjectDbTableOptions,
    ) -> Result<ProjectDbMutation, Error> {
        self.client
            .delete(format!(
                "/api/projects/{}/db/tables/{}/{}",
                self.project_id,
                schema_name.as_ref(),
                table_name.as_ref()
            ))
            .query(options)
            .ok()
            .await
    }

    pub async fn list_table_rows(
        &self,
        schema_name: impl AsRef<str>,
        table_name: impl AsRef<str>,
        options: &ListProjectDbRowsOptions,
    ) -> Result<Vec<ProjectDbRow>, Error> {
        self.client
            .get(format!(
                "/api/projects/{}/db/tables/{}/{}/rows",
                self.project_id,
                schema_name.as_ref(),
                table_name.as_ref()
            ))
            .query(options)
            .ok()
            .await
    }

    pub async fn insert_table_row(
        &self,
        schema_name: impl AsRef<str>,
        table_name: impl AsRef<str>,
        payload: &InsertProjectDbRow,
    ) -> Result<ProjectDbMutation, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/db/tables/{}/{}/rows",
                self.project_id,
                schema_name.as_ref(),
                table_name.as_ref()
            ))
            .json(payload)
            .ok()
            .await
    }

    pub async fn update_table_rows(
        &self,
        schema_name: impl AsRef<str>,
        table_name: impl AsRef<str>,
        payload: &UpdateProjectDbRows,
    ) -> Result<ProjectDbMutation, Error> {
        self.client
            .patch(format!(
                "/api/projects/{}/db/tables/{}/{}/rows",
                self.project_id,
                schema_name.as_ref(),
                table_name.as_ref()
            ))
            .json(payload)
            .ok()
            .await
    }

    pub async fn delete_table_rows(
        &self,
        schema_name: impl AsRef<str>,
        table_name: impl AsRef<str>,
        payload: &DeleteProjectDbRows,
    ) -> Result<ProjectDbMutation, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/db/tables/{}/{}/rows/delete",
                self.project_id,
                schema_name.as_ref(),
                table_name.as_ref()
            ))
            .json(payload)
            .ok()
            .await
    }

    pub async fn execute_sql(
        &self,
        payload: &ExecuteProjectSql,
    ) -> Result<ExecuteProjectSqlResponse, Error> {
        self.client
            .post(format!("/api/projects/{}/db/sql", self.project_id))
            .json(payload)
            .ok()
            .await
    }
}
