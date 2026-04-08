#!/usr/bin/env bash
set -euo pipefail

REPO_URL="${1:-https://github.com/nakamotosai/saicode.git}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/saicode}"
LINK_PATH="${LINK_PATH:-$HOME/.local/bin/saicode}"
RUNTIME_CONFIG_PATH="${RUNTIME_CONFIG_PATH:-$HOME/.saicode/config.json}"
OPENCLAW_CONFIG_PATH="${OPENCLAW_CONFIG_PATH:-$HOME/.openclaw/openclaw.json}"

log() {
  printf '[saicode-install] %s\n' "$*"
}

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

sync_repo() {
  require_cmd git

  if [[ -d "$INSTALL_DIR/.git" ]]; then
    log "updating existing repo in $INSTALL_DIR"
    git -C "$INSTALL_DIR" fetch --all --prune
    git -C "$INSTALL_DIR" checkout main
    git -C "$INSTALL_DIR" pull --ff-only origin main
  else
    log "cloning repo into $INSTALL_DIR"
    git clone "$REPO_URL" "$INSTALL_DIR"
  fi
}

build_native_launcher() {
  if ! command -v cargo >/dev/null 2>&1; then
    log "skip native launcher build: cargo not installed"
    return
  fi

  log "building native launcher"
  (
    cd "$INSTALL_DIR"
    cargo build --release --manifest-path native/saicode-launcher/Cargo.toml
  )
}

build_rust_one_shot() {
  if [[ ! -x "$INSTALL_DIR/scripts/rust-cargo.sh" ]]; then
    log "skip rust one-shot build: rust-cargo.sh not found"
    return
  fi

  log "building rust CLI binaries"
  (
    cd "$INSTALL_DIR/rust"
    "$INSTALL_DIR/scripts/rust-cargo.sh" build --release -q -p saicode-rust-cli
    "$INSTALL_DIR/scripts/rust-cargo.sh" build --release -q -p saicode-rust-one-shot
    "$INSTALL_DIR/scripts/rust-cargo.sh" build --release -q -p saicode-rust-local-tools
  )
}

ensure_env_file() {
  if [[ -f "$INSTALL_DIR/.env" ]]; then
    log ".env already exists"
    return
  fi

  if [[ -f "$INSTALL_DIR/.env.example" ]]; then
    log "creating .env from .env.example"
    cp "$INSTALL_DIR/.env.example" "$INSTALL_DIR/.env"
  else
    echo "missing .env.example in repo" >&2
    exit 1
  fi
}

bootstrap_runtime_config() {
  if [[ -f "$RUNTIME_CONFIG_PATH" ]]; then
    log "runtime config already exists"
    return
  fi

  if [[ ! -f "$OPENCLAW_CONFIG_PATH" ]]; then
    log "skip runtime config bootstrap: openclaw config not found"
    return
  fi

  if ! command -v jq >/dev/null 2>&1; then
    log "skip runtime config bootstrap: jq not installed"
    return
  fi

  if ! jq -e '.models.providers.cliproxyapi.apiKey // "" | length > 0' "$OPENCLAW_CONFIG_PATH" >/dev/null 2>&1; then
    log "skip runtime config bootstrap: cliproxyapi apiKey missing in openclaw config"
    return
  fi

  mkdir -p "$(dirname "$RUNTIME_CONFIG_PATH")"
  umask 077
  jq '{
    providers: {
      cpa: {
        api: (.models.providers.cliproxyapi.api // "openai-chat-completions"),
        baseUrl: .models.providers.cliproxyapi.baseUrl,
        apiKey: .models.providers.cliproxyapi.apiKey,
        headers: (.models.providers.cliproxyapi.headers // null)
      },
      cliproxyapi: {
        api: (.models.providers.cliproxyapi.api // "openai-chat-completions"),
        baseUrl: .models.providers.cliproxyapi.baseUrl,
        apiKey: .models.providers.cliproxyapi.apiKey,
        headers: (.models.providers.cliproxyapi.headers // null)
      }
    }
  }' "$OPENCLAW_CONFIG_PATH" > "$RUNTIME_CONFIG_PATH"
  log "bootstrapped runtime config from current openclaw cliproxyapi provider"
}

create_link() {
  mkdir -p "$(dirname "$LINK_PATH")"
  ln -sfn "$INSTALL_DIR/bin/saicode" "$LINK_PATH"
  log "linked $LINK_PATH -> $INSTALL_DIR/bin/saicode"
}

smoke_link() {
  log "smoke testing command entry"
  "$LINK_PATH" --help >/dev/null
  (
    cd "$HOME"
    SAICODE_DISABLE_NATIVE_LAUNCHER=1 "$LINK_PATH" mcp --help >/dev/null
  )
}

ensure_path() {
  local path_line='export PATH="$HOME/.local/bin:$PATH"'

  export PATH="$HOME/.local/bin:$PATH"

  if [[ -f "$HOME/.bashrc" ]] && grep -Fqx "$path_line" "$HOME/.bashrc"; then
    log "~/.local/bin already present in ~/.bashrc"
    return
  fi

  log "adding ~/.local/bin to ~/.bashrc"
  printf '\n%s\n' "$path_line" >> "$HOME/.bashrc"
}

print_next_steps() {
  cat <<EOF

Install complete.

Repo: $INSTALL_DIR
Command: $LINK_PATH

Before first real use, review:
  $INSTALL_DIR/.env
  $RUNTIME_CONFIG_PATH

Then reload the shell:
  source ~/.bashrc
EOF
}

main() {
  sync_repo
  build_native_launcher
  build_rust_one_shot
  ensure_env_file
  bootstrap_runtime_config
  create_link
  smoke_link
  ensure_path
  print_next_steps
}

main "$@"
