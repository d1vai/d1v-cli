use clap::{Args, Subcommand};

use crate::Context;
use crate::error::Result;
use crate::expose::{self, ExposeArgs, ExposeCloseArgs, ExposeCommand, ExposeListArgs};

const AGENT_CONTAINER_NAME: &str = "d1v-runtime-agent-platform";
const OPCODE_API_CONTAINER_NAME: &str = "d1v-opcode-api";

mod docker;
mod image;
pub(crate) mod ingress;
mod ingress_cmd;
mod ip;
mod logs;
mod start;
mod status;
mod stop;
mod upgrade;

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
    /// Detect public IP address of this node
    Ip(IpArgs),
    /// Manage node container images
    Image {
        #[command(subcommand)]
        command: ImageCommand,
    },
    /// Upgrade runtime-agent using the current container configuration
    Upgrade(UpgradeArgs),
    /// Detect or configure reverse-proxy ingress
    Ingress {
        #[command(subcommand)]
        command: IngressCommand,
    },
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

    /// Runtime agent control origin (e.g. https://my-node-node.d1v.dev)
    #[arg(long)]
    pub control_origin: Option<String>,

    /// Auto-detect an existing reverse-proxy public ingress for the runtime agent
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    pub auto_detect_public_ingress: bool,

    /// Restrict auto-detection to a specific provider (caddy|nginx|traefik|npm)
    #[arg(long)]
    pub ingress_provider: Option<String>,

    /// Hint the detector to prefer a specific public hostname
    #[arg(long)]
    pub public_hostname: Option<String>,

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

#[derive(Args)]
pub struct IpArgs {
    /// Output in JSON format
    #[arg(long)]
    pub json: bool,
}

#[derive(Subcommand)]
pub enum ImageCommand {
    /// Show local and running image details
    Status(ImageStatusArgs),
    /// Check whether a newer runtime-agent image is available
    Check(ImageCheckArgs),
    /// Pull the configured image when it is missing or explicitly requested
    Pull(ImagePullArgs),
    /// Remove old images that are not in use by any container
    Prune(ImagePruneArgs),
}

#[derive(Args)]
pub struct ImageStatusArgs {
    /// Output in JSON format
    #[arg(long)]
    pub json: bool,

    /// Override runtime-agent Docker image reference
    #[arg(long, env = "D1V_RUNTIME_AGENT_IMAGE")]
    pub runtime_agent_image: Option<String>,
}

#[derive(Args)]
pub struct ImageCheckArgs {
    /// Output in JSON format
    #[arg(long)]
    pub json: bool,

    /// Override runtime-agent Docker image reference
    #[arg(long, env = "D1V_RUNTIME_AGENT_IMAGE")]
    pub runtime_agent_image: Option<String>,
}

#[derive(Args)]
pub struct ImagePullArgs {
    /// Output in JSON format
    #[arg(long)]
    pub json: bool,

    /// Override runtime-agent Docker image reference
    #[arg(long, env = "D1V_RUNTIME_AGENT_IMAGE")]
    pub runtime_agent_image: Option<String>,

    /// Pull even when a local copy already exists
    #[arg(long)]
    pub force: bool,
}

#[derive(Args)]
pub struct ImagePruneArgs {
    /// Output in JSON format
    #[arg(long)]
    pub json: bool,

    /// Override runtime-agent Docker image reference
    #[arg(long, env = "D1V_RUNTIME_AGENT_IMAGE")]
    pub runtime_agent_image: Option<String>,
}

#[derive(Args)]
pub struct UpgradeArgs {
    /// Output in JSON format
    #[arg(long)]
    pub json: bool,

    /// Only check whether an upgrade is needed
    #[arg(long)]
    pub check: bool,

    /// Print the plan without changing the container
    #[arg(long)]
    pub dry_run: bool,

    /// Skip confirmation prompt
    #[arg(long, short = 'y')]
    pub yes: bool,

    /// Override runtime-agent Docker image reference
    #[arg(long, env = "D1V_RUNTIME_AGENT_IMAGE")]
    pub runtime_agent_image: Option<String>,

    /// Override opcode image that should be persisted into the recreated runtime-agent env
    #[arg(long, env = "D1V_OPCODE_IMAGE")]
    pub opcode_image: Option<String>,

    /// Keep old images instead of pruning them after a successful upgrade
    #[arg(long)]
    pub keep_old_images: bool,
}

#[derive(Subcommand)]
pub enum IngressCommand {
    /// Detect existing reverse-proxy ingress configuration
    Detect(IngressDetectArgs),
    /// Configure a new reverse-proxy ingress for the given hostname
    Configure(IngressConfigureArgs),
}

#[derive(Args)]
pub struct IngressDetectArgs {
    /// Runtime agent HTTP port to look for in proxy configs
    #[arg(long, default_value = "8080")]
    pub agent_port: u16,

    /// Restrict detection to a specific provider (caddy|nginx|traefik|npm)
    #[arg(long)]
    pub provider: Option<String>,

    /// Prefer a specific public hostname from detected candidates
    #[arg(long)]
    pub hostname: Option<String>,

    /// Output in JSON format
    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
pub struct IngressConfigureArgs {
    /// Public hostname to bind (e.g. node.example.com)
    pub hostname: String,

    /// Runtime agent HTTP port to proxy to
    #[arg(long, default_value = "8080")]
    pub agent_port: u16,

    /// Ingress provider to use (nginx|npm); auto-detected if omitted
    #[arg(long)]
    pub provider: Option<String>,

    /// Output in JSON format
    #[arg(long)]
    pub json: bool,
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
        NodeCommand::Ip(args) => ip::run(ctx, args).await,
        NodeCommand::Image { command } => image::run(ctx, command).await,
        NodeCommand::Upgrade(args) => upgrade::run(ctx, args).await,
        NodeCommand::Ingress { command } => ingress_cmd::run(ctx, command).await,
    }
}
