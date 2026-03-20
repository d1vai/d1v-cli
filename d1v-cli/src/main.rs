mod auth;
mod config;
mod debug;
#[cfg(feature = "record")]
mod recorder;
mod token;

use std::time::Duration;

use anyhow::Result;
use clap::{Parser, Subcommand};
use d1v_api::{Client, UserAgent};

use crate::config::Config;
use crate::token::{TokenChain, TokenLoader};

pub struct Context {
    pub client: Client,
    pub tokens: TokenChain,
}

impl Context {
    fn new() -> Result<Self> {
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
        })
    }
}

#[derive(Parser)]
#[command(name = "d1v", version, about = "D1V CLI")]
struct Cli {
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

async fn run() -> Result<()> {
    let cli = Cli::parse();

    #[cfg(feature = "record")]
    let _recorder = cli.record.map(|path| {
        d1v_api::set_recorder(recorder::FileRecorder::new(path)).expect("recorder already set")
    });

    let ctx = Context::new()?;

    match cli.command {
        Command::Auth { command } => match command {
            AuthCommand::Login => auth::login(&ctx).await,
            AuthCommand::Logout => auth::logout(&ctx).await,
        },
        Command::Debug => debug::run(),
    }
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("Error: {err:#}");
        std::process::exit(1);
    }
}
