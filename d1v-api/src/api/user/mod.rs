mod types;

pub use types::{DailyCount, PromptDailyActivity, UpdateUser, User};

use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;

use crate::{Client, Error};

pub struct UserApi {
    client: Client,
}

impl Client {
    pub fn user(&self) -> UserApi {
        UserApi {
            client: self.clone(),
        }
    }
}

impl UserApi {
    /// Sends a verification code to the given email.
    pub async fn send_code(&self, email: impl AsRef<str>) -> Result<(), Error> {
        #[derive(Serialize)]
        struct Query<'a> {
            email: &'a str,
        }

        self.client
            .post("/api/user/verify-code")
            .query(&Query {
                email: email.as_ref(),
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
            email: &'a str,
            code: &'a str,
            purpose: &'a str,
        }

        self.client
            .post("/api/user/verify-code/check")
            .json(&Payload {
                email: email.as_ref(),
                code: code.as_ref(),
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
            email: &'a str,
            verify_code: &'a str,
        }

        self.client
            .post("/api/user/login")
            .json(&Payload {
                email: email.as_ref(),
                verify_code: code.as_ref(),
            })
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
            email: &'a str,
            password: &'a str,
        }

        self.client
            .post("/api/user/login/password")
            .json(&Payload {
                email: email.as_ref(),
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
            email: &'a str,
            password: &'a str,
        }

        self.client
            .post("/api/user/password/login")
            .json(&Payload {
                email: email.as_ref(),
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

    /// Updates the current user's info.
    pub async fn update_info(&self, update: &UpdateUser) -> Result<User, Error> {
        self.client.put("/api/user/info").json(update).ok().await
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
    pub async fn send_forgot_password_email(&self, email: impl AsRef<str>) -> Result<(), Error> {
        #[derive(Serialize)]
        struct Payload<'a> {
            email: &'a str,
        }

        self.client
            .post("/api/user/password/forgot/send")
            .json(&Payload {
                email: email.as_ref(),
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
            email: &'a str,
            code: &'a str,
            new_password: &'a str,
        }

        self.client
            .post("/api/user/password/reset")
            .json(&Payload {
                email: email.as_ref(),
                code: code.as_ref(),
                new_password: new_password.expose_secret(),
            })
            .no_auth()
            .ok()
            .await
    }

    /// Sends a verification code to bind an email.
    pub async fn send_bind_email_code(&self, email: impl AsRef<str>) -> Result<(), Error> {
        #[derive(Serialize)]
        struct Payload<'a> {
            email: &'a str,
        }

        self.client
            .post("/api/user/bind-email/send")
            .json(&Payload {
                email: email.as_ref(),
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
            email: &'a str,
            code: &'a str,
        }

        self.client
            .post("/api/user/bind-email/confirm")
            .json(&Payload {
                email: email.as_ref(),
                code: code.as_ref(),
            })
            .ok()
            .await
    }

    /// Sends a verification code to change email.
    pub async fn send_change_email_code(&self, new_email: impl AsRef<str>) -> Result<(), Error> {
        #[derive(Serialize)]
        struct Payload<'a> {
            new_email: &'a str,
        }

        self.client
            .post("/api/user/email/change/send")
            .json(&Payload {
                new_email: new_email.as_ref(),
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
            new_email: &'a str,
            code: &'a str,
        }

        self.client
            .post("/api/user/email/change/confirm")
            .json(&Payload {
                new_email: new_email.as_ref(),
                code: code.as_ref(),
            })
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
            .no_auth()
            .ok()
            .await
    }
}
