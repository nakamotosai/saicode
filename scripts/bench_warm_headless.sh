#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

PROMPT="${1:-Use the Write tool to write exactly warm-bench followed by a newline to /tmp/saicode-warm-bench.txt. Then reply with exactly: done}"
RUNS="${RUNS:-3}"
WARM_SOCKET="/tmp/saicode-headless-warm-${USER:-saicode}.sock"

prime_warm_pool() {
  rm -f "$WARM_SOCKET"
  env SAICODE_FORCE_WARM_HEADLESS=1 \
    ./bin/saicode -p "$PROMPT" --allowedTools Write >/dev/null
}

run_case() {
  local label="$1"
  shift

  printf '== %s ==\n' "$label"
  for ((i = 1; i <= RUNS; i++)); do
    /usr/bin/time -f 'run=%e rss=%M' env "$@" \
      ./bin/saicode -p "$PROMPT" --allowedTools Write >/dev/null
  done
  printf '\n'
}

prime_warm_pool
run_case 'warm headless (primed)' SAICODE_FORCE_WARM_HEADLESS=1
run_case 'bun lightweight cold path' SAICODE_DISABLE_WARM_HEADLESS=1
