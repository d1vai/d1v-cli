// Stop node containers

use super::docker;
use super::StopArgs;
use crate::error::Result;
use crate::Context;

const AGENT_CONTAINER_NAME: &str = "d1v-runtime-agent-platform";
const OPCODE_API_CONTAINER_NAME: &str = "d1v-opcode-api";

pub async fn run(_ctx: &Context, args: StopArgs) -> Result<()> {
    let stop_agent = args.agent || (!args.agent && !args.opcode_api);
    let stop_opcode = args.opcode_api || (!args.agent && !args.opcode_api);

    let action = if args.force { "Killing" } else { "Stopping" };

    if stop_agent {
        eprintln!("{} runtime-agent container...", action);
        if docker::is_container_running(AGENT_CONTAINER_NAME)? {
            docker::stop_container(AGENT_CONTAINER_NAME, args.force)?;
            if args.remove {
                docker::remove_container(AGENT_CONTAINER_NAME)?;
                eprintln!("✅ Runtime-agent stopped and removed");
            } else {
                eprintln!("✅ Runtime-agent stopped");
            }
        } else {
            eprintln!("ℹ️  Runtime-agent is not running");
        }
    }

    if stop_opcode {
        eprintln!("{} opcode-api container...", action);
        if docker::is_container_running(OPCODE_API_CONTAINER_NAME)? {
            docker::stop_container(OPCODE_API_CONTAINER_NAME, args.force)?;
            if args.remove {
                docker::remove_container(OPCODE_API_CONTAINER_NAME)?;
                eprintln!("✅ Opcode-API stopped and removed");
            } else {
                eprintln!("✅ Opcode-API stopped");
            }
        } else {
            eprintln!("ℹ️  Opcode-API is not running");
        }
    }

    println!("\n✓ Node stopped successfully");
    Ok(())
}
