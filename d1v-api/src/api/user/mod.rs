mod types;

use bon::bon;
use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;
use serde_with::skip_serializing_none;
use url::Url;

use crate::locale::{IntoLocale, Locale};
use crate::validate::{Code, Email, Validate};
use crate::{Client, Error, UrlError};
pub use types::{CreatedApiKey, DailyCount, PromptDailyActivity, User, UserApiKey};

pub struct UserApi {
    client: Client,
}

impl Client {
    #[must_use]
    pub fn user(&self) -> UserApi {
        UserApi {
            client: self.clone(),
        }
    }
}

impl UserApi {
    /// Sends a verification code to the given email.
    pub async fn send_code(
        &self,
        email: impl AsRef<str>,
        locale: impl IntoLocale,
    ) -> Result<(), Error> {
        #[derive(Serialize)]
        struct Query<'a> {
            email: Email<'a>,
            #[serde(skip_serializing_if = "Option::is_none")]
            locale: Option<Locale>,
        }

        let email = Email(email.as_ref());
        email.validate()?;

        self.client
            .post("/api/user/verify-code")
            .query(&Query {
                email,
                locale: locale.into_locale(),
            })
            .no_auth()
            .ok()
            .await
    }

    /// Checks a verification code without logging in.
    pub async fn check_code(
        &self,
        email: impl AsRef<str>,
        code: impl AsRef<str>,
        purpose: impl AsRef<str>,
    ) -> Result<(), Error> {
        #[derive(Serialize)]
        struct Payload<'a> {
            email: Email<'a>,
            code: Code<'a>,
            purpose: &'a str,
        }

        let email = Email(email.as_ref());
        let code = Code(code.as_ref());
        email.validate()?;
        code.validate()?;

        self.client
            .post("/api/user/verify-code/check")
            .json(&Payload {
                email,
                code,
                purpose: purpose.as_ref(),
            })
            .no_auth()
            .ok()
            .await
    }

    /// Logs in with email and verification code, returns a token.
    pub async fn login(
        &self,
        email: impl AsRef<str>,
        code: impl AsRef<str>,
    ) -> Result<SecretString, Error> {
        #[derive(Serialize)]
        struct Payload<'a> {
            email: Email<'a>,
            verify_code: Code<'a>,
        }

        let email = Email(email.as_ref());
        let verify_code = Code(code.as_ref());
        email.validate()?;
        verify_code.validate()?;

        self.client
            .post("/api/user/login")
            .json(&Payload { email, verify_code })
            .no_auth()
            .ok()
            .await
    }

    /// Logs in with email and password, returns a token.
    pub async fn login_password(
        &self,
        email: impl AsRef<str>,
        password: &SecretString,
    ) -> Result<SecretString, Error> {
        #[derive(Serialize)]
        struct Payload<'a> {
            email: Email<'a>,
            password: &'a str,
        }

        let email = Email(email.as_ref());
        email.validate()?;

        self.client
            .post("/api/user/login/password")
            .json(&Payload {
                email,
                password: password.expose_secret(),
            })
            .no_auth()
            .ok()
            .await
    }

    /// Logs in with email and password, returns a token.
    pub async fn password_login(
        &self,
        email: impl AsRef<str>,
        password: &SecretString,
    ) -> Result<SecretString, Error> {
        #[derive(Serialize)]
        struct Payload<'a> {
            email: Email<'a>,
            password: &'a str,
        }

        let email = Email(email.as_ref());
        email.validate()?;

        self.client
            .post("/api/user/password/login")
            .json(&Payload {
                email,
                password: password.expose_secret(),
            })
            .no_auth()
            .ok()
            .await
    }

    /// Returns the current user's info.
    pub async fn info(&self) -> Result<User, Error> {
        self.client.get("/api/user/info").ok().await
    }

    /// Returns a public user by ID.
    pub async fn public_user(&self, user_id: i64) -> Result<User, Error> {
        self.client
            .get(format!("/api/user/public/{user_id}"))
            .no_auth()
            .ok()
            .await
    }

    /// Returns a public user by slug.
    pub async fn public_user_by_slug(&self, slug: impl AsRef<str>) -> Result<User, Error> {
        self.client
            .get(format!("/api/user/public/slug/{}", slug.as_ref()))
            .no_auth()
            .ok()
            .await
    }

    /// Returns all users.
    pub async fn all_users(&self) -> Result<Vec<User>, Error> {
        self.client.get("/api/user/all").ok().await
    }

    /// Sets a password for the current user.
    pub async fn set_password(&self, password: &SecretString) -> Result<(), Error> {
        #[derive(Serialize)]
        struct Payload<'a> {
            password: &'a str,
        }

        self.client
            .post("/api/user/password/set")
            .json(&Payload {
                password: password.expose_secret(),
            })
            .ok()
            .await
    }

    /// Sends a forgot-password email.
    pub async fn send_forgot_password_email(
        &self,
        email: impl AsRef<str>,
        locale: impl IntoLocale,
    ) -> Result<(), Error> {
        #[derive(Serialize)]
        struct Payload<'a> {
            email: Email<'a>,
            #[serde(skip_serializing_if = "Option::is_none")]
            locale: Option<Locale>,
        }

        let email = Email(email.as_ref());
        email.validate()?;

        self.client
            .post("/api/user/password/forgot/send")
            .json(&Payload {
                email,
                locale: locale.into_locale(),
            })
            .no_auth()
            .ok()
            .await
    }

    /// Resets password with email, verification code, and new password.
    pub async fn reset_password(
        &self,
        email: impl AsRef<str>,
        code: impl AsRef<str>,
        new_password: &SecretString,
    ) -> Result<(), Error> {
        #[derive(Serialize)]
        struct Payload<'a> {
            email: Email<'a>,
            code: Code<'a>,
            new_password: &'a str,
        }

        let email = Email(email.as_ref());
        let code = Code(code.as_ref());
        email.validate()?;
        code.validate()?;

        self.client
            .post("/api/user/password/reset")
            .json(&Payload {
                email,
                code,
                new_password: new_password.expose_secret(),
            })
            .no_auth()
            .ok()
            .await
    }

    /// Sends a verification code to bind an email.
    pub async fn send_bind_email_code(
        &self,
        email: impl AsRef<str>,
        locale: impl IntoLocale,
    ) -> Result<(), Error> {
        #[derive(Serialize)]
        struct Payload<'a> {
            email: Email<'a>,
            #[serde(skip_serializing_if = "Option::is_none")]
            locale: Option<Locale>,
        }

        let email = Email(email.as_ref());
        email.validate()?;

        self.client
            .post("/api/user/bind-email/send")
            .json(&Payload {
                email,
                locale: locale.into_locale(),
            })
            .ok()
            .await
    }

    /// Confirms binding an email with a verification code.
    pub async fn confirm_bind_email(
        &self,
        email: impl AsRef<str>,
        code: impl AsRef<str>,
    ) -> Result<(), Error> {
        #[derive(Serialize)]
        struct Payload<'a> {
            email: Email<'a>,
            code: Code<'a>,
        }

        let email = Email(email.as_ref());
        let code = Code(code.as_ref());
        email.validate()?;
        code.validate()?;

        self.client
            .post("/api/user/bind-email/confirm")
            .json(&Payload { email, code })
            .ok()
            .await
    }

    /// Sends a verification code to change email.
    pub async fn send_change_email_code(
        &self,
        new_email: impl AsRef<str>,
        locale: impl IntoLocale,
    ) -> Result<(), Error> {
        #[derive(Serialize)]
        struct Payload<'a> {
            new_email: Email<'a>,
            #[serde(skip_serializing_if = "Option::is_none")]
            locale: Option<Locale>,
        }

        let new_email = Email(new_email.as_ref());
        new_email.validate()?;

        self.client
            .post("/api/user/email/change/send")
            .json(&Payload {
                new_email,
                locale: locale.into_locale(),
            })
            .ok()
            .await
    }

    /// Confirms changing email with a verification code.
    pub async fn confirm_change_email(
        &self,
        new_email: impl AsRef<str>,
        code: impl AsRef<str>,
    ) -> Result<(), Error> {
        #[derive(Serialize)]
        struct Payload<'a> {
            new_email: Email<'a>,
            code: Code<'a>,
        }

        let new_email = Email(new_email.as_ref());
        let code = Code(code.as_ref());
        new_email.validate()?;
        code.validate()?;

        self.client
            .post("/api/user/email/change/confirm")
            .json(&Payload { new_email, code })
            .ok()
            .await
    }

    /// Accepts an invitation with the given invite code.
    pub async fn accept_invitation(&self, invite_code: impl AsRef<str>) -> Result<(), Error> {
        #[derive(Serialize)]
        struct Payload<'a> {
            invite_code: &'a str,
        }

        self.client
            .post("/api/user/invitation/accept")
            .json(&Payload {
                invite_code: invite_code.as_ref(),
            })
            .ok()
            .await
    }

    /// Lists users invited by the current user.
    pub async fn list_invitees(&self) -> Result<Vec<User>, Error> {
        self.client.get("/api/user/invitations").ok().await
    }

    /// Sets the onboarded status.
    pub async fn set_onboarded(&self, value: bool) -> Result<(), Error> {
        #[derive(Serialize)]
        struct Payload {
            value: bool,
        }

        self.client
            .post("/api/user/onboarded/set")
            .json(&Payload { value })
            .ok()
            .await
    }

    /// Returns the current user's daily prompt activity.
    pub async fn prompt_daily_activity(
        &self,
        days: Option<i32>,
    ) -> Result<PromptDailyActivity, Error> {
        self.client
            .get("/api/user/activity/prompt-daily")
            .query_if_some("days", days)
            .ok()
            .await
    }

    /// Returns daily prompt activity for a user by slug.
    pub async fn prompt_daily_activity_by_slug(
        &self,
        slug: impl AsRef<str>,
        days: Option<i32>,
    ) -> Result<PromptDailyActivity, Error> {
        self.client
            .get(format!(
                "/api/user/activity/prompt-daily/slug/{}",
                slug.as_ref()
            ))
            .query_if_some("days", days)
            .no_auth()
            .ok()
            .await
    }

    /// Returns daily prompt activity for a user by ID.
    pub async fn prompt_daily_activity_by_user(
        &self,
        user_id: i64,
        days: Option<i32>,
    ) -> Result<PromptDailyActivity, Error> {
        self.client
            .get(format!("/api/user/activity/prompt-daily/user/{user_id}"))
            .query_if_some("days", days)
            .ok()
            .await
    }

    /// Sends a verification code to delete the account.
    pub async fn send_delete_account_code(&self, locale: impl IntoLocale) -> Result<(), Error> {
        #[derive(Serialize)]
        struct Payload {
            #[serde(skip_serializing_if = "Option::is_none")]
            locale: Option<Locale>,
        }

        self.client
            .post("/api/user/account/delete/send")
            .json(&Payload {
                locale: locale.into_locale(),
            })
            .ok()
            .await
    }

    /// Deletes the current user's account with a verification code.
    pub async fn delete_account(&self, code: impl AsRef<str>) -> Result<(), Error> {
        #[derive(Serialize)]
        struct Payload<'a> {
            code: Code<'a>,
        }

        let code = Code(code.as_ref());
        code.validate()?;

        self.client
            .delete("/api/user/account")
            .json(&Payload { code })
            .ok()
            .await
    }

    /// Lists API keys for the current user.
    pub async fn api_keys(&self) -> Result<Vec<UserApiKey>, Error> {
        self.client.get("/api/user/api-keys").ok().await
    }

    /// Creates a new API key.
    pub async fn create_api_key(
        &self,
        name: impl AsRef<str>,
        description: Option<&str>,
    ) -> Result<CreatedApiKey, Error> {
        #[skip_serializing_none]
        #[derive(Serialize)]
        struct Payload<'a> {
            name: &'a str,
            description: Option<&'a str>,
        }

        self.client
            .post("/api/user/api-keys")
            .json(&Payload {
                name: name.as_ref(),
                description,
            })
            .ok()
            .await
    }

    /// Revokes an API key.
    pub async fn revoke_api_key(&self, api_key_id: i64) -> Result<UserApiKey, Error> {
        self.client
            .delete(format!("/api/user/api-keys/{api_key_id}"))
            .ok()
            .await
    }
}

#[bon]
impl UserApi {
    /// Updates the current user's info.
    #[builder]
    pub async fn update_info(
        &self,
        company_name: Option<&str>,
        company_website: Option<&str>,
        picture: Option<&str>,
        industry: Option<&str>,
        referral_code: Option<&str>,
    ) -> Result<User, Error> {
        // Note: do not add `is_company` here. The server ignores it.
        #[skip_serializing_none]
        #[derive(Serialize)]
        struct Payload<'a> {
            pub company_name: Option<&'a str>,
            pub company_website: Option<&'a str>,
            /// `None` leaves the picture unchanged; `Some("")` clears it on the server.
            pub picture: Option<&'a str>,
            pub industry: Option<&'a str>,
            pub referral_code: Option<&'a str>,
        }

        if let Some(url) = company_website {
            Url::parse(url).map_err(|_| UrlError::Invalid)?;
        }
        if let Some(url) = picture {
            Url::parse(url).map_err(|_| UrlError::Invalid)?;
        }

        self.client
            .put("/api/user/info")
            .json(&Payload {
                company_name,
                company_website,
                picture,
                industry,
                referral_code,
            })
            .ok()
            .await
    }
}
