#!/usr/bin/env bash
set -euo pipefail

resolve_script_path() {
  local source="$1"

  while [[ -h "$source" ]]; do
    local dir target
    dir="$(cd -P "$(dirname "$source")" && pwd)"
    target="$(readlink "$source")"

    if [[ "$target" != /* ]]; then
      source="$dir/$target"
    else
      source="$target"
    fi
  done

  printf '%s\n' "$(cd -P "$(dirname "$source")" && pwd)/$(basename "$source")"
}

log() {
  printf '[saicode-closeout] %s\n' "$*"
}

capture_saicode_pids() {
  local needle_a="${1:-}"
  local needle_b="${2:-}"

  {
    pgrep -af "$NATIVE_LAUNCHER|$RUST_FULL_CLI" 2>/dev/null || true
  } | awk -v a="$needle_a" -v b="$needle_b" '
    {
      pid = $1
      $1 = ""
      sub(/^ /, "", $0)
      if ((a == "" || index($0, a)) && (b == "" || index($0, b))) {
        print pid
      }
    }
  ' | sort -u
}

wait_for_probe_processes_to_exit() {
  local baseline="$1"
  local needle_a="${2:-}"
  local needle_b="${3:-}"
  local current
  local extra
  local attempt

  for attempt in $(seq 1 20); do
    current="$(capture_saicode_pids "$needle_a" "$needle_b")"
    extra="$(comm -13 <(printf '%s\n' "$baseline" | sed '/^$/d') <(printf '%s\n' "$current" | sed '/^$/d'))"
    if [[ -z "$extra" ]]; then
      return 0
    fi
    sleep 0.25
  done

  printf 'saicode process residue detected after probe: %s\n' "$extra" >&2
  pgrep -af "$NATIVE_LAUNCHER|$RUST_FULL_CLI" >&2 || true
  exit 1
}

SCRIPT_PATH="$(resolve_script_path "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd -P "$(dirname "$SCRIPT_PATH")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
NATIVE_LAUNCHER="$REPO_ROOT/native/saicode-launcher/target/release/saicode-launcher"
RUST_ONE_SHOT="$REPO_ROOT/rust/target/release/saicode-rust-one-shot"
RUST_LOCAL_TOOLS="$REPO_ROOT/rust/target/release/saicode-rust-local-tools"
RUST_FULL_CLI="$REPO_ROOT/rust/target/release/saicode-rust-cli"
INSTALLED_COMMAND="${HOME}/.local/bin/saicode"
TMP_DIR=""

cleanup() {
  if [[ -n "$TMP_DIR" && -d "$TMP_DIR" ]]; then
    rm -rf "$TMP_DIR"
  fi
}

trap cleanup EXIT

run_help_smoke() {
  local label="$1"
  shift
  log "$label"
  "$@" >/dev/null
}

run_full_cli_smoke() {
  local label="$1"
  shift
  log "$label"
  "$@" mcp --help >/dev/null
}

run_live_probe() {
  local command_path
  local output

  if [[ -x "$INSTALLED_COMMAND" ]]; then
    command_path="$INSTALLED_COMMAND"
  else
    command_path="$REPO_ROOT/bin/saicode"
  fi

  log "live print probe via $command_path"
  output="$("$command_path" -p 'Reply with exactly: ok')"
  if [[ "$output" != "ok" ]]; then
    printf 'expected live probe to return exactly "ok", got: %s\n' "$output" >&2
    exit 1
  fi
}

run_stream_json_probe() {
  local output

  log "stream-json probe via repo wrapper"
  output="$("$REPO_ROOT/bin/saicode" --bare -p --output-format stream-json 'Reply with exactly: ok')"
  if [[ "$output" != *'"type":"content_delta"'* || "$output" != *'"type":"final_message"'* ]]; then
    printf 'expected stream-json output to include content_delta and final_message, got: %s\n' "$output" >&2
    exit 1
  fi
}

run_interactive_progress_probe() {
  local baseline
  local output

  log "interactive tool progress probe"
  baseline="$(capture_saicode_pids "--bare --allowedTools Read")"
  bridge_baseline="$(capture_saicode_pids "ui-bridge")"
  output="$(printf 'Use Read to inspect README.md and reply with its first line only.\n/exit\n' | timeout 120 "$REPO_ROOT/bin/saicode" --bare --allowedTools Read)"
  wait_for_probe_processes_to_exit "$baseline" "--bare --allowedTools Read"
  wait_for_probe_processes_to_exit "$bridge_baseline" "ui-bridge"
  if [[ "$output" != *"[tool] Read"* || "$output" != *"# saicode"* ]]; then
    printf 'expected interactive output to show tool progress and README line, got: %s\n' "$output" >&2
    exit 1
  fi
}

run_rust_route_probe() {
  local output

  log "full cli route probe via repo wrapper"
  output="$(SAICODE_NATIVE_DRY_RUN=1 "$REPO_ROOT/bin/saicode" status 2>&1)"
  if [[ "$output" != *"route=full-cli target=$RUST_FULL_CLI"* ]]; then
    printf 'expected status to route to rust full CLI, got: %s\n' "$output" >&2
    exit 1
  fi

  log "recovery route probe via repo wrapper"
  output="$(SAICODE_NATIVE_DRY_RUN=1 "$REPO_ROOT/bin/saicode" -p 'Reply with exactly: ok' 2>&1)"
  if [[ "$output" != *"target=$RUST_FULL_CLI"* && "$output" != *"route=recovery"* ]]; then
    printf 'expected simple print to stay on a rust route, got: %s\n' "$output" >&2
    exit 1
  fi

  log "rust local-tools surface probe via repo wrapper"
  output="$("$REPO_ROOT/bin/saicode" --bare -p --allowedTools Read -- 'Use Read to inspect rust/Cargo.toml and reply with only the workspace package version.' 2>&1)"
  if [[ "$output" != *"0.1.0"* ]]; then
    printf 'expected rust local-tools surface probe to print 0.1.0, got: %s\n' "$output" >&2
    exit 1
  fi
}

main() {
  cd "$REPO_ROOT"

  run_help_smoke "repo wrapper help" ./bin/saicode --help
  run_help_smoke "rust full CLI help with native launcher disabled" env SAICODE_DISABLE_NATIVE_LAUNCHER=1 ./bin/saicode --help
  (
    cd "$HOME"
    run_full_cli_smoke "repo wrapper rust full CLI from home" env SAICODE_DISABLE_NATIVE_LAUNCHER=1 "$REPO_ROOT/bin/saicode"
  )

  TMP_DIR="$(mktemp -d)"
  ln -s "$REPO_ROOT/bin/saicode" "$TMP_DIR/saicode"
  run_help_smoke "symlinked rust CLI help" env SAICODE_DISABLE_NATIVE_LAUNCHER=1 "$TMP_DIR/saicode" --help
  (
    cd "$TMP_DIR"
    run_full_cli_smoke "symlinked rust full CLI outside repo" env SAICODE_DISABLE_NATIVE_LAUNCHER=1 "$TMP_DIR/saicode"
  )

  if [[ -x "$INSTALLED_COMMAND" ]]; then
    run_help_smoke "installed command help" "$INSTALLED_COMMAND" --help
    (
      cd "$HOME"
      run_full_cli_smoke "installed command rust full CLI from home" env SAICODE_DISABLE_NATIVE_LAUNCHER=1 "$INSTALLED_COMMAND"
    )
  else
    log "installed command not found at $INSTALLED_COMMAND; skipping installed-command smoke"
  fi

  if command -v cargo >/dev/null 2>&1; then
    if [[ -x "$NATIVE_LAUNCHER" ]]; then
      log "native launcher present: $NATIVE_LAUNCHER"
    else
      log "building native launcher for release-path smoke"
      cargo build --release --manifest-path native/saicode-launcher/Cargo.toml >/dev/null
    fi
    if [[ -x "$RUST_FULL_CLI" ]]; then
      log "rust full CLI present: $RUST_FULL_CLI"
    elif [[ -x "$REPO_ROOT/scripts/rust-cargo.sh" ]]; then
      log "building rust full CLI for route smoke"
      (
        cd "$REPO_ROOT/rust"
        "$REPO_ROOT/scripts/rust-cargo.sh" build --release -q -p saicode-rust-cli >/dev/null
      )
    else
      log "rust-cargo.sh not found; skipping rust full CLI build check"
    fi
    if [[ -x "$RUST_ONE_SHOT" ]]; then
      log "rust one-shot present: $RUST_ONE_SHOT"
    else
      log "rust one-shot optional for now; skipping dedicated build check"
    fi
    if [[ -x "$RUST_LOCAL_TOOLS" ]]; then
      log "rust local-tools present: $RUST_LOCAL_TOOLS"
    elif [[ -x "$REPO_ROOT/scripts/rust-cargo.sh" ]]; then
      log "building rust local-tools for local-tools smoke"
      (
        cd "$REPO_ROOT/rust"
        "$REPO_ROOT/scripts/rust-cargo.sh" build --release -q -p saicode-rust-local-tools >/dev/null
      )
    else
      log "rust-cargo.sh not found; skipping rust local-tools build check"
    fi
  else
    log "cargo not installed; skipping native release build check"
  fi

  run_help_smoke "repo wrapper full CLI rust route" "$REPO_ROOT/bin/saicode" status
  run_rust_route_probe
  run_stream_json_probe
  run_interactive_progress_probe

  if [[ "${SAICODE_CLOSEOUT_LIVE:-0}" == "1" ]]; then
    run_live_probe
  else
    log "live print probe skipped; set SAICODE_CLOSEOUT_LIVE=1 to enable"
  fi

  if [[ "${SAICODE_CLOSEOUT_ACCEPTANCE:-0}" == "1" ]]; then
    log "running rust tool acceptance suite"
    "$REPO_ROOT/scripts/rust_tool_acceptance.sh" >/dev/null
  else
    log "acceptance suite skipped; set SAICODE_CLOSEOUT_ACCEPTANCE=1 to enable"
  fi

  log "closeout preflight passed"
}

main "$@"
