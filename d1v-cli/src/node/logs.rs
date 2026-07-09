// View node logs

use super::LogsArgs;
use super::docker;
use super::{AGENT_CONTAINER_NAME, OPCODE_API_CONTAINER_NAME};
use crate::Context;
use crate::error::{Error, Result};
use anyhow::anyhow;

pub async fn run(_ctx: &Context, args: LogsArgs) -> Result<()> {
    let container_name = match args.container.as_str() {
        "agent" => AGENT_CONTAINER_NAME,
        "opcode-api" | "opcode_api" => OPCODE_API_CONTAINER_NAME,
        "all" => {
            // Show both
            eprintln!("Showing logs for runtime-agent:");
            eprintln!("═══════════════════════════════════════════════════════════════\n");
            let _ = docker::get_logs(
                AGENT_CONTAINER_NAME,
                args.tail,
                false,
                args.since.as_deref(),
            );

            eprintln!("\n\nShowing logs for opcode-api:");
            eprintln!("═══════════════════════════════════════════════════════════════\n");
            docker::get_logs(
                OPCODE_API_CONTAINER_NAME,
                args.tail,
                args.follow,
                args.since.as_deref(),
            )?;
            return Ok(());
        }
        _ => {
            return Err(Error::Other(anyhow!(
                "Invalid container '{}'. Use 'agent', 'opcode-api', or 'all'",
                args.container
            )));
        }
    };

    docker::get_logs(
        container_name,
        args.tail,
        args.follow,
        args.since.as_deref(),
    )?;

    Ok(())
}
