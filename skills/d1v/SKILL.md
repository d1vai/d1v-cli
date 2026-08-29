---
name: d1v
description: Operate d1v.ai projects and container workspaces with the d1v CLI. Use when a task involves D1V authentication, project binding, workspace synchronization, container commands, environment metadata, preview deployments, deployment status, or a user-confirmed production release.
---

# d1v CLI

Use the installed `d1v` command. For automation and structured inspection, use
`--format json`; never mix JSON output with an interactive shell.

## Safety

- Never print, log, commit, or add API keys, tokens, environment values, or shell tickets to source files, command output, or repository configuration.
- Inspect authentication and project state before changing anything. Check the command exit code and query resulting state; a submitted request is not proof of success.
- Run `pull --dry-run` or `push --dry-run` before changing a local workspace, then inspect repository instructions and local changes.
- Treat preview and production as separate environments. A production release requires an interactive TTY and explicit user confirmation; do not attempt or imply a non-interactive production release.
- If a deployment fails or times out, preserve its deployment/release ID and use `deploy status` or `deploy history` rather than blindly retrying.

## Authenticate and discover

```bash
d1v auth status --format json
d1v project list --format json
d1v github status --format json
```

If authentication is missing, ask the user to run `d1v auth login`. Do not ask
them to paste a token into chat or a command line. An API key supplied through a
secure standard-input workflow can be used with `d1v auth login --api-key`.

## Bind a local workspace

```bash
d1v project get <project_id> --format json
d1v init . --project-id <project_id>
d1v pull --dry-run --format json
d1v pull
d1v push --dry-run --format json
d1v push
```

`d1v init` writes local workspace metadata. Do not use `--force` unless the
user explicitly approves replacing an existing binding.

## Run commands in a workspace

Use an interactive shell only when a person needs a terminal:

```bash
d1v shell
d1v shell <project_id>
d1v shell --organization-id <organization_id>
```

- With no target, `d1v shell` opens the personal workspace root.
- A project ID opens that project's directory. Do not combine it with `--organization-id`.
- Interactive shells require a TTY and text output. Do not use `--format json` with `d1v shell`.

For automation, Agent actions, CI, or captured exit status, use `d1v exec` and
place the remote argv after `--`:

```bash
d1v exec -- git status --short
d1v exec --project-id <project_id> -- npm test
d1v exec --organization-id <organization_id> -- pwd
d1v --format json exec --project-id <project_id> -- sh -c 'printf ok; printf problem >&2; exit 7'
```

Read `stdout`, `stderr`, `cwd`, and `exit_code`; a non-zero remote exit is a
failed action. Do not wrap commands in `sh -c` unless shell syntax is required.
Shell access uses a short-lived, single-use ticket. Never print, persist,
forward, or reconstruct terminal WebSocket URLs from that ticket.

## Environment metadata

```bash
d1v env list --project <project_id> --format json
d1v env set --project <project_id> KEY=VALUE --sensitive
```

The default list output keeps sensitive values redacted. Do not use `--reveal`,
`env get --reveal`, or `env export` unless the user explicitly asks for the
secret material and has provided a safe destination outside version control.

## Deployments

Create a preview with the explicit command when a project ID is known:

```bash
d1v deploy preview <project_id> --format json
d1v deploy status <project_id> --format json
d1v deploy history <project_id> --environment preview --format json
```

`d1v deploy preview` waits for the deployment's terminal state. Report success
only when it returns READY, and use its `production_url` when present, otherwise
its `vercel_url`. On failure, report the error and deployment ID without
repeating the request automatically.

`d1v --preview` (or `d1v --prev`) is the current-directory shortcut. It may
prompt to select or create a project and write the resulting project binding;
use it only when the user wants that local-directory workflow.

For production, first explain the pending operation and then require a clear
user confirmation in an interactive terminal:

```bash
d1v deploy prod <project_id>
```

The CLI shows release preflight information, prompts for relevant environment
decisions, creates an idempotent release, and waits for its terminal state. It
rejects non-interactive production releases. On failure, report the release
phase, error code, and message. The `d1v --prod` shortcut follows the same
confirmation and release flow for the bound current directory.

## Local runtime

```bash
d1v runtime doctor --format json
d1v agent status --format json
d1v agent pair
d1v agent start
```

Do not silently switch a requested local runtime to cloud when it is
unavailable. Explain the runtime status and ask the user how to proceed.

## References

- https://github.com/d1vai/d1v-cli
- https://www.d1v.ai/cli-install
- https://www.d1v.ai/docs/cli
- https://www.d1v.ai/openapi
