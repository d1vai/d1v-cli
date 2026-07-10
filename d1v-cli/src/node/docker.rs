// Docker utility functions for node management

use crate::error::{Error, Result};
use anyhow::anyhow;
use serde_json::Value;
use std::collections::HashSet;
use std::process::{Command, Stdio};

#[derive(Debug)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub image: String,
    pub ports: String,
}

#[derive(Debug)]
pub struct ContainerStats {
    pub cpu_percent: String,
    pub mem_usage: String,
    pub mem_limit: String,
    pub mem_percent: String,
}

#[derive(Debug, Clone)]
pub struct ContainerRuntimeConfig {
    pub image_ref: String,
    pub env: Vec<String>,
    pub binds: Vec<String>,
    pub restart_policy_name: String,
    pub network_mode: String,
}

fn docker_status(args: &[&str], action: &str) -> Result<()> {
    let status = Command::new("docker")
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| Error::Other(anyhow!("Failed to {}: {}", action, e)))?;
    if !status.success() {
        return Err(Error::Other(anyhow!("Failed to {}", action)));
    }
    Ok(())
}

fn docker_output(args: &[&str], action: &str) -> Result<std::process::Output> {
    Command::new("docker")
        .args(args)
        .output()
        .map_err(|e| Error::Other(anyhow!("Failed to {}: {}", action, e)))
}

fn docker_output_checked(args: &[&str], action: &str) -> Result<std::process::Output> {
    let output = docker_output(args, action)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("exit={}", output.status)
        };
        return Err(Error::Other(anyhow!("Failed to {}: {}", action, detail)));
    }
    Ok(output)
}

/// Check if Docker is installed and running
pub fn check_docker() -> Result<()> {
    let output = Command::new("docker")
        .arg("--version")
        .output()
        .map_err(|_| Error::Other(anyhow!("Docker is not installed. Please install Docker from https://docs.docker.com/get-docker/")))?;

    if !output.status.success() {
        return Err(Error::Other(anyhow!("Docker command failed")));
    }

    docker_status(&["info"], "check Docker status").map_err(|_| {
        Error::Other(anyhow!(
            "Docker is not running. Please start Docker daemon."
        ))
    })
}

/// Check if a Docker image exists locally
pub fn image_exists_locally(image: &str) -> bool {
    docker_status(
        &["image", "inspect", image, "--format", "{{.Id}}"],
        "inspect image",
    )
    .is_ok()
}

/// Pull image to get latest, or use local cache.
///
/// - `skip_pull = true`:  use local image only; error if not found locally.
/// - `skip_pull = false`: prefer the local cache when present; otherwise pull;
///   on pull failure fall back to the local cache; error when neither is available.
pub fn pull_or_use_latest(image: &str, skip_pull: bool) -> Result<()> {
    if skip_pull {
        if image_exists_locally(image) {
            Ok(())
        } else {
            Err(Error::Other(anyhow!(
                "Image '{}' not found locally. Remove --skip-pull to pull it.",
                image
            )))
        }
    } else {
        if image_exists_locally(image) {
            eprintln!("⏭️  Local image already present, skipping pull: {}", image);
            return Ok(());
        }
        match pull_image(image) {
            Ok(()) => Ok(()),
            Err(pull_err) => {
                if image_exists_locally(image) {
                    eprintln!(
                        "⚠️  Pull failed ({}), falling back to cached image",
                        pull_err
                    );
                    Ok(())
                } else {
                    Err(Error::Other(anyhow!(
                        "Failed to pull image '{}' and no local cache found: {}",
                        image,
                        pull_err
                    )))
                }
            }
        }
    }
}

/// Pull a Docker image
pub fn pull_image(image: &str) -> Result<()> {
    eprintln!("Pulling image: {}", image);
    let status = Command::new("docker")
        .args(["pull", image])
        .status()
        .map_err(|e| Error::Other(anyhow!("Failed to pull image: {}", e)))?;
    if !status.success() {
        return Err(Error::Other(anyhow!("Failed to pull image: {}", image)));
    }
    Ok(())
}

pub fn local_image_id(image: &str) -> Result<Option<String>> {
    let output = docker_output(
        &["image", "inspect", image, "--format", "{{.Id}}"],
        "inspect local image",
    )?;
    if !output.status.success() {
        return Ok(None);
    }
    let image_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if image_id.is_empty() {
        return Ok(None);
    }
    Ok(Some(image_id))
}

fn extract_remote_digest(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => {
            if let Some(digest) = map
                .get("Descriptor")
                .and_then(|descriptor| descriptor.get("digest"))
                .and_then(Value::as_str)
            {
                return Some(digest.to_string());
            }
            if let Some(digest) = map
                .get("config")
                .and_then(|config| config.get("digest"))
                .and_then(Value::as_str)
            {
                return Some(digest.to_string());
            }
            for nested in map.values() {
                if let Some(digest) = extract_remote_digest(nested) {
                    return Some(digest);
                }
            }
            None
        }
        Value::Array(items) => items.iter().find_map(extract_remote_digest),
        _ => None,
    }
}

pub fn remote_image_digest(image: &str) -> Result<String> {
    let output = docker_output_checked(
        &["manifest", "inspect", image, "--verbose"],
        "inspect remote image manifest",
    )?;
    let parsed: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| Error::Other(anyhow!("Failed to parse remote image manifest: {}", e)))?;
    extract_remote_digest(&parsed)
        .ok_or_else(|| Error::Other(anyhow!("Remote image digest not found for {}", image)))
}

fn image_repository(image: &str) -> &str {
    let without_digest = image.split('@').next().unwrap_or(image);
    match without_digest.rfind(':') {
        Some(idx) => {
            let suffix = &without_digest[idx + 1..];
            if !suffix.contains('/') {
                &without_digest[..idx]
            } else {
                without_digest
            }
        }
        None => without_digest,
    }
}

fn select_old_image_ids_to_remove(
    target_repository: &str,
    target_image_id: &str,
    image_rows: &[(String, String)],
    used_image_ids: &HashSet<String>,
) -> Vec<String> {
    let mut stale_ids = Vec::new();
    let mut seen = HashSet::new();
    for (repo_tag, image_id) in image_rows {
        let repository = image_repository(repo_tag);
        if repository != target_repository {
            continue;
        }
        if image_id == target_image_id || used_image_ids.contains(image_id) {
            continue;
        }
        if seen.insert(image_id.clone()) {
            stale_ids.push(image_id.clone());
        }
    }
    stale_ids
}

pub fn cleanup_old_images_for_repository(image: &str) -> Result<Vec<String>> {
    let Some(target_image_id) = local_image_id(image)? else {
        return Ok(Vec::new());
    };
    if target_image_id.is_empty() {
        return Ok(Vec::new());
    }

    let images_output = docker_output(
        &["images", "--format", "{{.Repository}}:{{.Tag}}|{{.ID}}"],
        "list images",
    )?;
    if !images_output.status.success() {
        return Err(Error::Other(anyhow!("Failed to list docker images")));
    }
    let image_rows: Vec<(String, String)> = String::from_utf8_lossy(&images_output.stdout)
        .lines()
        .filter_map(|line| {
            let mut parts = line.split('|');
            let repo_tag = parts.next()?.trim().to_string();
            let image_id = parts.next()?.trim().to_string();
            if repo_tag.is_empty() || image_id.is_empty() {
                return None;
            }
            Some((repo_tag, image_id))
        })
        .collect();

    let used_output = docker_output(
        &["ps", "-a", "--format", "{{.ImageID}}"],
        "list used image ids",
    )?;
    if !used_output.status.success() {
        return Err(Error::Other(anyhow!("Failed to list container image ids")));
    }
    let used_image_ids: HashSet<String> = String::from_utf8_lossy(&used_output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect();

    let stale_ids = select_old_image_ids_to_remove(
        image_repository(image),
        &target_image_id,
        &image_rows,
        &used_image_ids,
    );

    let mut removed = Vec::new();
    for image_id in stale_ids {
        let status = Command::new("docker")
            .args(["rmi", "-f", &image_id])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| Error::Other(anyhow!("Failed to remove image {}: {}", image_id, e)))?;
        if status.success() {
            removed.push(image_id);
        }
    }
    Ok(removed)
}

/// Check if a container is running
pub fn is_container_running(name: &str) -> Result<bool> {
    let output = docker_output(
        &["ps", "-q", "-f", &format!("name={}", name)],
        "check container",
    )?;
    Ok(!output.stdout.is_empty())
}

/// Get container info
pub fn get_container_info(name: &str) -> Result<Option<ContainerInfo>> {
    let output = docker_output(
        &[
            "ps",
            "-a",
            "--filter",
            &format!("name={}", name),
            "--format",
            "{{.ID}}|{{.Names}}|{{.Status}}|{{.Image}}|{{.Ports}}",
        ],
        "get container info",
    )?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.trim();

    if line.is_empty() {
        return Ok(None);
    }

    let parts: Vec<&str> = line.split('|').collect();
    if parts.len() != 5 {
        return Ok(None);
    }

    Ok(Some(ContainerInfo {
        id: parts[0].to_string(),
        name: parts[1].to_string(),
        status: parts[2].to_string(),
        image: parts[3].to_string(),
        ports: parts[4].to_string(),
    }))
}

pub fn inspect_container_runtime_config(name: &str) -> Result<Option<ContainerRuntimeConfig>> {
    let output = docker_output(&["inspect", name], "inspect container runtime config")?;
    if !output.status.success() {
        return Ok(None);
    }
    let parsed: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| Error::Other(anyhow!("Failed to parse docker inspect JSON: {}", e)))?;
    let Some(container) = parsed.as_array().and_then(|items| items.first()) else {
        return Ok(None);
    };
    let image_ref = container
        .get("Config")
        .and_then(|value| value.get("Image"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let env = container
        .get("Config")
        .and_then(|value| value.get("Env"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let binds = container
        .get("HostConfig")
        .and_then(|value| value.get("Binds"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let restart_policy_name = container
        .get("HostConfig")
        .and_then(|value| value.get("RestartPolicy"))
        .and_then(|value| value.get("Name"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let network_mode = container
        .get("HostConfig")
        .and_then(|value| value.get("NetworkMode"))
        .and_then(Value::as_str)
        .unwrap_or("default")
        .to_string();
    Ok(Some(ContainerRuntimeConfig {
        image_ref,
        env,
        binds,
        restart_policy_name,
        network_mode,
    }))
}

pub fn run_container(
    name: &str,
    image: &str,
    env: &[String],
    binds: &[String],
    restart_policy_name: &str,
    network_mode: &str,
) -> Result<()> {
    let mut args = vec![
        "run".to_string(),
        "-d".to_string(),
        "--name".to_string(),
        name.to_string(),
    ];
    if !restart_policy_name.trim().is_empty() {
        args.push("--restart".to_string());
        args.push(restart_policy_name.to_string());
    }
    if !network_mode.trim().is_empty() {
        args.push("--network".to_string());
        args.push(network_mode.to_string());
    }
    for bind in binds {
        args.push("-v".to_string());
        args.push(bind.clone());
    }
    for item in env {
        args.push("-e".to_string());
        args.push(item.clone());
    }
    args.push(image.to_string());
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    docker_status(&arg_refs, &format!("start container {}", name))
}

/// Get container stats
pub fn get_container_stats(name: &str) -> Result<Option<ContainerStats>> {
    let output = docker_output(
        &[
            "stats",
            name,
            "--no-stream",
            "--format",
            "{{.CPUPerc}}|{{.MemUsage}}|{{.MemPerc}}",
        ],
        "get container stats",
    )?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.trim();

    if line.is_empty() {
        return Ok(None);
    }

    let parts: Vec<&str> = line.split('|').collect();
    if parts.len() != 3 {
        return Ok(None);
    }

    let mem_parts: Vec<&str> = parts[1].split('/').map(|s| s.trim()).collect();
    let mem_usage = mem_parts.first().unwrap_or(&"0B").to_string();
    let mem_limit = mem_parts.get(1).unwrap_or(&"0B").to_string();

    Ok(Some(ContainerStats {
        cpu_percent: parts[0].to_string(),
        mem_usage,
        mem_limit,
        mem_percent: parts[2].to_string(),
    }))
}

/// Stop a container
pub fn stop_container(name: &str, force: bool) -> Result<()> {
    let cmd = if force { "kill" } else { "stop" };
    docker_status(&[cmd, name], &format!("stop container {}", name))
}

/// Remove a container
pub fn remove_container(name: &str) -> Result<()> {
    docker_status(&["rm", "-f", name], &format!("remove container {}", name))
}

/// Get container logs
pub fn get_logs(name: &str, tail: u32, follow: bool, since: Option<&str>) -> Result<()> {
    let mut args = vec!["logs"];
    if follow {
        args.push("-f");
    }
    let tail_str = tail.to_string();
    args.push("--tail");
    args.push(&tail_str);
    if let Some(since_val) = since {
        args.push("--since");
        args.push(since_val);
    }
    args.push(name);

    let status = Command::new("docker")
        .args(&args)
        .status()
        .map_err(|e| Error::Other(anyhow!("Failed to get logs: {}", e)))?;
    if !status.success() {
        return Err(Error::Other(anyhow!("Failed to get logs for: {}", name)));
    }
    Ok(())
}

/// Estimate resource usage
pub fn estimate_resources(max_opcode_containers: u32) -> (String, String, String) {
    // Runtime agent: ~200MB RAM, 0.5 CPU
    // Opcode-API per container: ~100MB RAM, 0.2 CPU
    let agent_mem = 200;
    let opcode_mem_per_container = 100;
    let total_mem = agent_mem + (opcode_mem_per_container * max_opcode_containers);

    let agent_cpu = 0.5;
    let opcode_cpu_per_container = 0.2;
    let total_cpu = agent_cpu + (opcode_cpu_per_container * max_opcode_containers as f64);

    // Disk: workspaces + images (~5GB base + 1GB per project)
    let base_disk = 5;
    let disk_per_project = 1;
    let estimated_disk = base_disk + (disk_per_project * max_opcode_containers);

    (
        format!("{:.1} CPU cores", total_cpu),
        format!("{} MB RAM", total_mem),
        format!("{} GB disk space", estimated_disk),
    )
}

#[cfg(test)]
mod tests {
    use super::{extract_remote_digest, image_repository, select_old_image_ids_to_remove};
    use serde_json::json;
    use std::collections::HashSet;

    #[test]
    fn image_repository_strips_tag_but_keeps_registry_port() {
        assert_eq!(
            image_repository("ghcr.io/d1vai/d1v-runtime-agent:latest"),
            "ghcr.io/d1vai/d1v-runtime-agent"
        );
        assert_eq!(
            image_repository("299000395210.dkr.ecr.ap-southeast-1.amazonaws.com/d1v-opcode:latest"),
            "299000395210.dkr.ecr.ap-southeast-1.amazonaws.com/d1v-opcode"
        );
        assert_eq!(
            image_repository("localhost:5000/example/image:dev"),
            "localhost:5000/example/image"
        );
        assert_eq!(
            image_repository("ghcr.io/d1vai/d1v-runtime-agent@sha256:abc"),
            "ghcr.io/d1vai/d1v-runtime-agent"
        );
    }

    #[test]
    fn select_old_image_ids_to_remove_skips_current_and_used_images() {
        let image_rows = vec![
            (
                "ghcr.io/d1vai/d1v-runtime-agent:latest".to_string(),
                "sha256:current".to_string(),
            ),
            (
                "ghcr.io/d1vai/d1v-runtime-agent:old".to_string(),
                "sha256:old-unused".to_string(),
            ),
            (
                "ghcr.io/d1vai/d1v-runtime-agent:older".to_string(),
                "sha256:old-used".to_string(),
            ),
            (
                "ghcr.io/d1vai/opcode-api:latest".to_string(),
                "sha256:opcode".to_string(),
            ),
        ];
        let used_image_ids = HashSet::from(["sha256:old-used".to_string()]);

        let removable = select_old_image_ids_to_remove(
            "ghcr.io/d1vai/d1v-runtime-agent",
            "sha256:current",
            &image_rows,
            &used_image_ids,
        );

        assert_eq!(removable, vec!["sha256:old-unused".to_string()]);
    }

    #[test]
    fn extract_remote_digest_prefers_descriptor_digest() {
        let payload = json!({
            "Ref": "ghcr.io/d1vai/d1v-runtime-agent:latest",
            "Descriptor": {"digest": "sha256:manifest"},
            "SchemaV2Manifest": {"config": {"digest": "sha256:config"}}
        });
        assert_eq!(
            extract_remote_digest(&payload).as_deref(),
            Some("sha256:manifest")
        );
    }
}
