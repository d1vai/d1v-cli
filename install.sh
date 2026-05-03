#!/usr/bin/env bash
set -euo pipefail

REPO="${D1V_INSTALL_REPO:-d1v-ai/d1v-cli}"
INSTALL_DIR="${D1V_INSTALL_DIR:-$HOME/.local/bin}"
VERSION=""
PRINT_ONLY="false"
NO_MODIFY_PATH="false"

usage() {
  cat <<'EOF'
Install d1v-cli from GitHub Releases.

Usage:
  curl -fsSL https://d1v.ai/install/d1v-cli.sh | bash

Options:
  --version <tag>       Install a specific release tag
  --install-dir <dir>   Override target install directory
  --print-url           Print the resolved download URL and exit
  --no-modify-path      Do not append INSTALL_DIR to shell rc files
  --help                Show this help
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --version)
      VERSION="$2"
      shift 2
      ;;
    --install-dir)
      INSTALL_DIR="$2"
      shift 2
      ;;
    --print-url)
      PRINT_ONLY="true"
      shift
      ;;
    --no-modify-path)
      NO_MODIFY_PATH="true"
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

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "Missing required command: $1" >&2
    exit 1
  }
}

need_cmd curl
need_cmd tar
need_cmd uname
need_cmd mktemp
need_cmd install
need_cmd shasum

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "$OS" in
  darwin) TARGET_OS="apple-darwin" ;;
  linux) TARGET_OS="unknown-linux-gnu" ;;
  *)
    echo "Unsupported OS: $OS" >&2
    exit 1
    ;;
esac

case "$ARCH" in
  x86_64|amd64) TARGET_ARCH="x86_64" ;;
  arm64|aarch64) TARGET_ARCH="aarch64" ;;
  *)
    echo "Unsupported architecture: $ARCH" >&2
    exit 1
    ;;
esac

TARGET="${TARGET_ARCH}-${TARGET_OS}"

if [ -z "$VERSION" ]; then
  VERSION="$(
    curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
      | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' \
      | head -n 1
  )"
fi

if [ -z "$VERSION" ]; then
  echo "Failed to resolve latest release tag" >&2
  exit 1
fi

ARCHIVE="d1v-${TARGET}.tar.gz"
CHECKSUM_FILE="checksums.txt"
BASE_URL="https://github.com/${REPO}/releases/download/${VERSION}"
URL="${BASE_URL}/${ARCHIVE}"
CHECKSUM_URL="${BASE_URL}/${CHECKSUM_FILE}"

if [ "$PRINT_ONLY" = "true" ]; then
  printf '%s\n' "$URL"
  exit 0
fi

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Installing d1v-cli version: $VERSION"
echo "Target: $TARGET"
echo "Download: $URL"

mkdir -p "$INSTALL_DIR"
curl -fL "$URL" -o "$TMP_DIR/$ARCHIVE"
curl -fL "$CHECKSUM_URL" -o "$TMP_DIR/$CHECKSUM_FILE"

EXPECTED_SUM="$(awk -v name="$ARCHIVE" '$2 == name { print $1 }' "$TMP_DIR/$CHECKSUM_FILE")"
if [ -z "$EXPECTED_SUM" ]; then
  echo "Could not find checksum for $ARCHIVE" >&2
  exit 1
fi

ACTUAL_SUM="$(shasum -a 256 "$TMP_DIR/$ARCHIVE" | awk '{print $1}')"
if [ "$EXPECTED_SUM" != "$ACTUAL_SUM" ]; then
  echo "Checksum verification failed for $ARCHIVE" >&2
  exit 1
fi

tar -xzf "$TMP_DIR/$ARCHIVE" -C "$TMP_DIR"

if [ ! -f "$TMP_DIR/d1v" ]; then
  echo "Archive did not contain expected d1v binary" >&2
  exit 1
fi

install -m 0755 "$TMP_DIR/d1v" "$INSTALL_DIR/d1v"

append_path() {
  local rc_file="$1"
  local line="export PATH=\"$INSTALL_DIR:\$PATH\""

  [ -f "$rc_file" ] || touch "$rc_file"
  if ! grep -F "$line" "$rc_file" >/dev/null 2>&1; then
    printf '\n%s\n' "$line" >>"$rc_file"
    echo "Added d1v to PATH in $rc_file"
  fi
}

if [ "$NO_MODIFY_PATH" != "true" ]; then
  case "${SHELL##*/}" in
    zsh) append_path "$HOME/.zshrc" ;;
    bash) append_path "$HOME/.bashrc" ;;
  esac
fi

cat <<EOF

  __   __
 / /  / /_ _   __
/ /__/ /  ' \\ / /
\\____/_/_/_/_/_/

d1v-cli installed to: $INSTALL_DIR/d1v

Next:
  d1v auth login
  d1v project list
  d1v github status

More:
  https://d1v.ai/cli-install
EOF
