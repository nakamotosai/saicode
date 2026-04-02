#!/usr/bin/env bash
set -euo pipefail

REPO_URL="${1:-https://github.com/nakamotosai/saicode.git}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/saicode}"
LINK_PATH="${LINK_PATH:-$HOME/.local/bin/saicode}"

log() {
  printf '[saicode-install] %s\n' "$*"
}

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

install_bun() {
  if command -v bun >/dev/null 2>&1; then
    log "bun already installed: $(command -v bun)"
    return
  fi

  require_cmd curl
  log "installing bun"
  curl -fsSL https://bun.sh/install | bash

  export BUN_INSTALL="${BUN_INSTALL:-$HOME/.bun}"
  export PATH="$BUN_INSTALL/bin:$PATH"

  if ! command -v bun >/dev/null 2>&1; then
    echo "bun install completed but bun is not on PATH yet; reopen the shell and rerun" >&2
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

install_deps() {
  log "installing dependencies"
  (cd "$INSTALL_DIR" && bun install --frozen-lockfile)
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

create_link() {
  mkdir -p "$(dirname "$LINK_PATH")"
  ln -sfn "$INSTALL_DIR/bin/saicode" "$LINK_PATH"
  log "linked $LINK_PATH -> $INSTALL_DIR/bin/saicode"
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

Then reload the shell:
  source ~/.bashrc
EOF
}

main() {
  install_bun
  sync_repo
  install_deps
  ensure_env_file
  create_link
  ensure_path
  print_next_steps
}

main "$@"
