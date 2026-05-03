use crate::jwt::{Claims, DecodeError, Token};
use crate::{Error, HttpStatusError, Response, ServerValidationError, UserAgent};
use parking_lot::RwLock;
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue};
use reqwest::{Method, StatusCode};
use secrecy::ExposeSecret;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;
use url::Url;

#[derive(Debug, Clone)]
pub struct Client {
    inner: Arc<ClientInner>,
}

#[derive(Debug)]
struct ClientInner {
    http: reqwest::Client,
    base_url: Url,
    token: RwLock<Option<Token>>,
}

pub struct ClientBuilder {
    inner: reqwest::ClientBuilder,
    base_url: String,
    token: Option<Token>,
    client_name: Option<String>,
}

impl ClientBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn from_reqwest(builder: reqwest::ClientBuilder) -> Self {
        builder.into()
    }

    #[must_use]
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    #[must_use]
    pub fn user_agent(mut self, user_agent: &UserAgent) -> Self {
        self.inner = self.inner.user_agent(user_agent.to_string());
        self
    }

    #[must_use]
    pub fn client_name(mut self, client_name: impl Into<String>) -> Self {
        self.client_name = Some(client_name.into());
        self
    }

    #[must_use]
    pub fn token(mut self, token: impl Into<Token>) -> Self {
        self.token = Some(token.into());
        self
    }

    #[must_use]
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.inner = self.inner.connect_timeout(timeout);
        self
    }

    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.inner = self.inner.timeout(timeout);
        self
    }

    pub fn build(self) -> Result<Client, Error> {
        let mut headers = HeaderMap::new();

        #[cfg(feature = "mock")]
        if let Ok(scenario) = std::env::var("D1V_TEST_SCENARIO")
            && let Ok(value) = HeaderValue::from_str(&scenario)
        {
            headers.insert("X-Test-Scenario", value);
        }

        if let Some(client_name) = self.client_name
            && let Ok(value) = HeaderValue::from_str(&client_name)
        {
            headers.insert("X-D1V-Client", value);
        }

        Ok(Client {
            inner: Arc::new(ClientInner {
                http: self.inner.default_headers(headers).build()?,
                base_url: Url::parse(&self.base_url)?,
                token: RwLock::new(self.token),
            }),
        })
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        reqwest::Client::builder()
            .user_agent(UserAgent::new("d1v-api", env!("CARGO_PKG_VERSION")).to_string())
            .into()
    }
}

impl From<reqwest::ClientBuilder> for ClientBuilder {
    fn from(builder: reqwest::ClientBuilder) -> Self {
        ClientBuilder {
            inner: builder,
            base_url: crate::DEFAULT_BASE_URL.to_string(),
            token: None,
            client_name: None,
        }
    }
}

impl Client {
    pub fn new() -> Result<Self, Error> {
        ClientBuilder::new().build()
    }

    #[must_use]
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    pub fn from_reqwest(client: reqwest::Client, base_url: impl AsRef<str>) -> Result<Self, Error> {
        Ok(Client {
            inner: Arc::new(ClientInner {
                http: client,
                base_url: Url::parse(base_url.as_ref())?,
                token: RwLock::new(None),
            }),
        })
    }

    pub fn token(&self, token: impl Into<Token>) -> &Self {
        *self.inner.token.write() = Some(token.into());
        self
    }

    #[must_use]
    pub fn base_url(&self) -> &str {
        self.inner.base_url.as_str()
    }

    fn url(&self, path: impl AsRef<str>) -> Result<Url, Error> {
        self.inner.base_url.join(path.as_ref()).map_err(Error::Url)
    }

    pub fn request(&self, method: Method, path: impl AsRef<str>) -> RequestBuilder {
        let inner = self
            .url(path)
            .map(|url| self.inner.http.request(method, url));

        RequestBuilder {
            client: self.clone(),
            inner,
            auth: true,
        }
    }

    pub fn get(&self, path: impl AsRef<str>) -> RequestBuilder {
        self.request(Method::GET, path)
    }

    pub fn post(&self, path: impl AsRef<str>) -> RequestBuilder {
        self.request(Method::POST, path)
    }

    pub fn put(&self, path: impl AsRef<str>) -> RequestBuilder {
        self.request(Method::PUT, path)
    }

    pub fn delete(&self, path: impl AsRef<str>) -> RequestBuilder {
        self.request(Method::DELETE, path)
    }

    pub fn patch(&self, path: impl AsRef<str>) -> RequestBuilder {
        self.request(Method::PATCH, path)
    }

    /// Decodes JWT claims from the current token.
    ///
    /// Returns `None` if no token is set or the token cannot be decoded.
    pub fn claims(&self) -> Option<Claims> {
        self.decode_claims()?.ok()
    }

    pub fn decode_claims(&self) -> Option<Result<Claims, DecodeError>> {
        let guard = self.inner.token.read();
        let token = guard.as_ref()?;
        Some(token.claims())
    }

    /// Returns whether the current token has expired.
    pub fn is_token_expired(&self) -> bool {
        self.inner
            .token
            .read()
            .as_ref()
            .is_some_and(Token::is_expired)
    }
}

pub struct RequestBuilder {
    client: Client,
    inner: Result<reqwest::RequestBuilder, Error>,
    auth: bool,
}

impl RequestBuilder {
    #[must_use]
    pub fn query(self, query: &(impl Serialize + ?Sized)) -> Self {
        Self {
            inner: self.inner.map(|inner| inner.query(query)),
            ..self
        }
    }

    #[must_use]
    pub fn query_if_some<T>(self, key: &str, value: Option<T>) -> Self
    where
        T: Serialize,
    {
        match value {
            Some(value) => self.query(&[(key, value)]),
            None => self,
        }
    }

    #[must_use]
    pub fn json(self, json: &(impl Serialize + ?Sized)) -> Self {
        Self {
            inner: self.inner.map(|inner| inner.json(json)),
            ..self
        }
    }

    #[must_use]
    pub fn multipart(self, form: reqwest::multipart::Form) -> Self {
        Self {
            inner: self.inner.map(|inner| inner.multipart(form)),
            ..self
        }
    }

    #[must_use]
    pub fn no_auth(self) -> Self {
        Self {
            auth: false,
            ..self
        }
    }

    pub async fn send(self) -> Result<Response, Error> {
        let mut inner = self.inner?.header(ACCEPT, "application/json");

        if self.auth {
            if self.client.is_token_expired() {
                return Err(Error::TokenExpired);
            }

            if let Some(token) = self.client.inner.token.read().as_ref() {
                inner = inner.bearer_auth(token.expose_secret());
            }
        }

        let request = inner.build()?;
        debug!(method = %request.method(), url = %request.url(), "request");

        #[cfg(feature = "record")]
        let req_record = crate::record::Request::from(&request);

        let resp = self.client.inner.http.execute(request).await?;
        let status = resp.status();

        #[cfg(feature = "record")]
        let resp_headers = resp.headers().clone();

        let bytes = resp.bytes().await?;

        #[cfg(feature = "record")]
        crate::record::dispatch(
            &req_record,
            &crate::record::Response::new(status, &resp_headers, &bytes),
        );

        match Self::parse_response(status, &bytes) {
            Ok(resp) => {
                debug!(status = status.as_u16(), code = resp.code, msg = %resp.message, "response");
                Ok(resp)
            }
            Err(err) => {
                debug!(status = status.as_u16(), "response");
                Err(err)
            }
        }
    }

    fn parse_response(status: StatusCode, bytes: &[u8]) -> Result<Response, Error> {
        match status {
            StatusCode::OK => Ok(serde_json::from_slice(bytes)?),
            StatusCode::UNPROCESSABLE_ENTITY => {
                Err(serde_json::from_slice::<ServerValidationError>(bytes)?.into())
            }
            _ => Err(HttpStatusError::new(status, String::from_utf8_lossy(bytes)).into()),
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

    #[test]
    fn debug_redacts_token() {
        let client = Client::builder()
            .base_url("https://api.example.com")
            .token("secret-token")
            .build()
            .unwrap();

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

    #[test]
    fn new_uses_default_url() {
        let client = Client::new().unwrap();
        assert_eq!(
            client.inner.base_url.as_str(),
            format!("{}/", crate::DEFAULT_BASE_URL)
        );
    }

    #[test]
    fn builder_invalid_url() {
        let err = Client::builder().base_url("not a url").build().unwrap_err();
        assert!(matches!(err, Error::Url(_)));
    }

    #[test]
    fn builder_valid_url() {
        let client = Client::builder()
            .base_url("https://api.example.com")
            .build()
            .unwrap();
        assert_eq!(client.inner.base_url.as_str(), "https://api.example.com/");
    }

    #[test]
    fn query_if_some_adds() {
        let client = Client::builder()
            .base_url("https://api.example.com")
            .build()
            .unwrap();

        let request = client
            .get("/api/items")
            .query_if_some("days", Some(7))
            .inner
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(request.url().path(), "/api/items");
        assert_eq!(request.url().query(), Some("days=7"));
    }

    #[test]
    fn query_if_some_none() {
        let client = Client::builder()
            .base_url("https://api.example.com")
            .build()
            .unwrap();

        let request = client
            .get("/api/items")
            .query_if_some("days", None::<i32>)
            .inner
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(request.url().path(), "/api/items");
        assert_eq!(request.url().query(), None);
    }
}
