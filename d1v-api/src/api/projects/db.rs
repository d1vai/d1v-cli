use bon::{Builder, bon};
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
#[derive(Debug, Clone, Default, Serialize, Deserialize, Builder)]
pub struct DbColumn {
    #[builder(into)]
    pub name: String,
    #[builder(into)]
    pub data_type: String,
    pub is_nullable: Option<bool>,
    #[builder(into)]
    pub default_expr: Option<String>,
    pub identity: Option<ColumnIdentity>,
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

pub type DbRow = serde_json::Map<String, serde_json::Value>;
pub type ExecuteSqlResponse = serde_json::Value;

pub struct ProjectsDb {
    client: Client,
    project_id: String,
}

#[bon]
impl ProjectsDb {
    pub fn new(client: Client, project_id: String) -> Self {
        Self { client, project_id }
    }

    /// Returns the database schema (tables, columns, keys).
    #[builder]
    pub async fn schema(
        &self,
        branch: Option<&str>,
        include_views: Option<bool>,
        with_row_counts: Option<bool>,
        include_system_schemas: Option<bool>,
    ) -> Result<DatabaseSchema, Error> {
        #[skip_serializing_none]
        #[derive(Serialize)]
        struct Query<'a> {
            branch: Option<&'a str>,
            include_views: Option<bool>,
            with_row_counts: Option<bool>,
            include_system_schemas: Option<bool>,
        }

        self.client
            .get(format!("/api/projects/{}/db/schema", self.project_id))
            .query(&Query {
                branch,
                include_views,
                with_row_counts,
                include_system_schemas,
            })
            .ok()
            .await
    }

    #[builder]
    pub async fn data(
        &self,
        branch: Option<&str>,
        limit_per_table: Option<u32>,
        include_views: Option<bool>,
        include_system_schemas: Option<bool>,
    ) -> Result<DbData, Error> {
        #[skip_serializing_none]
        #[derive(Serialize)]
        struct Query<'a> {
            branch: Option<&'a str>,
            limit_per_table: Option<u32>,
            include_views: Option<bool>,
            include_system_schemas: Option<bool>,
        }

        self.client
            .get(format!("/api/projects/{}/db/data", self.project_id))
            .query(&Query {
                branch,
                limit_per_table,
                include_views,
                include_system_schemas,
            })
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
    #[builder]
    pub async fn create_table(
        &self,
        #[builder(start_fn)] table_name: impl AsRef<str>,
        columns: Vec<DbColumn>,
        schema_name: Option<&str>,
        primary_key: Option<Vec<String>>,
        branch: Option<&str>,
        create_schema_if_missing: Option<bool>,
    ) -> Result<String, Error> {
        #[derive(Deserialize)]
        struct MsgResult {
            message: String,
        }

        #[skip_serializing_none]
        #[derive(Serialize)]
        struct Payload<'a> {
            table_name: &'a str,
            columns: &'a [DbColumn],
            schema_name: Option<&'a str>,
            primary_key: Option<Vec<String>>,
            branch: Option<&'a str>,
            create_schema_if_missing: Option<bool>,
        }

        self.client
            .post(format!("/api/projects/{}/db/tables", self.project_id))
            .json(&Payload {
                table_name: table_name.as_ref(),
                columns: &columns,
                schema_name,
                primary_key,
                branch,
                create_schema_if_missing,
            })
            .ok::<MsgResult>()
            .await
            .map(|r| r.message)
    }

    /// Drops a table. Returns the server message.
    #[builder]
    pub async fn drop_table(
        &self,
        #[builder(start_fn)] schema_name: impl AsRef<str>,
        #[builder(start_fn)] table_name: impl AsRef<str>,
        branch: Option<&str>,
        cascade: Option<bool>,
    ) -> Result<String, Error> {
        #[skip_serializing_none]
        #[derive(Serialize)]
        struct Query<'a> {
            branch: Option<&'a str>,
            cascade: Option<bool>,
        }

        #[derive(Deserialize)]
        struct MsgResult {
            message: String,
        }

        self.client
            .delete(format!(
                "/api/projects/{}/db/tables/{}/{}",
                self.project_id,
                encode_segment(schema_name.as_ref()),
                encode_segment(table_name.as_ref())
            ))
            .query(&Query { branch, cascade })
            .ok::<MsgResult>()
            .await
            .map(|r| r.message)
    }

    #[builder]
    pub async fn list_table_rows(
        &self,
        #[builder(start_fn)] schema_name: impl AsRef<str>,
        #[builder(start_fn)] table_name: impl AsRef<str>,
        branch: Option<&str>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<DbRow>, Error> {
        #[skip_serializing_none]
        #[derive(Serialize)]
        struct Query<'a> {
            branch: Option<&'a str>,
            limit: Option<u32>,
            offset: Option<u32>,
        }

        self.client
            .get(format!(
                "/api/projects/{}/db/tables/{}/{}/rows",
                self.project_id,
                encode_segment(schema_name.as_ref()),
                encode_segment(table_name.as_ref())
            ))
            .query(&Query {
                branch,
                limit,
                offset,
            })
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

    #[builder]
    pub async fn execute_sql(
        &self,
        #[builder(start_fn)] sql: impl AsRef<str>,
        branch: Option<&str>,
        dry_run: Option<bool>,
        read_only: Option<bool>,
        approval_token: Option<&str>,
        max_rows: Option<u32>,
    ) -> Result<ExecuteSqlResponse, Error> {
        #[skip_serializing_none]
        #[derive(Serialize)]
        struct Payload<'a> {
            sql: &'a str,
            branch: Option<&'a str>,
            dry_run: Option<bool>,
            read_only: Option<bool>,
            approval_token: Option<&'a str>,
            max_rows: Option<u32>,
        }

        self.client
            .post(format!("/api/projects/{}/db/sql", self.project_id))
            .json(&Payload {
                sql: sql.as_ref(),
                branch,
                dry_run,
                read_only,
                approval_token,
                max_rows,
            })
            .ok()
            .await
    }
}
