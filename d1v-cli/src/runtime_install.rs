use std::fs::{self, File};
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};

use clap::Args;
use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, USER_AGENT};
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tar::Archive;
use url::Url;

use crate::Context;
use crate::config::Config;
use crate::error::{Error, Result};
use crate::output::Format;

const DEFAULT_RUNTIME_REPO: &str = "d1vai/opcode-api-runtime";
const RUNTIME_ARCHIVE_BASENAME: &str = "opcode-api";
const RUNTIME_CHECKSUM_FILE: &str = "checksums.txt";

#[derive(Debug, Args, Clone)]
pub struct InstallRuntimeArgs {
    /// Only check whether the runtime is available and compatible
    #[arg(long)]
    pub check: bool,
    /// Install a specific release tag instead of the latest release
    #[arg(long, value_name = "TAG")]
    pub version: Option<String>,
    /// Override install destination for opcode-api binary
    #[arg(long, value_name = "PATH")]
    pub destination: Option<PathBuf>,
}

#[derive(Debug, Args, Clone)]
pub struct UpgradeRuntimeArgs {
    /// Install a specific release tag instead of the latest release
    #[arg(long, value_name = "TAG")]
    pub version: Option<String>,
}

#[derive(Debug, Args, Clone, Default)]
pub struct DoctorArgs {}

#[derive(Debug, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum RuntimeInstallResult {
    UpToDate {
        version: String,
        executable_path: String,
    },
    InstallAvailable {
        current_version: Option<String>,
        target_version: String,
        executable_path: String,
    },
    Installed {
        previous_version: Option<String>,
        target_version: String,
        executable_path: String,
    },
    Doctor {
        executable_path: String,
        exists: bool,
        healthy: bool,
        configured_home: Option<String>,
        configured_device_id: Option<String>,
        current_version: Option<String>,
        latest_version: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

#[derive(Debug, Deserialize)]
struct RuntimeManifest {
    version: String,
    assets: Vec<RuntimeManifestAsset>,
}

#[derive(Debug, Deserialize)]
struct RuntimeManifestAsset {
    target: String,
    url: String,
    sha256: String,
    archive_name: Option<String>,
}

#[derive(Debug)]
struct ReleaseInfo {
    current_version: Option<String>,
    target_version: String,
    executable_path: PathBuf,
    source_label: String,
    archive_name: String,
    archive_url: String,
    checksum_url: Option<String>,
    checksum_sha256: Option<String>,
}

struct WorkingPaths {
    workspace_dir: PathBuf,
    archive_path: PathBuf,
    extracted_dir: PathBuf,
}

pub async fn run_install(ctx: &Context, args: InstallRuntimeArgs) -> Result<()> {
    let release = fetch_release_info(args.version.as_deref(), args.destination.as_deref()).await?;
    let needs_install = match &release.current_version {
        Some(current) => requires_install(current, &release.target_version, args.version.is_some()),
        None => true,
    };

    if !needs_install {
        let result = RuntimeInstallResult::UpToDate {
            version: release
                .current_version
                .clone()
                .unwrap_or_else(|| release.target_version.clone()),
            executable_path: release.executable_path.display().to_string(),
        };
        if matches!(ctx.output.format, Format::Text) {
            ctx.success(format!(
                "opcode-api runtime already installed at {}",
                release.executable_path.display()
            ));
            return Ok(());
        }
        return ctx.present(crate::text::Text::new(), &result);
    }

    if args.check {
        let result = RuntimeInstallResult::InstallAvailable {
            current_version: release.current_version.clone(),
            target_version: release.target_version.clone(),
            executable_path: release.executable_path.display().to_string(),
        };
        if matches!(ctx.output.format, Format::Text) {
            ctx.info(format!(
                "opcode-api runtime {} is available for install",
                release.target_version
            ));
            return Ok(());
        }
        return ctx.present(crate::text::Text::new(), &result);
    }

    install_release(ctx, &release).await
}

pub async fn run_upgrade(ctx: &Context, args: UpgradeRuntimeArgs) -> Result<()> {
    run_install(
        ctx,
        InstallRuntimeArgs {
            check: false,
            version: args.version,
            destination: None,
        },
    )
    .await
}

pub async fn run_doctor(ctx: &Context, _args: DoctorArgs) -> Result<()> {
    let binary = default_runtime_binary_path()?;
    let exists = binary.exists();
    let health_client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(other)?;
    let health = health_client
        .get("http://127.0.0.1:9191/health")
        .send()
        .await;
    let healthy = matches!(health, Ok(resp) if resp.status().is_success());
    let current_version = installed_runtime_version(&binary).await;
    let latest_version = available_runtime_update(Some(&binary))
        .await?
        .or_else(|| current_version.as_ref().and_then(|_| None));

    let config = crate::agent::load_runtime_doctor_config()?;
    let result = RuntimeInstallResult::Doctor {
        executable_path: binary.display().to_string(),
        exists,
        healthy,
        configured_home: config.home_root,
        configured_device_id: Some(config.device_id),
        current_version: current_version.clone(),
        latest_version: latest_version.clone(),
    };

    if matches!(ctx.output.format, Format::Text) {
        ctx.message(format!("opcode-api binary: {}", binary.display()));
        ctx.message(format!("installed: {}", if exists { "yes" } else { "no" }));
        ctx.message(format!("healthy: {}", if healthy { "yes" } else { "no" }));
        if let Some(version) = current_version {
            ctx.message(format!("version: {version}"));
        }
        if let Some(version) = latest_version {
            ctx.message(format!(
                "update available: {version} (run `d1v runtime upgrade`)"
            ));
        }
        if let Some(home) = result_configured_home(&result) {
            ctx.message(format!("agent home: {}", home));
        }
        if let Some(device_id) = result_configured_device_id(&result) {
            ctx.message(format!("device id: {}", device_id));
        }
        return Ok(());
    }

    ctx.present(crate::text::Text::new(), &result)
}

pub async fn ensure_runtime_installed(
    ctx: &Context,
    destination: Option<&Path>,
) -> Result<PathBuf> {
    let binary = destination
        .map(PathBuf::from)
        .unwrap_or(default_runtime_binary_path()?);
    if binary.exists() {
        return Ok(binary);
    }

    run_install(
        ctx,
        InstallRuntimeArgs {
            check: false,
            version: None,
            destination: Some(binary.clone()),
        },
    )
    .await?;

    Ok(binary)
}

pub async fn available_runtime_update(destination: Option<&Path>) -> Result<Option<String>> {
    let release = fetch_release_info(None, destination).await?;
    let Some(current) = release.current_version.as_deref() else {
        return Ok(Some(release.target_version));
    };
    if requires_install(current, &release.target_version, false) {
        return Ok(Some(release.target_version));
    }
    Ok(None)
}

pub fn default_runtime_binary_path() -> Result<PathBuf> {
    Ok(Config::dir()?.join("bin").join("opcode-api"))
}

async fn install_release(ctx: &Context, release: &ReleaseInfo) -> Result<()> {
    if matches!(ctx.output.format, Format::Text) {
        ctx.info(format!(
            "installing opcode-api runtime {}",
            release.target_version
        ));
    }

    let paths = working_paths(&release.executable_path);
    prepare_working_paths(&paths)?;
    let archive_path =
        download_archive(release, &paths, matches!(ctx.output.format, Format::Text)).await?;
    verify_checksum(&archive_path, release).await?;
    let extracted_binary = extract_binary(&archive_path, &paths)?;

    if let Some(parent) = release.executable_path.parent() {
        fs::create_dir_all(parent)?;
    }
    install_binary(&extracted_binary, &release.executable_path)?;
    let _ = fs::remove_dir_all(&paths.workspace_dir);

    let result = RuntimeInstallResult::Installed {
        previous_version: release.current_version.clone(),
        target_version: release.target_version.clone(),
        executable_path: release.executable_path.display().to_string(),
    };

    if matches!(ctx.output.format, Format::Text) {
        ctx.success(format!(
            "installed opcode-api runtime {} to {}",
            release.target_version,
            release.executable_path.display()
        ));
        return Ok(());
    }

    ctx.present(crate::text::Text::new(), &result)
}

async fn fetch_release_info(
    version: Option<&str>,
    destination: Option<&Path>,
) -> Result<ReleaseInfo> {
    let executable_path = match destination {
        Some(path) => path.to_path_buf(),
        None => default_runtime_binary_path()?,
    };
    let current_version = installed_runtime_version(&executable_path).await;
    let target = current_target()?;
    let explicit_manifest = std::env::var("D1V_OPCODE_INSTALL_MANIFEST_URL").ok();
    if version.is_none() && explicit_manifest.is_some() {
        return fetch_manifest_release_info(&executable_path, current_version, &target).await;
    }
    fetch_github_release_info(version, executable_path, current_version, &target).await
}

async fn fetch_manifest_release_info(
    executable_path: &Path,
    current_version: Option<String>,
    target: &str,
) -> Result<ReleaseInfo> {
    let manifest_url = std::env::var("D1V_OPCODE_INSTALL_MANIFEST_URL")
        .map_err(|_| other(anyhow::anyhow!("missing D1V_OPCODE_INSTALL_MANIFEST_URL")))?;
    let client = http_client_for_url("d1v-cli-runtime-installer", &manifest_url, "*/*")?;
    let manifest = client
        .get(&manifest_url)
        .send()
        .await
        .map_err(other)?
        .error_for_status()
        .map_err(other)?
        .json::<RuntimeManifest>()
        .await
        .map_err(other)?;
    let asset = manifest
        .assets
        .into_iter()
        .find(|asset| asset.target == target)
        .ok_or_else(|| other(anyhow::anyhow!("missing runtime asset for target {target}")))?;
    let archive_url = resolve_url(&manifest_url, &asset.url)?;
    let archive_name = asset
        .archive_name
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| archive_file_name(&archive_url));
    Ok(ReleaseInfo {
        current_version,
        target_version: normalize_tag(&manifest.version),
        executable_path: executable_path.to_path_buf(),
        source_label: manifest_url,
        archive_name,
        archive_url,
        checksum_url: None,
        checksum_sha256: Some(asset.sha256),
    })
}

async fn fetch_github_release_info(
    version: Option<&str>,
    executable_path: PathBuf,
    current_version: Option<String>,
    target: &str,
) -> Result<ReleaseInfo> {
    let repo = std::env::var("D1V_OPCODE_INSTALL_REPO")
        .unwrap_or_else(|_| DEFAULT_RUNTIME_REPO.to_string());
    let archive_name = format!("{RUNTIME_ARCHIVE_BASENAME}-{target}.tar.gz");
    let target_version = match version {
        Some(version) => normalize_tag(version),
        None => fetch_latest_tag(&repo).await?,
    };
    let base_url = format!("https://github.com/{repo}/releases/download/{target_version}");

    Ok(ReleaseInfo {
        current_version,
        target_version,
        executable_path,
        source_label: repo,
        archive_name: archive_name.clone(),
        archive_url: format!("{base_url}/{archive_name}"),
        checksum_url: Some(format!("{base_url}/{RUNTIME_CHECKSUM_FILE}")),
        checksum_sha256: None,
    })
}

async fn fetch_latest_tag(repo: &str) -> Result<String> {
    let client = github_client("d1v-cli-runtime-installer")?;
    let release = client
        .get(format!(
            "https://api.github.com/repos/{repo}/releases/latest"
        ))
        .send()
        .await
        .map_err(other)?
        .error_for_status()
        .map_err(other)?
        .json::<GitHubRelease>()
        .await
        .map_err(other)?;
    Ok(release.tag_name)
}

async fn installed_runtime_version(path: &Path) -> Option<String> {
    if !path.exists() {
        return None;
    }
    let output = std::process::Command::new(path)
        .arg("--version")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let token = stdout.split_whitespace().nth(1)?;
    Some(token.trim().to_string())
}

async fn download_archive(
    release: &ReleaseInfo,
    paths: &WorkingPaths,
    show_progress: bool,
) -> Result<PathBuf> {
    let client = http_client_for_url(
        "d1v-cli-runtime-installer",
        &release.archive_url,
        "application/octet-stream",
    )?;
    let response = client
        .get(&release.archive_url)
        .send()
        .await
        .map_err(other)?
        .error_for_status()
        .map_err(other)?;

    let total = response.content_length();
    let mut file = File::create(&paths.archive_path)?;
    let mut stream = response.bytes_stream();
    let mut downloaded = 0_u64;
    let progress = progress_bar(total, show_progress);

    use futures_util::StreamExt;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(other)?;
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;
        if let Some(bar) = &progress {
            bar.set_position(downloaded);
        }
    }

    file.flush()?;

    if let Some(bar) = progress {
        if total.is_some() {
            bar.finish_with_message("download complete");
        } else {
            bar.finish_and_clear();
        }
    }

    Ok(paths.archive_path.clone())
}

async fn verify_checksum(archive_path: &Path, release: &ReleaseInfo) -> Result<()> {
    let actual = sha256_file(archive_path)?;
    if let Some(expected) = &release.checksum_sha256 {
        if actual.eq_ignore_ascii_case(expected) {
            return Ok(());
        }
        return Err(Error::Other(anyhow::anyhow!(
            "checksum verification failed for {} from {}",
            release.archive_name,
            release.source_label
        )));
    }

    let checksum_url = release
        .checksum_url
        .as_ref()
        .ok_or_else(|| other(anyhow::anyhow!("missing checksum url")))?;
    let client = http_client_for_url("d1v-cli-runtime-installer", checksum_url, "text/plain")?;
    let checksum_body = client
        .get(checksum_url)
        .send()
        .await
        .map_err(other)?
        .error_for_status()
        .map_err(other)?
        .text()
        .await
        .map_err(other)?;

    let expected = parse_checksum(&checksum_body, &release.archive_name).ok_or_else(|| {
        Error::Other(anyhow::anyhow!(
            "missing checksum for {}",
            release.archive_name
        ))
    })?;

    if actual.eq_ignore_ascii_case(&expected) {
        return Ok(());
    }

    Err(Error::Other(anyhow::anyhow!(
        "checksum verification failed for {} from {}",
        release.archive_name,
        release.source_label
    )))
}

fn extract_binary(archive_path: &Path, paths: &WorkingPaths) -> Result<PathBuf> {
    let file = File::open(archive_path)?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    archive.unpack(&paths.extracted_dir)?;

    let extracted = paths.extracted_dir.join("opcode-api");
    if extracted.is_file() {
        Ok(extracted)
    } else {
        Err(Error::Other(anyhow::anyhow!(
            "archive did not contain expected opcode-api binary"
        )))
    }
}

fn install_binary(source: &Path, destination: &Path) -> Result<()> {
    ensure_executable(source)?;

    let replacement = destination.with_extension("new");
    fs::copy(source, &replacement)?;
    ensure_executable(&replacement)?;
    fs::rename(&replacement, destination)?;
    Ok(())
}

fn working_paths(executable_path: &Path) -> WorkingPaths {
    let base_dir = executable_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let workspace_dir = base_dir.join(".opcode-runtime-upgrade");

    WorkingPaths {
        archive_path: workspace_dir.join("release.tar.gz"),
        extracted_dir: workspace_dir.join("extract"),
        workspace_dir,
    }
}

fn prepare_working_paths(paths: &WorkingPaths) -> Result<()> {
    if paths.workspace_dir.exists() {
        fs::remove_dir_all(&paths.workspace_dir)?;
    }
    fs::create_dir_all(&paths.extracted_dir)?;
    Ok(())
}

fn current_target() -> Result<String> {
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" | "arm64" => "aarch64",
        other => {
            return Err(Error::Other(anyhow::anyhow!(
                "unsupported architecture: {other}"
            )));
        }
    };

    let os = match std::env::consts::OS {
        "macos" => "apple-darwin",
        "linux" => "unknown-linux-musl",
        other => return Err(Error::Other(anyhow::anyhow!("unsupported OS: {other}"))),
    };

    Ok(format!("{arch}-{os}"))
}

fn github_client(user_agent: &str) -> Result<reqwest::Client> {
    generic_http_client_with_accept(user_agent, "application/vnd.github+json")
}

fn http_client_for_url(user_agent: &str, url: &str, accept: &str) -> Result<reqwest::Client> {
    let mut builder = generic_http_client_builder(user_agent, accept)?;
    if is_loopback_url(url) {
        builder = builder.no_proxy();
    }
    builder.build().map_err(other)
}

fn generic_http_client_with_accept(user_agent: &str, accept: &str) -> Result<reqwest::Client> {
    generic_http_client_builder(user_agent, accept)?
        .build()
        .map_err(other)
}

fn generic_http_client_builder(user_agent: &str, accept: &str) -> Result<reqwest::ClientBuilder> {
    let mut headers = HeaderMap::new();
    headers.insert(
        ACCEPT,
        HeaderValue::from_str(accept).map_err(|err| Error::Other(anyhow::Error::new(err)))?,
    );
    headers.insert(
        USER_AGENT,
        HeaderValue::from_str(user_agent).map_err(|err| Error::Other(anyhow::Error::new(err)))?,
    );

    Ok(reqwest::Client::builder().default_headers(headers))
}

fn resolve_url(base: &str, value: &str) -> Result<String> {
    if value.starts_with("http://") || value.starts_with("https://") {
        return Ok(value.to_string());
    }
    let base = Url::parse(base).map_err(other)?;
    Ok(base.join(value).map_err(other)?.to_string())
}

fn archive_file_name(url: &str) -> String {
    Url::parse(url)
        .ok()
        .and_then(|parsed| {
            parsed
                .path_segments()
                .and_then(|segments| segments.last().map(str::to_string))
        })
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("{RUNTIME_ARCHIVE_BASENAME}.tar.gz"))
}

fn is_loopback_url(url: &str) -> bool {
    Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_string))
        .is_some_and(|host| host == "127.0.0.1" || host == "localhost")
}

fn parse_checksum(body: &str, archive_name: &str) -> Option<String> {
    body.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let checksum = parts.next()?;
        let name = parts.next()?.trim_start_matches('*');
        (name == archive_name).then(|| checksum.to_string())
    })
}

fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0_u8; 8192];
    loop {
        let read = file.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    let digest = hasher.finalize();
    Ok(format!("{:x}", base16ct::HexDisplay(&digest)))
}

fn ensure_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)?;
    }
    Ok(())
}

fn progress_bar(total: Option<u64>, show_progress: bool) -> Option<ProgressBar> {
    if !show_progress || !io::stderr().is_terminal() {
        return None;
    }

    let bar = match total {
        Some(total) => ProgressBar::new(total),
        None => ProgressBar::new_spinner(),
    };

    let style = if total.is_some() {
        ProgressStyle::with_template(
            "{spinner:.cyan} {bytes}/{total_bytes} [{bar:40.cyan/blue}] {percent:>3}%",
        )
    } else {
        ProgressStyle::with_template("{spinner:.cyan} {bytes} downloaded")
    }
    .unwrap_or_else(|_| ProgressStyle::default_bar())
    .progress_chars("=> ");

    bar.set_style(style);
    Some(bar)
}

fn normalize_version(version: &str) -> &str {
    version.trim_start_matches(['v', 'V'])
}

fn normalize_tag(version: &str) -> String {
    let trimmed = version.trim();
    if trimmed.starts_with(['v', 'V']) {
        trimmed.to_string()
    } else {
        format!("v{}", trimmed)
    }
}

fn is_upgrade_available(current: &str, latest: &str) -> bool {
    let current = normalize_version(current);
    let latest = normalize_version(latest);

    match (Version::parse(current), Version::parse(latest)) {
        (Ok(current), Ok(latest)) => latest > current,
        _ => latest != current,
    }
}

fn versions_match(current: &str, target: &str) -> bool {
    normalize_version(current) == normalize_version(target)
}

fn requires_install(current: &str, target: &str, explicit_version: bool) -> bool {
    if explicit_version {
        !versions_match(current, target)
    } else {
        is_upgrade_available(current, target)
    }
}

fn result_configured_home(result: &RuntimeInstallResult) -> Option<&str> {
    match result {
        RuntimeInstallResult::Doctor {
            configured_home, ..
        } => configured_home.as_deref(),
        _ => None,
    }
}

fn result_configured_device_id(result: &RuntimeInstallResult) -> Option<&str> {
    match result {
        RuntimeInstallResult::Doctor {
            configured_device_id,
            ..
        } => configured_device_id.as_deref(),
        _ => None,
    }
}

fn other(err: impl Into<anyhow::Error>) -> Error {
    Error::Other(err.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_checksum_lines() {
        let body = "abc123  opcode-api-aarch64-apple-darwin.tar.gz\nfff999  other.tar.gz\n";
        assert_eq!(
            parse_checksum(body, "opcode-api-aarch64-apple-darwin.tar.gz").as_deref(),
            Some("abc123")
        );
        assert_eq!(parse_checksum(body, "missing.tar.gz"), None);
    }

    #[test]
    fn normalizes_tags() {
        assert_eq!(normalize_tag("0.1.3"), "v0.1.3");
        assert_eq!(normalize_tag("v0.1.3"), "v0.1.3");
    }

    #[test]
    fn compares_versions() {
        assert!(is_upgrade_available("0.1.0", "v0.2.0"));
        assert!(!is_upgrade_available("v0.2.0", "0.2.0"));
    }

    #[test]
    fn resolves_relative_urls_against_manifest() {
        let url = resolve_url(
            "https://runtime.example.com/opcode-api/manifest.json",
            "v0.1.0/opcode-api-aarch64-apple-darwin.tar.gz",
        )
        .unwrap();
        assert_eq!(
            url,
            "https://runtime.example.com/opcode-api/v0.1.0/opcode-api-aarch64-apple-darwin.tar.gz"
        );
    }

    #[test]
    fn derives_archive_name_from_url() {
        assert_eq!(
            archive_file_name(
                "https://runtime.example.com/opcode-api/v0.1.0/opcode-api-aarch64-apple-darwin.tar.gz"
            ),
            "opcode-api-aarch64-apple-darwin.tar.gz"
        );
    }

    #[test]
    fn detects_loopback_urls() {
        assert!(is_loopback_url("http://127.0.0.1:18765/manifest.json"));
        assert!(is_loopback_url("http://localhost:18765/manifest.json"));
        assert!(!is_loopback_url(
            "https://runtime.example.com/opcode-api/manifest.json"
        ));
    }
}
