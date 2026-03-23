mod info;

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::Context;

#[derive(Subcommand)]
pub enum UserCommand {
    /// Show current user info
    Info,
    /// Update user info
    Update(UpdateArgs),
    /// Get a public user profile
    Get {
        #[command(subcommand)]
        target: GetArgs,
    },
    /// List all users
    List,
}

#[derive(Args)]
pub struct UpdateArgs {
    /// Company name
    #[arg(long)]
    pub company_name: Option<String>,
    /// Company website
    #[arg(long)]
    pub company_website: Option<String>,
    /// Avatar URL
    #[arg(long)]
    pub picture: Option<String>,
    /// Industry
    #[arg(long)]
    pub industry: Option<String>,
    /// Whether user is a company
    #[arg(long)]
    pub is_company: Option<bool>,
    /// Referral code
    #[arg(long)]
    pub referral_code: Option<String>,
}

#[derive(Subcommand)]
pub enum GetArgs {
    /// Look up by user ID
    Id { user_id: i64 },
    /// Look up by slug
    Slug { slug: String },
}

pub async fn run(ctx: &Context, command: UserCommand) -> Result<()> {
    match command {
        UserCommand::Info => info::info(ctx).await,
        UserCommand::Update(args) => info::update(ctx, args).await,
        UserCommand::Get { target } => info::get(ctx, target).await,
        UserCommand::List => info::list(ctx).await,
    }
}
