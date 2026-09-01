//! Current-directory deployment shortcuts (`d1v --preview` / `d1v --prod`).

use std::fs;
use std::io::IsTerminal;
use std::path::Path;

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

fn env_assignments(content: &str) -> Vec<(String, String)> {
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let line = line.strip_prefix("export ").unwrap_or(line);
            let (key, value) = line.split_once('=')?;
            let key = key.trim();
            if key.is_empty() || !key.chars().all(|c| c == '_' || c.is_ascii_alphanumeric()) {
                return None;
            }
            Some((key.to_string(), value.trim().to_string()))
        })
        .collect()
}

async fn merge_cloud_env(ctx: &Context, path: &Path, project_id: &str) -> Result<()> {
    let cloud = ctx.client.project(project_id).env().export_vars().await?;
    let local_content = fs::read_to_string(path.join(".env")).unwrap_or_default();
    let cloud_values = env_assignments(&cloud.content)
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut selected = std::collections::BTreeMap::new();
    let interactive = std::io::stdin().is_terminal();
    let local_values = env_assignments(&local_content)
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
    for (key, value) in &cloud_values {
        if let Some(existing) = local_values.get(key) {
            if existing != value && interactive {
                let choice = Select::new(format!("Use cloud value for {key}?"))
                    .options([
                        SelectOption::new(false, "Keep local value"),
                        SelectOption::new(true, "Use cloud value"),
                    ])
                    .default_index(0)
                    .prompt()?;
                if choice {
                    selected.insert(key.clone(), value.clone());
                }
            }
        } else if !local_values.contains_key(key) {
            selected.insert(key.clone(), value.clone());
        }
    }
    if selected.is_empty() {
        return Ok(());
    }
    let mut output = String::new();
    let mut written = std::collections::BTreeSet::new();
    for line in local_content.lines() {
        let trimmed = line.trim();
        if let Some((key, _)) = trimmed.split_once('=') {
            if let Some(value) = selected.get(key.trim_start_matches("export ").trim()) {
                let key = key.trim_start_matches("export ").trim();
                output.push_str(&format!("{key}={value}\n"));
                written.insert(key.to_string());
                continue;
            }
        }
        output.push_str(line);
        output.push('\n');
    }
    for (key, value) in selected {
        if !written.contains(&key) {
            output.push_str(&format!("{key}={value}\n"));
        }
    }
    let env_path = path.join(".env");
    let tmp = env_path.with_extension("env.d1v.tmp");
    fs::write(&tmp, output)?;
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
    let mut options = projects
        .iter()
        .map(|project| {
            SelectOption::new(
                project.id.clone(),
                format!("{} ({})", project.project_name, project.id),
            )
            .description(project.project_description.clone())
        })
        .collect::<Vec<_>>();
    options.push(SelectOption::new(String::new(), "Create a new project"));
    let id = Select::new("Select a D1V project")
        .options(options)
        .prompt()?;
    if id.is_empty() {
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("my-app");
        let result = ctx
            .client
            .projects()
            .create(name, &format!("Created from local directory {name}"))
            .call()
            .await?;
        let id = result.project.id;
        write_project_id(path, &id)?;
        merge_cloud_env(ctx, path, &id).await?;
        return Ok(id);
    }
    write_project_id(path, &id)?;
    Ok(id)
}

pub async fn run(ctx: &Context, preview: bool) -> Result<()> {
    let path = std::env::current_dir()?;
    let project_id = resolve_project(ctx, &path).await?;
    merge_cloud_env(ctx, &path, &project_id).await?;
    let deployment = if preview {
        crate::deploy::wait_for_preview(ctx, &project_id).await?
    } else {
        let release = crate::deploy::production_release(ctx, &project_id).await?;
        d1v_api::DeploymentResponse {
            success: true,
            message: release.status,
            commit_hash: None,
            production_url: release.production_url,
            vercel_url: None,
            deployment_id: release.deployment_id.or(release.id),
        }
    };
    ctx.present(
        crate::text::Line::raw(format!(
            "{} deployment requested for {}\n{}\nURL: {}",
            if preview { "Preview" } else { "Production" },
            project_id,
            deployment.message,
            deployment
                .production_url
                .as_deref()
                .or(deployment.vercel_url.as_deref())
                .unwrap_or("-"),
        )),
        &deployment,
    )?;
    let ready_message = if preview {
        "Preview deployment is READY"
    } else {
        "Production release is READY"
    };
    let ready_url = deployment
        .production_url
        .as_deref()
        .or(deployment.vercel_url.as_deref())
        .unwrap_or("-");
    ctx.success(format!("{ready_message}: {ready_url}"));
    Ok(())
}
