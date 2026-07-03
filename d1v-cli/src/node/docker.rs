// Docker utility functions for node management

use crate::error::{Error, Result};
use anyhow::anyhow;
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

/// Check if Docker is installed and running
pub fn check_docker() -> Result<()> {
    // Check if docker command exists
    let output = Command::new("docker")
        .arg("--version")
        .output()
        .map_err(|_| Error::Other(anyhow!("Docker is not installed. Please install Docker from https://docs.docker.com/get-docker/")))?;

    if !output.status.success() {
        return Err(Error::Other(anyhow!("Docker command failed")));
    }

    // Check if Docker daemon is running
    let output = Command::new("docker")
        .arg("info")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| Error::Other(anyhow!("Failed to check Docker status")))?;

    if !output.success() {
        return Err(Error::Other(anyhow!(
            "Docker is not running. Please start Docker daemon."
        )));
    }

    Ok(())
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

/// Check if a container is running
pub fn is_container_running(name: &str) -> Result<bool> {
    let output = Command::new("docker")
        .args(["ps", "-q", "-f", &format!("name={}", name)])
        .output()
        .map_err(|e| Error::Other(anyhow!("Failed to check container: {}", e)))?;

    Ok(!output.stdout.is_empty())
}

/// Get container info
pub fn get_container_info(name: &str) -> Result<Option<ContainerInfo>> {
    let output = Command::new("docker")
        .args([
            "ps",
            "-a",
            "--filter",
            &format!("name={}", name),
            "--format",
            "{{.ID}}|{{.Names}}|{{.Status}}|{{.Image}}|{{.Ports}}",
        ])
        .output()
        .map_err(|e| Error::Other(anyhow!("Failed to get container info: {}", e)))?;

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

/// Get container stats
pub fn get_container_stats(name: &str) -> Result<Option<ContainerStats>> {
    let output = Command::new("docker")
        .args([
            "stats",
            name,
            "--no-stream",
            "--format",
            "{{.CPUPerc}}|{{.MemUsage}}|{{.MemPerc}}",
        ])
        .output()
        .map_err(|e| Error::Other(anyhow!("Failed to get container stats: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.trim();

    if line.is_empty() {
        return Ok(None);
    }

    let parts: Vec<&str> = line.split('|').collect();
    if parts.len() != 3 {
        return Ok(None);
    }

    // Parse mem usage (format: "123MiB / 456MiB")
    let mem_parts: Vec<&str> = parts[1].split('/').map(|s| s.trim()).collect();
    let mem_usage = mem_parts.get(0).unwrap_or(&"0B").to_string();
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

    let status = Command::new("docker")
        .args([cmd, name])
        .stdout(Stdio::null())
        .status()
        .map_err(|e| Error::Other(anyhow!("Failed to stop container: {}", e)))?;

    if !status.success() {
        return Err(Error::Other(anyhow!("Failed to stop container: {}", name)));
    }

    Ok(())
}

/// Remove a container
pub fn remove_container(name: &str) -> Result<()> {
    let status = Command::new("docker")
        .args(["rm", "-f", name])
        .stdout(Stdio::null())
        .status()
        .map_err(|e| Error::Other(anyhow!("Failed to remove container: {}", e)))?;

    if !status.success() {
        return Err(Error::Other(anyhow!("Failed to remove container: {}", name)));
    }

    Ok(())
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
