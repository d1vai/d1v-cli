mod export;
mod get;
mod import;
mod list;
mod set;
mod sync;
mod unset;

use anyhow::anyhow;
use clap::{Args, Subcommand};
use std::fmt::Display;
use std::str::FromStr;

use crate::{Context, Result, t, workspace};

#[derive(Args)]
pub struct ProjectArgs {
    /// Project ID (defaults to D1V_PROJECT_ID or workspace binding)
    #[arg(short = 'p', long = "project", env = "D1V_PROJECT_ID")]
    pub project_id: Option<String>,
}

impl ProjectArgs {
    pub fn resolve(&self) -> Result<String> {
        if let Some(id) = self
            .project_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            return Ok(id.to_string());
        }

        if let Some(id) = workspace::resolve_bound_project_id(None)? {
            return Ok(id);
        }

        Err(anyhow!("{}", t!("project-required")).into())
    }
}

#[derive(Debug, Clone)]
pub struct KeyValue {
    key: String,
    value: String,
}

impl KeyValue {
    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

impl Display for KeyValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}={}", self.key, self.value)
    }
}

impl FromStr for KeyValue {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (key, value) = s
            .split_once('=')
            .ok_or_else(|| format!("expected KEY=value format, got: {s}"))?;

        Ok(Self {
            key: key.to_string(),
            value: value.to_string(),
        })
    }
}

#[derive(Subcommand)]
pub enum EnvCommand {
    /// List environment variables
    #[command(arg_required_else_help = true)]
    List(EnvListArgs),
    /// Get a specific variable
    #[command(arg_required_else_help = true)]
    Get(EnvGetArgs),
    /// Set variables (creates or updates)
    #[command(arg_required_else_help = true)]
    Set(EnvSetArgs),
    /// Remove a variable
    #[command(arg_required_else_help = true)]
    Unset(EnvUnsetArgs),
    /// Import variables from a .env file
    #[command(arg_required_else_help = true)]
    Import(EnvImportArgs),
    /// Export variables to a .env file
    #[command(arg_required_else_help = true)]
    Export(EnvExportArgs),
    /// Sync variables to Vercel
    #[command(arg_required_else_help = true)]
    Sync(EnvSyncArgs),
}

#[derive(Args)]
pub struct EnvListArgs {
    #[command(flatten)]
    pub project: ProjectArgs,
    /// Show plaintext values for sensitive variables
    #[arg(long)]
    pub reveal: bool,
}

#[derive(Args)]
pub struct EnvGetArgs {
    #[command(flatten)]
    pub project: ProjectArgs,
    /// Variable key name
    #[arg(required_unless_present = "id")]
    pub key: Option<String>,
    /// Get by numeric ID instead of key
    #[arg(long)]
    pub id: Option<i64>,
    /// Show plaintext value for sensitive variables
    #[arg(long)]
    pub reveal: bool,
}

#[derive(Args)]
pub struct EnvSetArgs {
    #[command(flatten)]
    pub project: ProjectArgs,
    /// KEY=value pairs
    pub vars: Vec<KeyValue>,
    /// Description for the variable(s)
    #[arg(long = "desc")]
    pub description: Option<String>,
    /// Mark variable(s) as sensitive
    #[arg(long)]
    pub sensitive: bool,
}

#[derive(Args)]
pub struct EnvUnsetArgs {
    #[command(flatten)]
    pub project: ProjectArgs,
    /// Variable key name
    #[arg(required_unless_present = "id")]
    pub key: Option<String>,
    /// Delete by numeric ID instead of key
    #[arg(long)]
    pub id: Option<i64>,
}

#[derive(Args)]
pub struct EnvImportArgs {
    #[command(flatten)]
    pub project: ProjectArgs,
    /// .env file path (defaults to stdin)
    #[arg(short = 'i', long)]
    pub input: Option<String>,
    /// Overwrite existing variables
    #[arg(long)]
    pub overwrite: bool,
}

#[derive(Args)]
pub struct EnvExportArgs {
    #[command(flatten)]
    pub project: ProjectArgs,
    /// Output file path (defaults to stdout)
    #[arg(short = 'o', long)]
    pub output: Option<String>,
}

#[derive(Args)]
pub struct EnvSyncArgs {
    #[command(flatten)]
    pub project: ProjectArgs,
    /// Confirm sync operation
    #[arg(long)]
    pub yes: bool,
}

pub async fn run(ctx: &Context, command: EnvCommand) -> Result<()> {
    match command {
        EnvCommand::List(args) => list::run(ctx, args).await,
        EnvCommand::Get(args) => get::run(ctx, args).await,
        EnvCommand::Set(args) => set::run(ctx, args).await,
        EnvCommand::Unset(args) => unset::run(ctx, args).await,
        EnvCommand::Import(args) => import::run(ctx, args).await,
        EnvCommand::Export(args) => export::run(ctx, args).await,
        EnvCommand::Sync(args) => sync::run(ctx, args).await,
    }
}
