use std::fs::{self, File};
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};

use clap::Args;
use flate2::read::GzDecoder;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, USER_AGENT};
use semver::Version;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tar::Archive;

use crate::error::{Error, Result};
use crate::output::Format;
use crate::{Context, t};

const DEFAULT_REPO: &str = "d1vai/d1v-cli";
const ARCHIVE_BASENAME: &str = "d1v";
const CHECKSUM_FILE: &str = "checksums.txt";

#[derive(Debug, Args, Clone)]
pub struct UpgradeArgs {
    /// Only check whether a newer version is available
    #[arg(long)]
    pub check: bool,
    /// Install a specific release tag instead of the latest release
    #[arg(long, value_name = "TAG")]
    pub version: Option<String>,
}

#[derive(Debug, Args, Clone)]
pub struct UninstallArgs {
    /// Leave shell rc PATH entries untouched
    #[arg(long)]
    pub keep_path: bool,
}

#[derive(Debug, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum UpgradeResult {
    UpToDate {
        current_version: String,
        target_version: String,
        executable_path: String,
    },
    UpgradeAvailable {
        current_version: String,
        target_version: String,
        executable_path: String,
    },
    Upgraded {
        previous_version: String,
        target_version: String,
        executable_path: String,
    },
    Uninstalled {
        executable_path: String,
    },
}

#[derive(Debug, serde::Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

#[derive(Debug)]
struct ReleaseInfo {
    current_version: String,
    target_version: String,
    executable_path: PathBuf,
    repo: String,
    archive_name: String,
    archive_url: String,
    checksum_url: String,
}

struct WorkingPaths {
    workspace_dir: PathBuf,
    archive_path: PathBuf,
    extracted_dir: PathBuf,
}

pub async fn run(ctx: &Context, args: UpgradeArgs) -> Result<()> {
    let release = fetch_release_info(args.version.as_deref()).await?;
    let needs_install = requires_install(
        &release.current_version,
        &release.target_version,
        args.version.is_some(),
    );

    if !needs_install {
        let result = UpgradeResult::UpToDate {
            current_version: release.current_version.clone(),
            target_version: release.target_version.clone(),
            executable_path: release.executable_path.display().to_string(),
        };

        if matches!(ctx.output.format, Format::Text) {
            ctx.info(t!(
                "upgrade-up-to-date",
                version = release.current_version.as_str()
            ));
            return Ok(());
        }

        return ctx.present(crate::text::Text::new(), &result);
    }

    if args.check {
        let result = UpgradeResult::UpgradeAvailable {
            current_version: release.current_version.clone(),
            target_version: release.target_version.clone(),
            executable_path: release.executable_path.display().to_string(),
        };

        if matches!(ctx.output.format, Format::Text) {
            ctx.info(t!(
                "upgrade-available",
                current = release.current_version.as_str(),
                latest = release.target_version.as_str()
            ));
            return Ok(());
        }

        return ctx.present(crate::text::Text::new(), &result);
    }

    if matches!(ctx.output.format, Format::Text) {
        ctx.info(t!(
            "upgrade-available",
            current = release.current_version.as_str(),
            latest = release.target_version.as_str()
        ));
        ctx.info(t!(
            "upgrade-downloading",
            version = release.target_version.as_str()
        ));
    }

    let paths = working_paths(&release.executable_path);
    prepare_working_paths(&paths)?;
    let archive_path =
        download_archive(&release, &paths, matches!(ctx.output.format, Format::Text)).await?;

    if matches!(ctx.output.format, Format::Text) {
        ctx.info(t!("upgrade-verifying"));
    }

    verify_checksum(&archive_path, &release).await?;
    let extracted_binary = extract_binary(&archive_path, &paths)?;

    if matches!(ctx.output.format, Format::Text) {
        ctx.info(t!("upgrade-installing"));
    }

    install_binary(&extracted_binary, &release.executable_path)?;
    let _ = fs::remove_dir_all(&paths.workspace_dir);

    let result = UpgradeResult::Upgraded {
        previous_version: release.current_version.clone(),
        target_version: release.target_version.clone(),
        executable_path: release.executable_path.display().to_string(),
    };

    if matches!(ctx.output.format, Format::Text) {
        ctx.success(t!(
            "upgrade-success",
            version = release.target_version.as_str()
        ));
        return Ok(());
    }

    ctx.present(crate::text::Text::new(), &result)
}

pub fn run_uninstall(ctx: &Context, args: UninstallArgs) -> Result<()> {
    #[cfg(windows)]
    {
        let _ = (ctx, args);
        return Err(Error::Other(anyhow::anyhow!(
            "uninstall is not currently supported on Windows"
        )));
    }

    #[cfg(not(windows))]
    {
        let executable_path = std::env::current_exe()?;
        let install_dir = executable_path
            .parent()
            .ok_or_else(|| Error::Other(anyhow::anyhow!("executable path has no parent")))?;

        if !args.keep_path {
            cleanup_path_entries(install_dir)?;
        }

        fs::remove_file(&executable_path)?;

        let result = UpgradeResult::Uninstalled {
            executable_path: executable_path.display().to_string(),
        };

        if matches!(ctx.output.format, Format::Text) {
            ctx.success(t!("uninstall-success", path = executable_path.display()));
            return Ok(());
        }

        return ctx.present(crate::text::Text::new(), &result);
    }
}

async fn fetch_release_info(version: Option<&str>) -> Result<ReleaseInfo> {
    let repo = std::env::var("D1V_INSTALL_REPO").unwrap_or_else(|_| DEFAULT_REPO.to_string());
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let executable_path = std::env::current_exe()?;
    let target = current_target()?;
    let archive_name = format!("{ARCHIVE_BASENAME}-{target}.tar.gz");
    let target_version = match version {
        Some(version) => normalize_tag(version),
        None => fetch_latest_tag(&repo).await?,
    };
    let base_url = format!("https://github.com/{repo}/releases/download/{target_version}");

    Ok(ReleaseInfo {
        current_version,
        target_version,
        executable_path,
        repo,
        archive_name: archive_name.clone(),
        archive_url: format!("{base_url}/{archive_name}"),
        checksum_url: format!("{base_url}/{CHECKSUM_FILE}"),
    })
}

async fn fetch_latest_tag(repo: &str) -> Result<String> {
    let client = github_client()?;
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

async fn download_archive(
    release: &ReleaseInfo,
    paths: &WorkingPaths,
    show_progress: bool,
) -> Result<PathBuf> {
    let client = github_client()?;
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
            bar.finish_with_message(t!("upgrade-download-complete"));
        } else {
            bar.finish_and_clear();
        }
    }

    Ok(paths.archive_path.clone())
}

async fn verify_checksum(archive_path: &Path, release: &ReleaseInfo) -> Result<()> {
    let client = github_client()?;
    let checksum_body = client
        .get(&release.checksum_url)
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
    let actual = sha256_file(archive_path)?;

    if actual.eq_ignore_ascii_case(&expected) {
        return Ok(());
    }

    Err(Error::Other(anyhow::anyhow!(
        "checksum verification failed for {} from {}",
        release.archive_name,
        release.repo
    )))
}

fn extract_binary(archive_path: &Path, paths: &WorkingPaths) -> Result<PathBuf> {
    let file = File::open(archive_path)?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    archive.unpack(&paths.extracted_dir)?;

    let extracted = paths.extracted_dir.join("d1v");
    if extracted.is_file() {
        Ok(extracted)
    } else {
        Err(Error::Other(anyhow::anyhow!(
            "archive did not contain expected d1v binary"
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
    let workspace_dir = base_dir.join(".d1v-upgrade");

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
        "aarch64" => "aarch64",
        "arm64" => "aarch64",
        other => {
            return Err(Error::Other(anyhow::anyhow!(
                "unsupported architecture: {other}"
            )));
        }
    };

    let os = match std::env::consts::OS {
        "macos" => "apple-darwin",
        "linux" => "unknown-linux-gnu",
        other => return Err(Error::Other(anyhow::anyhow!("unsupported OS: {other}"))),
    };

    Ok(format!("{arch}-{os}"))
}

fn cleanup_path_entries(install_dir: &Path) -> Result<()> {
    let line = format!("export PATH=\"{}:$PATH\"", install_dir.display());
    for rc_path in shell_rc_paths() {
        remove_line_if_present(&rc_path, &line)?;
    }
    Ok(())
}

fn shell_rc_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".zshrc"));
        paths.push(home.join(".bashrc"));
    }
    paths
}

fn remove_line_if_present(path: &Path, needle: &str) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(path)?;
    let filtered = content
        .lines()
        .filter(|line| line.trim() != needle)
        .collect::<Vec<_>>()
        .join("\n");
    let normalized = if filtered.is_empty() {
        String::new()
    } else {
        format!("{filtered}\n")
    };

    if normalized != content {
        fs::write(path, normalized)?;
    }

    Ok(())
}

fn github_client() -> Result<reqwest::Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/vnd.github+json"),
    );
    headers.insert(
        USER_AGENT,
        HeaderValue::from_str(&format!("d1v-cli/{}", env!("CARGO_PKG_VERSION")))
            .map_err(|err| Error::Other(anyhow::Error::new(err)))?,
    );

    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(other)
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

    Ok(format!("{:x}", hasher.finalize()))
}

fn other(err: impl Into<anyhow::Error>) -> Error {
    Error::Other(err.into())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_semver_versions() {
        assert!(is_upgrade_available("0.1.0", "v0.2.0"));
        assert!(!is_upgrade_available("v0.2.0", "0.2.0"));
        assert!(!is_upgrade_available("0.3.0", "0.2.9"));
    }

    #[test]
    fn explicit_target_installs_on_version_difference() {
        assert!(requires_install("0.1.2", "v0.1.1", true));
        assert!(!requires_install("0.1.2", "v0.1.2", true));
    }

    #[test]
    fn normalizes_tags() {
        assert_eq!(normalize_tag("0.1.3"), "v0.1.3");
        assert_eq!(normalize_tag("v0.1.3"), "v0.1.3");
    }

    #[test]
    fn parses_checksum_lines() {
        let body = "abc123  d1v-x86_64-apple-darwin.tar.gz\nfff999  other.tar.gz\n";
        assert_eq!(
            parse_checksum(body, "d1v-x86_64-apple-darwin.tar.gz").as_deref(),
            Some("abc123")
        );
        assert_eq!(parse_checksum(body, "missing.tar.gz"), None);
    }
}
