use crate::error::{Error, Result};
use anyhow::anyhow;
use std::fs;
use std::io::Write;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Clone, Debug)]
pub struct DetectedIngress {
    pub provider: &'static str,
    pub hostname: String,
    pub scheme: String,
    pub external_port: u16,
    pub upstream_host: String,
    pub upstream_port: u16,
    pub config_path: String,
    pub confidence: u8,
}

#[derive(Clone, Debug)]
pub struct ConfiguredIngress {
    pub provider: &'static str,
    pub hostname: String,
    pub config_path: String,
}

pub async fn detect_public_ip() -> Option<String> {
    for url in ["https://api.ipify.org", "https://ifconfig.me/ip"] {
        let response = match reqwest::get(url).await {
            Ok(value) => value,
            Err(_) => continue,
        };
        let text = match response.text().await {
            Ok(value) => value,
            Err(_) => continue,
        };
        let value = text.trim();
        if value.parse::<IpAddr>().is_ok() {
            return Some(value.to_string());
        }
    }
    None
}

pub fn detect_public_ingress(
    agent_port: u16,
    preferred_provider: Option<&str>,
    preferred_hostname: Option<&str>,
) -> Result<Option<DetectedIngress>> {
    let provider_filter = preferred_provider
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty());
    let preferred_hostname = preferred_hostname
        .map(|value| value.trim().trim_end_matches('.').to_ascii_lowercase())
        .filter(|value| !value.is_empty());

    let detectors: &[IngressDetector] = &[
        IngressDetector {
            provider: "caddy",
            detect: detect_caddy_ingress,
        },
        IngressDetector {
            provider: "nginx",
            detect: detect_nginx_ingress,
        },
        IngressDetector {
            provider: "traefik",
            detect: detect_traefik_ingress,
        },
        IngressDetector {
            provider: "npm",
            detect: detect_npm_ingress,
        },
    ];

    for detector in detectors {
        if let Some(provider) = provider_filter.as_deref() {
            if detector.provider != provider {
                continue;
            }
        }
        let detected = match (detector.detect)(agent_port, preferred_hostname.as_deref()) {
            Ok(value) => value,
            Err(_err) if provider_filter.is_none() => continue,
            Err(err) => return Err(err),
        };
        let Some(candidate) = detected else {
            continue;
        };
        return Ok(Some(candidate));
    }

    Ok(None)
}

pub fn configure_public_ingress(
    agent_port: u16,
    preferred_provider: Option<&str>,
    public_hostname: &str,
) -> Result<Option<ConfiguredIngress>> {
    let hostname = sanitize_host(public_hostname);
    if hostname.is_empty() || !hostname.contains('.') {
        return Err(Error::Other(anyhow!(
            "public hostname is required to configure public ingress"
        )));
    }
    let provider_filter = preferred_provider
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty());

    if let Some(provider) = provider_filter.as_deref() {
        return match provider {
            "nginx" => configure_nginx_ingress(agent_port, &hostname).map(Some),
            "npm" => configure_npm_ingress(agent_port, &hostname).map(Some),
            "caddy" | "traefik" => Ok(None),
            other => Err(Error::Other(anyhow!(
                "unsupported ingress provider for auto configuration: {}",
                other
            ))),
        };
    }

    if detect_npm_container()?.is_some() {
        return configure_npm_ingress(agent_port, &hostname).map(Some);
    }
    if Path::new("/etc/nginx").exists() {
        return configure_nginx_ingress(agent_port, &hostname).map(Some);
    }

    Ok(None)
}

struct IngressDetector {
    provider: &'static str,
    detect: fn(u16, Option<&str>) -> Result<Option<DetectedIngress>>,
}

fn detect_caddy_ingress(
    agent_port: u16,
    preferred_hostname: Option<&str>,
) -> Result<Option<DetectedIngress>> {
    let candidates = [
        PathBuf::from("/etc/caddy/Caddyfile"),
        PathBuf::from("/usr/local/etc/caddy/Caddyfile"),
        PathBuf::from("Caddyfile"),
    ];
    for path in candidates {
        if !path.exists() {
            continue;
        }
        let content = fs::read_to_string(&path)
            .map_err(|exc| Error::Other(anyhow!("failed to read {}: {}", path.display(), exc)))?;
        if let Some(candidate) = parse_caddy_content(
            &content,
            path.display().to_string(),
            agent_port,
            preferred_hostname,
        ) {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

fn detect_nginx_ingress(
    agent_port: u16,
    preferred_hostname: Option<&str>,
) -> Result<Option<DetectedIngress>> {
    let paths = collect_nginx_paths(&[
        "/etc/nginx/conf.d",
        "/etc/nginx/sites-enabled",
        "/etc/nginx/sites-available",
    ])?;
    parse_nginx_like_paths("nginx", &paths, agent_port, preferred_hostname)
}

fn configure_nginx_ingress(agent_port: u16, hostname: &str) -> Result<ConfiguredIngress> {
    let existing = detect_nginx_like_hostname(
        hostname,
        &collect_nginx_paths(&[
            "/etc/nginx/conf.d",
            "/etc/nginx/sites-enabled",
            "/etc/nginx/sites-available",
        ])?,
    )?;
    if let Some(candidate) = existing {
        if candidate.upstream_port == agent_port && is_local_host(&candidate.upstream_host) {
            return Ok(ConfiguredIngress {
                provider: "nginx",
                hostname: hostname.to_string(),
                config_path: candidate.config_path,
            });
        }
        return Err(Error::Other(anyhow!(
            "hostname {} is already configured by nginx and does not point to 127.0.0.1:{}",
            hostname,
            agent_port
        )));
    }

    let file_slug = safe_file_slug(hostname);
    let config_filename = format!("d1v-runtime-{}.conf", file_slug);
    let (config_path, symlink_path) = if Path::new("/etc/nginx/sites-available").exists()
        && Path::new("/etc/nginx/sites-enabled").exists()
    {
        (
            PathBuf::from("/etc/nginx/sites-available").join(&config_filename),
            Some(PathBuf::from("/etc/nginx/sites-enabled").join(&config_filename)),
        )
    } else if Path::new("/etc/nginx/conf.d").exists() {
        (
            PathBuf::from("/etc/nginx/conf.d").join(&config_filename),
            None,
        )
    } else {
        return Err(Error::Other(anyhow!(
            "nginx config directories not found under /etc/nginx"
        )));
    };

    let config = render_managed_nginx_config(hostname, agent_port);
    fs::write(&config_path, config).map_err(|exc| {
        Error::Other(anyhow!(
            "failed to write nginx config {}: {}",
            config_path.display(),
            exc
        ))
    })?;

    if let Some(link_path) = symlink_path.as_ref() {
        if link_path.exists() {
            fs::remove_file(link_path).map_err(|exc| {
                Error::Other(anyhow!(
                    "failed to replace nginx symlink {}: {}",
                    link_path.display(),
                    exc
                ))
            })?;
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(&config_path, link_path).map_err(|exc| {
            Error::Other(anyhow!(
                "failed to create nginx symlink {} -> {}: {}",
                link_path.display(),
                config_path.display(),
                exc
            ))
        })?;
    }

    reload_nginx()?;
    Ok(ConfiguredIngress {
        provider: "nginx",
        hostname: hostname.to_string(),
        config_path: config_path.display().to_string(),
    })
}

fn detect_npm_ingress(
    agent_port: u16,
    preferred_hostname: Option<&str>,
) -> Result<Option<DetectedIngress>> {
    let mut paths = collect_nginx_paths(&[
        "/data/nginx/proxy_host",
        "/opt/nginx-proxy-manager/nginx/proxy_host",
        "/var/lib/docker/volumes/nginx-proxy-manager_data/_data/nginx/proxy_host",
    ])?;
    if let Some(container) = detect_npm_container()? {
        let mounted = npm_proxy_host_paths_from_container(&container)?;
        for path in mounted {
            if path.is_file() {
                paths.push(path);
            } else if path.exists() {
                let extra = collect_nginx_paths(&[path.to_string_lossy().as_ref()])?;
                paths.extend(extra);
            }
        }
        paths.sort();
        paths.dedup();
    }
    parse_nginx_like_paths("npm", &paths, agent_port, preferred_hostname)
}

fn configure_npm_ingress(agent_port: u16, hostname: &str) -> Result<ConfiguredIngress> {
    let container = detect_npm_container()?.ok_or_else(|| {
        Error::Other(anyhow!(
            "nginx proxy manager container not found; cannot auto configure npm ingress"
        ))
    })?;

    let script = build_npm_upsert_script(hostname, agent_port);
    let mut child = Command::new("docker")
        .args([
            "exec",
            "-i",
            &container.name,
            "sh",
            "-lc",
            "cd /app && node --input-type=module -",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|exc| Error::Other(anyhow!("failed to exec npm container: {}", exc)))?;
    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| Error::Other(anyhow!("failed to open stdin for npm container exec")))?;
        stdin
            .write_all(script.as_bytes())
            .map_err(|exc| Error::Other(anyhow!("failed to send npm script: {}", exc)))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|exc| Error::Other(anyhow!("failed to wait for npm container exec: {}", exc)))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Other(anyhow!(
            "failed to configure nginx proxy manager ingress: {}",
            stderr.trim()
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).map_err(|exc| {
        Error::Other(anyhow!(
            "failed to parse nginx proxy manager output: {}; raw={}",
            exc,
            stdout.trim()
        ))
    })?;
    let config_path = parsed
        .get("config_path")
        .and_then(|value| value.as_str())
        .unwrap_or("docker://nginx-proxy-manager")
        .to_string();

    Ok(ConfiguredIngress {
        provider: "npm",
        hostname: hostname.to_string(),
        config_path,
    })
}

fn detect_traefik_ingress(
    agent_port: u16,
    preferred_hostname: Option<&str>,
) -> Result<Option<DetectedIngress>> {
    let output = Command::new("docker")
        .args(["ps", "--format", "{{.Names}}"])
        .output()
        .map_err(|exc| Error::Other(anyhow!("failed to list docker containers: {}", exc)))?;
    if !output.status.success() {
        return Ok(None);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    for container_name in stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let inspect = Command::new("docker")
            .args([
                "inspect",
                container_name,
                "--format",
                "{{json .Config.Labels}}",
            ])
            .output()
            .map_err(|exc| {
                Error::Other(anyhow!("failed to inspect {}: {}", container_name, exc))
            })?;
        if !inspect.status.success() {
            continue;
        }
        let labels_raw = String::from_utf8_lossy(&inspect.stdout);
        let labels = labels_raw.trim();
        if labels.is_empty() || labels == "null" {
            continue;
        }
        let parsed: serde_json::Value = match serde_json::from_str(labels) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let Some(map) = parsed.as_object() else {
            continue;
        };
        let mut hosts: Vec<String> = Vec::new();
        let mut service_port: Option<u16> = None;
        let mut tls_enabled = false;
        let mut explicit_entrypoint: Option<String> = None;
        for (key, value) in map {
            let value = value.as_str().unwrap_or("").trim();
            if key.contains(".rule") && value.contains("Host(") {
                hosts.extend(parse_traefik_hosts(value));
            }
            if key.contains("loadbalancer.server.port") {
                service_port = value.parse::<u16>().ok();
            }
            if key.contains(".tls") && value.eq_ignore_ascii_case("true") {
                tls_enabled = true;
            }
            if key.contains(".entrypoints") && !value.is_empty() {
                explicit_entrypoint = Some(value.to_ascii_lowercase());
            }
        }
        if service_port != Some(agent_port) || hosts.is_empty() {
            continue;
        }
        if !tls_enabled {
            if let Some(entrypoint) = explicit_entrypoint.as_deref() {
                tls_enabled = entrypoint.contains("websecure") || entrypoint.contains("https");
            }
        }
        if let Some(hostname) = choose_hostname(&hosts, preferred_hostname, tls_enabled) {
            return Ok(Some(DetectedIngress {
                provider: "traefik",
                hostname,
                scheme: if tls_enabled { "https" } else { "http" }.to_string(),
                external_port: if tls_enabled { 443 } else { 80 },
                upstream_host: "127.0.0.1".to_string(),
                upstream_port: agent_port,
                config_path: format!("docker://{}", container_name),
                confidence: 80,
            }));
        }
    }
    Ok(None)
}

fn collect_nginx_paths(prefixes: &[&str]) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for prefix in prefixes {
        let root = Path::new(prefix);
        if !root.exists() {
            continue;
        }
        let read_dir = fs::read_dir(root)
            .map_err(|exc| Error::Other(anyhow!("failed to read {}: {}", root.display(), exc)))?;
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_file() {
                paths.push(path);
            }
        }
    }
    paths.sort();
    Ok(paths)
}

fn detect_nginx_like_hostname(
    hostname: &str,
    paths: &[PathBuf],
) -> Result<Option<DetectedIngress>> {
    let preferred = sanitize_host(hostname);
    if preferred.is_empty() {
        return Ok(None);
    }
    for path in paths {
        let content = fs::read_to_string(path)
            .map_err(|exc| Error::Other(anyhow!("failed to read {}: {}", path.display(), exc)))?;
        if let Some(candidate) = parse_nginx_like_content(
            "nginx",
            &content,
            path.display().to_string(),
            0,
            Some(preferred.as_str()),
        ) {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

fn parse_nginx_like_paths(
    provider: &'static str,
    paths: &[PathBuf],
    agent_port: u16,
    preferred_hostname: Option<&str>,
) -> Result<Option<DetectedIngress>> {
    for path in paths {
        let content = fs::read_to_string(path)
            .map_err(|exc| Error::Other(anyhow!("failed to read {}: {}", path.display(), exc)))?;
        if let Some(candidate) = parse_nginx_like_content(
            provider,
            &content,
            path.display().to_string(),
            agent_port,
            preferred_hostname,
        ) {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

fn parse_caddy_content(
    content: &str,
    config_path: String,
    agent_port: u16,
    preferred_hostname: Option<&str>,
) -> Option<DetectedIngress> {
    let mut current_hosts: Vec<String> = Vec::new();
    let mut current_https = true;
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(prefix) = line.strip_suffix('{') {
            current_hosts = prefix
                .split(',')
                .map(|value| sanitize_host(value))
                .filter(|value| !value.is_empty())
                .collect();
            current_https = !prefix.to_ascii_lowercase().contains("http://");
            continue;
        }
        if !line.starts_with("reverse_proxy ") || current_hosts.is_empty() {
            continue;
        }
        let upstream = line.trim_start_matches("reverse_proxy ").trim();
        if let Some((host, port)) = parse_host_port(upstream) {
            if !is_local_host(&host) || port != agent_port {
                continue;
            }
            if let Some(hostname) =
                choose_hostname(&current_hosts, preferred_hostname, current_https)
            {
                let external_port = if current_https { 443 } else { 80 };
                return Some(DetectedIngress {
                    provider: "caddy",
                    hostname,
                    scheme: if current_https { "https" } else { "http" }.to_string(),
                    external_port,
                    upstream_host: host,
                    upstream_port: port,
                    config_path,
                    confidence: 95,
                });
            }
        }
    }
    None
}

fn parse_nginx_like_content(
    provider: &'static str,
    content: &str,
    config_path: String,
    agent_port: u16,
    preferred_hostname: Option<&str>,
) -> Option<DetectedIngress> {
    let mut depth = 0_i32;
    let mut server_names: Vec<String> = Vec::new();
    let mut scheme = "http".to_string();
    let mut external_port = 80_u16;
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with("server {") || line == "server{" {
            depth += 1;
            server_names.clear();
            scheme = "http".to_string();
            external_port = 80;
            continue;
        }
        if depth <= 0 {
            continue;
        }
        if line.starts_with("listen ") {
            if line.contains("443") {
                scheme = "https".to_string();
                external_port = 443;
            } else if let Some(port) = parse_listen_port(line) {
                external_port = port;
            }
        }
        if line.starts_with("server_name ") {
            let names = line
                .trim_start_matches("server_name")
                .trim()
                .trim_end_matches(';');
            server_names = names
                .split_whitespace()
                .map(sanitize_host)
                .filter(|value| !value.is_empty() && value != "_")
                .collect();
        }
        if line.starts_with("proxy_pass ") && !server_names.is_empty() {
            let upstream = line
                .trim_start_matches("proxy_pass")
                .trim()
                .trim_end_matches(';');
            if let Some((host, port)) = parse_proxy_pass_target(upstream) {
                if !is_local_host(&host) || (agent_port != 0 && port != agent_port) {
                    continue;
                }
                if let Some(hostname) =
                    choose_hostname(&server_names, preferred_hostname, scheme == "https")
                {
                    return Some(DetectedIngress {
                        provider,
                        hostname,
                        scheme,
                        external_port,
                        upstream_host: host,
                        upstream_port: port,
                        config_path,
                        confidence: if provider == "npm" { 82 } else { 90 },
                    });
                }
            }
        }
        let opens = line.matches('{').count() as i32;
        let closes = line.matches('}').count() as i32;
        depth += opens - closes;
        if depth < 0 {
            depth = 0;
        }
    }
    None
}

fn sanitize_host(raw: &str) -> String {
    raw.trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('{')
        .trim_matches('`')
        .trim_matches('"')
        .trim_matches('\'')
        .trim_end_matches(',')
        .trim_end_matches(';')
        .trim_end_matches('.')
        .to_ascii_lowercase()
}

fn safe_file_slug(raw: &str) -> String {
    raw.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn parse_host_port(raw: &str) -> Option<(String, u16)> {
    let target = raw.split_whitespace().next().map(sanitize_host)?;
    let target = target
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .to_string();
    let (host, port) = target.rsplit_once(':')?;
    let parsed = port.parse::<u16>().ok()?;
    Some((host.to_string(), parsed))
}

fn parse_proxy_pass_target(raw: &str) -> Option<(String, u16)> {
    parse_host_port(raw)
}

fn parse_listen_port(raw: &str) -> Option<u16> {
    raw.trim_start_matches("listen")
        .split_whitespace()
        .find_map(|segment| {
            let candidate = segment
                .trim_matches(';')
                .trim_matches('[')
                .trim_matches(']')
                .split(':')
                .next_back()?;
            candidate.parse::<u16>().ok()
        })
}

fn render_managed_nginx_config(hostname: &str, agent_port: u16) -> String {
    format!(
        "# Managed by d1v-cli\nserver {{\n    listen 80;\n    listen [::]:80;\n    server_name {hostname};\n\n    location / {{\n        proxy_pass http://127.0.0.1:{agent_port};\n        proxy_http_version 1.1;\n        proxy_set_header Host $host;\n        proxy_set_header X-Real-IP $remote_addr;\n        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;\n        proxy_set_header X-Forwarded-Proto http;\n        proxy_set_header X-Forwarded-Host $host;\n        proxy_set_header X-Forwarded-Port 80;\n        proxy_set_header Upgrade $http_upgrade;\n        proxy_set_header Connection \"upgrade\";\n        proxy_read_timeout 3600s;\n        proxy_send_timeout 3600s;\n        proxy_connect_timeout 60s;\n    }}\n}}\n"
    )
}

fn reload_nginx() -> Result<()> {
    let test = Command::new("nginx")
        .args(["-t"])
        .status()
        .map_err(|exc| Error::Other(anyhow!("failed to run nginx -t: {}", exc)))?;
    if !test.success() {
        return Err(Error::Other(anyhow!("nginx -t failed")));
    }

    let reload_attempts = [
        vec!["systemctl", "reload", "nginx"],
        vec!["service", "nginx", "reload"],
        vec!["nginx", "-s", "reload"],
    ];
    for attempt in reload_attempts {
        let mut cmd = Command::new(attempt[0]);
        cmd.args(&attempt[1..]);
        if let Ok(status) = cmd.status() {
            if status.success() {
                return Ok(());
            }
        }
    }
    Err(Error::Other(anyhow!("failed to reload nginx")))
}

#[derive(Clone, Debug)]
struct NpmContainer {
    name: String,
}

fn detect_npm_container() -> Result<Option<NpmContainer>> {
    let output = Command::new("docker")
        .args(["ps", "--format", "{{.Names}}\t{{.Image}}"])
        .output()
        .map_err(|exc| Error::Other(anyhow!("failed to list docker containers: {}", exc)))?;
    if !output.status.success() {
        return Ok(None);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_npm_container_from_output(&stdout))
}

fn npm_proxy_host_paths_from_container(container: &NpmContainer) -> Result<Vec<PathBuf>> {
    let output = Command::new("docker")
        .args(["inspect", &container.name, "--format", "{{json .Mounts}}"])
        .output()
        .map_err(|exc| {
            Error::Other(anyhow!(
                "failed to inspect nginx-proxy-manager mounts: {}",
                exc
            ))
        })?;
    if !output.status.success() {
        return Ok(Vec::new());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mounts: serde_json::Value =
        serde_json::from_str(stdout.trim()).unwrap_or(serde_json::Value::Null);
    let mut paths = Vec::new();
    let Some(items) = mounts.as_array() else {
        return Ok(paths);
    };
    for item in items {
        let destination = item
            .get("Destination")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let source = item
            .get("Source")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        if source.is_empty() {
            continue;
        }
        if destination == "/data" {
            paths.push(PathBuf::from(source).join("nginx/proxy_host"));
        } else if destination == "/data/nginx/proxy_host" {
            paths.push(PathBuf::from(source));
        }
    }
    Ok(paths)
}

fn parse_npm_container_from_output(stdout: &str) -> Option<NpmContainer> {
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.splitn(2, '\t');
        let name = parts.next().unwrap_or("").trim();
        let image = parts.next().unwrap_or("").trim().to_ascii_lowercase();
        let lowered_name = name.to_ascii_lowercase();
        if image.contains("nginx-proxy-manager")
            || lowered_name.contains("nginx-proxy-manager")
            || lowered_name == "npm"
        {
            return Some(NpmContainer {
                name: name.to_string(),
            });
        }
    }
    None
}

fn build_npm_upsert_script(hostname: &str, agent_port: u16) -> String {
    let hostname_json = serde_json::to_string(hostname).unwrap_or_else(|_| "\"\"".to_string());
    format!(
        r#"
import proxyHostModel from "./models/proxy_host.js";
import userModel from "./models/user.js";
import internalHost from "./internal/host.js";
import internalNginx from "./internal/nginx.js";

const HOSTNAME = {hostname_json};
const TARGET = {{
  forward_host: "127.0.0.1",
  forward_port: {agent_port},
  forward_scheme: "http",
}};

const allHosts = await proxyHostModel.query().where("is_deleted", 0);
let existing = allHosts.find((item) =>
  Array.isArray(item.domain_names) &&
  item.domain_names.some((value) => String(value || "").toLowerCase() === HOSTNAME.toLowerCase())
);

if (existing) {{
  const ownedByD1v = existing.meta && existing.meta.d1v_managed === true;
  const alreadyMatches =
    String(existing.forward_host || "") === TARGET.forward_host &&
    Number(existing.forward_port || 0) === TARGET.forward_port &&
    String(existing.forward_scheme || "") === TARGET.forward_scheme;
  if (!ownedByD1v && !alreadyMatches) {{
    throw new Error(`hostname ${{HOSTNAME}} already exists in nginx-proxy-manager and is not managed by d1v`);
  }}
}}

if (!existing) {{
  const taken = await internalHost.isHostnameTaken(HOSTNAME);
  if (taken.is_taken) {{
    throw new Error(`hostname ${{HOSTNAME}} is already taken in nginx-proxy-manager`);
  }}
  const owner = await userModel.query().where("is_deleted", 0).orderBy("id", "asc").first();
  if (!owner) {{
    throw new Error("no nginx-proxy-manager owner user found");
  }}
  existing = await proxyHostModel.query().insertAndFetch({{
    owner_user_id: owner.id,
    domain_names: [HOSTNAME],
    forward_host: TARGET.forward_host,
    forward_port: TARGET.forward_port,
    forward_scheme: TARGET.forward_scheme,
    access_list_id: 0,
    certificate_id: 0,
    ssl_forced: false,
    caching_enabled: false,
    block_exploits: false,
    allow_websocket_upgrade: true,
    http2_support: false,
    enabled: true,
    hsts_enabled: false,
    hsts_subdomains: false,
    trust_forwarded_proto: false,
    advanced_config: "",
    locations: [],
    meta: {{
      d1v_managed: true,
      d1v_target: `${{TARGET.forward_host}}:${{TARGET.forward_port}}`,
    }},
  }});
}} else {{
  await proxyHostModel.query().where("id", existing.id).patch({{
    forward_host: TARGET.forward_host,
    forward_port: TARGET.forward_port,
    forward_scheme: TARGET.forward_scheme,
    allow_websocket_upgrade: true,
    enabled: true,
    meta: {{
      ...(existing.meta || {{}}),
      d1v_managed: true,
      d1v_target: `${{TARGET.forward_host}}:${{TARGET.forward_port}}`,
    }},
  }});
}}

const row = await proxyHostModel
  .query()
  .where("id", existing.id)
  .allowGraph(proxyHostModel.defaultAllowGraph)
  .withGraphFetched(`[${{proxyHostModel.defaultExpand.join(", ")}}]`)
  .first();

await internalNginx.configure(proxyHostModel, "proxy_host", row);
console.log(JSON.stringify({{
  id: row.id,
  hostname: HOSTNAME,
  config_path: `/data/nginx/proxy_host/${{row.id}}.conf`,
}}));
"#
    )
}

fn parse_traefik_hosts(rule: &str) -> Vec<String> {
    let mut hosts = Vec::new();
    let mut rest = rule;
    while let Some(idx) = rest.find("Host(") {
        let after = &rest[idx + 5..];
        if let Some(end) = after.find(')') {
            let inner = &after[..end];
            for part in inner.split(',') {
                let host = sanitize_host(part);
                if !host.is_empty() {
                    hosts.push(host);
                }
            }
            rest = &after[end + 1..];
        } else {
            break;
        }
    }
    hosts
}

fn choose_hostname(
    hosts: &[String],
    preferred_hostname: Option<&str>,
    prefer_https: bool,
) -> Option<String> {
    if let Some(preferred) = preferred_hostname {
        let preferred = preferred.trim().trim_end_matches('.').to_ascii_lowercase();
        if let Some(found) = hosts.iter().find(|value| value.as_str() == preferred) {
            return Some(found.clone());
        }
    }
    let mut filtered: Vec<String> = hosts
        .iter()
        .filter(|value| value.contains('.'))
        .cloned()
        .collect();
    filtered.sort();
    if prefer_https {
        filtered
            .into_iter()
            .next()
            .or_else(|| hosts.first().cloned())
    } else {
        filtered
            .into_iter()
            .next()
            .or_else(|| hosts.first().cloned())
    }
}

fn is_local_host(host: &str) -> bool {
    matches!(
        host.trim().to_ascii_lowercase().as_str(),
        "127.0.0.1" | "localhost" | "0.0.0.0" | "host.docker.internal"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn parse_caddy_https_reverse_proxy() {
        let content = indoc! {r#"
            https://node-a.example.com {
                reverse_proxy 127.0.0.1:8080
            }
        "#};

        let detected =
            parse_caddy_content(content, "/etc/caddy/Caddyfile".to_string(), 8080, None).unwrap();

        assert_eq!(detected.provider, "caddy");
        assert_eq!(detected.hostname, "node-a.example.com");
        assert_eq!(detected.scheme, "https");
        assert_eq!(detected.external_port, 443);
    }

    #[test]
    fn parse_nginx_https_proxy_pass() {
        let content = indoc! {r#"
            server {
                listen 443 ssl;
                server_name node-b.example.com;
                location / {
                    proxy_pass http://127.0.0.1:8080;
                }
            }
        "#};

        let detected = parse_nginx_like_content(
            "nginx",
            content,
            "/etc/nginx/sites-enabled/node.conf".to_string(),
            8080,
            None,
        )
        .unwrap();

        assert_eq!(detected.provider, "nginx");
        assert_eq!(detected.hostname, "node-b.example.com");
        assert_eq!(detected.scheme, "https");
        assert_eq!(detected.external_port, 443);
    }

    #[test]
    fn parse_npm_non_default_port() {
        let content = indoc! {r#"
            server {
                listen 8088;
                server_name npm-node.example.com;
                location / {
                    proxy_pass http://localhost:8080;
                }
            }
        "#};

        let detected = parse_nginx_like_content(
            "npm",
            content,
            "/data/nginx/proxy_host/1.conf".to_string(),
            8080,
            None,
        )
        .unwrap();

        assert_eq!(detected.provider, "npm");
        assert_eq!(detected.hostname, "npm-node.example.com");
        assert_eq!(detected.scheme, "http");
        assert_eq!(detected.external_port, 8088);
    }

    #[test]
    fn parse_traefik_host_rule() {
        let hosts = parse_traefik_hosts("Host(`node-c.example.com`,`node-c2.example.com`)");
        assert_eq!(hosts, vec!["node-c.example.com", "node-c2.example.com"]);
    }

    #[test]
    fn preferred_hostname_wins() {
        let hosts = vec![
            "b.example.com".to_string(),
            "a.example.com".to_string(),
            "c.example.com".to_string(),
        ];
        let selected = choose_hostname(&hosts, Some("c.example.com"), true).unwrap();
        assert_eq!(selected, "c.example.com");
    }

    #[test]
    fn parse_npm_container_prefers_nginx_proxy_manager_image() {
        let parsed = parse_npm_container_from_output(
            "npm-app\tjc21/nginx-proxy-manager:latest\nother\tnginx:latest\n",
        )
        .unwrap();
        assert_eq!(parsed.name, "npm-app");
    }

    #[test]
    fn safe_file_slug_normalizes_hostname() {
        assert_eq!(
            safe_file_slug("Runtime.Node.Example.COM"),
            "runtime-node-example-com"
        );
    }

    #[test]
    fn render_managed_nginx_config_contains_proxy_headers() {
        let rendered = render_managed_nginx_config("node.example.com", 8080);
        assert!(rendered.contains("server_name node.example.com;"));
        assert!(rendered.contains("proxy_pass http://127.0.0.1:8080;"));
        assert!(rendered.contains("proxy_set_header Upgrade $http_upgrade;"));
    }

    #[test]
    fn npm_mount_paths_follow_data_bind_mount() {
        let mounts = serde_json::json!([
            {
                "Source": "/srv/npm-data",
                "Destination": "/data"
            }
        ]);
        let parsed = mounts
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|item| {
                let destination = item.get("Destination")?.as_str()?;
                let source = item.get("Source")?.as_str()?;
                if destination == "/data" {
                    Some(PathBuf::from(source).join("nginx/proxy_host"))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(
            parsed,
            vec![PathBuf::from("/srv/npm-data").join("nginx/proxy_host")]
        );
    }
}
