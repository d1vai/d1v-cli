use secrecy::SecretString;
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
        password: impl AsRef<str>,
    ) -> Result<SecretString, Error> {
        self.client
            .post("/api/user/login/password")
            .json(&json!({
                "email": email.as_ref(),
                "password": password.as_ref(),
            }))
            .no_auth()
            .ok()
            .await
    }
}
