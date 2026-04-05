#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

PROMPT="${1:-Use Bash to run 'command -v rg' and reply with exactly the executable path.}"
RUNS="${RUNS:-3}"

run_case() {
  local label="$1"
  shift

  printf '== %s ==\n' "$label"
  for ((i = 1; i <= RUNS; i++)); do
    /usr/bin/time -f 'run=%e rss=%M' "$@" ./bin/saicode -p "$PROMPT" --allowedTools Bash >/dev/null
  done
  printf '\n'
}

run_case 'native readonly bash' env
run_case 'bun lightweight fallback' env SAICODE_DISABLE_NATIVE_LOCAL_TOOLS=1
