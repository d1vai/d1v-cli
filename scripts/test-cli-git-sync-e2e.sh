#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$ROOT_DIR"

echo "[e2e] cargo test -p d1v-cli workspace git sync helpers"
cargo test -p d1v-cli workspace::tests::git_helpers_report_repo_state -- --exact
cargo test -p d1v-cli workspace::tests::git_helpers_pull_fast_forward_syncs_changes -- --exact
cargo test -p d1v-cli workspace::tests::git_helpers_push_head_to_remote_branch -- --exact

echo "[e2e] cargo test -p d1v-api github app cli access client"
cargo test -p d1v-api github_app_project_cli_access -- --exact
cargo test -p d1v-api github_app_project_git_credential -- --exact

echo "[e2e] done"
