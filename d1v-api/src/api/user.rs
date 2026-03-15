use secrecy::SecretString;
use serde::Serialize;

use crate::{Client, Error};

pub struct UserApi<'a> {
    client: &'a Client,
}

impl Client {
    pub fn user(&self) -> UserApi<'_> {
        UserApi { client: self }
    }
}

#[derive(Debug, Serialize)]
struct LoginRequest<'a> {
    email: &'a str,
    #[serde(rename = "verify_code")]
    code: &'a str,
}

impl UserApi<'_> {
    /// Sends a verification code to the given email.
    pub async fn send_code(&self, email: impl AsRef<str>) -> Result<(), Error> {
        self.client
            .post("/api/user/verify-code")
            .query(&[("email", email.as_ref())])
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
            .json(&LoginRequest {
                email: email.as_ref(),
                code: code.as_ref(),
            })
            .no_auth()
            .ok()
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use secrecy::ExposeSecret;

    fn test_client(server: &MockServer) -> Client {
        Client::new(reqwest::Client::new(), server.base_url()).unwrap()
    }

    #[tokio::test]
    async fn test_send_verification_code() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/api/user/verify-code")
                .query_param("email", "test@example.com")
                .header_missing("authorization");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"code": 0, "msg": "success", "data": null}"#);
        });

        let client = test_client(&server);
        client.user().send_code("test@example.com").await.unwrap();

        mock.assert();
    }

    #[tokio::test]
    async fn test_login() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/api/user/login")
                .header("content-type", "application/json")
                .json_body_obj(&LoginRequest {
                    email: "test@example.com",
                    code: "123456",
                })
                .header_missing("authorization");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"code": 0, "msg": "success", "data": "abc123"}"#);
        });

        let client = test_client(&server);
        let token = client
            .user()
            .login("test@example.com", "123456")
            .await
            .unwrap();
        assert_eq!(token.expose_secret(), "abc123");

        mock.assert();
    }

    #[tokio::test]
    async fn test_login_wrong_code() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST).path("/api/user/login");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"code": 1, "msg": "verification code is incorrect", "data": null}"#);
        });

        let client = test_client(&server);
        let err = client
            .user()
            .login("test@example.com", "wrong")
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "api error 1: verification code is incorrect"
        );

        mock.assert();
    }
}
