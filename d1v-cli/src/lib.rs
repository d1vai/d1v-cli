pub mod agent;
pub mod api_key;
pub mod auth;
pub mod banner;
pub mod base_url;
pub mod config;
pub mod db;
pub mod debug;
pub mod deploy;
pub mod env;
pub mod error;
pub mod expose;
pub mod github;
pub mod i18n;
pub mod localize;
pub mod logging;
pub mod output;
pub mod project;
pub mod prompt;
#[cfg(feature = "record")]
pub mod recorder;
pub mod runtime_install;
pub mod session;
pub mod symbols;
pub mod text;
pub mod theme;
pub mod token;
pub mod ui;
pub mod upgrade;
pub mod user;
pub mod workspace;

use std::fmt::Display;
use std::time::Duration;

use anstream::ColorChoice;
use d1v_api::{Client, UserAgent};
use serde::Serialize;

use crate::config::Config;
use crate::error::{Error, Result};
use crate::output::{Format, Output};
use crate::token::{TokenChain, TokenSource};
use text::Render;

pub use crate::base_url::{BaseUrl, BaseUrlCandidate, BaseUrlSource};

pub struct Context {
    pub client: Client,
    pub tokens: TokenChain,
    pub output: Output,
}

impl Context {
    pub fn new(
        format: Format,
        color: ColorChoice,
        base_url_override: BaseUrlCandidate,
    ) -> Result<Self> {
        let config = Config::load()?;
        let tokens = TokenChain::default();

        let candidates = [
            base_url_override,
            base_url::from_config(config.base_url_override().map(ToOwned::to_owned)),
        ];
        let base_url = BaseUrl::resolve(candidates);

        let mut builder = Client::builder()
            .base_url(base_url.as_str())
            .user_agent(&UserAgent::new("d1v-cli", env!("CARGO_PKG_VERSION")))
            .client_name("d1v-cli")
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30));

        if let Ok(Some(token)) = tokens.lookup() {
            builder = builder.token(token);
        }

        let client = builder.build().map_err(|err| match err {
            d1v_api::Error::Url(cause) => Error::InvalidBaseUrl {
                url: base_url,
                cause: cause.to_string(),
            },
            other => Error::from(other),
        })?;

        Ok(Self {
            client,
            tokens,
            output: Output::new(format, color),
        })
    }

    /// Writes a success message via the output formatter.
    pub fn success(&self, msg: impl Display) {
        self.output.success(msg);
    }

    /// Writes an informational message via the output formatter.
    pub fn info(&self, msg: impl Display) {
        self.output.info(msg);
    }

    /// Writes a status message via the output formatter.
    pub fn message(&self, msg: impl Display) {
        self.output.message(msg);
    }

    pub fn present<T, J>(&self, text: T, json: &J) -> Result
    where
        T: Render,
        J: Serialize + ?Sized,
    {
        self.output.present(text, json)
    }
}
