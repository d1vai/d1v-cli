use crate::error::HttpStatusError;
use crate::{Error, Response, ValidationError};
use reqwest::header::USER_AGENT;
use reqwest::{Method, StatusCode};
use secrecy::{ExposeSecret, SecretString};
use serde::de::DeserializeOwned;
use serde::Serialize;
use url::Url;

#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    base_url: Url,
    token: Option<SecretString>,
    user_agent: Option<String>,
}

impl Client {
    pub fn new(http: reqwest::Client, base_url: impl AsRef<str>) -> Result<Self, Error> {
        Ok(Client {
            http,
            base_url: Url::parse(base_url.as_ref())?,
            token: None,
            user_agent: None,
        })
    }

    pub fn token(&mut self, token: impl Into<SecretString>) -> &mut Self {
        self.token = Some(token.into());
        self
    }

    pub fn user_agent(&mut self, user_agent: impl Into<String>) -> &mut Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    fn url(&self, path: impl AsRef<str>) -> Result<Url, Error> {
        self.base_url.join(path.as_ref()).map_err(Error::Url)
    }

    pub fn request(
        &self,
        method: Method,
        path: impl AsRef<str>,
    ) -> Result<RequestBuilder<'_>, Error> {
        let url = self.url(path)?;
        let mut inner = self.http.request(method, url);

        if let Some(ua) = &self.user_agent {
            inner = inner.header(USER_AGENT, ua);
        }

        Ok(RequestBuilder {
            client: self,
            inner,
            auth: true,
        })
    }

    pub fn get(&self, path: impl AsRef<str>) -> Result<RequestBuilder<'_>, Error> {
        self.request(Method::GET, path)
    }

    pub fn post(&self, path: impl AsRef<str>) -> Result<RequestBuilder<'_>, Error> {
        self.request(Method::POST, path)
    }

    pub fn put(&self, path: impl AsRef<str>) -> Result<RequestBuilder<'_>, Error> {
        self.request(Method::PUT, path)
    }

    pub fn delete(&self, path: impl AsRef<str>) -> Result<RequestBuilder<'_>, Error> {
        self.request(Method::DELETE, path)
    }

    pub fn patch(&self, path: impl AsRef<str>) -> Result<RequestBuilder<'_>, Error> {
        self.request(Method::PATCH, path)
    }
}

pub struct RequestBuilder<'a> {
    client: &'a Client,
    inner: reqwest::RequestBuilder,
    auth: bool,
}

impl<'a> RequestBuilder<'a> {
    pub fn query(self, query: &(impl Serialize + ?Sized)) -> Self {
        Self {
            inner: self.inner.query(query),
            ..self
        }
    }

    pub fn json(self, json: &(impl Serialize + ?Sized)) -> Self {
        Self {
            inner: self.inner.json(json),
            ..self
        }
    }

    pub fn no_auth(self) -> Self {
        Self {
            auth: false,
            ..self
        }
    }

    pub async fn send(self) -> Result<Response, Error> {
        let mut inner = self.inner;

        if self.auth
            && let Some(token) = &self.client.token
        {
            inner = inner.bearer_auth(token.expose_secret());
        }

        let resp = inner.send().await?;
        let status = resp.status();

        match status {
            StatusCode::OK => Ok(resp.json::<Response>().await?),
            StatusCode::UNPROCESSABLE_ENTITY => Err(resp.json::<ValidationError>().await?.into()),
            _ => Err(HttpStatusError::new(status, resp.text().await.unwrap_or_default()).into()),
        }
    }

    pub async fn ok<T>(self) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        self.send().await?.ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use serde::Deserialize;

    #[test]
    fn test_debug_redacts_token() {
        let mut client = Client::new(reqwest::Client::new(), "https://api.example.com").unwrap();
        client.token("secret-token");

        let debug = format!("{:#?}", client);
        assert!(
            !debug.contains("secret-token"),
            "token leaked in debug output: {debug}"
        );
        assert!(
            debug.contains("api.example.com"),
            "base_url missing: {debug}"
        );
    }

    fn test_client(server: &MockServer) -> Client {
        Client::new(reqwest::Client::new(), server.base_url()).unwrap()
    }

    #[test]
    fn test_new_invalid_url() {
        let err = Client::new(reqwest::Client::new(), "not a url").unwrap_err();
        assert!(matches!(err, Error::Url(_)));
    }

    #[test]
    fn test_new_valid_url() {
        let client = Client::new(reqwest::Client::new(), "https://api.example.com").unwrap();
        assert_eq!(client.base_url.as_str(), "https://api.example.com/");
    }

    #[tokio::test]
    async fn test_get_ok() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/api/user/profile");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"code": 0, "msg": "success", "data": {"name": "d1v"}}"#);
        });

        #[derive(Debug, Deserialize, PartialEq)]
        struct User {
            name: String,
        }

        let client = test_client(&server);
        let user: User = client.get("/api/user/profile").unwrap().ok().await.unwrap();
        assert_eq!(user, User { name: "d1v".into() });

        mock.assert();
    }

    #[tokio::test]
    async fn test_post_void() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/api/user/verify-code")
                .query_param("email", "test@example.com");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"code": 0, "msg": "success", "data": null}"#);
        });

        let client = test_client(&server);
        client
            .post("/api/user/verify-code")
            .unwrap()
            .query(&[("email", "test@example.com")])
            .no_auth()
            .ok::<()>()
            .await
            .unwrap();

        mock.assert();
    }

    #[tokio::test]
    async fn test_api_error() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/api/resource");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"code": 401, "msg": "unauthorized", "data": null}"#);
        });

        let client = test_client(&server);
        let err = client
            .get("/api/resource")
            .unwrap()
            .ok::<()>()
            .await
            .unwrap_err();
        assert_eq!(err.to_string(), "api error 401: unauthorized");

        mock.assert();
    }

    #[tokio::test]
    async fn test_validation_error() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST).path("/api/user/login");
            then.status(422)
                .header("content-type", "application/json")
                .body(
                    r#"{"detail": [{"loc": ["body", "email"], "msg": "Field required", "type": "missing"}]}"#,
                );
        });

        let client = test_client(&server);
        let err = client
            .post("/api/user/login")
            .unwrap()
            .ok::<()>()
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Validation(_)));

        mock.assert();
    }

    #[tokio::test]
    async fn test_http_status_error() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/api/missing");
            then.status(404).body("not found");
        });

        let client = test_client(&server);
        let err = client
            .get("/api/missing")
            .unwrap()
            .ok::<()>()
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "http status error (404 Not Found): not found"
        );

        mock.assert();
    }

    #[tokio::test]
    async fn test_bearer_auth() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/api/protected")
                .header("authorization", "Bearer secret-token");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"code": 0, "msg": "ok", "data": null}"#);
        });

        let mut client = test_client(&server);
        client.token("secret-token");
        client
            .get("/api/protected")
            .unwrap()
            .ok::<()>()
            .await
            .unwrap();

        mock.assert();
    }

    #[tokio::test]
    async fn test_no_auth_skips_token() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/api/public")
                .header_missing("authorization");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"code": 0, "msg": "ok", "data": null}"#);
        });

        let mut client = test_client(&server);
        client.token("secret-token".to_string());
        client
            .get("/api/public")
            .unwrap()
            .no_auth()
            .ok::<()>()
            .await
            .unwrap();

        mock.assert();
    }

    #[tokio::test]
    async fn test_user_agent() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/api/test")
                .header("user-agent", "d1v-cli/0.1.0");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"code": 0, "msg": "ok", "data": null}"#);
        });

        let mut client = test_client(&server);
        client.user_agent("d1v-cli/0.1.0");
        client.get("/api/test").unwrap().ok::<()>().await.unwrap();

        mock.assert();
    }

    #[tokio::test]
    async fn test_send_returns_response() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/api/items");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"code": 0, "msg": "ok", "data": [1, 2, 3], "total": 100}"#);
        });

        let client = test_client(&server);
        let resp = client.get("/api/items").unwrap().send().await.unwrap();

        assert_eq!(resp.total, Some(100));
        let items: Vec<i32> = resp.ok().unwrap();
        assert_eq!(items, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_post_json_body() {
        let server = MockServer::start();

        #[derive(Debug, Serialize)]
        struct LoginRequest {
            email: String,
            code: String,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct LoginResponse {
            token: String,
        }

        let body = LoginRequest {
            email: "test@example.com".into(),
            code: "123456".into(),
        };

        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/api/user/login")
                .header("content-type", "application/json")
                .json_body_obj(&body);
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"code": 0, "msg": "success", "data": {"token": "abc123"}}"#);
        });

        let client = test_client(&server);
        let resp: LoginResponse = client
            .post("/api/user/login")
            .unwrap()
            .no_auth()
            .json(&body)
            .ok()
            .await
            .unwrap();
        assert_eq!(
            resp,
            LoginResponse {
                token: "abc123".into()
            }
        );

        mock.assert();
    }
}
