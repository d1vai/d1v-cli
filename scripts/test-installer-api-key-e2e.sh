#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

FAKE_BIN="$WORK_DIR/fake-bin"
RECORD_DIR="$WORK_DIR/records"
mkdir -p "$FAKE_BIN" "$RECORD_DIR"
export TEST_RECORD_DIR="$RECORD_DIR"

cat >"$FAKE_BIN/curl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

output=""
url=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    -o)
      output="$2"
      shift 2
      ;;
    -* ) shift ;;
    * ) url="$1"; shift ;;
  esac
done

if [ -z "$output" ]; then
  printf '%s\n' '{"tag_name":"v0.0.0-e2e"}'
elif [ "${url##*/}" = "checksums.txt" ]; then
  cat >"$output" <<'SUMS'
deadbeef  d1v-aarch64-apple-darwin.tar.gz
deadbeef  d1v-x86_64-apple-darwin.tar.gz
deadbeef  d1v-aarch64-unknown-linux-musl.tar.gz
deadbeef  d1v-x86_64-unknown-linux-musl.tar.gz
SUMS
else
  : >"$output"
fi
EOF

cat >"$FAKE_BIN/shasum" <<'EOF'
#!/usr/bin/env bash
printf 'deadbeef  %s\n' "${@: -1}"
EOF

cat >"$FAKE_BIN/tar" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

destination=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "-C" ]; then
    destination="$2"
    break
  fi
  shift
done

cat >"$destination/d1v" <<'CLI'
#!/usr/bin/env bash
set -euo pipefail

if [ "${1:-}" = "auth" ] && [ "${2:-}" = "login" ] && [ "${3:-}" = "--api-key" ]; then
  IFS= read -r key
  printf '%s' "$key" >"$TEST_RECORD_DIR/auth-stdin"
  printf '%s\n' "$*" >"$TEST_RECORD_DIR/auth-args"
  exit 0
fi

if [ "${1:-}" = "skill" ] && [ "${2:-}" = "install" ]; then
  printf '%s\n' "$*" >"$TEST_RECORD_DIR/skill-args"
  exit 0
fi

printf 'unexpected fake d1v invocation: %s\n' "$*" >&2
exit 1
CLI
chmod +x "$destination/d1v"
EOF

chmod +x "$FAKE_BIN/curl" "$FAKE_BIN/shasum" "$FAKE_BIN/tar"
printf '#!/usr/bin/env bash\n' >"$FAKE_BIN/codex"
chmod +x "$FAKE_BIN/codex"

run_installer() {
  local installer="$1"
  local label="$2"
  local install_dir="$WORK_DIR/$label-bin"
  local secret="sk-installer-e2e-$label"
  local output

  rm -f "$RECORD_DIR/auth-stdin" "$RECORD_DIR/auth-args" "$RECORD_DIR/skill-args"
  output="$(
    PATH="$FAKE_BIN:$PATH" bash "$installer" \
      --version v0.0.0-e2e \
      --install-dir "$install_dir" \
      --no-modify-path \
      --api-key "$secret" \
      --install-skill all
  )"

  [ "$(cat "$RECORD_DIR/auth-stdin")" = "$secret" ]
  [ "$(cat "$RECORD_DIR/auth-args")" = "auth login --api-key" ]
  [ "$(cat "$RECORD_DIR/skill-args")" = "skill install --agent all" ]
  if printf '%s' "$output" | grep -F "$secret" >/dev/null; then
    echo "$label installer leaked the API key in output" >&2
    exit 1
  fi
  printf '%s' "$output" | grep -F "d1v auth status" >/dev/null
}

run_plain_installer() {
  local installer="$1"
  local label="$2"
  local install_dir="$WORK_DIR/$label-plain-bin"
  local output

  output="$(
    PATH="$FAKE_BIN:$PATH" bash "$installer" \
      --version v0.0.0-e2e \
      --install-dir "$install_dir" \
      --no-modify-path
  )"

  printf '%s' "$output" | grep -F "d1v auth login" >/dev/null
  [ "$(cat "$RECORD_DIR/skill-args")" = "skill install --agent auto" ]
  if printf '%s' "$output" | grep -F "d1v auth status" >/dev/null; then
    echo "$label plain installer incorrectly reported an authenticated next step" >&2
    exit 1
  fi
}

run_plain_installer "$ROOT_DIR/install.sh" "cli"
run_plain_installer "$ROOT_DIR/../d1vai/public/install/d1v-cli.sh" "web"
run_installer "$ROOT_DIR/install.sh" "cli"
run_installer "$ROOT_DIR/../d1vai/public/install/d1v-cli.sh" "web"

run_no_agent_installer() {
  local installer="$1"
  local label="$2"
  local install_dir="$WORK_DIR/$label-no-agent-bin"
  local output

  rm -f "$RECORD_DIR/skill-args"
  mv "$FAKE_BIN/codex" "$WORK_DIR/codex-disabled"
  output="$(
    PATH="$FAKE_BIN:/usr/bin:/bin" bash "$installer" \
      --version v0.0.0-e2e \
      --install-dir "$install_dir" \
      --no-modify-path
  )"
  mv "$WORK_DIR/codex-disabled" "$FAKE_BIN/codex"

  printf '%s' "$output" | grep -F "skipping d1v skill installation" >/dev/null
  [ ! -e "$RECORD_DIR/skill-args" ]
}

run_no_agent_installer "$ROOT_DIR/install.sh" "cli"
run_no_agent_installer "$ROOT_DIR/../d1vai/public/install/d1v-cli.sh" "web"

echo "Installer API-key bootstrap E2E passed."
