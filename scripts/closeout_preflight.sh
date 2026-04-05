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

SCRIPT_PATH="$(resolve_script_path "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd -P "$(dirname "$SCRIPT_PATH")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
NATIVE_LAUNCHER="$REPO_ROOT/native/saicode-launcher/target/release/saicode-launcher"
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

main() {
  cd "$REPO_ROOT"

  run_help_smoke "repo wrapper help" ./bin/saicode --help
  run_help_smoke "bun fallback help" env SAICODE_DISABLE_NATIVE_LAUNCHER=1 ./bin/saicode --help
  (
    cd "$HOME"
    run_full_cli_smoke "repo wrapper full CLI fallback from home" env SAICODE_DISABLE_NATIVE_LAUNCHER=1 "$REPO_ROOT/bin/saicode"
  )

  TMP_DIR="$(mktemp -d)"
  ln -s "$REPO_ROOT/bin/saicode" "$TMP_DIR/saicode"
  run_help_smoke "symlinked command help" env SAICODE_DISABLE_NATIVE_LAUNCHER=1 "$TMP_DIR/saicode" --help
  (
    cd "$TMP_DIR"
    run_full_cli_smoke "symlinked full CLI fallback outside repo" env SAICODE_DISABLE_NATIVE_LAUNCHER=1 "$TMP_DIR/saicode"
  )

  if [[ -x "$INSTALLED_COMMAND" ]]; then
    run_help_smoke "installed command help" "$INSTALLED_COMMAND" --help
    (
      cd "$HOME"
      run_full_cli_smoke "installed command full CLI fallback from home" env SAICODE_DISABLE_NATIVE_LAUNCHER=1 "$INSTALLED_COMMAND"
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
  else
    log "cargo not installed; skipping native release build check"
  fi

  if [[ "${SAICODE_CLOSEOUT_LIVE:-0}" == "1" ]]; then
    run_live_probe
  else
    log "live print probe skipped; set SAICODE_CLOSEOUT_LIVE=1 to enable"
  fi

  log "closeout preflight passed"
}

main "$@"
