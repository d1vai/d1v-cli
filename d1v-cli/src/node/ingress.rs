use crate::error::{Error, Result};
use anyhow::anyhow;
use std::fs;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::Command;

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

fn detect_npm_ingress(
    agent_port: u16,
    preferred_hostname: Option<&str>,
) -> Result<Option<DetectedIngress>> {
    let paths = collect_nginx_paths(&[
        "/data/nginx/proxy_host",
        "/opt/nginx-proxy-manager/nginx/proxy_host",
        "/var/lib/docker/volumes/nginx-proxy-manager_data/_data/nginx/proxy_host",
    ])?;
    parse_nginx_like_paths("npm", &paths, agent_port, preferred_hostname)
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
                if !is_local_host(&host) || port != agent_port {
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
}
