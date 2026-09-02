<div align="center">

# D1V CLI

Deploy, inspect, and work inside D1V projects from your terminal.

[![CI](https://github.com/d1vai/d1v-cli/actions/workflows/ci.yaml/badge.svg)](https://github.com/d1vai/d1v-cli/actions/workflows/ci.yaml)
[![Release](https://img.shields.io/github/v/release/d1vai/d1v-cli?display_name=tag)](https://github.com/d1vai/d1v-cli/releases)
[![crates.io](https://img.shields.io/crates/v/d1v-cli)](https://crates.io/crates/d1v-cli)
[![Homebrew](https://img.shields.io/homebrew/v/d1v?tap=d1vai%2Ftap)](https://github.com/d1vai/homebrew-tap)
[![License](https://img.shields.io/github/license/d1vai/d1v-cli)](LICENSE)
[![Stars](https://img.shields.io/github/stars/d1vai/d1v-cli)](https://github.com/d1vai/d1v-cli/stargazers)
[![Rust](https://img.shields.io/badge/rust-1.95%2B-orange)](https://www.rust-lang.org/)

Project-aware by default. Works with Codex and Claude Code. Supports cloud and local runtimes.

<samp>

**[English](README.md)** ┃ **[简体中文](README.zh-Hans.md)**

</samp>

</div>

## Quick Start

Install, authenticate, and deploy the current directory:

```sh
curl -fsSL https://d1v.ai/install/d1v-cli.sh | bash
d1v auth login --browser
d1v --preview   # or: d1v --prev
d1v --prod
```

The CLI reads `D1V_PROJECT_ID` from the current directory's `.env` first, then
from the process environment and `.d1v/project.json`. Explicit project
arguments always win. When no project can be resolved interactively, choose one
from the project picker. Cloud environment variables are merged into `.env`;
local values are kept by default when keys conflict.

Run `d1v --help` for the complete command list.

## Coding Agent Skill

The official, versioned d1v Skill lives at
[`skills/d1v/SKILL.md`](skills/d1v/SKILL.md). It gives Codex and Claude Code
safe instructions for project workspaces, container commands, preview
deployments, and user-confirmed production releases.

The curl installer uses `--install-skill auto` by default. It installs only
for Codex and Claude Code executables already available on `PATH`; it does not
install either coding agent and succeeds without changes when neither is found.
Use the same behavior manually:

```sh
d1v skill install --agent auto
```

Use `--agent codex`, `--agent claude`, or `--agent all` to explicitly choose
targets. Skills are written below `${CODEX_HOME:-~/.codex}/skills/d1v` and
`${CLAUDE_CONFIG_DIR:-~/.claude}/skills/d1v`. An identical Skill is left alone;
a different existing `SKILL.md` is backed up beside it as
`SKILL.md.d1v-backup-<UTC timestamp>` before replacement. The legacy
`https://www.d1v.ai/cli-skill.md` URL remains available and redirects to this
canonical file.

## D1V Mobile App

Pair the terminal workflow with [`d1vai_app`](https://github.com/d1vai/d1vai_app),
the official open-source Flutter client for d1v.ai. Use the CLI for fast local
and CI workflows, and the mobile app to create projects, continue AI sessions,
inspect files, monitor deployments, and manage your workspace away from a desk.

[![d1vai_app Stars](https://img.shields.io/github/stars/d1vai/d1vai_app?label=d1vai_app%20stars)](https://github.com/d1vai/d1vai_app/stargazers)
[![Flutter](https://img.shields.io/badge/Flutter-3.x-02569B?logo=flutter&logoColor=white)](https://flutter.dev/)
[![Mobile](https://img.shields.io/badge/mobile-iOS%20%7C%20Android-16a34a)](https://github.com/d1vai/d1vai_app/releases)

The companion app supports iOS and Android and includes its own screenshots,
release downloads, and developer guide:

- [d1vai_app repository](https://github.com/d1vai/d1vai_app)
- [Mobile releases](https://github.com/d1vai/d1vai_app/releases)
- [d1v.ai documentation](https://www.d1v.ai/docs/overview)

## Local Runtime

D1V supports a local runtime in addition to the existing cloud runtime.

Role split:

- `opcode-api`: the local runtime server
- `d1v-cli`: installer, launcher, supervisor, connector
- `backend_admin`: control plane and runtime router
- `d1vai`: unified frontend

The frontend never connects directly to the user machine. The local agent opens an outbound connection to D1V cloud.

### Local Runtime Quickstart

1. Install and inspect the local runtime:

```sh
d1v runtime doctor
bash scripts/install-opcode-runtime.sh --home ~/d1v-home
```

2. Pair the machine:

```sh
d1v agent pair
```

Or keep using a pairing code generated in the web UI:

```sh
d1v agent pair --code <pairing-code>
```

3. Start the local runtime:

```sh
d1v agent start
```

4. Create or bind local project directories:

```sh
d1v agent project create --project-id <project_id> --name my-app
d1v agent project import --path ~/work/my-app --project-id <project_id>
d1v agent project bind --project-id <project_id> --path ~/work/my-app
```

Backward-compatible entry:

```sh
d1v agent init-runtime --project-id <project_id> --path ~/work/my-app
```

### Local Runtime Behavior

- Runtime switching affects new sessions only.
- Existing sessions stay pinned to the runtime where they started.
- If a project is bound to local runtime and the device is offline, requests return an explicit local-runtime error instead of silently falling back to cloud.
- Project creation/import flows may still use cloud opcode directly before runtime binding exists.

### Public Expose

CLI free expose does not require a running node agent:

```sh
d1v expose 3000
d1v expose list
d1v expose close <binding_id>
```

Platform node ingress stays on a separate command path:

```sh
d1v node expose 3000 --node-id <platform-node-id>
d1v node expose list
d1v node expose close <binding_id>
```

Current expose modes:

- `cli_free_relay`: returns a public `https://*.cli-free.d1v.dev` URL for login-backed temporary CLI relays
- `cloudflare_tunnel`: returns a public `https://*.node.d1v.dev` URL for platform nodes
- `reverse_relay`: internal fallback mode used by local/customer relays behind the CLI-free entry

`d1v expose` currently targets HTTP traffic. Browser terminal and session WebSocket flows still use the existing backend relay path.

### Platform Node

Platform nodes now use a single control origin for runtime-agent ingress. The recommended bootstrap form is:

```sh
d1v node start --key <platform-node-key> --control-origin https://{your-host-or-name}-node.d1v.dev
```

If `--control-origin` is omitted, the runtime-agent falls back to its own public IP detection and control-plane registration flow.

### Privacy Boundary

Current phase:

- paired devices only
- outbound-only relay
- short-lived pairing code
- stored device public key

Current limitation:

- relay traffic is not zero-knowledge
- platform metadata and chat/session records still follow existing D1V persistence behavior

For deeper architecture details, see [docs/d1v-agent-architecture.md](../docs/d1v-agent-architecture.md).

## Install

Recommended:

```sh
curl -fsSL https://d1v.ai/install/d1v-cli.sh | bash
```

Install page:

```sh
https://d1v.ai/cli-install
```

Alternatives:

```sh
brew install d1vai/tap/d1v
cargo binstall d1v-cli
cargo install --locked d1v-cli
```

After install:

```sh
d1v auth login
d1v project list
d1v github status
```

To authenticate through an existing browser session and save a revocable device
API key, use `d1v auth login --browser`. The browser approves a one-time,
10-minute session; the CLI stores the resulting key in `~/.d1v/config.toml`.

Upgrade later:

```sh
d1v upgrade
d1v upgrade --version v0.1.5
d1v uninstall
```

`d1v upgrade` replaces only the executable. Your login remains in
`~/.d1v/config.toml`; upgrading does not sign you out or copy credentials into
the release archive.

### Global Options

| Option     | Description                | Default         |
| ---------- | -------------------------- | --------------- |
| `--format` | Output format (text, json) | text            |
| `--color`  | Color output               | auto            |
| `--lang`   | Display language           | System / Config |
| `-v`       | Increase log verbosity     | warn            |

### Environment Variables

| Variable          | Description          |
| ----------------- | -------------------- |
| `D1V_API_KEY`     | API key              |
| `D1V_AUTH_TOKEN`  | Auth token           |
| `D1V_BASE_URL`    | API base URL         |
| `D1V_FORMAT`      | Output format        |
| `D1V_LANG`        | Display language     |
| `D1V_LOG_FILE`    | Log file path        |
| `D1V_RECORD_FILE` | HTTP recording file  |
| `NO_COLOR`        | Disable color output |
| `RUST_LOG`        | Log filter           |

### Authentication

| Command           | Description      |
| ----------------- | ---------------- |
| `d1v auth login`  | Log in           |
| `d1v auth logout` | Log out          |
| `d1v auth status` | Show auth status |

### Configuration

| Command            | Description                     |
| ------------------ | ------------------------------- |
| `d1v config show`  | Show current configuration      |
| `d1v config get`   | Get a config value              |
| `d1v config set`   | Set a config value              |
| `d1v config list`  | List available config keys      |
| `d1v config path`  | Print config file path          |
| `d1v config reset` | Reset configuration to defaults |
| `d1v config edit`  | Open config file in editor      |

Available config keys:

| Key        | Description               |
| ---------- | ------------------------- |
| `base_url` | API base URL              |
| `language` | Display language override |

### User

| Command           | Description            |
| ----------------- | ---------------------- |
| `d1v user info`   | Show current user info |
| `d1v user update` | Update user info       |
| `d1v user get`    | Get a public user      |
| `d1v user list`   | List all users         |

### Password

| Command                   | Description    |
| ------------------------- | -------------- |
| `d1v user password set`   | Set a password |
| `d1v user password reset` | Reset password |

### Email

| Command                 | Description   |
| ----------------------- | ------------- |
| `d1v user email bind`   | Bind an email |
| `d1v user email change` | Change email  |

### Invitation & Onboarding

| Command                      | Description                 |
| ---------------------------- | --------------------------- |
| `d1v user invitation accept` | Accept an invitation        |
| `d1v user invitation list`   | List invited users          |
| `d1v user onboard`           | Mark onboarding as complete |

### Activity

| Command             | Description                |
| ------------------- | -------------------------- |
| `d1v user activity` | View prompt daily activity |

### Diagnostics

| Command     | Description            |
| ----------- | ---------------------- |
| `d1v debug` | Show debug information |
| `d1v upgrade` | Check for updates and self-upgrade |
| `d1v uninstall` | Remove the current d1v executable |

## Project Workflows

These commands require authentication. Start with:

```sh
d1v auth status
d1v auth login
```

### Core Resources

| Area | Capability | Start here |
| --- | --- | --- |
| Projects | List, inspect, create, update, delete | `d1v project list` |
| Sessions | Run or continue AI coding sessions | `d1v session run ...` |
| Deployments | Preview, production, status, history, logs | `d1v --prev` |
| Containers | Open a PTY or run one command | `d1v shell` / `d1v exec -- ...` |
| Environment | Read, set, import, export, sync variables | `d1v env list` |
| Database | Inspect schema/data and manage migrations | `d1v db schema ...` |
| GitHub | Bind repositories and import projects | `d1v github status` |
| Expose | Publish a local HTTP port temporarily | `d1v expose 3000` |
| Runtime | Pair and run a local runtime | `d1v agent pair` |

Project resolution is consistent across project-scoped commands:

```text
explicit project ID
-> current .env: D1V_PROJECT_ID
-> D1V_PROJECT_ID environment variable
-> workspace binding
```

### Container Terminal And Exec

Open an interactive terminal in the project resolved from the current `.env`:

```sh
d1v shell
d1v shell <project_id>
d1v shell --workspace
d1v shell --workspace --organization-id <organization_id>
```

Use `--workspace` to explicitly open workspace-root. A positional project ID always overrides the current directory context.

The interactive terminal requires a TTY and uses the container's native Bash/Zsh completion. For agents, CI, and commands whose output or status must be captured, use non-interactive `d1v exec` and pass argv after `--`:

```sh
d1v exec -- pwd
d1v exec --project-id <project_id> -- npm test
d1v exec --workspace -- pwd
d1v exec --workspace --organization-id <organization_id> -- pwd
d1v --format json exec --project-id <project_id> -- sh -c 'printf ok; printf problem >&2; exit 7'
```

Without `--project-id`, `d1v exec` runs in the project resolved from the current `.env`. Use `--workspace` for workspace-root commands.

Text mode streams remote stdout and stderr to the matching local streams. JSON mode returns `session_id`, `project_id`, `cwd`, `exit_code`, `stdout`, and `stderr`, while the CLI process preserves a nonzero remote exit status. Interactive shell does not support JSON output.

The CLI automatically selects an eligible direct-node connection and otherwise uses the backend relay. It sends an application heartbeat every 20 seconds so long-lived sessions stay active through intermediaries. Shell tickets are short-lived and sent in the WebSocket header, never in the URL or command output. Terminal input and output are not persisted by the terminal service.

Start an AI coding session and choose the engine explicitly when reproducibility matters:

```sh
d1v session run <project_id> --engine codex --prompt "Run tests and fix failures"
d1v session run <project_id> --engine claude --prompt "Review the authentication flow"
```

`--engine` is optional; the backend infers it from the selected model.

### Container Integration Ensure

When a container runtime injects `D1V_API_KEY` (or `D1V_AUTH_TOKEN`), `D1V_BASE_URL`, and `D1V_PROJECT_ID`, agents can enable project integrations on demand without a browser login:

```sh
d1v project ensure database
d1v project ensure db analytics
d1v --format json project ensure pay
```

### GitHub Handoff

Use CLI first, then jump to web only when setup is required:

```sh
d1v github status
d1v github bind
d1v github installations
d1v github repos --installation-id 123456
```

If GitHub App installation or OAuth binding is incomplete, `d1v github bind` opens the correct handoff page, including `https://d1v.ai/setting?tab=github` when needed.

### Database And Migration Smoke Checklist

After logging in and choosing a project id, the smallest end-to-end validation flow is:

```sh
d1v db token issue <project_id> --scopes db:read,migrate
d1v db schema <project_id>
d1v db rows list <project_id> --schema public --table your_table
d1v db migrate plan <project_id> --sql 'CREATE TABLE IF NOT EXISTS smoke_cli(id serial primary key);'
```

Useful follow-up commands:

```sh
d1v db migrate validate <plan_id>
d1v db migrate approve <plan_id>
d1v db migrate auto-review <approval_id>
```

## Development

### Prerequisites

- [Rust](https://www.rust-lang.org/) (stable 1.95+)
- [Task](https://taskfile.dev/) (optional)

### Build

```sh
cargo build
```

### Test

```sh
cargo test
```

### Run

```sh
cargo run
```

### Install

```sh
cargo install --path d1v-cli
```

## Debugging

### Environment

Check version, config path, and token status:

```sh
d1v debug
```

### Logging

Daily log files are written to `~/.d1v/d1v.YYYY-MM-DD.log`, keeping the last 7 days.

Increase stderr verbosity with `-v` (`-v` info, `-vv` debug, `-vvv` trace):

```sh
d1v -vv auth login
```

`RUST_LOG` is also supported when `-v` is not set:

```sh
RUST_LOG=debug d1v auth login
```

Write logs to a custom file:

```sh
d1v --log-file trace.log auth login
```

### HTTP Recording

Capture HTTP traffic to a JSON file for inspection.

Enable the `record` feature:

```sh
cargo install --path d1v-cli --features record
```

Run with recording:

```sh
d1v --record trace.json auth login
```

Config keys available with the `record` feature:

| Key              | Description                             |
| ---------------- | --------------------------------------- |
| `record.enabled` | Enable recording by default             |
| `record.dir`     | Directory to write recording files into |
