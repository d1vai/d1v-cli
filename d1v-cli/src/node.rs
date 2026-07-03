use clap::{Args, Subcommand};

use crate::Context;
use crate::error::Result;
use crate::expose::{self, ExposeArgs, ExposeCloseArgs, ExposeCommand, ExposeListArgs};

mod docker;
mod start;
mod status;
mod stop;
mod logs;

#[derive(Subcommand)]
pub enum NodeCommand {
    /// Start runtime-agent and opcode-api containers
    Start(StartArgs),
    /// Stop running node containers
    Stop(StopArgs),
    /// Show node status and resource usage
    Status(StatusArgs),
    /// View node logs
    Logs(LogsArgs),
    /// Manage node-backed public ingress bindings
    Expose(NodeExposeArgs),
}

#[derive(Args)]
pub struct StartArgs {
    /// Platform node key (or use D1V_PLATFORM_NODE_KEY env var)
    #[arg(long, env = "D1V_PLATFORM_NODE_KEY")]
    pub key: Option<String>,

    /// Control plane URL
    #[arg(long, default_value = "https://api.d1v.ai/api/runtime/fabric")]
    pub control_plane: String,

    /// Node ID (defaults to hostname)
    #[arg(long)]
    pub node_id: Option<String>,

    /// Maximum number of opcode-api containers
    #[arg(long, default_value = "10")]
    pub max_opcode_containers: u32,

    /// Runtime agent HTTP port
    #[arg(long, default_value = "8080")]
    pub agent_port: u16,

    /// Runtime agent WebSocket port
    #[arg(long, default_value = "8081")]
    pub agent_ws_port: u16,

    /// Opcode-API port
    #[arg(long, default_value = "8090")]
    pub opcode_api_port: u16,

    /// Workspace root directory
    #[arg(long, default_value = "/var/lib/d1v-runtime/workspaces")]
    pub workspace_root: String,

    /// Override runtime-agent Docker image (default: pulls from ECR)
    #[arg(long, env = "D1V_RUNTIME_AGENT_IMAGE")]
    pub runtime_agent_image: Option<String>,

    /// Override opcode Docker image passed to runtime-agent
    #[arg(long, env = "D1V_OPCODE_IMAGE")]
    pub opcode_image: Option<String>,

    /// Skip image pull (use locally cached images)
    #[arg(long)]
    pub skip_pull: bool,

    /// Skip Docker installation check
    #[arg(long)]
    pub skip_docker_check: bool,

    /// Skip resource usage confirmation
    #[arg(long, short = 'y')]
    pub yes: bool,
}

#[derive(Args)]
pub struct StopArgs {
    /// Stop runtime-agent container
    #[arg(long)]
    pub agent: bool,

    /// Stop opcode-api container
    #[arg(long)]
    pub opcode_api: bool,

    /// Force stop (docker kill)
    #[arg(long, short = 'f')]
    pub force: bool,

    /// Remove containers after stopping
    #[arg(long)]
    pub remove: bool,
}

#[derive(Args)]
pub struct StatusArgs {
    /// Output in JSON format
    #[arg(long)]
    pub json: bool,

    /// Watch mode (auto refresh every N seconds)
    #[arg(long)]
    pub watch: Option<u64>,
}

#[derive(Args)]
pub struct LogsArgs {
    /// Container to show logs for (agent|opcode-api|all)
    #[arg(default_value = "agent")]
    pub container: String,

    /// Follow log output
    #[arg(long, short = 'f')]
    pub follow: bool,

    /// Number of lines to show
    #[arg(long, default_value = "100")]
    pub tail: u32,

    /// Show logs since timestamp (e.g., "1h", "30m")
    #[arg(long)]
    pub since: Option<String>,
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
        NodeCommand::Start(args) => start::run(ctx, args).await,
        NodeCommand::Stop(args) => stop::run(ctx, args).await,
        NodeCommand::Status(args) => status::run(ctx, args).await,
        NodeCommand::Logs(args) => logs::run(ctx, args).await,
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
