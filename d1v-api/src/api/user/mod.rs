mod types;

pub use types::{UpdateUser, User};

use secrecy::{ExposeSecret, SecretString};
use serde_json::json;

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
        self.client
            .post("/api/user/verify-code")
            .query(&[("email", email.as_ref())])
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
        self.client
            .post("/api/user/verify-code/check")
            .json(&json!({
                "email": email.as_ref(),
                "code": code.as_ref(),
                "purpose": purpose.as_ref(),
            }))
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
        self.client
            .post("/api/user/login")
            .json(&json!({
                "email": email.as_ref(),
                "verify_code": code.as_ref(),
            }))
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
        self.client
            .post("/api/user/login/password")
            .json(&json!({
                "email": email.as_ref(),
                "password": password.expose_secret(),
            }))
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
        self.client
            .post("/api/user/password/set")
            .json(&json!({ "password": password.expose_secret() }))
            .ok()
            .await
    }

    /// Sends a forgot-password email.
    pub async fn send_forgot_password_email(&self, email: impl AsRef<str>) -> Result<(), Error> {
        self.client
            .post("/api/user/password/forgot/send")
            .json(&json!({ "email": email.as_ref() }))
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
        self.client
            .post("/api/user/password/reset")
            .json(&json!({
                "email": email.as_ref(),
                "code": code.as_ref(),
                "new_password": new_password.expose_secret(),
            }))
            .no_auth()
            .ok()
            .await
    }
}
