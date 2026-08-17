use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, anyhow};
use clap::{Args, Subcommand, ValueEnum};

use crate::Context;
use crate::error::Result;

const DEFAULT_SKILL_URL: &str = "https://www.d1v.ai/cli-skill.md";

#[derive(Subcommand)]
pub enum SkillCommand {
    /// Install or update the d1v skill
    Install(InstallSkillArgs),
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum AgentTarget {
    Codex,
    Claude,
    All,
}

#[derive(Args, Debug)]
pub struct InstallSkillArgs {
    /// Coding agent to configure
    #[arg(long, value_enum, default_value_t = AgentTarget::All)]
    pub agent: AgentTarget,

    /// Override the skill source URL
    #[arg(long, default_value = DEFAULT_SKILL_URL)]
    pub url: String,
}

pub async fn run(ctx: &Context, command: SkillCommand) -> Result<()> {
    match command {
        SkillCommand::Install(args) => install(ctx, args).await,
    }
}

async fn install(ctx: &Context, args: InstallSkillArgs) -> Result<()> {
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

    let home = dirs::home_dir().ok_or_else(|| anyhow!("could not resolve the home directory"))?;
    let mut destinations = Vec::new();
    if matches!(args.agent, AgentTarget::Codex | AgentTarget::All) {
        let root = env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".codex"));
        destinations.push(("Codex", root.join("skills/d1v/SKILL.md")));
    }
    if matches!(args.agent, AgentTarget::Claude | AgentTarget::All) {
        let root = env::var_os("CLAUDE_CONFIG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".claude"));
        destinations.push(("Claude Code", root.join("skills/d1v/SKILL.md")));
    }

    for (agent, destination) in destinations {
        write_atomic(&destination, contents.as_bytes())?;
        ctx.success(format!(
            "installed d1v skill for {agent}: {}",
            destination.display()
        ));
    }
    Ok(())
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

fn write_atomic(destination: &Path, contents: &[u8]) -> Result<()> {
    let parent = destination
        .parent()
        .ok_or_else(|| anyhow!("invalid skill destination: {}", destination.display()))?;
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;
    let temporary = parent.join(".SKILL.md.tmp");
    fs::write(&temporary, contents)
        .with_context(|| format!("failed to write {}", temporary.display()))?;
    fs::rename(&temporary, destination)
        .with_context(|| format!("failed to install {}", destination.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_skill;

    #[test]
    fn accepts_d1v_skill_frontmatter() {
        validate_skill("---\nname: d1v\ndescription: Use d1v CLI.\n---\n# d1v\n").unwrap();
    }

    #[test]
    fn rejects_untrusted_markdown_shape() {
        assert!(validate_skill("# Not a skill").is_err());
    }
}
