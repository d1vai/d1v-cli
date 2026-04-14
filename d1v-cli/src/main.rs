use std::io::{stdin, IsTerminal};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use jiff::SignedDuration;
use tracing::info;

use d1v_cli::config::Config;
use d1v_cli::error::{Error, Result};
use d1v_cli::output::{format_duration, Color, Format, Output};
use d1v_cli::token::TokenLoader;
use d1v_cli::{auth, debug, i18n, logging, t, user, Context};

#[derive(Parser)]
#[command(name = "d1v", version, about = "D1V CLI")]
struct Cli {
    /// Output format
    #[arg(short, long, global = true, default_value_t, env = "D1V_FORMAT")]
    format: Format,

    /// Color output
    #[arg(long, global = true, default_value_t, env = "D1V_COLOR")]
    color: Color,

    /// Language override
    #[arg(long, global = true)]
    lang: Option<String>,

    /// Log file path [default: ~/.d1v/d1v.YYYY-MM-DD.log]
    #[arg(long, env = "D1V_LOG_FILE")]
    log_file: Option<std::path::PathBuf>,

    /// Override API base URL
    #[arg(long, global = true, env = "D1V_BASE_URL")]
    base_url: Option<String>,

    /// Increase log verbosity (-v info, -vv debug, -vvv trace)
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,

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
        #[arg(short, long, conflicts_with = "with_token")]
        password: bool,
        /// Log in with an authentication token
        #[arg(long)]
        with_token: bool,
    },
    /// Log out and clear stored credentials
    Logout,
    /// Show authentication status
    Status,
}

async fn run(cli: Cli) -> Result<()> {
    #[cfg(feature = "record")]
    let _recorder = cli
        .record
        .or_else(|| Config::load().ok()?.record.resolve_path())
        .map(|path| d1v_api::set_recorder(d1v_cli::recorder::FileRecorder::new(path)))
        .transpose()
        .map_err(anyhow::Error::from)?;

    let ctx = Context::new(cli.format, cli.color, cli.base_url)?;

    if cli.command.requires_auth() {
        if ctx.tokens.load()?.is_none() {
            return Err(Error::NotLoggedIn);
        }

        if ctx.client.is_token_expired() {
            if stdin().is_terminal() && auth::prompt_relogin(&ctx).await? {
                ctx.success(t!("auth-relogin-success"));
            } else {
                return Err(Error::TokenExpired);
            }
        }

        if let Some(claims) = ctx.client.claims()
            && let Some(remaining) = claims.expires_in()
            && remaining < SignedDuration::from_hours(24)
        {
            let duration = format_duration(remaining.as_secs());
            ctx.output
                .hint(&t!("warn-token-expiring", duration = duration));
        }
    }

    match cli.command {
        Command::Auth { command } => match command {
            AuthCommand::Login {
                password,
                with_token,
            } => {
                if with_token {
                    auth::login_with_token(&ctx)
                } else {
                    auth::login(&ctx, password).await
                }
            }
            AuthCommand::Logout => auth::logout(&ctx).await,
            AuthCommand::Status => auth::status(&ctx),
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
    let _log = logging::init(cli.log_file.take(), cli.verbose).ok();
    i18n::init(locale_sources(cli.lang.as_deref()));

    let output = Output::new(cli.format, cli.color.resolve());

    info!(version = env!("CARGO_PKG_VERSION"), "D1V CLI");

    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => err.handle(&output),
    }
}
