#!/usr/bin/env bash
set -euo pipefail

HOME_PATH="${D1V_AGENT_HOME_PATH:-$HOME/.d1v/agent/home}"
DEVICE_NAME="${D1V_AGENT_DEVICE_NAME:-}"
PAIR_CODE="${D1V_AGENT_PAIR_CODE:-}"
INSTALL_PACKAGES="${D1V_INSTALL_PACKAGES:-auto}"
START_AGENT="${D1V_AGENT_START_AFTER_SETUP:-false}"
DOCTOR_ONLY="false"

usage() {
  cat <<'EOF'
Bootstrap the local opcode-api runtime for d1v.

Usage:
  scripts/install-opcode-runtime.sh [options]

Options:
  --home <path>         Initialize the agent home at this path
  --device-name <name>  Set the local device display name
  --pair-code <code>    Pair the device after setup
  --start               Start d1v agent after setup
  --doctor-only         Only print dependency/runtime diagnostics
  --help                Show this help

Environment:
  D1V_INSTALL_PACKAGES=auto|never
    auto  Attempt to install missing packages via brew/apt-get when available
    never Only report missing packages
EOF
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --home)
      HOME_PATH="$2"
      shift 2
      ;;
    --device-name)
      DEVICE_NAME="$2"
      shift 2
      ;;
    --pair-code)
      PAIR_CODE="$2"
      shift 2
      ;;
    --start)
      START_AGENT="true"
      shift
      ;;
    --doctor-only)
      DOCTOR_ONLY="true"
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
  command -v "$1" >/dev/null 2>&1
}

install_missing_package() {
  local package="$1"
  if [ "$INSTALL_PACKAGES" = "never" ]; then
    return 1
  fi

  if need_cmd brew; then
    brew install "$package"
    return 0
  fi

  if need_cmd apt-get; then
    sudo apt-get update
    sudo apt-get install -y "$package"
    return 0
  fi

  return 1
}

ensure_command() {
  local command_name="$1"
  local package_name="${2:-$1}"
  if need_cmd "$command_name"; then
    return 0
  fi
  echo "Missing command: $command_name"
  if install_missing_package "$package_name"; then
    return 0
  fi
  echo "Unable to install $command_name automatically. Please install it and retry." >&2
  exit 1
}

detect_platform() {
  local os arch
  os="$(uname -s | tr '[:upper:]' '[:lower:]')"
  arch="$(uname -m)"
  echo "Platform: ${os}/${arch}"
}

ensure_dependencies() {
  ensure_command curl curl
  ensure_command tar tar
  ensure_command git git
  ensure_command python3 python3
  ensure_command d1v d1v
}

run_doctor() {
  d1v runtime doctor
}

init_home() {
  if [ -n "$DEVICE_NAME" ]; then
    d1v agent init-home --path "$HOME_PATH" --name "$DEVICE_NAME"
    return
  fi
  d1v agent init-home --path "$HOME_PATH"
}

pair_device() {
  if [ -n "$PAIR_CODE" ]; then
    if [ -n "$DEVICE_NAME" ]; then
      d1v agent pair --code "$PAIR_CODE" --name "$DEVICE_NAME"
    else
      d1v agent pair --code "$PAIR_CODE"
    fi
  fi
}

main() {
  detect_platform
  ensure_dependencies
  run_doctor || true

  if [ "$DOCTOR_ONLY" = "true" ]; then
    exit 0
  fi

  d1v runtime install
  init_home
  pair_device
  d1v runtime doctor

  if [ "$START_AGENT" = "true" ]; then
    d1v agent start
  fi
}

main "$@"
