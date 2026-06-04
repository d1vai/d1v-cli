<div align="center">

# D1V CLI

Experimental CLI for [d1v.ai](https://www.d1v.ai/).
<br><br>
<a href="https://github.com/fhluo/d1v-cli/actions/workflows/ci.yaml">
<img src="https://github.com/fhluo/d1v-cli/actions/workflows/ci.yaml/badge.svg" alt="ci workflow"></a>

<samp>

**[English](README.md)** ┃ **[简体中文](README.zh-Hans.md)**

</samp>

</div>

## Commands

Run `d1v --help` for all available commands.

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
brew install d1v-ai/tap/d1v
cargo binstall d1v-cli
cargo install --locked d1v-cli
```

After install:

```sh
d1v auth login
d1v project list
d1v github status
```

Upgrade later:

```sh
d1v upgrade
d1v upgrade --version v0.1.4
d1v uninstall
```

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

| Area      | Commands |
| --------- | -------- |
| Projects  | `d1v project list|get|create|update|delete|templates` |
| Sessions  | `d1v session run|continue|status|history|cancel` |
| Deploys   | `d1v deploy preview|prod|status|history|logs` |
| GitHub    | `d1v github status|bind|installations|repos|import` |
| Database  | `d1v db schema|data|branches|tables|rows|token|migrate` |

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
