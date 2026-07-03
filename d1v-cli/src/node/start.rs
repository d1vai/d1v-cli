// Start node containers

use super::docker;
use super::StartArgs;
use crate::error::{Error, Result};
use crate::Context;
use anyhow::anyhow;
use std::process::Command;

const RUNTIME_AGENT_IMAGE: &str =
    "299000395210.dkr.ecr.ap-southeast-1.amazonaws.com/d1v-runtime-agent:latest";
const OPCODE_API_IMAGE: &str = "ghcr.io/d1vai/opcode-api:latest";
const AGENT_CONTAINER_NAME: &str = "d1v-runtime-agent-platform";
const OPCODE_API_CONTAINER_NAME: &str = "d1v-opcode-api";

pub async fn run(_ctx: &Context, args: StartArgs) -> Result<()> {
    // 1. Check Docker installation
    if !args.skip_docker_check {
        eprintln!("🔍 Checking Docker installation...");
        if let Err(e) = docker::check_docker() {
            eprintln!("\n❌ {}", e);
            eprintln!("\n💡 Install Docker:");
            eprintln!("   • Visit: https://docs.docker.com/get-docker/");
            eprintln!("   • For Ubuntu: curl -fsSL https://get.docker.com | sh");
            eprintln!("   • For macOS: Download Docker Desktop");
            eprintln!("\nAfter installation, run this command again.");
            return Err(e);
        }
        eprintln!("✅ Docker is installed and running\n");
    }

    // 2. Validate platform key
    let platform_key = args.key.as_ref().ok_or_else(|| {
        Error::Other(anyhow!(
            "Platform node key is required. Use --key or set D1V_PLATFORM_NODE_KEY env var"
        ))
    })?.clone();

    // 3. Show resource estimation
    let (cpu, mem, disk) = docker::estimate_resources(args.max_opcode_containers);
    eprintln!("📊 Estimated resource usage:");
    eprintln!("   • CPU: {}", cpu);
    eprintln!("   • Memory: {}", mem);
    eprintln!("   • Disk: {}", disk);
    eprintln!("   • Max opcode containers: {}\n", args.max_opcode_containers);

    // 4. Confirm with user
    if !args.yes {
        eprint!("Continue? [y/N] ");
        use std::io::{self, BufRead};
        let stdin = io::stdin();
        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        let answer = line.trim().to_lowercase();
        if answer != "y" && answer != "yes" {
            eprintln!("Aborted.");
            return Ok(());
        }
    }

    // 5. Check if containers already running
    if docker::is_container_running(AGENT_CONTAINER_NAME)? {
        eprintln!("⚠️  Runtime agent container is already running");
        eprintln!("   Use 'd1v node stop' to stop it first, or 'd1v node status' to check status");
        return Err(Error::Other(anyhow!("Container already running")));
    }

    // 6. Pull images
    eprintln!("\n📥 Pulling Docker images...");
    eprintln!("   This may take a few minutes on first run.\n");

    // Pull opcode-api (optional — image may not be available yet)
    eprintln!("📥 Pulling opcode-api image...");
    match docker::pull_image(OPCODE_API_IMAGE) {
        Ok(()) => eprintln!("✅ Opcode-API image ready"),
        Err(e) => {
            eprintln!("⚠️  Opcode-API image not available ({})", e);
            eprintln!("   Runtime-agent will use the built-in opcode image for workspaces.");
        }
    }

    // Pull runtime-agent (may need ECR auth)
    eprintln!("\n📥 Pulling runtime-agent image...");
    if let Err(e) = docker::pull_image(RUNTIME_AGENT_IMAGE) {
        eprintln!("⚠️  Failed to pull runtime-agent image: {}", e);
        eprintln!("   Trying to login to AWS ECR...");

        // Try to login to ECR
        if let Err(login_err) = login_ecr() {
            eprintln!("❌ ECR login failed: {}", login_err);
            eprintln!("\n💡 To use ECR images, configure AWS credentials:");
            eprintln!("   • Install AWS CLI: https://aws.amazon.com/cli/");
            eprintln!("   • Run: aws configure");
            eprintln!("   • Or use IAM role (for EC2 instances)");
            return Err(e);
        }

        // Retry pull after login
        docker::pull_image(RUNTIME_AGENT_IMAGE)?;
    }

    eprintln!("\n✅ Images pulled successfully\n");

    // 7. Start runtime-agent container
    eprintln!("🚀 Starting runtime-agent container...");

    let node_id = args.node_id.as_ref().map(|s| s.clone()).unwrap_or_else(|| {
        std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("HOST"))
            .unwrap_or_else(|_| "unknown-node".to_string())
    });

    // Prepare volume and env strings (must live longer than docker_args)
    let docker_config_mount = format!(
        "{}/.docker:/root/.docker:ro",
        std::env::var("HOME").unwrap_or_default()
    );
    let workspace_mount = format!("{}:{}", args.workspace_root, args.workspace_root);
    let control_plane_env = format!("D1V_CONTROL_PLANE_URL={}", args.control_plane);
    let platform_key_env = format!("D1V_RUNTIME_PLATFORM_NODE_KEY={}", platform_key);
    let node_id_env = format!("D1V_RUNTIME_NODE_ID={}", node_id);
    let agent_port_env = format!("D1V_RUNTIME_AGENT_PORT={}", args.agent_port);
    let agent_ws_port_env = format!("D1V_RUNTIME_AGENT_WS_PORT={}", args.agent_ws_port);
    let max_containers_env = format!(
        "D1V_RUNTIME_MAX_OPCODE_CONTAINERS={}",
        args.max_opcode_containers
    );

    let docker_args = vec![
        "run",
        "-d",
        "--name",
        AGENT_CONTAINER_NAME,
        "--restart",
        "unless-stopped",
        "--network",
        "host",
        "-v",
        "/var/run/docker.sock:/var/run/docker.sock",
        "-v",
        &docker_config_mount,
        "-v",
        &workspace_mount,
        "-e",
        &control_plane_env,
        "-e",
        &platform_key_env,
        "-e",
        &node_id_env,
        "-e",
        &agent_port_env,
        "-e",
        &agent_ws_port_env,
        "-e",
        "D1V_RUNTIME_AUTO_DETECT_PUBLIC_IP=1",
        "-e",
        &max_containers_env,
        RUNTIME_AGENT_IMAGE,
    ];

    let status = Command::new("docker")
        .args(&docker_args)
        .status()
        .map_err(|e| Error::Other(anyhow!("Failed to start container: {}", e)))?;

    if !status.success() {
        return Err(Error::Other(anyhow!("Failed to start runtime-agent container")));
    }

    eprintln!("✅ Runtime-agent started: {}\n", AGENT_CONTAINER_NAME);

    // 8. Wait a bit and check health
    eprintln!("⏳ Waiting for runtime-agent to be healthy...");
    std::thread::sleep(std::time::Duration::from_secs(3));

    // Check if still running
    if !docker::is_container_running(AGENT_CONTAINER_NAME)? {
        eprintln!("❌ Runtime-agent container stopped unexpectedly");
        eprintln!("\n📋 Last 20 lines of logs:");
        let _ = docker::get_logs(AGENT_CONTAINER_NAME, 20, false, None);
        return Err(Error::Other(anyhow!("Container stopped")));
    }

    eprintln!("✅ Runtime-agent is running\n");

    // 9. Print summary
    print_summary(&node_id, &args);

    Ok(())
}

fn login_ecr() -> Result<()> {
    // Extract region from image URL
    let region = "ap-southeast-1";

    // Get ECR password and pipe to docker login
    let ecr_output = Command::new("aws")
        .args(["ecr", "get-login-password", "--region", region])
        .output()
        .map_err(|e| Error::Other(anyhow!("Failed to get ECR password: {}", e)))?;

    if !ecr_output.status.success() {
        return Err(Error::Other(anyhow!("Failed to get ECR password")));
    }

    let password = String::from_utf8_lossy(&ecr_output.stdout);
    let registry = "299000395210.dkr.ecr.ap-southeast-1.amazonaws.com";

    // Docker login
    let mut login_cmd = Command::new("docker")
        .args(["login", "--username", "AWS", "--password-stdin", registry])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| Error::Other(anyhow!("Failed to start docker login: {}", e)))?;

    {
        use std::io::Write;
        let stdin = login_cmd.stdin.as_mut().unwrap();
        stdin
            .write_all(password.as_bytes())
            .map_err(|e| Error::Other(anyhow!("Failed to write password: {}", e)))?;
    }

    let status = login_cmd
        .wait()
        .map_err(|e| Error::Other(anyhow!("Failed to wait for docker login: {}", e)))?;

    if !status.success() {
        return Err(Error::Other(anyhow!("Docker login failed")));
    }

    Ok(())
}

fn print_summary(node_id: &str, args: &StartArgs) {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║          D1V Platform Node Started Successfully!               ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();
    println!("Node Information:");
    println!("  Node ID:           {}", node_id);
    println!("  Control Plane:     {}", args.control_plane);
    println!();
    println!("Container Details:");
    println!("  Runtime Agent:     {} (ports {}, {})", AGENT_CONTAINER_NAME, args.agent_port, args.agent_ws_port);
    println!();
    println!("Workspace:");
    println!("  Root:              {}", args.workspace_root);
    println!();
    println!("Management Commands:");
    println!("  View status:       d1v node status");
    println!("  View logs:         d1v node logs -f");
    println!("  Stop node:         d1v node stop");
    println!();
    println!("✓ Node is ready to accept workloads");
    println!();
}
