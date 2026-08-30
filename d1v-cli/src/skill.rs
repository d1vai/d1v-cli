use std::env;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, anyhow};
use clap::{Args, Subcommand, ValueEnum};
use serde::Serialize;

use crate::Context;
use crate::error::Result;
use crate::text::Text;

const DEFAULT_SKILL_URL: &str =
    "https://raw.githubusercontent.com/d1vai/d1v-cli/main/skills/d1v/SKILL.md";
#[cfg(test)]
const OFFICIAL_SKILL: &str = include_str!("../../skills/d1v/SKILL.md");

#[derive(Subcommand)]
pub enum SkillCommand {
    /// Install or update the d1v skill
    Install(InstallSkillArgs),
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum AgentTarget {
    /// Install only for coding agents available on PATH
    Auto,
    Codex,
    Claude,
    All,
}

#[derive(Args, Debug)]
pub struct InstallSkillArgs {
    /// Coding agent to configure (auto detects Codex and Claude Code on PATH)
    #[arg(long, value_enum, default_value_t = AgentTarget::Auto)]
    pub agent: AgentTarget,

    /// Override the skill source URL
    #[arg(long, default_value = DEFAULT_SKILL_URL)]
    pub url: String,
}

#[derive(Debug, Serialize)]
struct InstallSummary {
    target: String,
    results: Vec<InstallResult>,
}

#[derive(Debug, Serialize)]
struct InstallResult {
    agent: &'static str,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    backup_path: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<&'static str>,
}

#[derive(Debug)]
struct Destination {
    agent: &'static str,
    path: PathBuf,
}

pub async fn run(ctx: &Context, command: SkillCommand) -> Result<()> {
    match command {
        SkillCommand::Install(args) => install(ctx, args).await,
    }
}

async fn install(ctx: &Context, args: InstallSkillArgs) -> Result<()> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("could not resolve the home directory"))?;
    let destinations = resolve_destinations(args.agent, &home, env::var_os("PATH"));
    if destinations.is_empty() {
        return present_summary(
            ctx,
            InstallSummary {
                target: target_name(args.agent).to_owned(),
                results: vec![InstallResult {
                    agent: "coding agents",
                    status: "skipped",
                    path: None,
                    backup_path: None,
                    message: Some("Codex and Claude Code were not found on PATH"),
                }],
            },
        );
    }

    let response = reqwest::Client::builder()
        .user_agent(concat!("d1v-cli/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("failed to create the skill download client")?
        .get(&args.url)
        .send()
        .await
        .with_context(|| format!("failed to download d1v skill from {}", args.url))?
        .error_for_status()
        .with_context(|| format!("d1v skill download failed at {}", args.url))?;
    let contents = response
        .text()
        .await
        .context("failed to read the downloaded d1v skill")?;
    validate_skill(&contents)?;

    let results = destinations
        .into_iter()
        .map(|destination| {
            let outcome = write_skill(&destination.path, contents.as_bytes())?;
            Ok(InstallResult {
                agent: destination.agent,
                status: outcome.status(),
                path: Some(destination.path),
                backup_path: outcome.backup_path(),
                message: None,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    present_summary(
        ctx,
        InstallSummary {
            target: target_name(args.agent).to_owned(),
            results,
        },
    )
}

fn present_summary(ctx: &Context, summary: InstallSummary) -> Result<()> {
    let text = summary.results.iter().fold(Text::new(), |text, result| {
        let line = match (&result.path, &result.backup_path, result.message) {
            (_, _, Some(message)) => format!("d1v skill skipped: {message}"),
            (Some(path), Some(backup), _) => format!(
                "updated d1v skill for {}: {} (previous version backed up to {})",
                result.agent,
                path.display(),
                backup.display()
            ),
            (Some(path), None, _) => format!(
                "{} d1v skill for {}: {}",
                result.status,
                result.agent,
                path.display()
            ),
            _ => unreachable!("install result must have a path or a skip message"),
        };
        text.line(line)
    });
    ctx.present(text, &summary)
}

fn target_name(target: AgentTarget) -> &'static str {
    match target {
        AgentTarget::Auto => "auto",
        AgentTarget::Codex => "codex",
        AgentTarget::Claude => "claude",
        AgentTarget::All => "all",
    }
}

fn resolve_destinations(
    target: AgentTarget,
    home: &Path,
    path: Option<OsString>,
) -> Vec<Destination> {
    let codex_root = env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".codex"));
    let claude_root = env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".claude"));
    resolve_destinations_with_roots(target, path, codex_root, claude_root)
}

fn resolve_destinations_with_roots(
    target: AgentTarget,
    path: Option<OsString>,
    codex_root: PathBuf,
    claude_root: PathBuf,
) -> Vec<Destination> {
    let codex = matches!(target, AgentTarget::Codex | AgentTarget::All)
        || matches!(target, AgentTarget::Auto) && command_in_path("codex", path.as_deref());
    let claude = matches!(target, AgentTarget::Claude | AgentTarget::All)
        || matches!(target, AgentTarget::Auto) && command_in_path("claude", path.as_deref());

    [
        codex.then(|| Destination {
            agent: "Codex",
            path: codex_root.join("skills/d1v/SKILL.md"),
        }),
        claude.then(|| Destination {
            agent: "Claude Code",
            path: claude_root.join("skills/d1v/SKILL.md"),
        }),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn command_in_path(command: &str, path: Option<&std::ffi::OsStr>) -> bool {
    path.map(env::split_paths)
        .into_iter()
        .flatten()
        .map(|directory| directory.join(command))
        .any(|candidate| candidate.is_file() && is_executable(&candidate))
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .is_ok_and(|metadata| metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(_path: &Path) -> bool {
    true
}

fn validate_skill(contents: &str) -> Result<()> {
    let trimmed = contents.trim_start();
    if !trimmed.starts_with("---\n")
        || !trimmed.contains("\nname: d1v\n")
        || !trimmed.contains("\ndescription:")
    {
        return Err(anyhow!("downloaded file is not a valid d1v SKILL.md").into());
    }
    Ok(())
}

enum WriteOutcome {
    Installed,
    Unchanged,
    Updated(PathBuf),
}

impl WriteOutcome {
    fn status(&self) -> &'static str {
        match self {
            Self::Installed => "installed",
            Self::Unchanged => "unchanged",
            Self::Updated(_) => "updated",
        }
    }

    fn backup_path(&self) -> Option<PathBuf> {
        match self {
            Self::Updated(path) => Some(path.clone()),
            Self::Installed | Self::Unchanged => None,
        }
    }
}

fn write_skill(destination: &Path, contents: &[u8]) -> Result<WriteOutcome> {
    if destination.is_file() && fs::read(destination)? == contents {
        return Ok(WriteOutcome::Unchanged);
    }

    let parent = destination
        .parent()
        .ok_or_else(|| anyhow!("invalid skill destination: {}", destination.display()))?;
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;
    let temporary = create_temporary_skill(parent, contents)?;

    if destination.exists() {
        let backup = backup_path(parent)?;
        if let Err(error) = fs::rename(destination, &backup) {
            let _ = fs::remove_file(&temporary);
            return Err(error)
                .with_context(|| format!("failed to back up {}", destination.display()))
                .map_err(Into::into);
        }
        if let Err(error) = fs::rename(&temporary, destination) {
            let restore = fs::rename(&backup, destination);
            return match restore {
                Ok(()) => Err(error)
                    .with_context(|| format!("failed to install {}", destination.display()))
                    .map_err(Into::into),
                Err(restore_error) => Err(anyhow!(
                    "failed to install {} and restore its backup {}: {restore_error}",
                    destination.display(),
                    backup.display()
                )
                .into()),
            };
        }
        Ok(WriteOutcome::Updated(backup))
    } else {
        fs::rename(&temporary, destination)
            .with_context(|| format!("failed to install {}", destination.display()))?;
        Ok(WriteOutcome::Installed)
    }
}

fn create_temporary_skill(parent: &Path, contents: &[u8]) -> Result<PathBuf> {
    for attempt in 0..100 {
        let temporary = parent.join(format!(
            ".SKILL.md.d1v-tmp-{}-{attempt}",
            std::process::id()
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(mut file) => {
                if let Err(error) = file.write_all(contents).and_then(|_| file.sync_all()) {
                    let _ = fs::remove_file(&temporary);
                    return Err(error)
                        .with_context(|| format!("failed to write {}", temporary.display()))
                        .map_err(Into::into);
                }
                return Ok(temporary);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to create {}", temporary.display()))
                    .map_err(Into::into);
            }
        }
    }
    Err(anyhow!("could not create a temporary d1v skill file").into())
}

fn backup_path(parent: &Path) -> Result<PathBuf> {
    let timestamp = jiff::Timestamp::now()
        .strftime("%Y%m%dT%H%M%SZ")
        .to_string();
    for suffix in 0..100 {
        let name = if suffix == 0 {
            format!("SKILL.md.d1v-backup-{timestamp}")
        } else {
            format!("SKILL.md.d1v-backup-{timestamp}-{suffix}")
        };
        let candidate = parent.join(name);
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(anyhow!("could not allocate a d1v skill backup path").into())
}

#[cfg(test)]
mod tests {
    use super::{
        AgentTarget, OFFICIAL_SKILL, WriteOutcome, backup_path, command_in_path,
        resolve_destinations, resolve_destinations_with_roots, validate_skill, write_skill,
    };
    use std::ffi::OsString;
    use std::fs;

    #[test]
    fn accepts_d1v_skill_frontmatter() {
        validate_skill("---\nname: d1v\ndescription: Use d1v CLI.\n---\n# d1v\n").unwrap();
    }

    #[test]
    fn rejects_untrusted_markdown_shape() {
        assert!(validate_skill("# Not a skill").is_err());
    }

    #[test]
    fn official_skill_is_valid_and_has_deployment_safeguards() {
        validate_skill(OFFICIAL_SKILL).unwrap();
        assert!(OFFICIAL_SKILL.contains("explicit user confirmation"));
        assert!(OFFICIAL_SKILL.contains("rejects non-interactive production releases"));
        assert!(OFFICIAL_SKILL.contains("`d1v deploy preview` waits"));
    }

    #[test]
    fn auto_uses_only_detected_agents() {
        let temp = tempfile::tempdir().unwrap();
        let bin = temp.path().join("bin");
        fs::create_dir(&bin).unwrap();
        let codex = bin.join("codex");
        fs::write(&codex, "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&codex, fs::Permissions::from_mode(0o755)).unwrap();
        }

        let destinations = resolve_destinations(
            AgentTarget::Auto,
            temp.path(),
            Some(OsString::from(bin.as_os_str())),
        );
        assert_eq!(destinations.len(), 1);
        assert_eq!(destinations[0].agent, "Codex");
        assert!(command_in_path("codex", Some(bin.as_os_str())));
        assert!(!command_in_path("claude", Some(bin.as_os_str())));
    }

    #[test]
    fn auto_detects_claude_and_both_agents() {
        let temp = tempfile::tempdir().unwrap();
        let bin = temp.path().join("bin");
        fs::create_dir(&bin).unwrap();
        for name in ["codex", "claude"] {
            let executable = bin.join(name);
            fs::write(&executable, "#!/bin/sh\n").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
            }
        }
        let destinations = resolve_destinations_with_roots(
            AgentTarget::Auto,
            Some(OsString::from(bin.as_os_str())),
            temp.path().join("custom-codex"),
            temp.path().join("custom-claude"),
        );
        assert_eq!(destinations.len(), 2);
        assert!(
            destinations[0]
                .path
                .starts_with(temp.path().join("custom-codex"))
        );
        assert!(
            destinations[1]
                .path
                .starts_with(temp.path().join("custom-claude"))
        );
    }

    #[test]
    fn auto_with_no_agents_has_no_destinations() {
        let temp = tempfile::tempdir().unwrap();
        assert!(
            resolve_destinations(
                AgentTarget::Auto,
                temp.path(),
                Some(OsString::from(temp.path().join("missing"))),
            )
            .is_empty()
        );
    }

    #[test]
    fn explicit_all_does_not_require_detected_agents() {
        let temp = tempfile::tempdir().unwrap();
        let destinations =
            resolve_destinations(AgentTarget::All, temp.path(), Some(OsString::new()));
        assert_eq!(destinations.len(), 2);
    }

    #[test]
    fn unchanged_skill_is_not_rewritten_or_backed_up() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("skills/d1v/SKILL.md");
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::write(&destination, b"same").unwrap();
        assert!(matches!(
            write_skill(&destination, b"same").unwrap(),
            WriteOutcome::Unchanged
        ));
        assert_eq!(fs::read(&destination).unwrap(), b"same");
        assert_eq!(
            fs::read_dir(destination.parent().unwrap()).unwrap().count(),
            1
        );
    }

    #[test]
    fn changed_skill_is_backed_up_before_replacement() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("skills/d1v/SKILL.md");
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::write(&destination, b"old").unwrap();
        let outcome = write_skill(&destination, b"new").unwrap();
        let WriteOutcome::Updated(backup) = outcome else {
            panic!("expected a backup")
        };
        assert_eq!(fs::read(&destination).unwrap(), b"new");
        assert_eq!(fs::read(backup).unwrap(), b"old");
    }

    #[test]
    fn backup_allocation_failure_keeps_original_file() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("skills/d1v/SKILL.md");
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::write(&destination, b"old").unwrap();
        let first = backup_path(destination.parent().unwrap()).unwrap();
        let prefix = first.file_name().unwrap().to_string_lossy().to_owned();
        for suffix in 0..100 {
            let name = if suffix == 0 {
                prefix.to_string()
            } else {
                format!("{prefix}-{suffix}")
            };
            fs::write(destination.parent().unwrap().join(name), b"occupied").unwrap();
        }
        assert!(write_skill(&destination, b"new").is_err());
        assert_eq!(fs::read(&destination).unwrap(), b"old");
    }
}
