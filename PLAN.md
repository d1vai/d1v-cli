# Goal: Make `d1v init` environment-first, resumable, and deployment-explicit

## Current Execution

- Goal: detect an existing `D1V_PROJECT_ID` before any upload, repair only
  missing selected development integrations, and otherwise default to an
  environment-only project rather than importing local files.
- Background: local import currently waits for integration setup in one 30s
  request and implicitly queues deployment, so a timeout loses the project
  binding and causes repeated uploads.
- Selected validators: `@cli-ux-qa`, `@cli-json-qa`, `@api-backend-qa`,
  `@project-lifecycle-qa`, `@docs-adoption-qa`.
- Todo:
  - [x] Extend the existing-project integration API to provision all five
    services idempotently and report per-service status.
    - Evidence: `ensure` now accepts/returns PAI, storage, Pay, database, and
      Resend; the Rust client contract test validates the exact request body.
  - [x] Add explicit local-import deployment scheduling and separate import,
    binding, provisioning, and local `.env` synchronization.
    - Evidence: `auto_deploy=false` suppresses the local-import fallback task;
      CLI persists the binding before provider setup and syncs secrets only via
      the protected environment export endpoint.
  - [x] Redesign CLI init mode/service prompts and non-interactive flags around
    existing project detection and resume behavior.
    - Evidence: new `--mode` supports environment-only, import-auto-deploy,
      and import-no-deploy; an existing `.env` project ID bypasses the mode
      prompt and asks only about missing service keys.
  - [x] Add focused API/backend/CLI tests, docs, formatter, test, help, and
    JSON output validation with recorded evidence.
    - Evidence: `cargo fmt --all -- --check`; `cargo test` (167 CLI + 2 binary
      tests); focused API integration contract test; valid JSON init dry run;
      and backend dev-environment tests (7 passed) all passed.

### Validator Handoff

- Result: passed for repository-scoped implementation.
- Checked: existing-project detection/resume, service-key selection, all-service
  provisioning contract, import deployment selection, `.env` merge, JSON
  output, help, Rust tests, and backend development-environment tests.
- Passed: all checks above.
- Failed: none.
- Not checked: live external provisioning or GitHub import, which would create
  cloud resources.
- Risk: provider calls can outlast ordinary HTTP requests; init must retain a
  completed import binding before later provisioning errors are returned.
- Plan update: complete; CLI requests now allow up to five minutes for remote
  import/workspace creation, while provisioning failures remain resumable.

### Authentication Correction

- Result: passed.
- Checked: a real protected `d1v user info --format json` request now succeeds
  with the config credential; the original direct-upload output was the older
  installed init behavior, which did not load a token before its cloud request.
- Passed: `auth status` now loads the saved credential and verifies it against
  the API, reporting a stable `invalid` state only for explicit auth failures;
  `cargo test` passed with 168 CLI unit tests and the backend target suite
  passed with 7 tests.
- Failed: none.
- Not checked: a live `init`, because it creates provider resources.
- Risk: a revoked credential will now correctly require `d1v auth login`; the
  CLI intentionally does not delete it until a user explicitly logs out or
  completes a replacement login.

### Current Execution

- Goal: display the authenticated user's email (falling back to slug) in
  `d1v auth status` after the server validation succeeds.
- Selected validators: `@auth-state-qa`, `@cli-json-qa`, `@cli-ux-qa`.
- Todo:
  - [x] Populate the existing status identity field from `/api/user/info`.
    - Evidence: verified status uses nonempty email first and falls back to slug.
  - [x] Cover text and JSON output, then format, test, and install the CLI.
    - Evidence: focused auth suite passed (9 tests); formatter and diff checks
      passed before release installation.
