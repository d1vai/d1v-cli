use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::{Client, Error};

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Msg {
    pub message: String,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanRequest {
    pub project_id: String,
    pub intent: Option<String>,
    pub proposed_sql: String,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanResponse {
    pub plan_id: String,
    pub project_id: String,
    pub intent: String,
    pub created_at: String,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidateRequest {
    pub plan_id: String,
    pub sql: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidateResponse {
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
pub struct ApprovalRequest {
    pub plan_id: String,
    pub risk_summary: Option<String>,
    pub expires_in_minutes: Option<u32>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApprovalResponse {
    pub approval_id: String,
    pub plan_id: String,
    pub status: String,
    pub approval_token: Option<String>,
    pub created_at: String,
    pub expires_at: String,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AutoReviewResponse {
    pub status: String,
    pub approval_token: Option<String>,
    pub risk_score: Option<f64>,
    pub reasons: Option<Vec<String>>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecuteRequest {
    pub plan_id: String,
    pub strategy: Option<String>,
    pub approval_token: String,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecuteResponse {
    pub job_id: String,
    pub plan_id: String,
    pub stage: String,
    pub status: String,
    pub created_at: String,
    pub fallback_to_dry_run: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HistoryResponse {
    pub plans: Vec<serde_json::Value>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

pub struct MigrationApi {
    client: Client,
}

impl Client {
    #[must_use]
    pub fn migrations(&self) -> MigrationApi {
        MigrationApi {
            client: self.clone(),
        }
    }
}

impl MigrationApi {
    pub async fn plan(&self, payload: &PlanRequest) -> Result<PlanResponse, Error> {
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

    pub async fn validate(&self, payload: &ValidateRequest) -> Result<ValidateResponse, Error> {
        self.client
            .post("/api/migrations/validate")
            .json(payload)
            .ok()
            .await
    }

    pub async fn create_approval(
        &self,
        payload: &ApprovalRequest,
    ) -> Result<ApprovalResponse, Error> {
        self.client
            .post("/api/migrations/approvals")
            .json(payload)
            .ok()
            .await
    }

    pub async fn auto_review(
        &self,
        approval_id: impl AsRef<str>,
    ) -> Result<AutoReviewResponse, Error> {
        self.client
            .post(format!(
                "/api/migrations/approvals/{}/auto-review",
                approval_id.as_ref()
            ))
            .json(&serde_json::json!({}))
            .ok()
            .await
    }

    pub async fn approve(&self, approval_id: impl AsRef<str>) -> Result<Msg, Error> {
        self.client
            .post(format!(
                "/api/migrations/approvals/{}/approve",
                approval_id.as_ref()
            ))
            .json(&serde_json::json!({}))
            .ok()
            .await
    }

    pub async fn execute(&self, payload: &ExecuteRequest) -> Result<ExecuteResponse, Error> {
        self.client
            .post("/api/migrations/execute")
            .json(payload)
            .ok()
            .await
    }

    pub async fn history(
        &self,
        project_id: impl AsRef<str>,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<HistoryResponse, Error> {
        self.client
            .get(format!("/api/migrations/history/{}", project_id.as_ref()))
            .query_if_some("limit", limit)
            .query_if_some("offset", offset)
            .ok()
            .await
    }

    pub async fn detail(&self, plan_id: impl AsRef<str>) -> Result<serde_json::Value, Error> {
        self.client
            .get(format!("/api/migrations/plans/{}/detail", plan_id.as_ref()))
            .ok()
            .await
    }
}
