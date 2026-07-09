// Stop node containers

use super::StopArgs;
use super::docker;
use super::{AGENT_CONTAINER_NAME, OPCODE_API_CONTAINER_NAME};
use crate::Context;
use crate::error::Result;

pub async fn run(_ctx: &Context, args: StopArgs) -> Result<()> {
    let stop_agent = args.agent || (!args.agent && !args.opcode_api);
    let stop_opcode = args.opcode_api || (!args.agent && !args.opcode_api);

    if stop_agent {
        stop_one_container(AGENT_CONTAINER_NAME, "Runtime-agent", args.force, args.remove)?;
    }
    if stop_opcode {
        stop_one_container(OPCODE_API_CONTAINER_NAME, "Opcode-API", args.force, args.remove)?;
    }

    println!("\n✓ Node stopped successfully");
    Ok(())
}

fn stop_one_container(name: &str, label: &str, force: bool, remove: bool) -> Result<()> {
    let action = if force { "Killing" } else { "Stopping" };
    eprintln!("{} {} container...", action, label);
    if docker::is_container_running(name)? {
        docker::stop_container(name, force)?;
        if remove {
            docker::remove_container(name)?;
            eprintln!("✅ {} stopped and removed", label);
        } else {
            eprintln!("✅ {} stopped", label);
        }
    } else {
        eprintln!("ℹ️  {} is not running", label);
    }
    Ok(())
}
