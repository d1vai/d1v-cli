use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context as AnyhowContext, anyhow};
use clap::Args;
use d1v_api::api::projects::LocalImportFile;
use d1v_api::{GitHubProjectCliAccess, GitHubProjectGitCredential, PullWorkspaceRequest};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::Context;
use crate::error::Result;
use crate::text::{Field, Fields, Line, Span, Text};
use crate::theme;
use crate::token::TokenSource;

const WORKSPACE_DIR: &str = ".d1v";
const WORKSPACE_FILE: &str = "project.json";
const IGNORE_FILE: &str = ".d1vignore";
const IGNORE_PROFILE_VERSION: u32 = 1;
const COMMIT_SUMMARY_PROMPT_MAX_CHARS: usize = 24_000;

const DEFAULT_EXCLUDED_DIRS: &[&str] = &[
    ".git",
    ".next",
    ".d1v",
    ".venv",
    "__pycache__",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "target",
    "venv",
];

const DEFAULT_EXCLUDED_FILES: &[&str] = &[".DS_Store"];

#[derive(Args)]
pub struct InitArgs {
    /// Local directory to initialize as a d1v workspace
    #[arg(default_value = ".")]
    pub path: PathBuf,
    /// Override the stored project name
    #[arg(long)]
    pub name: Option<String>,
    /// Bind an existing remote project id without validating it
    #[arg(long)]
    pub project_id: Option<String>,
    /// Overwrite an existing local workspace binding
    #[arg(long)]
    pub force: bool,
    /// Preview the scan and binding result without writing files
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args)]
pub struct PullArgs {
    /// Resolve workspace metadata from this path instead of the current directory
    #[arg(long)]
    pub path: Option<PathBuf>,
    /// Preview pull readiness only; no files are written
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Args)]
pub struct PushArgs {
    /// Resolve workspace metadata from this path instead of the current directory
    #[arg(long)]
    pub path: Option<PathBuf>,
    /// Preview push readiness only; no network changes are made
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMetadata {
    pub version: u32,
    pub project_id: Option<String>,
    pub workspace_id: Option<String>,
    pub project_name: String,
    pub root_path: String,
    pub framework: Option<String>,
    pub package_manager: Option<String>,
    pub remote_revision: Option<String>,
    pub last_pull_revision: Option<String>,
    pub last_push_revision: Option<String>,
    pub created_by_cli_version: String,
    pub ignore_profile_version: u32,
    pub bound_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct InitResultJson<'a> {
    metadata: &'a WorkspaceMetadata,
    scan: &'a ScanSummary,
    wrote_binding: bool,
}

#[derive(Debug, Clone, Serialize)]
struct SyncResultJson<'a> {
    operation: &'a str,
    metadata: &'a WorkspaceMetadata,
    repository_full_name: Option<&'a str>,
    local_branch: Option<&'a str>,
    remote_branch: Option<&'a str>,
    git_ready: bool,
    dirty: bool,
    can_sync: bool,
    status: &'a str,
}

#[derive(Debug, Clone)]
struct CommitSummaryInput {
    status: String,
    unstaged_stat: String,
    staged_stat: String,
    unstaged_patch: String,
    staged_patch: String,
    raw_diff: String,
}

#[derive(Debug, Clone, Default, Serialize)]
struct ScanSummary {
    included_files: usize,
    excluded_files: usize,
    included_bytes: u64,
    excluded_bytes: u64,
    framework: Option<String>,
    package_manager: Option<String>,
    risky_files: Vec<String>,
    included_samples: Vec<String>,
    excluded_samples: Vec<String>,
}

struct InitResultView<'a> {
    metadata: &'a WorkspaceMetadata,
    scan: &'a ScanSummary,
    wrote_binding: bool,
}

impl crate::text::Render for InitResultView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        let title = if self.wrote_binding {
            "Initialized local workspace"
        } else {
            "Scanned local workspace"
        };

        Text::new()
            .line(Line::styled(title.to_string(), theme::ansi::success()))
            .render(ctx)?;

        Fields::new([
            field("Project", &self.metadata.project_name),
            field("Path", &self.metadata.root_path),
            field_opt("Project ID", self.metadata.project_id.as_deref()),
            field_opt("Framework", self.metadata.framework.as_deref()),
            field_opt("Package manager", self.metadata.package_manager.as_deref()),
            field("Included files", &self.scan.included_files.to_string()),
            field("Excluded files", &self.scan.excluded_files.to_string()),
            field("Included bytes", &self.scan.included_bytes.to_string()),
        ])
        .indent(2)
        .render(ctx)?;

        if !self.scan.risky_files.is_empty() {
            writeln!(ctx.writer)?;
            Text::new()
                .line(Line::styled("Warnings".to_string(), theme::ansi::warning()))
                .render(ctx)?;
            Fields::new(self.scan.risky_files.iter().map(|path| {
                Field::new(
                    Span::styled("Risk", theme::ansi::label()),
                    Line::styled(path.clone(), theme::ansi::value()),
                )
            }))
            .indent(2)
            .render(ctx)?;
        }

        Ok(())
    }
}

struct SyncResultView<'a> {
    operation: &'a str,
    metadata: &'a WorkspaceMetadata,
    repository_full_name: Option<&'a str>,
    local_branch: Option<&'a str>,
    remote_branch: Option<&'a str>,
    git_ready: bool,
    dirty: bool,
    can_sync: bool,
    status: &'a str,
}

#[derive(Debug, Clone)]
struct PaiConfig {
    base_url: String,
    api_key: String,
    model: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChatCompletionsRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    max_tokens: u32,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionsResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct ChatChoiceMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelItem>,
}

#[derive(Debug, Deserialize)]
struct ModelItem {
    id: String,
}

impl crate::text::Render for SyncResultView<'_> {
    fn render(&self, ctx: &mut crate::text::RenderContext<'_>) -> std::io::Result<()> {
        let style = if self.can_sync {
            theme::ansi::success()
        } else {
            theme::ansi::warning()
        };

        Text::new()
            .line(Line::styled(self.status.to_string(), style))
            .render(ctx)?;

        Fields::new([
            field("Operation", self.operation),
            field("Project", &self.metadata.project_name),
            field("Path", &self.metadata.root_path),
            field_opt("Project ID", self.metadata.project_id.as_deref()),
            field_opt("Repository", self.repository_full_name),
            field_opt("Local branch", self.local_branch),
            field_opt("Remote branch", self.remote_branch),
            field("Git ready", if self.git_ready { "true" } else { "false" }),
            field("Dirty", if self.dirty { "true" } else { "false" }),
            field("Sync ready", if self.can_sync { "true" } else { "false" }),
        ])
        .indent(2)
        .render(ctx)
    }
}

fn field(label: &'static str, value: &str) -> Field {
    Field::new(
        Span::styled(label, theme::ansi::label()),
        Line::styled(value.to_string(), theme::ansi::value()),
    )
}

fn field_opt(label: &'static str, value: Option<&str>) -> Field {
    field(label, value.unwrap_or("-"))
}

pub async fn init(ctx: &Context, args: InitArgs) -> Result<()> {
    let root = fs::canonicalize(&args.path)
        .with_context(|| format!("failed to resolve {}", args.path.display()))?;

    if !root.is_dir() {
        return Err(anyhow!("{} is not a directory", root.display()).into());
    }

    let metadata_path = workspace_file_path(&root);
    if metadata_path.exists() && !args.force {
        return Err(anyhow!(
            "workspace already initialized at {}. Re-run with --force to overwrite.",
            metadata_path.display()
        )
        .into());
    }

    let scan = scan_workspace(&root)?;
    let now = Timestamp::now().to_string();
    let project_name = args.name.clone().unwrap_or_else(|| {
        root.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    });

    let mut metadata = WorkspaceMetadata {
        version: 1,
        project_id: args.project_id.clone(),
        workspace_id: None,
        project_name,
        root_path: root.display().to_string(),
        framework: scan.framework.clone(),
        package_manager: scan.package_manager.clone(),
        remote_revision: None,
        last_pull_revision: None,
        last_push_revision: None,
        created_by_cli_version: env!("CARGO_PKG_VERSION").to_string(),
        ignore_profile_version: IGNORE_PROFILE_VERSION,
        bound_at: now.clone(),
        updated_at: now,
    };

    if !args.dry_run {
        if metadata.project_id.is_none() && ctx.tokens.lookup()?.is_some() {
            let upload_files = collect_upload_files(&root)?;
            ctx.info(format!(
                "Uploading {} filtered files to /api/projects/cli-import-local",
                upload_files.len()
            ));
            let result = ctx
                .client
                .projects()
                .cli_import_local()
                .project_name(metadata.project_name.clone())
                .files(upload_files)
                .call()
                .await?;
            metadata.project_id = Some(result.project.id.clone());
            metadata.project_name = result.project.project_name;
        }
        write_workspace_metadata(&root, &metadata)?;
        ctx.success(format!("Initialized {}", root.display()));
    } else {
        ctx.info(format!("Dry run for {}", root.display()));
    }

    ctx.present(
        InitResultView {
            metadata: &metadata,
            scan: &scan,
            wrote_binding: !args.dry_run,
        },
        &InitResultJson {
            metadata: &metadata,
            scan: &scan,
            wrote_binding: !args.dry_run,
        },
    )
}

pub async fn pull(ctx: &Context, args: PullArgs) -> Result<()> {
    sync_workspace(ctx, args.path, args.dry_run, "pull").await
}

pub async fn push(ctx: &Context, args: PushArgs) -> Result<()> {
    sync_workspace(ctx, args.path, args.dry_run, "push").await
}

pub fn resolve_bound_project_id(path: Option<&Path>) -> Result<Option<String>> {
    let target = match path {
        Some(path) => fs::canonicalize(path)?,
        None => std::env::current_dir()?,
    };

    let Some(workspace_root) = find_workspace_root(&target)? else {
        return Ok(None);
    };

    let metadata = read_workspace_metadata(&workspace_root)?;
    Ok(metadata.project_id.filter(|value| !value.trim().is_empty()))
}

/// Reads a project ID from the exact execution directory's `.env` file.
pub fn resolve_env_project_id(path: Option<&Path>) -> Result<Option<String>> {
    let directory = match path {
        Some(path) => path.to_path_buf(),
        None => std::env::current_dir()?,
    };
    let content = match fs::read_to_string(directory.join(".env")) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    for line in content.lines() {
        let line = line.trim();
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != "D1V_PROJECT_ID" {
            continue;
        }
        let value = value.trim().trim_matches(['"', '\'']);
        if !value.is_empty() {
            return Ok(Some(value.to_string()));
        }
    }
    Ok(None)
}

fn write_workspace_metadata(root: &Path, metadata: &WorkspaceMetadata) -> Result<()> {
    let dir = root.join(WORKSPACE_DIR);
    fs::create_dir_all(&dir)?;
    let path = workspace_file_path(root);
    let json = serde_json::to_string_pretty(metadata).map_err(anyhow::Error::from)?;
    fs::write(path, json)?;
    Ok(())
}

fn read_workspace_metadata(root: &Path) -> Result<WorkspaceMetadata> {
    let path = workspace_file_path(root);
    let content = fs::read_to_string(&path)?;
    serde_json::from_str(&content)
        .map_err(anyhow::Error::from)
        .map_err(Into::into)
}

async fn sync_workspace(
    ctx: &Context,
    path: Option<PathBuf>,
    dry_run: bool,
    operation: &'static str,
) -> Result<()> {
    let target = match path {
        Some(path) => fs::canonicalize(path)?,
        None => std::env::current_dir()?,
    };

    let workspace_root = find_workspace_root(&target)?.ok_or_else(|| {
        anyhow!(
            "no .d1v/project.json found from {} upward. Run `d1v init .` first.",
            target.display()
        )
    })?;

    let mut metadata = read_workspace_metadata(&workspace_root)?;
    let project_id = metadata
        .project_id
        .clone()
        .ok_or_else(|| anyhow!("local workspace is not bound to a remote project yet"))?;
    let access = ctx
        .client
        .github_app()
        .project_cli_access(&project_id)
        .await?;
    let repository_full_name = access.repository_full_name.as_deref();
    let remote_branch = access
        .current_branch
        .as_deref()
        .or(access.default_branch.as_deref());
    let git_ready = is_git_repository(&workspace_root)?;
    let local_branch = if git_ready {
        git_current_branch(&workspace_root)?
    } else {
        None
    };
    let dirty = if git_ready {
        git_is_dirty(&workspace_root)?
    } else {
        false
    };
    let can_sync = if operation == "pull" {
        access.can_pull
    } else {
        access.can_push
    };

    let status = sync_status_message(operation, &access, git_ready, dirty);

    let should_block_for_dirty = operation == "pull" && dirty;

    if dry_run || !can_sync || !git_ready || should_block_for_dirty {
        return ctx.present(
            SyncResultView {
                operation,
                metadata: &metadata,
                repository_full_name,
                local_branch: local_branch.as_deref(),
                remote_branch,
                git_ready,
                dirty,
                can_sync,
                status: &status,
            },
            &SyncResultJson {
                operation,
                metadata: &metadata,
                repository_full_name,
                local_branch: local_branch.as_deref(),
                remote_branch,
                git_ready,
                dirty,
                can_sync,
                status: &status,
            },
        );
    }

    let repo_url = access
        .repository_url
        .as_deref()
        .ok_or_else(|| anyhow!("project repository URL is missing"))?;
    let branch = remote_branch.ok_or_else(|| anyhow!("project branch metadata is missing"))?;
    let credential = ctx
        .client
        .github_app()
        .project_git_credential(&project_id)
        .await?;

    if operation == "pull" {
        with_temp_git_credentials(&workspace_root, repo_url, &credential, |config| {
            git_fetch_branch(&workspace_root, repo_url, branch, Some(config))?;
            git_merge_fetch_head(&workspace_root)?;
            Ok(())
        })?;
        let head = git_head_revision(&workspace_root)?;
        metadata.remote_revision = Some(head.clone());
        metadata.last_pull_revision = Some(head);
    } else {
        if dirty {
            let commit_message = generate_commit_message(&workspace_root).await?;
            git_stage_all(&workspace_root)?;
            git_commit_all(&workspace_root, &commit_message)?;
        }
        with_temp_git_credentials(&workspace_root, repo_url, &credential, |config| {
            git_push_head(&workspace_root, repo_url, branch, Some(config))
        })?;
        let _ = ctx
            .client
            .github_ops()
            .pull_workspace(
                &project_id,
                &PullWorkspaceRequest {
                    branch: Some(branch.to_string()),
                },
            )
            .await?;
        let head = git_head_revision(&workspace_root)?;
        metadata.remote_revision = Some(head.clone());
        metadata.last_push_revision = Some(head);
    }
    metadata.updated_at = Timestamp::now().to_string();
    write_workspace_metadata(&workspace_root, &metadata)?;

    let done_status = format!(
        "{} completed against {}:{}",
        operation,
        repository_full_name.unwrap_or(repo_url),
        branch
    );
    ctx.present(
        SyncResultView {
            operation,
            metadata: &metadata,
            repository_full_name,
            local_branch: local_branch.as_deref(),
            remote_branch: Some(branch),
            git_ready,
            dirty: false,
            can_sync: true,
            status: &done_status,
        },
        &SyncResultJson {
            operation,
            metadata: &metadata,
            repository_full_name,
            local_branch: local_branch.as_deref(),
            remote_branch: Some(branch),
            git_ready,
            dirty: false,
            can_sync: true,
            status: &done_status,
        },
    )
}

fn workspace_file_path(root: &Path) -> PathBuf {
    root.join(WORKSPACE_DIR).join(WORKSPACE_FILE)
}

fn find_workspace_root(start: &Path) -> Result<Option<PathBuf>> {
    let mut current = if start.is_dir() {
        start.to_path_buf()
    } else {
        start
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| anyhow!("unable to resolve parent directory"))?
    };

    loop {
        if workspace_file_path(&current).exists() {
            return Ok(Some(current));
        }
        if !current.pop() {
            return Ok(None);
        }
    }
}

fn scan_workspace(root: &Path) -> Result<ScanSummary> {
    let extra_ignores = load_ignore_patterns(root)?;
    let mut summary = ScanSummary {
        framework: detect_framework(root),
        package_manager: detect_package_manager(root),
        ..ScanSummary::default()
    };

    visit_dir(root, root, &extra_ignores, &mut summary)?;
    Ok(summary)
}

fn collect_upload_files(root: &Path) -> Result<Vec<LocalImportFile>> {
    let extra_ignores = load_ignore_patterns(root)?;
    let mut files = Vec::new();
    collect_files_recursive(root, root, &extra_ignores, &mut files)?;
    Ok(files)
}

fn load_ignore_patterns(root: &Path) -> Result<BTreeSet<String>> {
    let path = root.join(IGNORE_FILE);
    if !path.exists() {
        return Ok(BTreeSet::new());
    }

    let content = fs::read_to_string(path)?;
    Ok(content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .collect())
}

fn visit_dir(
    root: &Path,
    dir: &Path,
    extra_ignores: &BTreeSet<String>,
    summary: &mut ScanSummary,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let file_type = entry.file_type()?;

        if should_ignore(&rel, file_type.is_dir(), extra_ignores) {
            let size = file_size(&path);
            summary.excluded_files += 1;
            summary.excluded_bytes += size;
            push_sample(&mut summary.excluded_samples, rel);
            continue;
        }

        if file_type.is_dir() {
            visit_dir(root, &path, extra_ignores, summary)?;
            continue;
        }

        let size = file_size(&path);
        summary.included_files += 1;
        summary.included_bytes += size;
        push_sample(&mut summary.included_samples, rel.clone());

        if is_risky_file(&rel) {
            summary.risky_files.push(rel);
        }
    }

    Ok(())
}

fn collect_files_recursive(
    root: &Path,
    dir: &Path,
    extra_ignores: &BTreeSet<String>,
    files: &mut Vec<LocalImportFile>,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let file_type = entry.file_type()?;

        if should_ignore(&rel, file_type.is_dir(), extra_ignores) {
            continue;
        }

        if file_type.is_dir() {
            collect_files_recursive(root, &path, extra_ignores, files)?;
            continue;
        }

        files.push(LocalImportFile {
            path: rel,
            bytes: fs::read(&path)?,
        });
    }

    Ok(())
}

fn sync_status_message(
    operation: &str,
    access: &GitHubProjectCliAccess,
    git_ready: bool,
    dirty: bool,
) -> String {
    if access.binding_required {
        return access.reason.clone().unwrap_or_else(|| {
            "GitHub binding is required before CLI sync can continue.".to_string()
        });
    }
    let can_sync = if operation == "pull" {
        access.can_pull
    } else {
        access.can_push
    };
    if !can_sync {
        return access
            .reason
            .clone()
            .unwrap_or_else(|| format!("{operation} is not ready for this project."));
    }
    if !git_ready {
        return "Current workspace is not a git repository. Initialize or clone the repository first."
            .to_string();
    }
    if dirty {
        if operation == "push" {
            return "Working tree has uncommitted changes. `d1v push` will auto-stage, auto-commit, and then sync."
                .to_string();
        }
        return format!(
            "Working tree has uncommitted changes. Commit or stash them before `d1v {operation}`."
        );
    }
    format!("{operation} is ready.")
}

async fn generate_commit_message(root: &Path) -> Result<String> {
    let input = collect_commit_summary_input(root)?;
    if input.raw_diff.trim().is_empty() {
        return Ok("chore: sync local changes".to_string());
    }

    if let Some(config) = load_pai_config(root)?
        && let Ok(summary) = summarize_diff_with_pai(&config, &input).await
        && !summary.trim().is_empty()
    {
        return Ok(summary.trim().to_string());
    }

    Ok(fallback_commit_message(&input.raw_diff))
}

fn load_pai_config(root: &Path) -> Result<Option<PaiConfig>> {
    let env_path = root.join(".env");
    let mut merged = HashMap::new();
    if env_path.exists() {
        merged.extend(parse_env_file(&env_path)?);
    }
    for key in ["D1V_PAI_BASE_URL", "D1V_PAI_API_KEY", "D1V_PAI_MODEL"] {
        if let Ok(value) = std::env::var(key)
            && !value.trim().is_empty()
        {
            merged.insert(key.to_string(), value);
        }
    }

    let base_url = merged
        .get("D1V_PAI_BASE_URL")
        .cloned()
        .unwrap_or_default()
        .trim()
        .trim_end_matches('/')
        .to_string();
    let api_key = merged
        .get("D1V_PAI_API_KEY")
        .cloned()
        .unwrap_or_default()
        .trim()
        .to_string();
    let model = merged
        .get("D1V_PAI_MODEL")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    if base_url.is_empty() || api_key.is_empty() {
        return Ok(None);
    }

    Ok(Some(PaiConfig {
        base_url,
        api_key,
        model,
    }))
}

fn parse_env_file(path: &Path) -> Result<HashMap<String, String>> {
    let mut env = HashMap::new();
    let content = fs::read_to_string(path)?;
    for raw_line in content.lines() {
        let mut line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || !line.contains('=') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("export ") {
            line = rest.trim();
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim().trim_matches('"').trim_matches('\'');
        if !key.is_empty() {
            env.insert(key.to_string(), value.to_string());
        }
    }
    Ok(env)
}

async fn summarize_diff_with_pai(config: &PaiConfig, input: &CommitSummaryInput) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(anyhow::Error::from)?;
    let model = if let Some(model) = config.model.as_deref() {
        model.to_string()
    } else {
        select_pai_model(&client, config).await?
    };
    let prompts = build_commit_summary_prompt_variants(input);
    let mut last_error: Option<crate::error::Error> = None;
    for prompt in prompts {
        match request_commit_summary_with_pai(&client, config, &model, &prompt).await {
            Ok(message) if !message.trim().is_empty() => return Ok(message),
            Ok(_) => last_error = Some(anyhow!("PAI returned an empty commit summary").into()),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| anyhow!("PAI did not return a commit summary").into()))
}

async fn request_commit_summary_with_pai(
    client: &reqwest::Client,
    config: &PaiConfig,
    model: &str,
    prompt: &str,
) -> Result<String> {
    let response = client
        .post(format!("{}/v1/chat/completions", config.base_url))
        .bearer_auth(&config.api_key)
        .json(&ChatCompletionsRequest {
            model,
            messages: vec![
                ChatMessage {
                    role: "system",
                    content: "You write concise conventional git commit messages.",
                },
                ChatMessage {
                    role: "user",
                    content: &prompt,
                },
            ],
            max_tokens: 60,
            stream: false,
        })
        .send()
        .await
        .map_err(anyhow::Error::from)?;
    let response = response.error_for_status().map_err(anyhow::Error::from)?;
    let payload: ChatCompletionsResponse = response.json().await.map_err(anyhow::Error::from)?;
    payload
        .choices
        .into_iter()
        .next()
        .map(|choice| sanitize_commit_message(&choice.message.content))
        .filter(|message| !message.is_empty())
        .ok_or_else(|| anyhow!("PAI did not return a commit summary").into())
}

async fn select_pai_model(client: &reqwest::Client, config: &PaiConfig) -> Result<String> {
    let response = client
        .get(format!("{}/v1/models", config.base_url))
        .bearer_auth(&config.api_key)
        .send()
        .await
        .map_err(anyhow::Error::from)?;
    let response = response.error_for_status().map_err(anyhow::Error::from)?;
    let payload: ModelsResponse = response.json().await.map_err(anyhow::Error::from)?;
    let preferred = [
        "gpt-4.1-mini",
        "gpt-4o-mini",
        "claude-3-5-haiku",
        "gemini-2.0-flash",
    ];
    for preferred_model in preferred {
        if payload.data.iter().any(|item| item.id == preferred_model) {
            return Ok(preferred_model.to_string());
        }
    }
    payload
        .data
        .into_iter()
        .map(|item| item.id)
        .find(|id| !id.trim().is_empty())
        .ok_or_else(|| anyhow!("No models available from D1V_PAI_BASE_URL").into())
}

fn build_commit_summary_prompt_variants(input: &CommitSummaryInput) -> Vec<String> {
    let status = truncate_for_prompt(&input.status, 2_000);
    let diff_stats = render_commit_diff_stats(input);
    let detailed_patch = render_commit_patch_sections(input, 8, 4, 1_600, 12_000);
    let compact_patch = render_commit_patch_sections(input, 4, 2, 800, 4_000);

    let variants = vec![
        build_commit_summary_prompt(&status, &diff_stats, Some(&detailed_patch)),
        build_commit_summary_prompt(&status, &diff_stats, Some(&compact_patch)),
        build_commit_summary_prompt(&status, &diff_stats, None),
    ];

    let mut seen = BTreeSet::new();
    variants
        .into_iter()
        .filter(|prompt| seen.insert(prompt.clone()))
        .collect()
}

fn build_commit_summary_prompt(
    status: &str,
    diff_stats: &str,
    patch_excerpt: Option<&str>,
) -> String {
    let mut prompt = String::from(
        "Write one concise git commit message under 72 characters.\n\
Use conventional-commit style when appropriate.\n\
Return only the commit message.\n\n",
    );
    prompt.push_str("Status:\n");
    prompt.push_str(status);
    prompt.push_str("\n\nDiff stats:\n");
    prompt.push_str(diff_stats);
    if let Some(patch_excerpt) = patch_excerpt
        && !patch_excerpt.trim().is_empty()
    {
        prompt.push_str("\n\nPatch excerpts:\n");
        prompt.push_str(patch_excerpt);
    }
    truncate_for_prompt(&prompt, COMMIT_SUMMARY_PROMPT_MAX_CHARS)
}

fn truncate_for_prompt(text: &str, max_chars: usize) -> String {
    let mut iter = text.chars();
    let truncated: String = iter.by_ref().take(max_chars).collect();
    if iter.next().is_some() {
        format!("{truncated}\n...[truncated]")
    } else {
        truncated
    }
}

fn render_commit_diff_stats(input: &CommitSummaryInput) -> String {
    let mut sections = Vec::new();
    if !input.unstaged_stat.trim().is_empty() {
        sections.push(format!("Unstaged:\n{}", input.unstaged_stat.trim()));
    }
    if !input.staged_stat.trim().is_empty() {
        sections.push(format!("Staged:\n{}", input.staged_stat.trim()));
    }
    if sections.is_empty() {
        sections.push("No diff stats available".to_string());
    }
    sections.join("\n\n")
}

fn render_commit_patch_sections(
    input: &CommitSummaryInput,
    max_files: usize,
    max_hunks_per_file: usize,
    max_chars_per_file: usize,
    max_total_chars: usize,
) -> String {
    let mut sections = Vec::new();
    let unstaged = compact_patch_for_prompt(
        &input.unstaged_patch,
        max_files,
        max_hunks_per_file,
        max_chars_per_file,
        max_total_chars / 2,
    );
    if !unstaged.trim().is_empty() {
        sections.push(format!("Unstaged:\n{unstaged}"));
    }

    let staged = compact_patch_for_prompt(
        &input.staged_patch,
        max_files,
        max_hunks_per_file,
        max_chars_per_file,
        max_total_chars / 2,
    );
    if !staged.trim().is_empty() {
        sections.push(format!("Staged:\n{staged}"));
    }

    let joined = sections.join("\n\n");
    truncate_for_prompt(&joined, max_total_chars)
}

fn compact_patch_for_prompt(
    diff: &str,
    max_files: usize,
    max_hunks_per_file: usize,
    max_chars_per_file: usize,
    max_total_chars: usize,
) -> String {
    let mut output = String::new();
    let mut files_seen = 0usize;
    let mut chars_in_file = 0usize;
    let mut hunks_in_file = 0usize;
    let mut file_truncated = false;

    for line in diff.lines() {
        if line.starts_with("diff --git ") {
            if files_seen >= max_files {
                if !output.contains("\n...[additional files truncated]") {
                    output.push_str("\n...[additional files truncated]");
                }
                break;
            }
            files_seen += 1;
            chars_in_file = 0;
            hunks_in_file = 0;
            file_truncated = false;
        } else if files_seen == 0 {
            continue;
        }

        if line.starts_with("@@") {
            if hunks_in_file >= max_hunks_per_file {
                if !file_truncated {
                    append_compact_line(
                        &mut output,
                        "...[additional hunks truncated]",
                        max_total_chars,
                    );
                    file_truncated = true;
                }
                continue;
            }
            hunks_in_file += 1;
        }

        let line_len = line.chars().count() + 1;
        let is_metadata = line.starts_with("diff --git ")
            || line.starts_with("index ")
            || line.starts_with("--- ")
            || line.starts_with("+++ ")
            || line.starts_with("@@");
        if !is_metadata && chars_in_file + line_len > max_chars_per_file {
            if !file_truncated {
                append_compact_line(&mut output, "...[file excerpt truncated]", max_total_chars);
                file_truncated = true;
            }
            continue;
        }

        if !append_compact_line(&mut output, line, max_total_chars) {
            break;
        }
        chars_in_file += line_len;
    }

    output.trim().to_string()
}

fn append_compact_line(output: &mut String, line: &str, max_total_chars: usize) -> bool {
    let candidate_len = output.chars().count() + line.chars().count() + 1;
    if candidate_len > max_total_chars {
        if !output.contains("\n...[patch truncated]") {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str("...[patch truncated]");
        }
        return false;
    }
    if !output.is_empty() {
        output.push('\n');
    }
    output.push_str(line);
    true
}

fn sanitize_commit_message(message: &str) -> String {
    message
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .trim_matches('`')
        .trim()
        .to_string()
}

fn fallback_commit_message(diff: &str) -> String {
    let mut kinds = Vec::new();
    if diff.contains("new file mode") || diff.contains("+++ b/") {
        kinds.push("add");
    }
    if diff.contains("--- a/") && diff.contains("+++ /dev/null") {
        kinds.push("remove");
    }
    if diff.contains("@@") {
        kinds.push("update");
    }
    let verb = kinds.first().copied().unwrap_or("sync");
    format!("chore: {verb} local changes")
}

fn collect_commit_summary_input(root: &Path) -> Result<CommitSummaryInput> {
    let status = run_git(
        root,
        &["status", "--short", "--untracked-files=all"],
        None,
        None,
    )?;
    let unstaged_stat = run_git(
        root,
        &[
            "diff",
            "--no-ext-diff",
            "--minimal",
            "--stat=160,120",
            "--compact-summary",
        ],
        None,
        None,
    )?;
    let staged_stat = run_git(
        root,
        &[
            "diff",
            "--no-ext-diff",
            "--minimal",
            "--cached",
            "--stat=160,120",
            "--compact-summary",
        ],
        None,
        None,
    )?;
    let unstaged_patch = run_git(
        root,
        &["diff", "--no-ext-diff", "--minimal", "--unified=0"],
        None,
        None,
    )?;
    let staged_patch = run_git(
        root,
        &[
            "diff",
            "--no-ext-diff",
            "--minimal",
            "--cached",
            "--unified=0",
        ],
        None,
        None,
    )?;
    let raw_diff = format!(
        "Status:\n{status}\n\nUnstaged diff:\n{unstaged_patch}\n\nStaged diff:\n{staged_patch}"
    );
    Ok(CommitSummaryInput {
        status,
        unstaged_stat,
        staged_stat,
        unstaged_patch,
        staged_patch,
        raw_diff,
    })
}

fn git_stage_all(root: &Path) -> Result<()> {
    run_git(root, &["add", "-A"], None, None).map(|_| ())
}

fn git_has_staged_changes(root: &Path) -> Result<bool> {
    let output = run_git(root, &["diff", "--cached", "--name-only"], None, None)?;
    Ok(!output.trim().is_empty())
}

fn git_commit_all(root: &Path, message: &str) -> Result<()> {
    if !git_has_staged_changes(root)? {
        return Ok(());
    }
    let mut args = vec!["commit", "-m", message];
    let has_name = run_git(root, &["config", "--get", "user.name"], None, None).is_ok();
    let has_email = run_git(root, &["config", "--get", "user.email"], None, None).is_ok();
    let mut prefixed: Vec<&str> = Vec::new();
    if !has_name {
        prefixed.extend(["-c", "user.name=d1v-cli"]);
    }
    if !has_email {
        prefixed.extend(["-c", "user.email=noreply@d1v.ai"]);
    }
    prefixed.append(&mut args);
    run_git(root, &prefixed, None, None).map(|_| ())
}

fn is_git_repository(root: &Path) -> Result<bool> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .with_context(|| format!("failed to run git in {}", root.display()))?;
    Ok(output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "true")
}

fn git_current_branch(root: &Path) -> Result<Option<String>> {
    let branch = run_git(root, &["rev-parse", "--abbrev-ref", "HEAD"], None, None)?;
    let trimmed = branch.trim();
    if trimmed.is_empty() || trimmed == "HEAD" {
        Ok(None)
    } else {
        Ok(Some(trimmed.to_string()))
    }
}

fn git_is_dirty(root: &Path) -> Result<bool> {
    let output = run_git(root, &["status", "--porcelain"], None, None)?;
    Ok(!output.trim().is_empty())
}

fn git_head_revision(root: &Path) -> Result<String> {
    Ok(run_git(root, &["rev-parse", "HEAD"], None, None)?
        .trim()
        .to_string())
}

fn git_fetch_branch(
    root: &Path,
    repo_url: &str,
    branch: &str,
    credential_config: Option<&str>,
) -> Result<()> {
    let mut args = Vec::new();
    if let Some(config) = credential_config {
        args.push("-c");
        args.push(config);
    }
    args.extend(["fetch", "--quiet", repo_url, branch]);
    run_git(root, &args, None, None).map(|_| ())
}

fn git_merge_fetch_head(root: &Path) -> Result<()> {
    run_git(root, &["merge", "--ff-only", "FETCH_HEAD"], None, None).map(|_| ())
}

fn git_push_head(
    root: &Path,
    repo_url: &str,
    branch: &str,
    credential_config: Option<&str>,
) -> Result<()> {
    let refspec = format!("HEAD:{branch}");
    let mut args = Vec::new();
    if let Some(config) = credential_config {
        args.push("-c");
        args.push(config);
    }
    args.extend(["push", "--quiet", repo_url, refspec.as_str()]);
    run_git(root, &args, None, None).map(|_| ())
}

fn with_temp_git_credentials<T>(
    root: &Path,
    repo_url: &str,
    credential: &GitHubProjectGitCredential,
    op: impl FnOnce(&str) -> Result<T>,
) -> Result<T> {
    let host = repository_host(repo_url);
    let credential_path = std::env::temp_dir().join(format!(
        "d1v-git-credential-{}-{}.txt",
        std::process::id(),
        Timestamp::now().as_second()
    ));
    fs::write(&credential_path, "")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&credential_path, fs::Permissions::from_mode(0o600))?;
    }

    let helper_value = format!(
        "credential.helper=store --file={}",
        credential_path.display()
    );
    let approve_input = format!(
        "protocol=https\nhost={host}\nusername={}\npassword={}\n\n",
        credential.username, credential.password
    );
    let approve_args = ["-c", helper_value.as_str(), "credential", "approve"];
    run_git(root, &approve_args, None, Some(&approve_input))?;

    let result = op(helper_value.as_str());
    let _ = fs::remove_file(&credential_path);
    result
}

fn repository_host(repo_url: &str) -> &str {
    repo_url
        .strip_prefix("https://")
        .and_then(|value| value.split('/').next())
        .filter(|value| !value.is_empty())
        .unwrap_or("github.com")
}

fn run_git(
    root: &Path,
    args: &[&str],
    envs: Option<&[(&str, &str)]>,
    stdin_input: Option<&str>,
) -> Result<String> {
    let mut command = Command::new("git");
    command.arg("-C").arg(root).args(args);
    if let Some(envs) = envs {
        command.envs(envs.iter().copied());
    }
    if stdin_input.is_some() {
        command.stdin(Stdio::piped());
    }
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .with_context(|| format!("failed to spawn git in {}", root.display()))?;
    if let Some(input) = stdin_input
        && let Some(stdin) = child.stdin.as_mut()
    {
        stdin.write_all(input.as_bytes())?;
    }
    let output = child.wait_with_output()?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).to_string());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if stderr.is_empty() { stdout } else { stderr };
    Err(anyhow!("git {} failed: {}", args.join(" "), detail).into())
}

fn file_size(path: &Path) -> u64 {
    fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

fn push_sample(samples: &mut Vec<String>, value: String) {
    if samples.len() < 8 {
        samples.push(value);
    }
}

fn should_ignore(rel: &str, is_dir: bool, extra_ignores: &BTreeSet<String>) -> bool {
    let name = rel.rsplit('/').next().unwrap_or(rel);

    if extra_ignores.contains(rel) || extra_ignores.contains(name) {
        return true;
    }

    if is_dir {
        DEFAULT_EXCLUDED_DIRS.contains(&name)
    } else {
        DEFAULT_EXCLUDED_FILES.contains(&name)
            || name.ends_with(".log")
            || name.ends_with(".zip")
            || name.ends_with(".tar")
            || name.ends_with(".gz")
    }
}

fn is_risky_file(rel: &str) -> bool {
    let name = rel.rsplit('/').next().unwrap_or(rel).to_ascii_lowercase();
    name == ".env"
        || name.ends_with(".pem")
        || name.ends_with(".key")
        || name.contains("secret")
        || name.contains("credential")
}

fn detect_framework(root: &Path) -> Option<String> {
    let has = |name: &str| root.join(name).exists();

    if has("remix.config.js") || has("remix.config.mjs") || has("remix.config.ts") {
        Some("remix".to_string())
    } else if has("next.config.js") || has("next.config.mjs") || has("next.config.ts") {
        Some("nextjs".to_string())
    } else if has("vite.config.js") || has("vite.config.ts") || has("vite.config.mjs") {
        Some("vite".to_string())
    } else if has("Cargo.toml") {
        Some("rust".to_string())
    } else if has("pyproject.toml") || has("requirements.txt") {
        Some("python".to_string())
    } else if has("package.json") {
        Some("node".to_string())
    } else if has("Dockerfile") {
        Some("docker".to_string())
    } else {
        None
    }
}

fn detect_package_manager(root: &Path) -> Option<String> {
    let has = |name: &str| root.join(name).exists();

    if has("pnpm-lock.yaml") {
        Some("pnpm".to_string())
    } else if has("bun.lockb") || has("bun.lock") {
        Some("bun".to_string())
    } else if has("yarn.lock") {
        Some("yarn".to_string())
    } else if has("package-lock.json") {
        Some("npm".to_string())
    } else if has("Cargo.lock") || has("Cargo.toml") {
        Some("cargo".to_string())
    } else if has("uv.lock") {
        Some("uv".to_string())
    } else if has("poetry.lock") {
        Some("poetry".to_string())
    } else if has("requirements.txt") {
        Some("pip".to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let unique = format!(
            "d1v-cli-workspace-test-{name}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn detects_framework_and_package_manager() {
        let dir = temp_dir("detect");
        fs::write(dir.join("package.json"), "{}").unwrap();
        fs::write(dir.join("remix.config.ts"), "export default {}").unwrap();
        fs::write(dir.join("pnpm-lock.yaml"), "lockfileVersion: 9").unwrap();

        let scan = scan_workspace(&dir).unwrap();
        assert_eq!(scan.framework.as_deref(), Some("remix"));
        assert_eq!(scan.package_manager.as_deref(), Some("pnpm"));

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn respects_default_and_custom_ignores() {
        let dir = temp_dir("ignore");
        fs::create_dir_all(dir.join("node_modules/pkg")).unwrap();
        fs::write(dir.join("node_modules/pkg/index.js"), "ignored").unwrap();
        fs::write(dir.join(".d1vignore"), "tmp.txt\n").unwrap();
        fs::write(dir.join("tmp.txt"), "ignored").unwrap();
        fs::write(dir.join("app.ts"), "included").unwrap();

        let scan = scan_workspace(&dir).unwrap();
        assert_eq!(scan.included_files, 2);
        assert!(scan.excluded_files >= 2);
        assert!(
            scan.excluded_samples
                .iter()
                .any(|item| item == "node_modules")
        );
        assert!(scan.excluded_samples.iter().any(|item| item == "tmp.txt"));

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn writes_and_finds_workspace_metadata() {
        let dir = temp_dir("metadata");
        let nested = dir.join("src/routes");
        fs::create_dir_all(&nested).unwrap();

        let metadata = WorkspaceMetadata {
            version: 1,
            project_id: Some("proj_123".to_string()),
            workspace_id: None,
            project_name: "sample".to_string(),
            root_path: dir.display().to_string(),
            framework: Some("remix".to_string()),
            package_manager: Some("pnpm".to_string()),
            remote_revision: None,
            last_pull_revision: None,
            last_push_revision: None,
            created_by_cli_version: "0.1.0".to_string(),
            ignore_profile_version: IGNORE_PROFILE_VERSION,
            bound_at: Timestamp::now().to_string(),
            updated_at: Timestamp::now().to_string(),
        };

        write_workspace_metadata(&dir, &metadata).unwrap();
        let root = find_workspace_root(&nested).unwrap().unwrap();
        let loaded = read_workspace_metadata(&root).unwrap();

        assert_eq!(loaded.project_id.as_deref(), Some("proj_123"));
        assert_eq!(loaded.project_name, "sample");

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn resolves_bound_project_id_from_nested_workspace_path() {
        let dir = temp_dir("resolve-project-id");
        let nested = dir.join("app/pages");
        fs::create_dir_all(&nested).unwrap();

        let metadata = WorkspaceMetadata {
            version: 1,
            project_id: Some("proj_nested".to_string()),
            workspace_id: None,
            project_name: "sample".to_string(),
            root_path: dir.display().to_string(),
            framework: None,
            package_manager: None,
            remote_revision: None,
            last_pull_revision: None,
            last_push_revision: None,
            created_by_cli_version: "0.1.0".to_string(),
            ignore_profile_version: IGNORE_PROFILE_VERSION,
            bound_at: Timestamp::now().to_string(),
            updated_at: Timestamp::now().to_string(),
        };

        write_workspace_metadata(&dir, &metadata).unwrap();

        let resolved = resolve_bound_project_id(Some(&nested)).unwrap();
        assert_eq!(resolved.as_deref(), Some("proj_nested"));

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn resolves_project_id_from_env_file() {
        let dir = temp_dir("resolve-env-project-id");
        fs::write(
            dir.join(".env"),
            "OTHER=value\nexport D1V_PROJECT_ID=project_from_env\n",
        )
        .unwrap();
        assert_eq!(
            resolve_env_project_id(Some(&dir)).unwrap().as_deref(),
            Some("project_from_env")
        );
    }

    #[test]
    fn collect_upload_files_keeps_root_env_and_ignores_build_outputs() {
        let dir = temp_dir("upload-files");
        fs::create_dir_all(dir.join("target/debug")).unwrap();
        fs::write(dir.join(".env"), "SECRET=1\n").unwrap();
        fs::write(dir.join("package.json"), "{}").unwrap();
        fs::write(dir.join("target/debug/app"), "ignored").unwrap();

        let files = collect_upload_files(&dir).unwrap();
        let paths = files
            .iter()
            .map(|item| item.path.as_str())
            .collect::<Vec<_>>();

        assert!(paths.contains(&".env"));
        assert!(paths.contains(&"package.json"));
        assert!(!paths.iter().any(|path| path.starts_with("target/")));

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn parse_env_file_reads_pai_keys() {
        let dir = temp_dir("pai-env");
        let env_path = dir.join(".env");
        fs::write(
            &env_path,
            "D1V_PAI_BASE_URL=https://pai.d1v.ai\nexport D1V_PAI_API_KEY=test-key\nD1V_PAI_MODEL=gpt-4.1-mini\n",
        )
        .unwrap();

        let parsed = parse_env_file(&env_path).unwrap();
        assert_eq!(
            parsed.get("D1V_PAI_BASE_URL").map(String::as_str),
            Some("https://pai.d1v.ai")
        );
        assert_eq!(
            parsed.get("D1V_PAI_API_KEY").map(String::as_str),
            Some("test-key")
        );
        assert_eq!(
            parsed.get("D1V_PAI_MODEL").map(String::as_str),
            Some("gpt-4.1-mini")
        );

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn fallback_commit_message_is_stable() {
        let diff = "Status:\n M src/main.rs\n\nUnstaged diff:\n@@\n+hello\n";
        assert_eq!(fallback_commit_message(diff), "chore: update local changes");
        assert_eq!(
            sanitize_commit_message("`feat: add auth flow`\nbody"),
            "feat: add auth flow"
        );
    }

    fn run_git_ok(root: &Path, args: &[&str]) {
        run_git(root, args, None, None).unwrap();
    }

    fn normalize_text(value: String) -> String {
        value.replace("\r\n", "\n")
    }

    #[test]
    fn git_helpers_report_repo_state() {
        let dir = temp_dir("git-state");
        run_git_ok(&dir, &["init", "-b", "main"]);
        fs::write(dir.join("README.md"), "hello\n").unwrap();
        run_git_ok(&dir, &["add", "README.md"]);
        run_git_ok(
            &dir,
            &[
                "-c",
                "user.name=D1V Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "-m",
                "init",
            ],
        );

        assert!(is_git_repository(&dir).unwrap());
        assert_eq!(git_current_branch(&dir).unwrap().as_deref(), Some("main"));
        assert!(!git_is_dirty(&dir).unwrap());

        fs::write(dir.join("README.md"), "changed\n").unwrap();
        assert!(git_is_dirty(&dir).unwrap());

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn git_helpers_pull_fast_forward_syncs_changes() {
        let remote = temp_dir("remote-bare");
        run_git_ok(&remote, &["init", "--bare"]);

        let seed = temp_dir("seed");
        run_git_ok(&seed, &["init", "-b", "dev"]);
        run_git_ok(
            &seed,
            &["remote", "add", "origin", remote.to_str().unwrap()],
        );
        fs::write(seed.join("app.txt"), "v1\n").unwrap();
        run_git_ok(&seed, &["add", "app.txt"]);
        run_git_ok(
            &seed,
            &[
                "-c",
                "user.name=D1V Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "-m",
                "seed",
            ],
        );
        run_git_ok(&seed, &["push", "-u", "origin", "dev"]);

        let workspace = temp_dir("pull-workspace");
        run_git_ok(
            Path::new(std::env::temp_dir().as_path()),
            &[
                "clone",
                "--branch",
                "dev",
                remote.to_str().unwrap(),
                workspace.to_str().unwrap(),
            ],
        );

        fs::write(seed.join("app.txt"), "v2\n").unwrap();
        run_git_ok(&seed, &["add", "app.txt"]);
        run_git_ok(
            &seed,
            &[
                "-c",
                "user.name=D1V Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "-m",
                "update",
            ],
        );
        run_git_ok(&seed, &["push", "origin", "dev"]);

        git_fetch_branch(&workspace, remote.to_str().unwrap(), "dev", None).unwrap();
        git_merge_fetch_head(&workspace).unwrap();
        assert_eq!(
            normalize_text(fs::read_to_string(workspace.join("app.txt")).unwrap()),
            "v2\n"
        );

        fs::remove_dir_all(seed).unwrap();
        fs::remove_dir_all(workspace).unwrap();
        fs::remove_dir_all(remote).unwrap();
    }

    #[test]
    fn git_helpers_push_head_to_remote_branch() {
        let remote = temp_dir("push-remote");
        run_git_ok(&remote, &["init", "--bare"]);

        let seed = temp_dir("push-seed");
        run_git_ok(&seed, &["init", "-b", "dev"]);
        run_git_ok(
            &seed,
            &["remote", "add", "origin", remote.to_str().unwrap()],
        );
        fs::write(seed.join("app.txt"), "v1\n").unwrap();
        run_git_ok(&seed, &["add", "app.txt"]);
        run_git_ok(
            &seed,
            &[
                "-c",
                "user.name=D1V Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "-m",
                "seed",
            ],
        );
        run_git_ok(&seed, &["push", "-u", "origin", "dev"]);

        let workspace = temp_dir("push-workspace");
        run_git_ok(
            Path::new(std::env::temp_dir().as_path()),
            &[
                "clone",
                "--branch",
                "dev",
                remote.to_str().unwrap(),
                workspace.to_str().unwrap(),
            ],
        );
        fs::write(workspace.join("app.txt"), "v2\n").unwrap();
        run_git_ok(&workspace, &["add", "app.txt"]);
        run_git_ok(
            &workspace,
            &[
                "-c",
                "user.name=D1V Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "-m",
                "local update",
            ],
        );

        git_push_head(&workspace, remote.to_str().unwrap(), "dev", None).unwrap();

        let verify = temp_dir("push-verify");
        run_git_ok(
            Path::new(std::env::temp_dir().as_path()),
            &[
                "clone",
                "--branch",
                "dev",
                remote.to_str().unwrap(),
                verify.to_str().unwrap(),
            ],
        );
        assert_eq!(
            normalize_text(fs::read_to_string(verify.join("app.txt")).unwrap()),
            "v2\n"
        );

        fs::remove_dir_all(seed).unwrap();
        fs::remove_dir_all(workspace).unwrap();
        fs::remove_dir_all(verify).unwrap();
        fs::remove_dir_all(remote).unwrap();
    }

    #[test]
    fn compact_patch_for_prompt_truncates_large_patch_gracefully() {
        let diff = [
            "diff --git a/a.txt b/a.txt",
            "index 1111111..2222222 100644",
            "--- a/a.txt",
            "+++ b/a.txt",
            "@@ -1 +1 @@",
            "-old line",
            "+new line",
            "@@ -10 +10 @@",
            "-another old line",
            "+another new line",
            "@@ -20 +20 @@",
            "-third old line",
            "+third new line",
            "diff --git a/b.txt b/b.txt",
            "index 3333333..4444444 100644",
            "--- a/b.txt",
            "+++ b/b.txt",
            "@@ -1 +1 @@",
            "-before",
            "+after",
        ]
        .join("\n");

        let compact = compact_patch_for_prompt(&diff, 1, 2, 200, 600);
        assert!(compact.contains("diff --git a/a.txt b/a.txt"));
        assert!(compact.contains("...[additional hunks truncated]"));
        assert!(!compact.contains("diff --git a/b.txt b/b.txt"));
        assert!(compact.contains("...[additional files truncated]"));
    }

    #[test]
    fn build_commit_summary_prompt_variants_shrink_context_progressively() {
        let input = CommitSummaryInput {
            status: " M src/main.rs\n?? src/new.rs".to_string(),
            unstaged_stat: " src/main.rs | 12 ++++++------".to_string(),
            staged_stat: " src/new.rs | 30 ++++++++++++++++++++++++++++++".to_string(),
            unstaged_patch: [
                "diff --git a/src/main.rs b/src/main.rs",
                "@@ -1 +1 @@",
                "-old",
                "+new",
            ]
            .join("\n"),
            staged_patch: [
                "diff --git a/src/new.rs b/src/new.rs",
                "@@ -0,0 +1,3 @@",
                "+one",
                "+two",
                "+three",
            ]
            .join("\n"),
            raw_diff: "placeholder".to_string(),
        };

        let prompts = build_commit_summary_prompt_variants(&input);
        assert!(prompts.len() >= 2);
        assert!(prompts[0].len() >= prompts[1].len());
        assert!(prompts.last().unwrap().contains("Diff stats:"));
        assert!(!prompts.last().unwrap().contains("Patch excerpts:"));
    }
}
