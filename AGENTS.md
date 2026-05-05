# Agent Guide (AGENTS.md)

Use this file as the execution contract for work in `d1v-cli`.

## Planning First

- Every non-trivial task must start by pruning stale or already-compressed completed items from `PLAN.md`, then writing the current goal, background, selected validators, and todo list before editing.
- `PLAN.md` is the source of truth for scope, order, verification, evidence, and residual risk.
- Keep at most one todo `in_progress` per execution thread unless work was intentionally delegated in parallel.

## Checkoff Rule

- Do not check off a todo until its assigned validators have passed and the evidence is written into `PLAN.md`.
- If verification fails, keep the todo open, record the failure, and add the corrective next step.
- Never mark work done based on implementation alone.

## Minimum Verification Before Checkoff

- `@cli-ux-qa`: before checking off a CLI UX todo, validate the final command shape, help text, flag names, exit semantics, and human-readable output against the intended user workflow.
- `@cli-json-qa`: before checking off an automation-facing todo, confirm `--format json` returns stable machine-readable output and avoids leaking human-only hints into stdout.
- `@api-backend-qa`: before checking off an API-integrated todo, run at least one local smoke command proving the happy path works, auth/error handling is correct, and returned data matches the expected shape.
- `@auth-state-qa`: before checking off auth work, verify logged-out, logged-in, expired-token, and non-interactive flows.
- `@docs-adoption-qa`: before checking off command-surface or workflow work, update the plan and command examples so a new CLI user can discover the path without reading source code.

## Default Commands

- `cargo fmt --all`
- `cargo test`
- `cargo run -p d1v-cli -- --help`
- `cargo run -p d1v-cli -- --format json debug`

Treat failing formatting, tests, or help-output regressions as blockers and record any environment blocker in `PLAN.md`.

## Rust Toolchain

- MSRV is Rust 1.95. Any stable feature available at this MSRV is allowed, including `if let` guards in `match` arms (`Some(x) if let Ok(y) = ... => ...`).

## Validator Registry

- `@cli-ux-qa`: command naming, flag ergonomics, help text, interactive vs non-interactive behavior, exit-code expectations
- `@cli-json-qa`: JSON stdout shape, scripting compatibility, stderr/stdout separation, stable field names
- `@api-backend-qa`: request flow, auth gating, error handling, response normalization, pagination assumptions
- `@auth-state-qa`: login/logout/status, token precedence, expiry handling, re-login prompts, CI-safe token usage
- `@docs-adoption-qa`: README/PLAN examples, discoverability, migration guidance from web flow to CLI flow
- `@project-lifecycle-qa`: project create/list/get/update/delete, template/model/auto-deploy assumptions
- `@session-runtime-qa`: AI run/continue/history/status flows, workspace readiness, long-running session handling
- `@deploy-release-qa`: preview/prod deploys, logs, release history, merge/revert semantics
- `@db-workflow-qa`: schema inspection, row operations, migration planning/approval/execution/history
- `@github-integration-qa`: GitHub binding, installation discovery, import flows, fallback browser redirects
- `@billing-analytics-qa`: payment, webhooks, analytics, export/reporting behavior and scope boundaries

## Working Rules

- Keep diffs focused. Avoid unrelated refactors.
- Prefer existing Rust modules, shared output helpers, and request clients over introducing parallel abstractions.
- Do not hardcode secrets or commit credentials, build output, recordings, or local caches.
- Prefer `rg` for search and read files in small chunks.
- For commands that depend on web-only setup, record the browser handoff explicitly in `PLAN.md` before implementing it.
- When a CLI flow cannot complete fully in terminal, prefer a first-class `open` or `bind` command that explains and launches the correct web page instead of leaving the user with hidden prerequisites.

## Handoff Format

- Each validator handoff must include `Result`, `Checked`, `Passed`, `Failed`, `Not checked`, `Risk`, and `Plan update`.
