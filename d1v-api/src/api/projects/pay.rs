use bon::{Builder, bon};
use jiff::Timestamp;
use serde::Serialize;
use serde_with::skip_serializing_none;

use crate::{Client, Error};

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
pub type PayBankAccounts = serde_json::Value;
pub type PayBankAccount = serde_json::Value;
pub type DeletePayBankAccountResponse = serde_json::Value;
pub type PayWithdrawals = serde_json::Value;
pub type PayWithdrawal = serde_json::Value;
pub type PayTokens = serde_json::Value;
pub type PayToken = serde_json::Value;
pub type DeletePayTokenResponse = serde_json::Value;

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Builder)]
pub struct PayPaginatedTransactionsOptions {
    pub page: u32,
    #[serde(rename = "pageSize")]
    pub page_size: u32,
    #[serde(with = "jiff::fmt::serde::timestamp::second::optional")]
    pub created_after: Option<Timestamp>,
    #[builder(into)]
    pub status: Option<String>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Builder)]
#[serde(rename_all = "camelCase")]
pub struct CreatePayBankAccount {
    #[builder(into)]
    pub account_holder_name: String,
    #[builder(into)]
    pub bank_name: String,
    #[builder(into)]
    pub account_number: String,
    #[builder(into)]
    pub routing_number: String,
    #[builder(into)]
    pub account_type: String,
    #[builder(into)]
    pub currency: String,
    #[builder(into)]
    pub country: String,
    pub is_default: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Builder)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePayBankAccount {
    #[builder(into)]
    pub account_holder_name: Option<String>,
    #[builder(into)]
    pub bank_name: Option<String>,
    #[builder(into)]
    pub account_number: Option<String>,
    #[builder(into)]
    pub routing_number: Option<String>,
    #[builder(into)]
    pub account_type: Option<String>,
    #[builder(into)]
    pub currency: Option<String>,
    #[builder(into)]
    pub country: Option<String>,
    pub is_default: Option<bool>,
}

#[skip_serializing_none]
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WithdrawalPayload<'a> {
    amount: f64,
    currency: &'a str,
    bank_account_id: &'a str,
    note: Option<&'a str>,
}

pub struct ProjectPay {
    client: Client,
    project_id: String,
}

#[bon]
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

    #[builder]
    pub async fn create_product(
        &self,
        #[builder(start_fn)] user_id: &str,
        #[builder(start_fn)] name: &str,
        description: Option<&str>,
        category: Option<&str>,
        active: Option<bool>,
        platform_fee_percentage: Option<f64>,
        price: Option<&serde_json::Value>,
    ) -> Result<PayProduct, Error> {
        #[skip_serializing_none]
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Payload<'a> {
            user_id: &'a str,
            name: &'a str,
            description: Option<&'a str>,
            category: Option<&'a str>,
            active: Option<bool>,
            platform_fee_percentage: Option<f64>,
            price: Option<&'a serde_json::Value>,
        }

        self.client
            .post(format!("/api/projects/{}/pay/products", self.project_id))
            .json(&Payload {
                user_id,
                name,
                description,
                category,
                active,
                platform_fee_percentage,
                price,
            })
            .ok()
            .await
    }

    pub async fn product_payment_link(
        &self,
        product_id: impl AsRef<str>,
        prefilled_email: Option<&str>,
    ) -> Result<PayPaymentLink, Error> {
        #[skip_serializing_none]
        #[derive(Serialize)]
        struct Query<'a> {
            prefilled_email: Option<&'a str>,
        }

        self.client
            .get(format!(
                "/api/projects/{}/pay/products/{}/payment-link",
                self.project_id,
                product_id.as_ref()
            ))
            .query(&Query { prefilled_email })
            .ok()
            .await
    }

    pub async fn create_payment_link(
        &self,
        product_id: &str,
        user_id: &str,
        success_url: &str,
        cancel_url: &str,
        custom_fields: Option<&serde_json::Value>,
    ) -> Result<PayPaymentLink, Error> {
        #[skip_serializing_none]
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Payload<'a> {
            product_id: &'a str,
            user_id: &'a str,
            success_url: &'a str,
            cancel_url: &'a str,
            custom_fields: Option<&'a serde_json::Value>,
        }

        self.client
            .post(format!(
                "/api/projects/{}/pay/create-payment-link",
                self.project_id
            ))
            .json(&Payload {
                product_id,
                user_id,
                success_url,
                cancel_url,
                custom_fields,
            })
            .ok()
            .await
    }

    pub async fn create_payment_intent(
        &self,
        price_id: impl AsRef<str>,
        customer_email: Option<&str>,
    ) -> Result<PayPaymentIntent, Error> {
        #[skip_serializing_none]
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Payload<'a> {
            price_id: &'a str,
            customer_email: Option<&'a str>,
        }

        self.client
            .post(format!(
                "/api/projects/{}/pay/create-payment-intent",
                self.project_id
            ))
            .json(&Payload {
                price_id: price_id.as_ref(),
                customer_email,
            })
            .ok()
            .await
    }

    pub async fn transactions(
        &self,
        created_after: Option<Timestamp>,
        status: Option<&str>,
    ) -> Result<PayTransactions, Error> {
        #[skip_serializing_none]
        #[derive(Serialize)]
        struct Query<'a> {
            #[serde(with = "jiff::fmt::serde::timestamp::second::optional")]
            created_after: Option<Timestamp>,
            status: Option<&'a str>,
        }

        self.client
            .get(format!(
                "/api/projects/{}/pay/transactions",
                self.project_id
            ))
            .query(&Query {
                created_after,
                status,
            })
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
        created_after: Option<Timestamp>,
        status: Option<&str>,
    ) -> Result<PayTransactionStats, Error> {
        #[skip_serializing_none]
        #[derive(Serialize)]
        struct Query<'a> {
            #[serde(with = "jiff::fmt::serde::timestamp::second::optional")]
            created_after: Option<Timestamp>,
            status: Option<&'a str>,
        }

        self.client
            .get(format!(
                "/api/projects/{}/pay/transactions/stats",
                self.project_id
            ))
            .query(&Query {
                created_after,
                status,
            })
            .ok()
            .await
    }

    pub async fn dashboard_metrics(&self, days: Option<u32>) -> Result<PayDashboardMetrics, Error> {
        #[skip_serializing_none]
        #[derive(Serialize)]
        struct Query {
            days: Option<u32>,
        }

        self.client
            .get(format!(
                "/api/projects/{}/pay/dashboard/metrics",
                self.project_id
            ))
            .query(&Query { days })
            .ok()
            .await
    }

    pub async fn revenue(&self, days: Option<u32>) -> Result<PayRevenue, Error> {
        #[skip_serializing_none]
        #[derive(Serialize)]
        struct Query {
            days: Option<u32>,
        }

        self.client
            .get(format!("/api/projects/{}/pay/revenue", self.project_id))
            .query(&Query { days })
            .ok()
            .await
    }

    pub async fn dashboard_revenue(&self, days: Option<u32>) -> Result<PayRevenue, Error> {
        #[skip_serializing_none]
        #[derive(Serialize)]
        struct Query {
            days: Option<u32>,
        }

        self.client
            .get(format!(
                "/api/projects/{}/pay/dashboard/revenue",
                self.project_id
            ))
            .query(&Query { days })
            .ok()
            .await
    }

    pub async fn webhooks(&self) -> Result<PayWebhooks, Error> {
        self.client
            .get(format!("/api/projects/{}/pay/webhooks", self.project_id))
            .ok()
            .await
    }

    #[builder]
    pub async fn create_webhook(
        &self,
        #[builder(start_fn)] name: &str,
        #[builder(start_fn)] url: &str,
        events: Option<Vec<String>>,
        is_active: Option<bool>,
    ) -> Result<PayWebhook, Error> {
        #[skip_serializing_none]
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Payload<'a> {
            name: &'a str,
            url: &'a str,
            events: Option<Vec<String>>,
            is_active: Option<bool>,
        }

        self.client
            .post(format!("/api/projects/{}/pay/webhooks", self.project_id))
            .json(&Payload {
                name,
                url,
                events,
                is_active,
            })
            .ok()
            .await
    }

    #[builder]
    pub async fn update_webhook(
        &self,
        #[builder(start_fn)] webhook_id: &str,
        name: Option<&str>,
        url: Option<&str>,
        events: Option<Vec<String>>,
        is_active: Option<bool>,
    ) -> Result<PayWebhook, Error> {
        #[skip_serializing_none]
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Payload<'a> {
            name: Option<&'a str>,
            url: Option<&'a str>,
            events: Option<Vec<String>>,
            is_active: Option<bool>,
        }

        self.client
            .patch(format!(
                "/api/projects/{}/pay/webhooks/{}",
                self.project_id, webhook_id
            ))
            .json(&Payload {
                name,
                url,
                events,
                is_active,
            })
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

    pub async fn bank_accounts(&self) -> Result<PayBankAccounts, Error> {
        self.client
            .get(format!(
                "/api/projects/{}/pay/bank-accounts",
                self.project_id
            ))
            .ok()
            .await
    }

    pub async fn create_bank_account(
        &self,
        payload: &CreatePayBankAccount,
    ) -> Result<PayBankAccount, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/pay/bank-accounts",
                self.project_id
            ))
            .json(payload)
            .ok()
            .await
    }

    pub async fn bank_account(&self, bank_id: impl AsRef<str>) -> Result<PayBankAccount, Error> {
        self.client
            .get(format!(
                "/api/projects/{}/pay/bank-accounts/{}",
                self.project_id,
                bank_id.as_ref()
            ))
            .ok()
            .await
    }

    pub async fn update_bank_account(
        &self,
        bank_id: impl AsRef<str>,
        payload: &UpdatePayBankAccount,
    ) -> Result<PayBankAccount, Error> {
        self.client
            .put(format!(
                "/api/projects/{}/pay/bank-accounts/{}",
                self.project_id,
                bank_id.as_ref()
            ))
            .json(payload)
            .ok()
            .await
    }

    pub async fn set_default_bank_account(
        &self,
        bank_id: impl AsRef<str>,
    ) -> Result<PayBankAccount, Error> {
        self.client
            .put(format!(
                "/api/projects/{}/pay/bank-accounts/{}/set-default",
                self.project_id,
                bank_id.as_ref()
            ))
            .ok()
            .await
    }

    pub async fn delete_bank_account(
        &self,
        bank_id: impl AsRef<str>,
    ) -> Result<DeletePayBankAccountResponse, Error> {
        self.client
            .delete(format!(
                "/api/projects/{}/pay/bank-accounts/{}",
                self.project_id,
                bank_id.as_ref()
            ))
            .ok()
            .await
    }

    pub async fn withdrawal_requests(&self) -> Result<PayWithdrawals, Error> {
        self.client
            .get(format!(
                "/api/projects/{}/pay/withdrawal-requests",
                self.project_id
            ))
            .ok()
            .await
    }

    pub async fn create_withdrawal_request(
        &self,
        amount: f64,
        currency: &str,
        bank_account_id: &str,
        note: Option<&str>,
    ) -> Result<PayWithdrawal, Error> {
        self.client
            .post(format!(
                "/api/projects/{}/pay/withdrawal-requests",
                self.project_id
            ))
            .json(&WithdrawalPayload {
                amount,
                currency,
                bank_account_id,
                note,
            })
            .ok()
            .await
    }

    pub async fn withdrawals(&self) -> Result<PayWithdrawals, Error> {
        self.client
            .get(format!("/api/projects/{}/pay/withdrawals", self.project_id))
            .ok()
            .await
    }

    pub async fn create_withdrawal(
        &self,
        amount: f64,
        currency: &str,
        bank_account_id: &str,
        note: Option<&str>,
    ) -> Result<PayWithdrawal, Error> {
        self.client
            .post(format!("/api/projects/{}/pay/withdrawals", self.project_id))
            .json(&WithdrawalPayload {
                amount,
                currency,
                bank_account_id,
                note,
            })
            .ok()
            .await
    }

    pub async fn tokens(&self) -> Result<PayTokens, Error> {
        self.client
            .get(format!("/api/projects/{}/pay/tokens", self.project_id))
            .ok()
            .await
    }

    #[builder]
    pub async fn create_token(
        &self,
        #[builder(start_fn)] name: &str,
        permissions: Option<Vec<String>>,
        is_active: Option<bool>,
        expires_at: Option<Timestamp>,
    ) -> Result<PayToken, Error> {
        #[skip_serializing_none]
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Payload<'a> {
            name: &'a str,
            permissions: Option<Vec<String>>,
            is_active: Option<bool>,
            expires_at: Option<Timestamp>,
        }

        self.client
            .post(format!("/api/projects/{}/pay/tokens", self.project_id))
            .json(&Payload {
                name,
                permissions,
                is_active,
                expires_at,
            })
            .ok()
            .await
    }

    pub async fn delete_token(
        &self,
        token_id: impl AsRef<str>,
    ) -> Result<DeletePayTokenResponse, Error> {
        self.client
            .delete(format!(
                "/api/projects/{}/pay/tokens/{}",
                self.project_id,
                token_id.as_ref()
            ))
            .ok()
            .await
    }
}
