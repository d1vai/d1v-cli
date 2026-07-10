use super::docker::{self, ContainerRuntimeConfig};
use super::{AGENT_CONTAINER_NAME, UpgradeArgs};
use crate::Context;
use crate::error::{Error, Result};
use anyhow::anyhow;
use reqwest::Client;
use serde_json::json;
use std::io::{self, BufRead};
use std::time::Duration;

const RUNTIME_AGENT_IMAGE: &str = "ghcr.io/d1vai/d1v-runtime-agent:latest";

fn target_image_ref(override_ref: Option<&str>) -> String {
    override_ref.unwrap_or(RUNTIME_AGENT_IMAGE).to_string()
}

fn runtime_agent_port(config: &ContainerRuntimeConfig) -> u16 {
    config
        .env
        .iter()
        .find_map(|entry| entry.strip_prefix("D1V_RUNTIME_AGENT_PORT="))
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8080)
}

fn apply_opcode_image_override(config: &mut ContainerRuntimeConfig, opcode_image: Option<&str>) {
    let Some(opcode_image) = opcode_image else {
        return;
    };
    let mut replaced = false;
    for item in &mut config.env {
        if item.starts_with("D1V_OPCODE_IMAGE=") {
            *item = format!("D1V_OPCODE_IMAGE={opcode_image}");
            replaced = true;
            break;
        }
    }
    if !replaced {
        config.env.push(format!("D1V_OPCODE_IMAGE={opcode_image}"));
    }
}

async fn wait_for_health(port: u16) -> Result<()> {
    let client = Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .map_err(|e| Error::Other(anyhow!("Failed to build HTTP client: {}", e)))?;
    let url = format!("http://127.0.0.1:{port}/health");
    for _ in 0..20 {
        match client.get(&url).send().await {
            Ok(response) if response.status().is_success() => return Ok(()),
            _ => tokio::time::sleep(Duration::from_secs(1)).await,
        }
    }
    Err(Error::Other(anyhow!(
        "Runtime-agent did not become healthy on {}",
        url
    )))
}

fn confirm(yes: bool, image_ref: &str) -> Result<bool> {
    if yes {
        return Ok(true);
    }
    eprint!("Upgrade runtime-agent to {}? [y/N] ", image_ref);
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    let answer = line.trim().to_lowercase();
    Ok(answer == "y" || answer == "yes")
}

pub async fn run(_ctx: &Context, args: UpgradeArgs) -> Result<()> {
    let image_ref = target_image_ref(args.runtime_agent_image.as_deref());
    let current = docker::inspect_container_runtime_config(AGENT_CONTAINER_NAME)?
        .ok_or_else(|| Error::Other(anyhow!("Runtime-agent container is not running")))?;
    let current_image_id = docker::local_image_id(&current.image_ref)?.ok_or_else(|| {
        Error::Other(anyhow!(
            "Current runtime-agent image is not available locally"
        ))
    })?;
    let remote_digest = docker::remote_image_digest(&image_ref)?;
    let upgrade_needed = current_image_id != remote_digest;

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "image_ref": image_ref,
                "current_image_ref": current.image_ref,
                "current_image_id": current_image_id,
                "remote_digest": remote_digest,
                "upgrade_needed": upgrade_needed,
                "check_only": args.check,
                "dry_run": args.dry_run,
            }))
            .map_err(|e| Error::Other(anyhow!("Failed to serialize JSON: {}", e)))?
        );
        if args.check || args.dry_run || !upgrade_needed {
            return Ok(());
        }
    }

    if args.check {
        if upgrade_needed {
            eprintln!("Runtime-agent update available");
        } else {
            eprintln!("Runtime-agent image is already up to date");
        }
        return Ok(());
    }

    if !upgrade_needed {
        eprintln!("Runtime-agent image is already up to date");
        return Ok(());
    }

    let mut next_config = current.clone();
    apply_opcode_image_override(&mut next_config, args.opcode_image.as_deref());
    let port = runtime_agent_port(&next_config);

    if args.dry_run {
        eprintln!(
            "Would upgrade runtime-agent from {} to {}",
            current.image_ref, image_ref
        );
        return Ok(());
    }

    if !confirm(args.yes, &image_ref)? {
        eprintln!("Aborted.");
        return Ok(());
    }

    docker::pull_image(&image_ref)?;
    docker::remove_container(AGENT_CONTAINER_NAME)?;
    if let Err(err) = docker::run_container(
        AGENT_CONTAINER_NAME,
        &image_ref,
        &next_config.env,
        &next_config.binds,
        &next_config.restart_policy_name,
        &next_config.network_mode,
    ) {
        let _ = docker::run_container(
            AGENT_CONTAINER_NAME,
            &current.image_ref,
            &current.env,
            &current.binds,
            &current.restart_policy_name,
            &current.network_mode,
        );
        return Err(err);
    }

    if let Err(err) = wait_for_health(port).await {
        let _ = docker::remove_container(AGENT_CONTAINER_NAME);
        let _ = docker::run_container(
            AGENT_CONTAINER_NAME,
            &current.image_ref,
            &current.env,
            &current.binds,
            &current.restart_policy_name,
            &current.network_mode,
        );
        return Err(err);
    }

    if !args.keep_old_images {
        let _ = docker::cleanup_old_images_for_repository(&image_ref);
    }

    eprintln!(
        "Runtime-agent upgraded successfully\n  Previous image: {}\n  Current image: {}",
        current.image_ref, image_ref
    );
    Ok(())
}
