#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INSTALL_SCRIPT="$ROOT_DIR/install.sh"
REPO="${D1V_INSTALL_REPO:-d1vai/d1v-cli}"
INITIAL_VERSION=""
TARGET_VERSION=""
WORK_DIR=""
KEEP_WORK_DIR="false"

usage() {
  cat <<'EOF'
Run install -> upgrade -> uninstall E2E against GitHub Releases.

Usage:
  scripts/test-install-upgrade-uninstall-e2e.sh --initial-version <tag> --target-version <tag>

Options:
  --initial-version <tag>  Starting release to install
  --target-version <tag>   Target release to upgrade/install to
  --install-script <path>  Override installer path
  --repo <owner/name>      Override GitHub repo
  --work-dir <dir>         Reuse a specific temp directory
  --keep-work-dir          Do not delete the work directory on exit
  --help                   Show this help
EOF
}

normalize_version() {
  local input="$1"
  if [ "${input#v}" != "$input" ] || [ "${input#V}" != "$input" ]; then
    printf '%s\n' "$input"
  else
    printf 'v%s\n' "$input"
  fi
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --initial-version)
      INITIAL_VERSION="$(normalize_version "$2")"
      shift 2
      ;;
    --target-version)
      TARGET_VERSION="$(normalize_version "$2")"
      shift 2
      ;;
    --install-script)
      INSTALL_SCRIPT="$2"
      shift 2
      ;;
    --repo)
      REPO="$2"
      shift 2
      ;;
    --work-dir)
      WORK_DIR="$2"
      shift 2
      ;;
    --keep-work-dir)
      KEEP_WORK_DIR="true"
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      exit 1
      ;;
  esac
done

if [ -z "$INITIAL_VERSION" ] || [ -z "$TARGET_VERSION" ]; then
  usage >&2
  exit 1
fi

command -v shasum >/dev/null 2>&1 || {
  echo "Missing required command: shasum" >&2
  exit 1
}

if [ -z "$WORK_DIR" ]; then
  WORK_DIR="$(mktemp -d)"
fi

if [ "$KEEP_WORK_DIR" != "true" ]; then
  trap 'rm -rf "$WORK_DIR"' EXIT
fi

INSTALL_DIR="$WORK_DIR/bin"
mkdir -p "$INSTALL_DIR"

echo "E2E work dir: $WORK_DIR"
echo "Repo: $REPO"
echo "Install script: $INSTALL_SCRIPT"
echo "Initial version: $INITIAL_VERSION"
echo "Target version: $TARGET_VERSION"

export D1V_INSTALL_REPO="$REPO"

echo
echo "==> Install initial release"
bash "$INSTALL_SCRIPT" \
  --version "$INITIAL_VERSION" \
  --install-dir "$INSTALL_DIR" \
  --no-modify-path

if [ ! -x "$INSTALL_DIR/d1v" ]; then
  echo "Install failed: $INSTALL_DIR/d1v not found" >&2
  exit 1
fi

before_sha="$(shasum -a 256 "$INSTALL_DIR/d1v" | awk '{print $1}')"
echo "Installed SHA: $before_sha"
echo "Installed version output: $("$INSTALL_DIR/d1v" --version)"

echo
echo "==> Upgrade to target release"
if "$INSTALL_DIR/d1v" upgrade --help >/dev/null 2>&1; then
  echo "Using self-upgrade command"
  "$INSTALL_DIR/d1v" upgrade --version "$TARGET_VERSION"
else
  echo "Installed release has no upgrade command; falling back to installer-based upgrade"
  bash "$INSTALL_SCRIPT" \
    --version "$TARGET_VERSION" \
    --install-dir "$INSTALL_DIR" \
    --no-modify-path
fi

after_sha="$(shasum -a 256 "$INSTALL_DIR/d1v" | awk '{print $1}')"
echo "Upgraded SHA: $after_sha"
echo "Upgraded version output: $("$INSTALL_DIR/d1v" --version)"

if [ "$before_sha" = "$after_sha" ] && [ "$INITIAL_VERSION" != "$TARGET_VERSION" ]; then
  echo "Upgrade failed: binary checksum did not change" >&2
  exit 1
fi

echo
echo "==> Post-upgrade check"
if "$INSTALL_DIR/d1v" upgrade --help >/dev/null 2>&1; then
  "$INSTALL_DIR/d1v" upgrade --check
else
  echo "Skipping post-upgrade self-check because target release has no upgrade command"
fi

echo
echo "==> Uninstall"
if "$INSTALL_DIR/d1v" uninstall --help >/dev/null 2>&1; then
  "$INSTALL_DIR/d1v" uninstall --keep-path
else
  bash "$INSTALL_SCRIPT" \
    --install-dir "$INSTALL_DIR" \
    --uninstall \
    --no-modify-path
fi

if [ -e "$INSTALL_DIR/d1v" ]; then
  echo "Uninstall failed: $INSTALL_DIR/d1v still exists" >&2
  exit 1
fi

echo
echo "E2E success: install -> upgrade -> uninstall completed."
