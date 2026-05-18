use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::encode::encode_segment;
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
                encode_segment(schema),
                encode_segment(table)
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
                encode_segment(schema),
                encode_segment(table)
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
                encode_segment(schema),
                encode_segment(table)
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
                encode_segment(schema),
                encode_segment(table)
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
                encode_segment(schema),
                encode_segment(table)
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
                encode_segment(schema),
                encode_segment(table)
            ))
            .json(&Payload {
                where_: &payload.where_,
                branch: payload.branch.as_deref(),
            })
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
