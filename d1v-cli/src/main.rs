mod auth;
mod config;
mod debug;
mod i18n;
mod logging;
mod output;
#[cfg(feature = "record")]
mod recorder;
mod token;

use std::fmt::Display;
use std::time::Duration;

use anyhow::Result;
use clap::{Parser, Subcommand};
use d1v_api::{Client, UserAgent};
use serde::Serialize;
use tracing::{error, info};

use crate::config::Config;
use crate::output::{Format, Output};
use crate::token::{TokenChain, TokenLoader};

pub struct Context {
    pub client: Client,
    pub tokens: TokenChain,
    pub output: Output,
}

impl Context {
    fn new(format: Format) -> Result<Self> {
        let config = Config::load()?;
        let tokens = TokenChain::default();

        let mut builder = Client::builder()
            .base_url(config.base_url)
            .user_agent(UserAgent::new("d1v-cli", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30));

        if let Ok(Some(token)) = tokens.load() {
            builder = builder.token(token);
        }

        Ok(Self {
            client: builder.build()?,
            tokens,
            output: Output::new(format),
        })
    }

    /// Writes a status message via the output formatter.
    pub fn message(&self, msg: impl Display) {
        self.output.message(msg);
    }

    /// Writes structured data via the output formatter.
    pub fn print(&self, value: &(impl Display + Serialize)) -> Result<()> {
        self.output.print(value)
    }
}

#[derive(Parser)]
#[command(name = "d1v", version, about = "D1V CLI")]
struct Cli {
    /// Output format
    #[arg(short, long, global = true, default_value_t, env = "D1V_FORMAT")]
    format: Format,

    /// Language override
    #[arg(long, global = true)]
    lang: Option<String>,

    /// Log file path [default: ~/.d1v/d1v.log]
    #[arg(long, env = "D1V_LOG_FILE")]
    log_file: Option<std::path::PathBuf>,

    /// Save HTTP exchanges to a JSON file
    #[cfg(feature = "record")]
    #[arg(long, env = "D1V_RECORD_FILE")]
    record: Option<std::path::PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Manage authentication
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    /// Show debug information
    Debug,
}

#[derive(Subcommand)]
enum AuthCommand {
    /// Log in with email and verification code
    Login,
    /// Log out and clear stored credentials
    Logout,
}

async fn run(cli: Cli) -> Result<()> {
    #[cfg(feature = "record")]
    let _recorder = cli
        .record
        .map(|path| d1v_api::set_recorder(recorder::FileRecorder::new(path)))
        .transpose()?;

    let ctx = Context::new(cli.format)?;

    match cli.command {
        Command::Auth { command } => match command {
            AuthCommand::Login => auth::login(&ctx).await,
            AuthCommand::Logout => auth::logout(&ctx).await,
        },
        Command::Debug => debug::run(&ctx),
    }
}

fn locale_sources(cli_lang: &Option<String>) -> impl Iterator<Item = String> {
    [
        cli_lang.clone(),
        std::env::var("D1V_LANG").ok().filter(|s| !s.is_empty()),
        Config::load().ok().and_then(|c| c.language),
        sys_locale::get_locale(),
    ]
    .into_iter()
    .flatten()
}

#[tokio::main]
async fn main() {
    let mut cli = Cli::parse();
    let _log = logging::init(cli.log_file.take()).ok();
    i18n::init(locale_sources(&cli.lang));

    let output = Output::new(cli.format);

    info!(version = env!("CARGO_PKG_VERSION"), "D1V CLI");

    if let Err(err) = run(cli).await {
        error!(%err, "fatal error");
        output.error(&err);
        std::process::exit(1);
    }
}
