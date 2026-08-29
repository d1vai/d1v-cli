//! Current-directory deployment shortcuts (`d1v --preview` / `d1v --prod`).

use std::fs;
use std::io::IsTerminal;
use std::path::Path;
use std::time::Duration;

use crate::ui::{Select, SelectOption};
use crate::{Context, Result, workspace};
use anyhow::anyhow;

fn env_project_id(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path.join(".env")).ok()?;
    content.lines().find_map(|line| {
        let line = line.trim();
        let value = line.strip_prefix("D1V_PROJECT_ID=")?.trim();
        let value = value.trim_matches(['"', '\'']);
        (!value.is_empty()).then(|| value.to_string())
    })
}

fn write_project_id(path: &Path, project_id: &str) -> Result<()> {
    let env_path = path.join(".env");
    let mut lines = fs::read_to_string(&env_path)
        .unwrap_or_default()
        .lines()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let mut found = false;
    for line in &mut lines {
        if line.trim_start().starts_with("D1V_PROJECT_ID=") {
            *line = format!("D1V_PROJECT_ID={project_id}");
            found = true;
        }
    }
    if !found {
        lines.push(format!("D1V_PROJECT_ID={project_id}"));
    }
    let mut content = lines.join("\n");
    content.push('\n');
    let tmp = env_path.with_extension("env.d1v.tmp");
    fs::write(&tmp, content)?;
    fs::rename(tmp, env_path)?;
    Ok(())
}

async fn resolve_project(ctx: &Context, path: &Path) -> Result<String> {
    if let Some(id) = env_project_id(path) {
        return Ok(id);
    }
    if let Some(id) = workspace::resolve_bound_project_id(Some(path))? {
        write_project_id(path, &id)?;
        return Ok(id);
    }
    if !std::io::stdin().is_terminal() {
        return Err(anyhow!("D1V_PROJECT_ID is missing from .env; run `d1v project list` and set it before using a non-interactive shortcut").into());
    }
    let projects = ctx.client.projects().list().await?;
    if projects.is_empty() {
        return Err(anyhow!("no D1V projects found; create one with `d1v project create --name <name> --description <description>`").into());
    }
    let options = projects.iter().map(|project| {
        SelectOption::new(
            project.id.clone(),
            format!("{} ({})", project.project_name, project.id),
        )
        .description(project.project_description.clone())
    });
    let id = Select::new("Select a D1V project")
        .options(options)
        .prompt()?;
    write_project_id(path, &id)?;
    Ok(id)
}

pub async fn run(ctx: &Context, preview: bool) -> Result<()> {
    let path = std::env::current_dir()?;
    let project_id = resolve_project(ctx, &path).await?;
    let deployment = if preview {
        ctx.client.deployment().preview(&project_id).await?
    } else {
        ctx.client.deployment().production(&project_id).await?
    };
    ctx.present(
        crate::text::Line::raw(format!(
            "{} deployment requested for {}\n{}",
            if preview { "Preview" } else { "Production" },
            project_id,
            deployment.message
        )),
        &deployment,
    )?;
    if !preview {
        if deployment.success {
            ctx.success("Production deployment is READY");
        }
        return Ok(());
    }
    for _ in 0..90 {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let status = ctx.client.deployment().preview_status(&project_id).await?;
        if status.message == "deployment state: READY" {
            ctx.success("Deployment is READY");
            return Ok(());
        }
        if status.message.contains("ERROR") || status.message.contains("FAILED") {
            return Err(anyhow!(status.message).into());
        }
    }
    Err(anyhow!("deployment did not become READY within 90 seconds").into())
}
