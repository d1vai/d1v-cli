# Goal: Turn d1v-cli From Account Utility Into A Real Project Workflow CLI

## Design Thinking And Demand Background

- ICP: developers, founders, and internal operators who want to drive `d1v.ai` from terminal, scripts, CI, and remote shells without depending on the web UI for every project action.
- Core pain: `d1v-cli` currently authenticates users and exposes account utilities, but it cannot yet create, inspect, run, deploy, import, or operate projects end to end.
- Desired outcome: a CLI that covers the high-frequency workflow spine first: project lifecycle, AI execution sessions, deployment, GitHub import/bind handoff, and database operations.
- CLI product standard for this cycle: every new command should be scriptable, discoverable via `--help`, usable in both human and automation contexts, and explicit about when the user must jump to web.
- Scope constraint: do not mechanically mirror every `d1vai` screen. Prefer terminal-native flows and browser handoff for setup-heavy or highly visual experiences.

## Validators For This Cycle

- Reuse the validator registry from `AGENTS.md`.
- Selected validators: `@cli-ux-qa`, `@cli-json-qa`, `@api-backend-qa`, `@auth-state-qa`, `@docs-adoption-qa`, `@project-lifecycle-qa`, `@session-runtime-qa`, `@deploy-release-qa`, `@db-workflow-qa`, `@github-integration-qa`, `@billing-analytics-qa`.

## Current Product Read

- Existing public surface is account-centric: `auth`, `user`, `config`, `debug`, and `banner`.
- Existing strengths worth preserving: token chain precedence, keyring/config fallback, localized text rendering, and `--format json`.
- Main product gap: missing project-centric command tree.
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
  - Evidence: added `d1v-api` GitHub App client plus `d1v github status|bind|installations|repos|import`; `bind` resolves OAuth connect URL when needed, otherwise falls back to install/settings URLs; `repos` is keyed by `--installation-id`; `import` maps the backend GitHub App import response including deployability hints; verified `cargo run -q -p d1v-cli -- github --help` and `cargo run -q -p d1v-cli -- --format json debug`
  - Notes: `bind` must either call API-backed connect state or open the browser to `https://d1v.ai/setting?tab=github`; if installation must happen on GitHub, the command should surface the install URL directly instead of hiding it. Repository discovery now depends on installation scope instead of a global repo list, which fits GitHub App semantics better.

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
- Prefer terminal verbs that map to backend actions directly: `list`, `get`, `create`, `update`, `delete`, `status`, `history`, `logs`, `run`, `import`.
- Keep human-readable summaries in text mode and stable machine contracts in JSON mode.
- Avoid hiding web-only prerequisites. If the user must complete setup in browser, the command should say so and ideally open the right page.
- Do not chase UI parity for charts, canvases, or relationship graphs. CLI should expose data, state, and operations.

## Open Product Questions

- Should `session run` stream assistant output inline by default, or only show session IDs unless `--follow` is set?
- Should `project create` accept free-form prompt only, or also explicit template/model flags from day one?
- Should GitHub `bind` be a pure browser launcher first, with API-backed device-style binding later?
- Should migration approval steps stay explicit subcommands, or should `db migrate execute` optionally chain `plan -> validate -> approval -> execute` in one guarded flow?
