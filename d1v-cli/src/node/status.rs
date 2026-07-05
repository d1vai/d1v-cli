// Show node status

use super::StatusArgs;
use super::docker;
use crate::Context;
use crate::error::{Error, Result};
use serde_json::json;

const AGENT_CONTAINER_NAME: &str = "d1v-runtime-agent-platform";
const OPCODE_API_CONTAINER_NAME: &str = "d1v-opcode-api";

pub async fn run(_ctx: &Context, args: StatusArgs) -> Result<()> {
    if let Some(interval) = args.watch {
        // Watch mode
        loop {
            print!("\x1B[2J\x1B[1;1H"); // Clear screen
            show_status(args.json)?;
            std::thread::sleep(std::time::Duration::from_secs(interval));
        }
    } else {
        show_status(args.json)?;
    }

    Ok(())
}

fn show_status(json_output: bool) -> Result<()> {
    let agent_info = docker::get_container_info(AGENT_CONTAINER_NAME)?;
    let opcode_info = docker::get_container_info(OPCODE_API_CONTAINER_NAME)?;

    if json_output {
        let output = json!({
            "runtime_agent": container_to_json(&agent_info),
            "opcode_api": container_to_json(&opcode_info),
        });
        let json_str = serde_json::to_string_pretty(&output)
            .map_err(|e| Error::Other(anyhow::anyhow!("Failed to serialize JSON: {}", e)))?;
        println!("{}", json_str);
        return Ok(());
    }

    // Text output
    println!("D1V Platform Node Status");
    println!("═══════════════════════════════════════════════════════════════");
    println!();

    // Runtime Agent
    println!("Runtime Agent:");
    if let Some(info) = &agent_info {
        println!("  Container:  {}", info.name);
        println!("  Status:     {}", format_status(&info.status));
        println!("  Image:      {}", info.image);
        println!(
            "  Ports:      {}",
            if info.ports.is_empty() {
                "host network"
            } else {
                &info.ports
            }
        );

        // Get stats if running
        if info.status.starts_with("Up") {
            if let Ok(Some(stats)) = docker::get_container_stats(&info.name) {
                println!("  CPU:        {}", stats.cpu_percent);
                println!(
                    "  Memory:     {} / {} ({})",
                    stats.mem_usage, stats.mem_limit, stats.mem_percent
                );
            }
        }
    } else {
        println!("  Status:     ❌ Not found");
    }
    println!();

    // Opcode API
    println!("Opcode API:");
    if let Some(info) = &opcode_info {
        println!("  Container:  {}", info.name);
        println!("  Status:     {}", format_status(&info.status));
        println!("  Image:      {}", info.image);
        println!("  Ports:      {}", info.ports);

        // Get stats if running
        if info.status.starts_with("Up") {
            if let Ok(Some(stats)) = docker::get_container_stats(&info.name) {
                println!("  CPU:        {}", stats.cpu_percent);
                println!(
                    "  Memory:     {} / {} ({})",
                    stats.mem_usage, stats.mem_limit, stats.mem_percent
                );
            }
        }
    } else {
        println!("  Status:     ℹ️  Not found (optional)");
    }
    println!();

    // Overall status
    let agent_running = agent_info
        .as_ref()
        .map(|i| i.status.starts_with("Up"))
        .unwrap_or(false);

    if agent_running {
        println!("Overall Status: ✅ RUNNING");
    } else {
        println!("Overall Status: ❌ STOPPED");
    }

    Ok(())
}

fn format_status(status: &str) -> String {
    if status.starts_with("Up") {
        format!("✅ {}", status)
    } else if status.starts_with("Exited") {
        format!("❌ {}", status)
    } else {
        format!("⚠️  {}", status)
    }
}

fn container_to_json(info: &Option<docker::ContainerInfo>) -> serde_json::Value {
    match info {
        Some(info) => json!({
            "name": info.name,
            "id": info.id,
            "status": info.status,
            "image": info.image,
            "ports": info.ports,
            "running": info.status.starts_with("Up"),
        }),
        None => json!({
            "status": "not_found",
            "running": false,
        }),
    }
}
