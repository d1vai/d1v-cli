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
}
