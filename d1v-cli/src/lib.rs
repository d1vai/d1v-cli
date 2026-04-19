pub mod auth;
pub mod banner;
pub mod config;
pub mod debug;
pub mod error;
pub mod i18n;
pub mod localize;
pub mod logging;
pub mod output;
pub mod prompt;
#[cfg(feature = "record")]
pub mod recorder;
pub mod symbols;
pub mod theme;
pub mod token;
pub mod ui;
pub mod user;

use std::fmt::Display;
use std::time::Duration;

use d1v_api::{Client, UserAgent};
use serde::Serialize;

use crate::config::Config;
use crate::error::Result;
use crate::output::{Color, Format, Output};
use crate::token::{TokenChain, TokenLoader};

pub struct Context {
    pub client: Client,
    pub tokens: TokenChain,
    pub output: Output,
}

impl Context {
    pub fn new(format: Format, color: Color, base_url: Option<String>) -> Result<Self> {
        let config = Config::load()?;
        let tokens = TokenChain::default();

        let mut builder = Client::builder()
            .base_url(base_url.unwrap_or(config.base_url))
            .user_agent(UserAgent::new("d1v-cli", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30));

        if let Ok(Some(token)) = tokens.load() {
            builder = builder.token(token);
        }

        Ok(Self {
            client: builder.build()?,
            tokens,
            output: Output::new(format, color.resolve()),
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

    /// Writes structured data via the output formatter.
    pub fn print(&self, value: &(impl Display + Serialize)) -> Result {
        self.output.print(value)
    }

    /// Writes a list of structured data via the output formatter.
    pub fn print_list(&self, values: impl IntoIterator<Item = impl Display + Serialize>) -> Result {
        self.output.print_list(values)
    }
}
