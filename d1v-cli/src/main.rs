mod auth;
mod config;
mod debug;
mod error;
mod i18n;
mod logging;
mod output;
mod prompt;
#[cfg(feature = "record")]
mod recorder;
mod token;
mod user;

use std::fmt::Display;
use std::process::ExitCode;
use std::time::Duration;

use anyhow::Result;
use clap::{Parser, Subcommand};
use d1v_api::{Client, UserAgent};
use serde::Serialize;
use tracing::info;

use crate::config::Config;
use crate::error::{handle_error, CliError};
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

    /// Writes a list of structured data via the output formatter.
    pub fn print_list(
        &self,
        values: impl IntoIterator<Item = impl Display + Serialize>,
    ) -> Result<()> {
        self.output.print_list(values)
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
    /// Manage user account
    User {
        #[command(subcommand)]
        command: user::UserCommand,
    },
    /// Show debug information
    Debug,
}

impl Command {
    fn requires_auth(&self) -> bool {
        match self {
            Command::Auth { .. } | Command::Debug => false,
            Command::User { command } => command.requires_auth(),
        }
    }
}

#[derive(Subcommand)]
enum AuthCommand {
    /// Log in with email and verification code
    Login {
        /// Use password instead of verification code
        #[arg(short, long)]
        password: bool,
    },
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

    if cli.command.requires_auth() && ctx.tokens.load()?.is_none() {
        return Err(CliError::NotLoggedIn.into());
    }

    match cli.command {
        Command::Auth { command } => match command {
            AuthCommand::Login { password } => auth::login(&ctx, password).await,
            AuthCommand::Logout => auth::logout(&ctx).await,
        },
        Command::User { command } => user::run(&ctx, command).await,
        Command::Debug => debug::run(&ctx),
    }
}

fn locale_sources(cli_lang: Option<&str>) -> impl Iterator<Item = String> {
    [
        cli_lang.map(ToOwned::to_owned),
        std::env::var("D1V_LANG").ok().filter(|s| !s.is_empty()),
        Config::load().ok().and_then(|c| c.language),
        sys_locale::get_locale(),
    ]
    .into_iter()
    .flatten()
}

#[tokio::main]
async fn main() -> ExitCode {
    let mut cli = Cli::parse();
    let _log = logging::init(cli.log_file.take()).ok();
    i18n::init(locale_sources(cli.lang.as_deref()));

    let output = Output::new(cli.format);

    info!(version = env!("CARGO_PKG_VERSION"), "D1V CLI");

    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => handle_error(&output, err),
    }
}
