use crate::error::HttpStatusError;
use crate::{Error, Response, ValidationError};
use reqwest::header::USER_AGENT;
use reqwest::{Method, StatusCode};
use secrecy::{ExposeSecret, SecretString};
use serde::de::DeserializeOwned;
use serde::Serialize;
use url::Url;

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
