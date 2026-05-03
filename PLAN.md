# Goal: Turn d1v-cli From Account Utility Into A Real Project Workflow CLI

## Design Thinking And Demand Background

- ICP: developers, founders, and internal operators who want to drive `d1v.ai` from terminal, scripts, CI, and remote shells without depending on the web UI for every project action.
- Core pain: `d1v-cli` currently authenticates users and exposes account utilities, but it cannot yet create, inspect, run, deploy, import, or operate projects end to end.
- Desired outcome: a CLI that covers the high-frequency workflow spine first: project lifecycle, AI execution sessions, deployment, GitHub import/bind handoff, and database operations.
- New workflow priority: local-first workspace onboarding should become a primary entry path, not a secondary fallback. Users should be able to stand inside a desktop project, run `d1v init ./xxx`, then use `d1v pull` / `d1v push` style flows from the same directory.
- Current architecture decision: `pull` / `push` should now be GitHub-first, not custom workspace-snapshot-first. The CLI should reuse project-linked GitHub repositories, GitHub App installation tokens, and real git behavior wherever possible.
- CLI product standard for this cycle: every new command should be scriptable, discoverable via `--help`, usable in both human and automation contexts, and explicit about when the user must jump to web.
- Scope constraint: do not mechanically mirror every `d1vai` screen. Prefer terminal-native flows and browser handoff for setup-heavy or highly visual experiences.

## Validators For This Cycle

- Reuse the validator registry from `AGENTS.md`.
- Selected validators: `@cli-ux-qa`, `@cli-json-qa`, `@api-backend-qa`, `@auth-state-qa`, `@docs-adoption-qa`, `@project-lifecycle-qa`, `@session-runtime-qa`, `@deploy-release-qa`, `@db-workflow-qa`, `@github-integration-qa`, `@billing-analytics-qa`.

## Current Product Read

- Existing public surface is account-centric: `auth`, `user`, `config`, `debug`, and `banner`.
- Existing strengths worth preserving: token chain precedence, keyring/config fallback, localized text rendering, and `--format json`.
- Main product gap: missing project-centric command tree.
- Main adoption gap: missing local-workspace entry point for users whose source of truth starts on disk rather than on GitHub.
- Main design principle: deliver automation-first resource commands before CLI-only polish for secondary modules.

## Command Surface Draft

- `d1v auth ...`
  - Keep current commands and semantics.
- `d1v config ...`
  - Keep current commands and semantics.
- `d1v user ...`
  - Keep current commands and semantics.
- `d1v project list|get|create|update|delete`
  - Support template selection, model selection, and `--auto-deploy`.
- `d1v init <path>`
  - Create or bind a `d1v` project from a local directory.
  - Detect framework, package manager, env file patterns, and deployability before upload/sync.
  - Default to safe import: manifest + key files + filtered source set, not blind full-directory upload.
  - Write local workspace metadata so follow-up commands can run from inside the directory without repeatedly passing `project_id`.
- `d1v pull`
  - Sync the project-linked GitHub branch into the current local git workspace.
  - Gate on project GitHub access readiness, dirty-worktree safety, and branch metadata before applying a fast-forward merge.
  - Support text mode summary and `--format json` for automation.
- `d1v push`
  - Push local committed `HEAD` to the project-linked GitHub branch using short-lived GitHub App credentials.
  - Refuse to proceed when the workspace has uncommitted changes or project GitHub access is not ready.
- `d1v session run|continue|history|status|cancel`
  - `run` starts an AI development session against a project.
  - `continue` reuses an active or selected session.
  - `history` returns chat/session history.
  - `status` shows active session state and latest runtime/deploy hints.
- `d1v deploy preview|prod|status|logs|history`
  - `preview` triggers preview deploy.
  - `prod` triggers production deploy.
  - `status` shows latest deployment state and URLs.
  - `logs` returns deployment log output for a selected deployment.
- `d1v github status|bind|repos|import`
  - `bind` should either complete a backend connect flow or open the correct browser page.
  - Preferred browser handoff targets:
    - `https://d1v.ai/setting?tab=github`
    - GitHub App install/connect page when available from API
  - `repos` lists importable repositories after binding.
  - `import` imports a selected repository and supports root-directory configuration.
  - `import` remains important, but should no longer be the only serious onboarding path.
- `d1v db schema|tables|rows|migrate`
  - `schema` reads current structure.
  - `tables` supports create/rename/delete.
  - `rows` supports list/insert/update/delete with JSON payloads.
  - `migrate` supports `plan|validate|approve|execute|history`.
- `d1v pay products|transactions|webhooks`
  - First CLI slice should favor operational and automation use cases.
- `d1v analytics overview|events|sessions|export`
  - Favor tabular output and JSON export, not chart parity with web.

## Priority Roadmap

- [ ] Define and ship the local-workspace onboarding spine: `d1v init <path>` plus local metadata binding. `@cli-ux-qa` `@cli-json-qa` `@api-backend-qa` `@project-lifecycle-qa` `@docs-adoption-qa`
  - Owner: main agent
  - Verification: `cargo run -p d1v-cli -- init --help`; smoke flow for `d1v init ./example` on a sample app; verify resulting local metadata lets later commands resolve project context automatically
  - Status: pending
  - Evidence: pending
  - Notes: this should be treated as a top-tier workflow, on par with `project create` and `github import`, because many real projects start as local folders on desktop machines. First version should prefer scanning + safe filtered sync over blind archive upload.

- [ ] Define and ship workspace sync read/write path: `d1v pull` + `d1v push`. `@cli-ux-qa` `@cli-json-qa` `@api-backend-qa` `@docs-adoption-qa` `@github-integration-qa`
  - Owner: main agent
  - Verification: `cargo run -p d1v-cli -- pull --help`; `cargo run -p d1v-cli -- push --help`; Rust git-helper tests for fast-forward pull and branch push; backend CLI GitHub access API tests
  - Status: in_progress
  - Evidence: added backend `GET /api/github-app/projects/{project_id}/cli-access`; reused `POST /api/github-app/projects/{project_id}/git-credential`; wired `d1v pull` and `d1v push` to real git fetch/merge and push flows with temporary credential storage; verified `cargo test -p d1v-cli -p d1v-api`
  - Notes: this is now GitHub-first. The backend stays responsible for project/repository authorization and short-lived credentials; the CLI stays responsible for local git safety checks and execution.

- [ ] Define and ship the project-lifecycle spine: `project list|get|create|update|delete`. `@cli-ux-qa` `@cli-json-qa` `@api-backend-qa` `@project-lifecycle-qa`
  - Owner: main agent
  - Verification: `cargo run -p d1v-cli -- project --help`; at least one smoke command per read/write path; `--format json` output reviewed for stable field names
  - Status: in_progress
  - Evidence: command surface wired into `src/main.rs`; added `d1v-api` project client plus `d1v project list|get|create|update|delete|templates`; `create` now supports direct create or one-step prompt/template flow; verified `cargo test` passes and `cargo run -q -p d1v-cli -- project --help` renders the expanded command tree
  - Notes: this is the minimum bar for `d1v-cli` to stop being account-only.

- [ ] Define and ship the AI runtime spine: `session run|continue|history|status|cancel`. `@cli-ux-qa` `@cli-json-qa` `@api-backend-qa` `@session-runtime-qa`
  - Owner: main agent
  - Verification: local smoke flow covering start -> inspect status -> continue -> inspect history; verify interactive and non-interactive behavior
  - Status: in_progress
  - Evidence: added `d1v-api` session client plus `d1v session run|continue|history|status|cancel`; verified `cargo test` passes and `cargo run -q -p d1v-cli -- session --help` renders the new command tree
  - Notes: stdout/stderr separation matters because long-running sessions will be scripted.

- [ ] Define and ship deployment operations: `deploy preview|prod|status|logs|history`. `@cli-ux-qa` `@cli-json-qa` `@api-backend-qa` `@deploy-release-qa`
  - Owner: main agent
  - Verification: help output review plus local smoke coverage for latest deployment status and one trigger path
  - Status: in_progress
  - Evidence: added `d1v-api` deployment client plus `d1v deploy preview|prod|status|history|logs`; verified `cargo test` passes and `cargo run -q -p d1v-cli -- deploy --help` renders the new command tree
  - Notes: prefer explicit resource IDs and URLs in JSON mode; human mode should summarize latest deploy clearly.

- [ ] Design GitHub binding and import around CLI-first handoff semantics. `@cli-ux-qa` `@docs-adoption-qa` `@github-integration-qa`
  - Owner: main agent
  - Verification: final help/copy reviewed for three cases: already bound, needs `d1v.ai` settings handoff, needs GitHub App install handoff
  - Status: in_progress
  - Evidence: added `d1v-api` GitHub App client plus `d1v github status|bind|installations|repos|import`; `bind` resolves OAuth connect URL when needed, otherwise falls back to install/settings URLs; `repos` is keyed by `--installation-id`; `import` maps the backend GitHub App import response including deployability hints; new CLI sync contract uses `cli-access` to surface binding requirements, repo URLs, target branch, and whether platform-managed repos can proceed without an explicit user GitHub bind
  - Notes: `bind` must either call API-backed connect state or open the browser to `https://d1v.ai/setting?tab=github`; if installation must happen on GitHub, the command should surface the install URL directly instead of hiding it. For `pull` / `push`, prefer real GitHub permissions and GitHub App installation tokens over a parallel custom sync protocol.

- [ ] Design database operations for terminal-native workflows: schema, rows, and migrations before visual parity. `@cli-ux-qa` `@cli-json-qa` `@api-backend-qa` `@db-workflow-qa`
  - Owner: main agent
  - Verification: command tree reviewed against expected workflows; smoke coverage for one schema read, one row read, and one migration planning path
  - Status: in_progress
  - Evidence: added `d1v-api` DB/migration client plus `d1v db schema|data|branches|tables|rows|token|migrate`; table and row write paths accept JSON payload flags; migration subcommands cover `plan|validate|approve|auto-review|manual-approve|execute|history|detail`; project-token automation path now exposed as `d1v db token issue|refresh`; verified `cargo test` passes and `cargo run -q -p d1v-cli -- db --help`, `db rows --help`, `db migrate --help` render the expected command tree
  - Notes: avoid web-style naming; optimize for composable subcommands and JSON payloads. Real authenticated smoke execution against live DB/migration APIs is still pending, so this item remains open until at least one schema read, one row read, and one migration plan are exercised end to end.

- [ ] Add secondary operations only after the core spine exists: `pay products|transactions|webhooks` and `analytics overview|events|sessions|export`. `@cli-ux-qa` `@cli-json-qa` `@billing-analytics-qa`
  - Owner: main agent
  - Verification: command tree and output contract review; at least one read-path smoke command per module
  - Status: pending
  - Evidence: pending
  - Notes: payment banking/withdraw and analytics report builders can remain later-phase if API maturity or UX fit is weak.

- [ ] Keep docs and migration guidance aligned with the shipped command surface. `@docs-adoption-qa` `@cli-ux-qa`
  - Owner: main agent
  - Verification: README/PLAN examples updated to reflect final command names and browser handoff behavior
  - Status: in_progress
  - Evidence: updated `README.md` and `README.zh-Hans.md` with current project/session/deploy/github/db command families, GitHub browser handoff guidance, and a post-login DB/migration smoke checklist; `PLAN.md` remains the execution log for shipped command scope and verification status
  - Notes: users should understand when CLI is sufficient and when web setup is still required. Live smoke examples are documented, but cannot be marked complete until they are exercised with a real authenticated account.

## Design Rules For Future d1v-cli Work

- Prefer nouns as top-level resources: `project`, `session`, `deploy`, `github`, `db`, `pay`, `analytics`.
- Allow a small number of workspace-native top-level verbs when they represent the user mental model better than resource nesting: `init`, `pull`, and later `push`.
- Prefer terminal verbs that map to backend actions directly: `list`, `get`, `create`, `update`, `delete`, `status`, `history`, `logs`, `run`, `import`.
- Keep human-readable summaries in text mode and stable machine contracts in JSON mode.
- Avoid hiding web-only prerequisites. If the user must complete setup in browser, the command should say so and ideally open the right page.
- Do not chase UI parity for charts, canvases, or relationship graphs. CLI should expose data, state, and operations.
- Local-file workflows must default to safe filtering: support `.d1vignore`, skip common heavy/build directories, warn on secrets, and preview payload size before upload or sync.

## Local-First Workflow Draft

- Primary happy path:
  - `cd ~/Desktop/my-app`
  - `d1v init .`
  - `d1v session run --prompt "understand this project and prepare a deploy plan"`
  - `d1v pull`
- `d1v init <path>` expected behaviors:
  - Resolve absolute path and refuse obviously invalid targets.
  - Detect framework and package manager from files like `package.json`, `pnpm-lock.yaml`, `Cargo.toml`, `requirements.txt`, `Dockerfile`, `remix.config.*`, `next.config.*`.
  - Build a candidate manifest: project name, runtime hints, start/build commands, lockfiles, env example files, database clues.
  - Apply `.d1vignore` plus built-in excludes such as `.git`, `node_modules`, `dist`, `build`, `.next`, `target`, large binaries, and secret-like files.
  - Present a preview before first upload: file count, total size, excluded paths, risky files.
  - Create or bind the remote project, then persist local metadata such as project id and workspace binding in a local state file.
- `d1v pull` expected behaviors:
  - Resolve bound project from current directory metadata.
  - Ask backend for latest workspace snapshot or patch set.
  - Show changed files summary before apply unless `--yes`.
  - Warn when local files are dirty or diverged.
  - Support dry run mode for CI or scripted inspection.
- Suggested local metadata:
  - `.d1v/project.json` for project binding, remote workspace id, last sync revision, framework guess, and ignore profile version.
  - `.d1vignore` for user-controlled excludes beyond defaults.

## Local-First Priority Opinion

- P0: `d1v init <path>` with directory scan, ignore model, preview, and remote binding
- P0: `d1v pull` with dirty-state detection and safe apply preview
- P1: `d1v status` should auto-resolve current workspace context from `.d1v/project.json`
- P1: `d1v session run` should accept current-directory project binding without explicit `project_id`
- P1: `d1v push` for local-to-cloud sync after the pull model is stable
- P2: full archive upload mode for edge cases where filtered sync is insufficient

## Full Execution Plan For Local-First Workspace Flow

### Product Goal

- Make local directory onboarding a first-class workflow:
  - `d1v init ./xxx`
  - `cd ./xxx`
  - `d1v session run ...`
  - `d1v pull`
  - later `d1v push`
- Reduce onboarding friction for users whose project source of truth starts on desktop or local disk.
- Preserve GitHub import as an important path, but no longer require GitHub as the prerequisite for serious CLI usage.

### Scope Definition

- In scope for first serious release:
  - local directory detection
  - safe manifest generation
  - filtered upload
  - remote project binding
  - workspace snapshot versioning
  - cloud-to-local sync via `pull`
  - local metadata persistence
  - dirty-worktree checks
  - dry run / preview output
- Out of scope for first release:
  - binary-large-media optimized sync
  - IDE live file watcher sync
  - Git conflict engine parity
  - bi-directional collaborative merge UI
  - arbitrary full-home-directory import

### User-Facing Command Design

- `d1v init <path>`
  - Create a new remote project from a local directory, or bind the directory to an existing remote project.
  - Default mode should be interactive in TTY and explicit in non-interactive mode.
  - Core flags:
    - `--name <name>`
    - `--project-id <id>` for binding existing project
    - `--prompt <text>` for one-step AI understanding during import
    - `--template-repo <repo>`
    - `--auto-deploy`
    - `--include <glob>`
    - `--exclude <glob>`
    - `--yes`
    - `--dry-run`
    - `--json`
- `d1v pull`
  - Sync latest cloud workspace changes into the current bound local directory.
  - Core flags:
    - `--project-id <id>` override local binding
    - `--revision <rev>` pull a specific remote revision
    - `--dry-run`
    - `--yes`
    - `--force`
    - `--json`
- `d1v push`
  - Sync eligible local changes to the remote workspace after pull semantics are stable.
  - Core flags:
    - `--dry-run`
    - `--yes`
    - `--force`
    - `--message <text>`
    - `--json`
- `d1v status`
  - Should auto-resolve current workspace metadata and display:
    - bound project id
    - local path
    - current revision
    - dirty local changes
    - pending pull / pending push state

### Local Metadata Design

- Add `.d1v/project.json`
  - Proposed shape:
    - `project_id`
    - `workspace_id`
    - `root_path`
    - `framework`
    - `package_manager`
    - `remote_revision`
    - `last_pull_revision`
    - `last_push_revision`
    - `created_by_cli_version`
    - `ignore_profile_version`
    - `bound_at`
    - `updated_at`
- Add optional `.d1vignore`
  - Merges with built-in ignore rules.
  - Built-in defaults should exclude:
    - `.git`
    - `.DS_Store`
    - `node_modules`
    - `.next`
    - `dist`
    - `build`
    - `coverage`
    - `target`
    - `.venv`
    - `venv`
    - `__pycache__`
    - large archives
    - secret-like files unless explicitly included
- Keep `.d1v/manifest.json` optional as a cache artifact for debugging and support.

### Local Scanner Design

- Scanner responsibilities:
  - resolve path
  - detect project root
  - walk file tree with ignore rules
  - classify files by kind:
    - config
    - source
    - lockfile
    - env example
    - asset
    - binary
    - risky secret-like file
  - infer framework/runtime:
    - Remix
    - Next.js
    - Vite
    - Node generic
    - Rust
    - Python
    - Dockerized app
  - infer package manager:
    - pnpm
    - npm
    - yarn
    - bun
    - cargo
    - pip/uv/poetry
  - infer likely commands:
    - install
    - dev
    - build
    - start
    - test
- Scanner outputs:
  - manifest summary
  - file inventory
  - included file list
  - excluded file list
  - risky file warnings
  - estimated upload size

### Sync Model Recommendation

- Do not start with blind archive upload as the default sync protocol.
- Preferred first sync protocol:
  - Step 1: upload manifest and file index
  - Step 2: backend decides which files it needs
  - Step 3: CLI uploads only missing or changed files
  - Step 4: backend writes a normalized workspace snapshot and returns a revision id
- For `pull`:
  - Step 1: CLI sends current project binding and local revision
  - Step 2: backend returns changed file manifest since that revision
  - Step 3: CLI previews patch summary
  - Step 4: CLI applies files after confirmation
- Later optional fallback:
  - `--archive` or `--full-upload` for rare cases where indexed sync is insufficient

### Backend Data Model Additions

- Add workspace-level sync concepts:
  - `workspace_id`
  - `workspace_revision`
  - `workspace_snapshot`
  - `workspace_file`
  - `workspace_sync_event`
- Suggested entities:
  - `project_workspaces`
    - `id`
    - `project_id`
    - `source_type` (`local`, `github`, `generated`)
    - `root_label`
    - `created_by_user_id`
    - `created_at`
    - `updated_at`
  - `workspace_revisions`
    - `id`
    - `workspace_id`
    - `base_revision_id`
    - `source_event` (`init`, `pull`, `push`, `session_apply`, `github_import`)
    - `summary`
    - `file_count`
    - `total_size`
    - `created_by`
    - `created_at`
  - `workspace_files`
    - `workspace_revision_id`
    - `path`
    - `sha256`
    - `size`
    - `content_type`
    - `storage_key`
    - `is_binary`
  - `workspace_sync_events`
    - `id`
    - `workspace_id`
    - `direction` (`client_to_cloud`, `cloud_to_client`)
    - `status`
    - `started_at`
    - `finished_at`
    - `actor_user_id`
    - `client_version`

### Backend API Additions

- Project binding and workspace discovery:
  - `POST /api/projects/init-local`
    - Purpose: create a new project from a local manifest or bind a local directory to an existing project.
    - Request:
      - `project_id?`
      - `name?`
      - `description?`
      - `prompt?`
      - `template_repo?`
      - `auto_deploy?`
      - `manifest`
      - `client`
    - Response:
      - `project`
      - `workspace`
      - `required_upload_strategy`
      - `remote_revision`
  - `GET /api/projects/{project_id}/workspace`
    - Purpose: resolve current workspace metadata for CLI status and binding checks.

- Manifest and file planning:
  - `POST /api/workspaces/{workspace_id}/upload-plan`
    - Purpose: accept local file index and tell the CLI which files must be uploaded.
    - Request:
      - `base_revision?`
      - `files[] { path, sha256, size, executable?, binary? }`
    - Response:
      - `upload_files[]`
      - `skip_files[]`
      - `max_chunk_size`
      - `upload_mode`
      - `proposed_revision`
  - `POST /api/workspaces/{workspace_id}/commit-upload`
    - Purpose: finalize uploaded files into a new workspace revision.
    - Request:
      - `proposed_revision`
      - `uploaded_files[]`
      - `summary`
    - Response:
      - `workspace_revision`
      - `project_state`

- File upload:
  - `POST /api/workspaces/{workspace_id}/files/presign`
    - Purpose: issue signed upload URLs for file blobs.
    - Request:
      - `files[] { path, sha256, size, content_type }`
    - Response:
      - `uploads[] { path, upload_url, headers, storage_key }`
  - Optional direct upload fallback:
    - `POST /api/workspaces/{workspace_id}/files`
    - For smaller files or simpler early implementation.

- Pull planning and apply:
  - `POST /api/workspaces/{workspace_id}/pull-plan`
    - Purpose: compute cloud-to-local changes from a client revision.
    - Request:
      - `local_revision`
      - `client_files?`
    - Response:
      - `target_revision`
      - `changes[] { path, status, sha256, size, binary }`
      - `conflicts[]`
      - `warnings[]`
  - `GET /api/workspaces/{workspace_id}/revisions/{revision_id}/files`
    - Purpose: list files for a target revision.
  - `POST /api/workspaces/{workspace_id}/download-plan`
    - Purpose: return signed URLs or inline content references for changed files.
    - Response:
      - `downloads[] { path, url, sha256, size, mode }`

- Push planning:
  - `POST /api/workspaces/{workspace_id}/push-plan`
    - Purpose: compute whether local changes can be applied cleanly on top of remote revision.
    - Request:
      - `base_revision`
      - `files[]`
      - `summary`
    - Response:
      - `can_push`
      - `upload_files[]`
      - `conflicts[]`
      - `warnings[]`

- Revision history and diagnostics:
  - `GET /api/workspaces/{workspace_id}/revisions`
  - `GET /api/workspaces/{workspace_id}/sync-events`
  - `GET /api/workspaces/{workspace_id}/diff?from=...&to=...`

### CLI Execution Flow: `d1v init <path>`

- Phase 1: local preflight
  - validate path
  - detect repo root / app root
  - load `.d1vignore`
  - scan files
  - build manifest
  - print preview
- Phase 2: remote binding
  - if `--project-id` is present, bind existing project
  - otherwise create project from manifest/prompt/template
  - receive workspace id and initial revision
- Phase 3: upload plan
  - send file index
  - receive changed file set
  - upload required files
  - commit upload
- Phase 4: local persistence
  - write `.d1v/project.json`
  - optionally write starter `.d1vignore`
  - print next-step commands

### CLI Execution Flow: `d1v pull`

- Phase 1: local preflight
  - resolve `.d1v/project.json`
  - detect local dirty files
  - detect local revision
- Phase 2: plan
  - ask backend for pull plan
  - render file summary and conflicts
- Phase 3: apply
  - if `--dry-run`, stop here
  - if conflicts exist and no `--force`, stop
  - download changed files
  - write files safely
  - update `.d1v/project.json`
- Phase 4: report
  - print changed file count, new revision, next actions

### CLI Execution Flow: `d1v push`

- Phase 1: local diff scan
  - compare local files against last bound revision
- Phase 2: push plan
  - backend validates base revision and mergeability
- Phase 3: upload changed files
  - same blob upload path as `init`
- Phase 4: finalize revision
  - backend commits new remote revision
  - local metadata updated

### Safety Rules

- Always preview first on first import unless `--yes`.
- Never upload ignored files by accident.
- Warn explicitly on:
  - `.env`
  - `.pem`
  - `.key`
  - service account json
  - SSH keys
  - files over size threshold
- Support `--dry-run` on `init`, `pull`, and `push`.
- Support machine-readable output with `--format json`.
- Prefer content-addressed blob storage and checksum verification.

### Verification Plan

- Unit-level:
  - ignore matcher
  - framework detector
  - manifest builder
  - dirty-state detector
  - revision metadata read/write
- Integration-level:
  - `d1v init ./fixtures/remix-app --dry-run`
  - `d1v init ./fixtures/next-app --json`
  - bind existing project into local folder
  - `d1v pull --dry-run`
  - `d1v push --dry-run`
- End-to-end:
  - create local sample app
  - `d1v init .`
  - start session that edits cloud workspace
  - `d1v pull`
  - edit local file
  - `d1v push`
  - validate resulting remote revision and local metadata

### Delivery Phases

- Phase 0: design and contracts
  - finalize `.d1v/project.json`
  - finalize ignore behavior
  - finalize backend sync model
- Phase 1: `init` MVP
  - local scan
  - manifest upload
  - new project creation
  - metadata write
- Phase 2: `pull` MVP
  - pull plan
  - download changed files
  - dirty state warning
- Phase 3: `push` MVP
  - local diff
  - upload changed files
  - revision commit
- Phase 4: polish
  - better conflict reporting
  - resumable transfer
  - binary handling strategy
  - Git-aware UX improvements

## Open Product Questions

- Should `d1v init <path>` always create a new remote project first, or should it also support binding an existing project into a local directory?
- Should local metadata live in `.d1v/project.json`, `.d1v/config.toml`, or git-compatible local config to reduce repo noise?
- Should `d1v pull` write files directly, or stage them into a preview area unless `--apply` is passed?
- Should `session run` stream assistant output inline by default, or only show session IDs unless `--follow` is set?
- Should `project create` accept free-form prompt only, or also explicit template/model flags from day one?
- Should GitHub `bind` be a pure browser launcher first, with API-backed device-style binding later?
- Should migration approval steps stay explicit subcommands, or should `db migrate execute` optionally chain `plan -> validate -> approval -> execute` in one guarded flow?
