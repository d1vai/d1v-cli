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
pub mod node;
pub mod output;
pub mod project;
pub mod prompt;
pub mod quick_deploy;
#[cfg(feature = "record")]
pub mod recorder;
pub mod runtime_install;
pub mod session;
pub mod shell;
pub mod skill;
pub mod symbols;
pub mod text;
pub mod theme;
pub mod token;
pub mod ui;
pub mod upgrade;
pub mod user;
pub mod workspace;

use std::fmt::Display;
use std::io::{IsTerminal, stderr};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use anstream::ColorChoice;
use d1v_api::{Client, ProgressEvent, UserAgent};
use indicatif::{ProgressBar, ProgressStyle};
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
    progress_bar: Option<ProgressBar>,
    deployment_progress_active: Arc<AtomicBool>,
}

pub struct DeploymentProgress {
    progress_bar: Option<ProgressBar>,
    active: Arc<AtomicBool>,
}

impl Drop for DeploymentProgress {
    fn drop(&mut self) {
        self.active.store(false, Ordering::Release);
        if let Some(progress_bar) = &self.progress_bar {
            progress_bar.finish_and_clear();
        }
    }
}

impl Context {
    pub fn new(
        format: Format,
        color: ColorChoice,
        base_url_override: BaseUrlCandidate,
    ) -> Result<Self> {
        Self::build(format, color, base_url_override, true)
    }

    /// Builds a context without querying token providers.
    pub fn new_without_token_lookup(
        format: Format,
        color: ColorChoice,
        base_url_override: BaseUrlCandidate,
    ) -> Result<Self> {
        Self::build(format, color, base_url_override, false)
    }

    fn build(
        format: Format,
        color: ColorChoice,
        base_url_override: BaseUrlCandidate,
        load_token: bool,
    ) -> Result<Self> {
        let config = Config::load()?;
        let tokens = TokenChain::default();

        let candidates = [
            base_url_override,
            base_url::from_config(config.base_url_override().map(ToOwned::to_owned)),
        ];
        let base_url = BaseUrl::resolve(candidates);

        let spinner = if matches!(format, Format::Text) && stderr().is_terminal() {
            let spinner = indicatif::ProgressBar::new_spinner();
            spinner.enable_steady_tick(Duration::from_millis(80));
            spinner.set_style(
                ProgressStyle::with_template("{spinner:.cyan} {msg} {elapsed_precise}")
                    .expect("valid progress template"),
            );
            spinner.set_message("Working...");
            spinner.set_draw_target(indicatif::ProgressDrawTarget::hidden());
            Some(spinner)
        } else {
            None
        };
        let spinner_for_handler = spinner.clone();
        let deployment_progress_active = Arc::new(AtomicBool::new(false));
        let progress_active_for_handler = deployment_progress_active.clone();

        let mut builder = Client::builder()
            .base_url(base_url.as_str())
            .user_agent(&UserAgent::new("d1v-cli", env!("CARGO_PKG_VERSION")))
            .client_name("d1v-cli")
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30));

        if let Some(spinner) = spinner_for_handler {
            builder = builder.progress_handler(Arc::new(move |event| match event {
                ProgressEvent::Started => {
                    spinner.set_draw_target(indicatif::ProgressDrawTarget::stderr());
                    if progress_active_for_handler.load(Ordering::Acquire) {
                        spinner.set_message("Deploying...");
                    } else {
                        spinner.set_message("Working...");
                    }
                }
                ProgressEvent::Finished => {
                    if !progress_active_for_handler.load(Ordering::Acquire) {
                        spinner.set_draw_target(indicatif::ProgressDrawTarget::hidden());
                        spinner.tick();
                    }
                }
            }));
        }

        if load_token {
            if let Ok(Some(token)) = tokens.lookup() {
                builder = builder.token(token);
            }
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
            progress_bar: spinner,
            deployment_progress_active,
        })
    }

    /// Shows a single terminal line for the full lifetime of a deployment.
    pub fn deployment_progress(&self) -> DeploymentProgress {
        self.deployment_progress_active
            .store(true, Ordering::Release);
        if let Some(progress_bar) = &self.progress_bar {
            progress_bar.reset_elapsed();
            progress_bar.set_message("Deploying...");
            progress_bar.set_draw_target(indicatif::ProgressDrawTarget::stderr());
            progress_bar.tick();
        }
        DeploymentProgress {
            progress_bar: self.progress_bar.clone(),
            active: self.deployment_progress_active.clone(),
        }
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
