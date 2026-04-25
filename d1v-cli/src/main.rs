use std::io::{stdin, IsTerminal};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use jiff::SignedDuration;
use tracing::info;

use d1v_cli::banner::Banner;
use d1v_cli::config::cmd::ConfigKey;
use d1v_cli::config::Config;
use d1v_cli::error::{Error, Result};
use d1v_cli::output::{format_duration, Color, Format, Output};
use d1v_cli::token::TokenLoader;
use d1v_cli::{auth, config, debug, i18n, logging, t, user, Context};

#[derive(Parser)]
#[command(name = "d1v", version, before_help = banner())]
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

    /// Save HTTP exchanges to a JSON file [default: ~/.d1v/recordings/{date}.json]
    #[cfg(feature = "record")]
    #[arg(long, value_name = "FILE", num_args = 0..=1, env = "D1V_RECORD_FILE")]
    record: Option<Option<std::path::PathBuf>>,

    #[command(subcommand)]
    command: Command,
}

fn banner() -> String {
    Banner::new()
        .padding_top("\n\n\n")
        .padding_bottom("")
        .render()
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
    /// Manage CLI configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Show debug information
    Debug,
    /// Print the ASCII art banner
    Banner,
}

impl Command {
    fn requires_auth(&self) -> bool {
        match self {
            Command::Auth { .. } | Command::Config { .. } | Command::Debug | Command::Banner => {
                false
            }
            Command::User { command } => command.requires_auth(),
        }
    }
}

#[derive(Subcommand)]
enum ConfigCommand {
    /// Show current configuration
    Show,
    /// Get a config value
    Get {
        /// Config key
        key: ConfigKey,
    },
    /// Set a config value
    Set {
        /// Config key
        key: ConfigKey,
        /// New value (empty string clears optional fields)
        value: String,
    },
    /// List available config keys
    List,
    /// Print config file path
    Path,
    /// Reset configuration to defaults
    Reset,
    /// Open config file in editor
    Edit,
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
    let _recorder = {
        use d1v_cli::config::record_path;
        use d1v_cli::recorder::FileRecorder;

        let path = match cli.record {
            Some(Some(path)) => Some(path),
            Some(None) => {
                let dir = Config::load().ok().and_then(|c| c.record.dir);
                Some(record_path(dir.as_deref())?)
            }
            None => Config::load().ok().and_then(|c| c.record.resolve_path()),
        };

        path.map(|path| d1v_api::set_recorder(FileRecorder::new(path)))
            .transpose()
            .map_err(anyhow::Error::from)?
    };

    let ctx = Context::new(cli.format, cli.color, cli.base_url)?;

    if cli.command.requires_auth() {
        if ctx.tokens.load()?.is_none() {
            return Err(Error::NotLoggedIn);
        }

        if ctx.client.is_token_expired() {
            if stdin().is_terminal() {
                match auth::prompt_relogin(&ctx).await {
                    Ok(true) => ctx.success(t!("auth-relogin-success")),
                    Ok(false) => return Ok(()),
                    Err(err) => return Err(err),
                }
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
                } else if password {
                    auth::login(&ctx, true).await
                } else if stdin().is_terminal() {
                    auth::login_interactive(&ctx).await
                } else {
                    auth::login(&ctx, false).await
                }
            }
            AuthCommand::Logout => auth::logout(&ctx),
            AuthCommand::Status => auth::status(&ctx),
        },
        Command::User { command } => user::run(&ctx, command).await,
        Command::Config { command } => match command {
            ConfigCommand::Show => config::cmd::show(&ctx),
            ConfigCommand::Get { key } => config::cmd::get(&ctx, key),
            ConfigCommand::Set { key, value } => config::cmd::set(&ctx, key, &value),
            ConfigCommand::List => config::cmd::list(&ctx),
            ConfigCommand::Path => config::cmd::path(&ctx),
            ConfigCommand::Reset => config::cmd::reset(&ctx),
            ConfigCommand::Edit => config::cmd::edit(),
        },
        Command::Debug => debug::run(&ctx),
        Command::Banner => {
            print!("{}", Banner::new().render());
            Ok(())
        }
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
    d1v_cli::theme::ansi::set_override(output.color);

    info!(version = env!("CARGO_PKG_VERSION"), "D1V CLI");

    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => err.handle(&output),
    }
}
