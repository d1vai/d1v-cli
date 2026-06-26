use clap::{Args, Subcommand};

use crate::Context;
use crate::error::Result;
use crate::expose::{self, ExposeArgs, ExposeCloseArgs, ExposeCommand, ExposeListArgs};

#[derive(Subcommand)]
pub enum NodeCommand {
    /// Manage node-backed public ingress bindings
    Expose(NodeExposeArgs),
}

#[derive(Args)]
pub struct NodeExposeArgs {
    /// Expose a local port through the active runtime agent
    pub port: Option<u16>,
    #[command(subcommand)]
    pub command: Option<NodeExposeSubcommand>,
    #[arg(long)]
    pub project_id: Option<String>,
    #[arg(long)]
    pub hostname: Option<String>,
    #[arg(long)]
    pub node_id: Option<String>,
    #[arg(long)]
    pub host_port: Option<u16>,
}

#[derive(Subcommand)]
pub enum NodeExposeSubcommand {
    /// List active node expose bindings
    List(ExposeListArgs),
    /// Close a node expose binding
    Close(ExposeCloseArgs),
}

pub async fn run(ctx: &Context, command: NodeCommand) -> Result<()> {
    match command {
        NodeCommand::Expose(args) => {
            let command = args.command.map(|value| match value {
                NodeExposeSubcommand::List(list_args) => ExposeCommand::List(list_args),
                NodeExposeSubcommand::Close(close_args) => ExposeCommand::Close(close_args),
            });
            expose::run_node_mode(
                ctx,
                ExposeArgs {
                    port: args.port,
                    command,
                    project_id: args.project_id,
                    hostname: args.hostname,
                    node_id: args.node_id,
                    host_port: args.host_port,
                },
            )
            .await
        }
    }
}
