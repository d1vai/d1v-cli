use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::encode::encode_segment;
use crate::{Client, Error};

#[derive(Debug, Clone, Copy, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Granularity {
    #[default]
    Daily,
    Hourly,
    Monthly,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize)]
pub struct DbSchemaOptions {
    pub branch: Option<String>,
    pub include_views: Option<bool>,
    pub with_row_counts: Option<bool>,
    pub include_system_schemas: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize)]
pub struct DbDataOptions {
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
    pub granularity: Option<Granularity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSchema {
    pub tables: Vec<TableSchema>,
    pub default_schema: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSchema {
    pub schema: String,
    pub name: String,
    pub kind: String,
    pub columns: Vec<ColumnSchema>,
    pub primary_key: Vec<String>,
    pub foreign_keys: Vec<ForeignKeySchema>,
    pub row_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnSchema {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub r#default: Option<String>,
    pub ordinal_position: i64,
    pub character_maximum_length: Option<i64>,
    pub numeric_precision: Option<i64>,
    pub numeric_scale: Option<i64>,
    pub is_primary_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKeySchema {
    pub constraint_name: String,
    pub column_name: String,
    pub ref_schema: String,
    pub ref_table: String,
    pub ref_column: String,
}

pub type DbData = serde_json::Value;
pub type DbBranch = serde_json::Value;
pub type NeonUsage = serde_json::Value;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnIdentity {
    #[default]
    ByDefault,
    Always,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DbColumn {
    pub name: String,
    pub data_type: String,
    pub is_nullable: Option<bool>,
    pub default_expr: Option<String>,
    pub identity: Option<ColumnIdentity>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateDbTable {
    pub schema_name: Option<String>,
    pub table_name: String,
    pub columns: Vec<DbColumn>,
    pub primary_key: Option<Vec<String>>,
    pub branch: Option<String>,
    pub create_schema_if_missing: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize)]
pub struct DropDbTableOptions {
    pub branch: Option<String>,
    pub cascade: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize)]
pub struct ListDbRowsOptions {
    pub branch: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InsertDbRow {
    pub values: serde_json::Map<String, serde_json::Value>,
    pub branch: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateDbRows {
    #[serde(rename = "where")]
    pub where_: serde_json::Map<String, serde_json::Value>,
    pub values: serde_json::Map<String, serde_json::Value>,
    pub branch: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeleteDbRows {
    #[serde(rename = "where")]
    pub where_: serde_json::Map<String, serde_json::Value>,
    pub branch: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecuteSql {
    pub sql: String,
    pub branch: Option<String>,
    pub dry_run: Option<bool>,
    pub read_only: Option<bool>,
    pub approval_token: Option<String>,
    pub max_rows: Option<u32>,
}

pub type DbRow = serde_json::Map<String, serde_json::Value>;
pub type ExecuteSqlResponse = serde_json::Value;

pub struct ProjectsDb {
    client: Client,
    project_id: String,
}

impl ProjectsDb {
    pub fn new(client: Client, project_id: String) -> Self {
        Self { client, project_id }
    }

    /// Returns the database schema (tables, columns, keys).
    pub async fn schema(&self, options: &DbSchemaOptions) -> Result<DatabaseSchema, Error> {
        self.client
            .get(format!("/api/projects/{}/db/schema", self.project_id))
            .query(options)
            .ok()
            .await
    }

    pub async fn data(&self, options: &DbDataOptions) -> Result<DbData, Error> {
        self.client
            .get(format!("/api/projects/{}/db/data", self.project_id))
            .query(options)
            .ok()
            .await
    }

    pub async fn branches(&self) -> Result<Vec<DbBranch>, Error> {
        self.client
            .get(format!("/api/projects/{}/db/branches", self.project_id))
            .ok()
            .await
    }

    /// Creates a table. Returns the server message.
    pub async fn create_table(&self, payload: &CreateDbTable) -> Result<String, Error> {
        #[derive(Deserialize)]
        struct MsgResult {
            message: String,
        }

        self.client
            .post(format!("/api/projects/{}/db/tables", self.project_id))
            .json(payload)
            .ok::<MsgResult>()
            .await
            .map(|r| r.message)
    }

    /// Drops a table. Returns the server message.
    pub async fn drop_table(
        &self,
        schema_name: impl AsRef<str>,
        table_name: impl AsRef<str>,
        options: &DropDbTableOptions,
    ) -> Result<String, Error> {
        #[derive(Deserialize)]
        struct MsgResult {
            message: String,
        }

        self.client
            .delete(format!(
                "/api/projects/{}/db/tables/{}/{}",
                self.project_id,
                encode_segment(schema_name),
                encode_segment(table_name)
            ))
            .query(options)
            .ok::<MsgResult>()
            .await
            .map(|r| r.message)
    }

    pub async fn list_table_rows(
        &self,
        schema_name: impl AsRef<str>,
        table_name: impl AsRef<str>,
        options: &ListDbRowsOptions,
    ) -> Result<Vec<DbRow>, Error> {
        self.client
            .get(format!(
                "/api/projects/{}/db/tables/{}/{}/rows",
                self.project_id,
                encode_segment(schema_name),
                encode_segment(table_name)
            ))
            .query(options)
            .ok()
            .await
    }

    /// Inserts a row. Returns the number of affected rows.
    pub async fn insert_table_row(
        &self,
        schema_name: impl AsRef<str>,
        table_name: impl AsRef<str>,
        payload: &InsertDbRow,
    ) -> Result<i64, Error> {
        #[derive(Deserialize)]
        struct RowResult {
            affected: i64,
        }

        self.client
            .post(format!(
                "/api/projects/{}/db/tables/{}/{}/rows",
                self.project_id,
                encode_segment(schema_name),
                encode_segment(table_name)
            ))
            .json(payload)
            .ok::<RowResult>()
            .await
            .map(|r| r.affected)
    }

    /// Updates rows. Returns the number of affected rows.
    pub async fn update_table_rows(
        &self,
        schema_name: impl AsRef<str>,
        table_name: impl AsRef<str>,
        payload: &UpdateDbRows,
    ) -> Result<i64, Error> {
        #[derive(Deserialize)]
        struct RowResult {
            affected: i64,
        }

        self.client
            .patch(format!(
                "/api/projects/{}/db/tables/{}/{}/rows",
                self.project_id,
                encode_segment(schema_name),
                encode_segment(table_name)
            ))
            .json(payload)
            .ok::<RowResult>()
            .await
            .map(|r| r.affected)
    }

    /// Deletes rows. Returns the number of affected rows.
    pub async fn delete_table_rows(
        &self,
        schema_name: impl AsRef<str>,
        table_name: impl AsRef<str>,
        payload: &DeleteDbRows,
    ) -> Result<i64, Error> {
        #[derive(Deserialize)]
        struct RowResult {
            affected: i64,
        }

        self.client
            .post(format!(
                "/api/projects/{}/db/tables/{}/{}/rows/delete",
                self.project_id,
                encode_segment(schema_name),
                encode_segment(table_name)
            ))
            .json(payload)
            .ok::<RowResult>()
            .await
            .map(|r| r.affected)
    }

    pub async fn execute_sql(&self, payload: &ExecuteSql) -> Result<ExecuteSqlResponse, Error> {
        self.client
            .post(format!("/api/projects/{}/db/sql", self.project_id))
            .json(payload)
            .ok()
            .await
    }
}
