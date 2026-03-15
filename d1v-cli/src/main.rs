mod auth;
mod config;
mod token;

use crate::config::Config;
use crate::token::{TokenChain, TokenLoader};
use anyhow::Result;
use clap::{Parser, Subcommand};
use d1v_api::Client;
use std::sync::LazyLock;

pub static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    let config = Config::load().expect("failed to load config");

    let client =
        Client::new(reqwest::Client::new(), config.base_url).expect("invalid base URL");

    if let Ok(Some(token)) = TokenChain::default().load() {
        client.token(token);
    }

    client
});

#[derive(Parser)]
#[command(name = "d1v", version, about = "D1V CLI")]
struct Cli {
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
