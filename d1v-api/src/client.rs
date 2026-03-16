use crate::{Error, HttpStatusError, Response, ValidationError};
use parking_lot::RwLock;
use reqwest::header::USER_AGENT;
use reqwest::{Method, StatusCode};
use secrecy::{ExposeSecret, SecretString};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::Arc;
use url::Url;

#[derive(Debug, Clone)]
pub struct Client {
    inner: Arc<ClientInner>,
}

#[derive(Debug)]
struct ClientInner {
    http: reqwest::Client,
    base_url: Url,
    user_agent: Option<String>,
    token: RwLock<Option<SecretString>>,
}

pub struct ClientBuilder {
    http: Option<reqwest::Client>,
    base_url: String,
    user_agent: Option<String>,
    token: Option<SecretString>,
}

impl ClientBuilder {
    pub fn new() -> Self {
        ClientBuilder {
            http: None,
            base_url: crate::DEFAULT_BASE_URL.to_string(),
            user_agent: None,
            token: None,
        }
    }

    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn client(mut self, client: reqwest::Client) -> Self {
        self.http = Some(client);
        self
    }

    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    pub fn token(mut self, token: impl Into<SecretString>) -> Self {
        self.token = Some(token.into());
        self
    }

    pub fn build(self) -> Result<Client, Error> {
        Ok(Client {
            inner: Arc::new(ClientInner {
                http: self.http.unwrap_or_default(),
                base_url: Url::parse(&self.base_url)?,
                user_agent: self.user_agent,
                token: RwLock::new(self.token),
            }),
        })
    }
}

impl Client {
    pub fn new() -> Result<Self, Error> {
        ClientBuilder::new().build()
    }

    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    pub fn token(&self, token: impl Into<SecretString>) -> &Self {
        *self.inner.token.write() = Some(token.into());
        self
    }

    fn url(&self, path: impl AsRef<str>) -> Result<Url, Error> {
        self.inner.base_url.join(path.as_ref()).map_err(Error::Url)
    }

    pub fn request(&self, method: Method, path: impl AsRef<str>) -> RequestBuilder {
        let inner = self.url(path).map(|url| {
            let mut builder = self.inner.http.request(method, url);

            if let Some(ua) = self.inner.user_agent.as_deref() {
                builder = builder.header(USER_AGENT, ua);
            }

            builder
        });

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
}

pub struct RequestBuilder {
    client: Client,
    inner: Result<reqwest::RequestBuilder, Error>,
    auth: bool,
}

impl RequestBuilder {
    pub fn query(self, query: &(impl Serialize + ?Sized)) -> Self {
        Self {
            inner: self.inner.map(|inner| inner.query(query)),
            ..self
        }
    }

    pub fn json(self, json: &(impl Serialize + ?Sized)) -> Self {
        Self {
            inner: self.inner.map(|inner| inner.json(json)),
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
        let mut inner = self.inner?;

        if self.auth
            && let Some(token) = self.client.inner.token.read().as_ref()
        {
            inner = inner.bearer_auth(token.expose_secret());
        }

        #[cfg(feature = "record")]
        {
            crate::record::execute(&self.client.inner.http, inner).await
        }

        #[cfg(not(feature = "record"))]
        {
            let resp = inner.send().await?;
            let status = resp.status();

            match status {
                StatusCode::OK => Ok(resp.json::<Response>().await?),
                StatusCode::UNPROCESSABLE_ENTITY => {
                    Err(resp.json::<ValidationError>().await?.into())
                }
                _ => {
                    Err(HttpStatusError::new(status, resp.text().await.unwrap_or_default()).into())
                }
            }
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
    fn test_debug_redacts_token() {
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
    fn test_new_uses_default_url() {
        let client = Client::new().unwrap();
        assert_eq!(
            client.inner.base_url.as_str(),
            format!("{}/", crate::DEFAULT_BASE_URL)
        );
    }

    #[test]
    fn test_builder_invalid_url() {
        let err = Client::builder().base_url("not a url").build().unwrap_err();
        assert!(matches!(err, Error::Url(_)));
    }

    #[test]
    fn test_builder_valid_url() {
        let client = Client::builder()
            .base_url("https://api.example.com")
            .build()
            .unwrap();
        assert_eq!(client.inner.base_url.as_str(), "https://api.example.com/");
    }
}
