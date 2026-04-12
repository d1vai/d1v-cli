mod activity;
mod email;
mod info;
mod invitation;
mod password;

use crate::error::Result;
use clap::{Args, Subcommand};
use tracing::debug;

use crate::{t, Context};

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
    /// Password management
    Password {
        #[command(subcommand)]
        command: PasswordCommand,
    },
    /// Email management
    Email {
        #[command(subcommand)]
        command: EmailCommand,
    },
    /// Invitation management
    Invitation {
        #[command(subcommand)]
        command: InvitationCommand,
    },
    /// Mark onboarding as complete
    Onboard,
    /// View prompt daily activity
    Activity(ActivityArgs),
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

#[derive(Subcommand)]
pub enum PasswordCommand {
    /// Set a password
    Set,
    /// Reset password
    Reset,
}

#[derive(Subcommand)]
pub enum EmailCommand {
    /// Bind an email address
    Bind,
    /// Change email address
    Change,
}

#[derive(Subcommand)]
pub enum InvitationCommand {
    /// Accept an invitation
    Accept {
        /// Invitation code
        invite_code: String,
    },
    /// List invited users
    List,
}

#[derive(Args)]
pub struct ActivityArgs {
    /// Number of days to include (max 365)
    #[arg(long)]
    pub days: Option<i32>,
    #[command(subcommand)]
    pub target: Option<ActivityTarget>,
}

#[derive(Subcommand)]
pub enum ActivityTarget {
    /// Look up by user ID
    User { user_id: i64 },
    /// Look up by slug
    Slug { slug: String },
}

impl UserCommand {
    pub fn requires_auth(&self) -> bool {
        match self {
            Self::Get { .. } => false,
            Self::Password {
                command: PasswordCommand::Reset,
            } => false,
            Self::Activity(args) => args.target.is_none(),
            _ => true,
        }
    }
}

pub async fn run(ctx: &Context, command: UserCommand) -> Result<()> {
    match command {
        UserCommand::Info => info::info(ctx).await,
        UserCommand::Update(args) => info::update(ctx, args).await,
        UserCommand::Get { target } => info::get(ctx, target).await,
        UserCommand::List => info::list(ctx).await,
        UserCommand::Password { command } => match command {
            PasswordCommand::Set => password::set(ctx).await,
            PasswordCommand::Reset => password::reset(ctx).await,
        },
        UserCommand::Email { command } => match command {
            EmailCommand::Bind => email::bind(ctx).await,
            EmailCommand::Change => email::change(ctx).await,
        },
        UserCommand::Invitation { command } => match command {
            InvitationCommand::Accept { invite_code } => {
                invitation::accept(ctx, &invite_code).await
            }
            InvitationCommand::List => invitation::list(ctx).await,
        },
        UserCommand::Onboard => {
            debug!("marking user as onboarded");
            ctx.client.user().set_onboarded(true).await?;
            ctx.success(t!("onboard-success"));
            Ok(())
        }
        UserCommand::Activity(args) => activity::run(ctx, args).await,
    }
}
