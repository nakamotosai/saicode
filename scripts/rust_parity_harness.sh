#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUST_CLI="$REPO_ROOT/rust/target/release/saicode-rust-cli"

log() {
  printf '[rust-parity] %s\n' "$*"
}

if [[ ! -x "$RUST_CLI" ]]; then
  log "building saicode-rust-cli"
  (
    cd "$REPO_ROOT/rust"
    "$REPO_ROOT/scripts/rust-cargo.sh" build --release -q -p saicode-rust-cli
  )
fi

log "help smoke"
"$REPO_ROOT/bin/saicode" --help >/dev/null

log "route smoke"
route_output="$(SAICODE_NATIVE_DRY_RUN=1 "$REPO_ROOT/bin/saicode" status)"
if [[ "$route_output" != *"route=full-cli target=$RUST_CLI"* ]]; then
  printf 'expected FullCli to route to %s, got: %s\n' "$RUST_CLI" "$route_output" >&2
  exit 1
fi

log "status smoke"
"$RUST_CLI" status >/dev/null

log "live prompt smoke"
prompt_output="$(timeout 45 "$RUST_CLI" -p 'Reply with exactly: ok')"
if [[ "$prompt_output" != "ok" ]]; then
  printf 'expected live prompt to return ok, got: %s\n' "$prompt_output" >&2
  exit 1
fi

log "interactive help smoke"
interactive_output="$(printf '/help\n/exit\n' | timeout 15 "$RUST_CLI")"
if [[ "$interactive_output" != *"Slash commands"* ]]; then
  printf 'expected interactive help output, got: %s\n' "$interactive_output" >&2
  exit 1
fi

log "parity smoke passed"
