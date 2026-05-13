use serde::Serialize;
use serde_with::skip_serializing_none;

use crate::{Client, Error};

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePayProduct {
    pub user_id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub active: Option<bool>,
    pub platform_fee_percentage: Option<f64>,
    pub price: Option<serde_json::Value>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize)]
pub struct PayProductPaymentLinkOptions {
    pub prefilled_email: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePayPaymentLink {
    pub product_id: String,
    pub user_id: String,
    pub success_url: String,
    pub cancel_url: String,
    pub custom_fields: Option<serde_json::Value>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePayPaymentIntent {
    pub price_id: String,
    pub customer_email: Option<String>,
}

pub type PayProducts = serde_json::Value;
pub type PayProduct = serde_json::Value;
pub type PayPaymentLink = serde_json::Value;
pub type PayPaymentIntent = serde_json::Value;
pub type PayTransactions = serde_json::Value;
pub type PayTransactionStats = serde_json::Value;
pub type PayDashboardMetrics = serde_json::Value;
pub type PayRevenue = serde_json::Value;
pub type PayWebhooks = serde_json::Value;
pub type PayWebhook = serde_json::Value;
pub type DeletePayWebhookResponse = serde_json::Value;
pub type RegeneratePayWebhookSecretResponse = serde_json::Value;

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize)]
pub struct PayTransactionsOptions {
    pub created_after: Option<i64>,
    pub status: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize)]
pub struct PayPaginatedTransactionsOptions {
    pub page: u32,
    #[serde(rename = "pageSize")]
    pub page_size: u32,
    pub created_after: Option<i64>,
    pub status: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize)]
pub struct PayAnalyticsOptions {
    pub days: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePayWebhook {
    pub name: String,
    pub url: String,
    pub events: Option<Vec<String>>,
    pub is_active: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePayWebhook {
    pub name: Option<String>,
    pub url: Option<String>,
    pub events: Option<Vec<String>>,
    pub is_active: Option<bool>,
}

pub struct ProjectPay {
    client: Client,
    project_id: String,
}

impl ProjectPay {
    pub fn new(client: Client, project_id: String) -> Self {
        Self { client, project_id }
    }

    pub async fn products(&self) -> Result<PayProducts, Error> {
        self.client
            .get(format!("/api/projects/{}/pay/products", self.project_id))
            .ok()
            .await
    }

    pub async fn create_product(&self, payload: &CreatePayProduct) -> Result<PayProduct, Error> {
        self.client
            .post(format!("/api/projects/{}/pay/products", self.project_id))
            .json(payload)
            .ok()
            .await
    }

    pub async fn product_payment_link(
        &self,
        product_id: impl AsRef<str>,
        options: &PayProductPaymentLinkOptions,
    ) -> Result<PayPaymentLink, Error> {
        self.client
            .get(format!(
                "/api/projects/{}/pay/products/{}/payment-link",
                self.project_id,
                product_id.as_ref()
            ))
            .query(options)
            .ok()
            .await
    }

    pub async fn create_payment_link(
        &self,
        payload: &CreatePayPaymentLink,
    ) -> Result<PayPaymentLink, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/pay/create-payment-link",
                self.project_id
            ))
            .json(payload)
            .ok()
            .await
    }

    pub async fn create_payment_intent(
        &self,
        payload: &CreatePayPaymentIntent,
    ) -> Result<PayPaymentIntent, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/pay/create-payment-intent",
                self.project_id
            ))
            .json(payload)
            .ok()
            .await
    }

    pub async fn transactions(
        &self,
        options: &PayTransactionsOptions,
    ) -> Result<PayTransactions, Error> {
        self.client
            .get(format!(
                "/api/projects/{}/pay/transactions",
                self.project_id
            ))
            .query(options)
            .ok()
            .await
    }

    pub async fn transactions_paginated(
        &self,
        options: &PayPaginatedTransactionsOptions,
    ) -> Result<PayTransactions, Error> {
        self.client
            .get(format!(
                "/api/projects/{}/pay/transactions/paginated",
                self.project_id
            ))
            .query(options)
            .ok()
            .await
    }

    pub async fn transaction_stats(
        &self,
        options: &PayTransactionsOptions,
    ) -> Result<PayTransactionStats, Error> {
        self.client
            .get(format!(
                "/api/projects/{}/pay/transactions/stats",
                self.project_id
            ))
            .query(options)
            .ok()
            .await
    }

    pub async fn dashboard_metrics(
        &self,
        options: &PayAnalyticsOptions,
    ) -> Result<PayDashboardMetrics, Error> {
        self.client
            .get(format!(
                "/api/projects/{}/pay/dashboard/metrics",
                self.project_id
            ))
            .query(options)
            .ok()
            .await
    }

    pub async fn revenue(&self, options: &PayAnalyticsOptions) -> Result<PayRevenue, Error> {
        self.client
            .get(format!("/api/projects/{}/pay/revenue", self.project_id))
            .query(options)
            .ok()
            .await
    }

    pub async fn dashboard_revenue(
        &self,
        options: &PayAnalyticsOptions,
    ) -> Result<PayRevenue, Error> {
        self.client
            .get(format!(
                "/api/projects/{}/pay/dashboard/revenue",
                self.project_id
            ))
            .query(options)
            .ok()
            .await
    }

    pub async fn webhooks(&self) -> Result<PayWebhooks, Error> {
        self.client
            .get(format!("/api/projects/{}/pay/webhooks", self.project_id))
            .ok()
            .await
    }

    pub async fn create_webhook(&self, payload: &CreatePayWebhook) -> Result<PayWebhook, Error> {
        self.client
            .post(format!("/api/projects/{}/pay/webhooks", self.project_id))
            .json(payload)
            .ok()
            .await
    }

    pub async fn update_webhook(
        &self,
        webhook_id: impl AsRef<str>,
        payload: &UpdatePayWebhook,
    ) -> Result<PayWebhook, Error> {
        self.client
            .patch(format!(
                "/api/projects/{}/pay/webhooks/{}",
                self.project_id,
                webhook_id.as_ref()
            ))
            .json(payload)
            .ok()
            .await
    }

    pub async fn delete_webhook(
        &self,
        webhook_id: impl AsRef<str>,
    ) -> Result<DeletePayWebhookResponse, Error> {
        self.client
            .delete(format!(
                "/api/projects/{}/pay/webhooks/{}",
                self.project_id,
                webhook_id.as_ref()
            ))
            .ok()
            .await
    }

    pub async fn regenerate_webhook_secret(
        &self,
        webhook_id: impl AsRef<str>,
    ) -> Result<RegeneratePayWebhookSecretResponse, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/pay/webhooks/{}/regenerate-secret",
                self.project_id,
                webhook_id.as_ref()
            ))
            .ok()
            .await
    }
}
