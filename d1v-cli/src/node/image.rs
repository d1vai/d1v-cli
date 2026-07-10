use super::docker;
use super::{
    AGENT_CONTAINER_NAME, ImageCheckArgs, ImageCommand, ImagePruneArgs, ImagePullArgs,
    ImageStatusArgs,
};
use crate::Context;
use crate::error::{Error, Result};
use serde_json::json;

const RUNTIME_AGENT_IMAGE: &str = "ghcr.io/d1vai/d1v-runtime-agent:latest";

#[derive(Debug)]
struct RuntimeAgentImageState {
    image_ref: String,
    running_container_name: Option<String>,
    running_image_ref: Option<String>,
    local_image_id: Option<String>,
    remote_digest: Option<String>,
}

fn target_image_ref(override_ref: Option<&str>) -> String {
    override_ref.unwrap_or(RUNTIME_AGENT_IMAGE).to_string()
}

fn collect_image_state(target_image: &str) -> Result<RuntimeAgentImageState> {
    let container = docker::get_container_info(AGENT_CONTAINER_NAME)?;
    let running_container_name = container.as_ref().map(|info| info.name.clone());
    let running_image_ref = container.as_ref().map(|info| info.image.clone());
    let local_image_id = docker::local_image_id(target_image)?;
    let remote_digest = docker::remote_image_digest(target_image).ok();
    Ok(RuntimeAgentImageState {
        image_ref: target_image.to_string(),
        running_container_name,
        running_image_ref,
        local_image_id,
        remote_digest,
    })
}

fn update_available(state: &RuntimeAgentImageState) -> Option<bool> {
    match (&state.local_image_id, &state.remote_digest) {
        (Some(local), Some(remote)) => Some(local != remote),
        _ => None,
    }
}

pub async fn run(_ctx: &Context, command: ImageCommand) -> Result<()> {
    match command {
        ImageCommand::Status(args) => status(args).await,
        ImageCommand::Check(args) => check(args).await,
        ImageCommand::Pull(args) => pull(args).await,
        ImageCommand::Prune(args) => prune(args).await,
    }
}

async fn status(args: ImageStatusArgs) -> Result<()> {
    let image_ref = target_image_ref(args.runtime_agent_image.as_deref());
    let state = collect_image_state(&image_ref)?;
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "image_ref": state.image_ref,
                "running_container_name": state.running_container_name,
                "running_image_ref": state.running_image_ref,
                "local_image_id": state.local_image_id,
                "remote_digest": state.remote_digest,
                "update_available": update_available(&state),
            }))
            .map_err(|e| Error::Other(anyhow::anyhow!("Failed to serialize JSON: {}", e)))?
        );
        return Ok(());
    }

    eprintln!("Runtime-agent image status");
    eprintln!("  Target image:      {}", state.image_ref);
    eprintln!(
        "  Running container: {}",
        state
            .running_container_name
            .clone()
            .unwrap_or_else(|| "-".to_string())
    );
    eprintln!(
        "  Running image:     {}",
        state
            .running_image_ref
            .clone()
            .unwrap_or_else(|| "-".to_string())
    );
    eprintln!(
        "  Local image id:    {}",
        state
            .local_image_id
            .clone()
            .unwrap_or_else(|| "-".to_string())
    );
    eprintln!(
        "  Remote digest:     {}",
        state
            .remote_digest
            .clone()
            .unwrap_or_else(|| "-".to_string())
    );
    match update_available(&state) {
        Some(true) => eprintln!("  Update available:  yes"),
        Some(false) => eprintln!("  Update available:  no"),
        None => eprintln!("  Update available:  unknown"),
    }
    Ok(())
}

async fn check(args: ImageCheckArgs) -> Result<()> {
    let image_ref = target_image_ref(args.runtime_agent_image.as_deref());
    let state = collect_image_state(&image_ref)?;
    let available = update_available(&state);
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "image_ref": state.image_ref,
                "local_image_id": state.local_image_id,
                "remote_digest": state.remote_digest,
                "update_available": available,
            }))
            .map_err(|e| Error::Other(anyhow::anyhow!("Failed to serialize JSON: {}", e)))?
        );
        return Ok(());
    }

    match available {
        Some(true) => eprintln!("Runtime-agent update available for {}", image_ref),
        Some(false) => eprintln!("Runtime-agent image is up to date"),
        None => eprintln!("Unable to determine whether {} is up to date", image_ref),
    }
    Ok(())
}

async fn pull(args: ImagePullArgs) -> Result<()> {
    let image_ref = target_image_ref(args.runtime_agent_image.as_deref());
    if args.force {
        docker::pull_image(&image_ref)?;
    } else {
        docker::pull_or_use_latest(&image_ref, false)?;
    }
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "image_ref": image_ref,
                "local_image_id": docker::local_image_id(&image_ref)?,
                "pulled": true,
                "forced": args.force,
            }))
            .map_err(|e| Error::Other(anyhow::anyhow!("Failed to serialize JSON: {}", e)))?
        );
        return Ok(());
    }
    eprintln!("Runtime-agent image ready: {}", image_ref);
    Ok(())
}

async fn prune(args: ImagePruneArgs) -> Result<()> {
    let image_ref = target_image_ref(args.runtime_agent_image.as_deref());
    let removed = docker::cleanup_old_images_for_repository(&image_ref)?;
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "image_ref": image_ref,
                "removed_image_ids": removed,
            }))
            .map_err(|e| Error::Other(anyhow::anyhow!("Failed to serialize JSON: {}", e)))?
        );
        return Ok(());
    }
    eprintln!("Removed {} old image(s) for {}", removed.len(), image_ref);
    Ok(())
}
