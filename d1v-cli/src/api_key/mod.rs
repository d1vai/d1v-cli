mod create;
mod get;
mod list;
mod revoke;

use clap::{Args, Subcommand};

use crate::{Context, Result};

#[derive(Subcommand)]
pub enum ApiKeyCommand {
    /// List API keys
    #[command(arg_required_else_help = true)]
    List,
    /// Get a specific API key
    #[command(arg_required_else_help = true)]
    Get(ApiKeyGetArgs),
    /// Create a new API key
    #[command(arg_required_else_help = true)]
    Create(ApiKeyCreateArgs),
    /// Revoke an API key
    #[command(arg_required_else_help = true)]
    Revoke(ApiKeyRevokeArgs),
}

#[derive(Args)]
pub struct ApiKeyGetArgs {
    /// API key name
    #[arg(required_unless_present = "id")]
    pub name: Option<String>,
    /// Get by numeric ID instead of name
    #[arg(long)]
    pub id: Option<i64>,
}

#[derive(Args)]
pub struct ApiKeyCreateArgs {
    /// API key name (prompts interactively if omitted)
    pub name: Option<String>,
    /// Description for the API key
    #[arg(long = "desc")]
    pub description: Option<String>,
}

#[derive(Args)]
pub struct ApiKeyRevokeArgs {
    /// API key name
    #[arg(required_unless_present = "id")]
    pub name: Option<String>,
    /// Revoke by numeric ID instead of name
    #[arg(long)]
    pub id: Option<i64>,
    /// Skip confirmation prompt
    #[arg(long)]
    pub yes: bool,
}

pub async fn run(ctx: &Context, command: ApiKeyCommand) -> Result<()> {
    match command {
        ApiKeyCommand::List => list::run(ctx).await,
        ApiKeyCommand::Get(args) => get::run(ctx, args).await,
        ApiKeyCommand::Create(args) => create::run(ctx, args).await,
        ApiKeyCommand::Revoke(args) => revoke::run(ctx, args).await,
    }
}
