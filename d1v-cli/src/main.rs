mod auth;
mod config;
#[cfg(feature = "record")]
mod recorder;
mod token;

use crate::config::Config;
use crate::token::{TokenChain, TokenLoader};
use anyhow::Result;
use clap::{Parser, Subcommand};
use d1v_api::Client;
use std::sync::LazyLock;

pub static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    let config = Config::load().expect("failed to load config");

    let mut client = Client::builder().base_url(config.base_url);

    if let Ok(Some(token)) = TokenChain::default().load() {
        client = client.token(token);
    }

    client.build().expect("invalid base URL")
});

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
    if let Some(path) = cli.record {
        let recorder = recorder::FileRecorder::new(path);
        d1v_api::set_recorder(recorder).ok();
    }

    match cli.command {
        Command::Auth { command } => match command {
            AuthCommand::Login => auth::login().await,
            AuthCommand::Logout => auth::logout().await,
        },
    }
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("Error: {err:#}");
        std::process::exit(1);
    }
}
