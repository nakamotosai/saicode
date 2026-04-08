#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CLI="$REPO_ROOT/bin/saicode"
TMP_DIR="$REPO_ROOT/.tmp_acceptance"
NATIVE_LAUNCHER="$REPO_ROOT/native/saicode-launcher/target/release/saicode-launcher"
RUST_FULL_CLI="$REPO_ROOT/rust/target/release/saicode-rust-cli"
ACCEPT_MODEL="${SAICODE_ACCEPT_MODEL:-cpa/qwen/qwen3.5-122b-a10b}"
FAST_ACCEPT_MODEL="${SAICODE_FAST_ACCEPT_MODEL:-cpa/gpt-5.4-mini}"

log() {
  printf '[rust-tools] %s\n' "$*"
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

  printf 'saicode process residue detected after interactive probe: %s\n' "$extra" >&2
  pgrep -af "$NATIVE_LAUNCHER|$RUST_FULL_CLI" >&2 || true
  exit 1
}

normalize_text_output() {
  awk '
    /^\[tool\] / { next }
    {
      if (seen) {
        printf "\n%s", $0
      } else {
        printf "%s", $0
        seen = 1
      }
    }
  '
}

cli_prompt() {
  "$CLI" --model "$ACCEPT_MODEL" "$@"
}

run_case() {
  local name="$1"
  shift
  local output
  local elapsed
  local tmp
  tmp="$(mktemp)"
  if ! output="$(cd "$REPO_ROOT" && /usr/bin/time -f 'elapsed=%e' "$@" 2>"$tmp")"; then
    cat "$tmp" >&2
    rm -f "$tmp"
    printf 'case failed: %s\n' "$name" >&2
    exit 1
  fi
  elapsed="$(tr -d '\n' < "$tmp")"
  rm -f "$tmp"
  output="$(printf '%s' "$output" | normalize_text_output)"
  printf '%s\t%s\t%s\n' "$name" "$elapsed" "$output"
}

run_text_case() {
  local name="$1"
  shift
  local output
  local elapsed
  local tmp
  tmp="$(mktemp)"
  if ! output="$(cd "$REPO_ROOT" && /usr/bin/time -f 'elapsed=%e' "$@" 2>"$tmp")"; then
    cat "$tmp" >&2
    rm -f "$tmp"
    printf 'case failed: %s\n' "$name" >&2
    exit 1
  fi
  elapsed="$(tr -d '\n' < "$tmp")"
  rm -f "$tmp"
  output="$(printf '%s' "$output" | normalize_text_output)"
  printf '%s\t%s\t%s\n' "$name" "$elapsed" "$output"
}

assert_eq() {
  local actual="$1"
  local expected="$2"
  local label="$3"
  if [[ "$actual" != "$expected" ]]; then
    printf '%s expected %s but got %s\n' "$label" "$expected" "$actual" >&2
    exit 1
  fi
}

run_ttft_bench() {
  python3 - <<'PY'
import json
import os
import select
import subprocess
import sys
import time
from pathlib import Path

config = json.loads(Path("/home/ubuntu/.saicode/config.json").read_text())
provider = config.get("providers", {}).get("cpa") or config.get("providers", {}).get("cliproxyapi") or {}
base_url = provider.get("baseUrl") or provider.get("base_url")
api_key = provider.get("apiKey") or provider.get("api_key")
if not base_url:
    raise SystemExit("missing cpa/cliproxyapi baseUrl in /home/ubuntu/.saicode/config.json")
if not api_key:
    raise SystemExit("missing cpa/cliproxyapi apiKey in /home/ubuntu/.saicode/config.json")

models_cmd = [
    "curl", "-sS", f"{base_url.rstrip('/')}/models",
    "-H", f"Authorization: Bearer {api_key}",
]
models_raw = subprocess.check_output(models_cmd, text=True)
models_payload = json.loads(models_raw)
model_ids = [item["id"] for item in models_payload.get("data", []) if item.get("id")]
preferred = [
    "gpt-5.4-mini",
    "qwen/qwen3.5-122b-a10b",
    "gpt-5.4",
    "qwen/qwen3.5-397b-a17b",
]
selected = []
for model in preferred:
    if model in model_ids and model not in selected:
        selected.append(model)
    if len(selected) == 2:
        break
if len(selected) < 2:
    for model in model_ids:
        if model not in selected:
            selected.append(model)
        if len(selected) == 2:
            break
if len(selected) != 2:
    raise SystemExit(f"expected at least 2 models from /models, got {model_ids}")

results = []
skipped = []
per_model_timeout = float(os.environ.get("SAICODE_TTFT_MODEL_TIMEOUT_SECONDS", "20"))
for model in selected:
    try:
        body = json.dumps({
            "model": model,
            "messages": [{"role": "user", "content": "Reply with exactly: ok"}],
            "max_tokens": 64,
            "stream": True,
        })
        cmd = [
            "curl", "-sS", "-N",
            "-X", "POST", f"{base_url.rstrip('/')}/chat/completions",
            "-H", "Content-Type: application/json",
            "-H", f"Authorization: Bearer {api_key}",
            "--data-binary", body,
        ]
        start = time.perf_counter()
        deadline = start + per_model_timeout
        proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        ttft = None
        try:
            while True:
                remaining = deadline - time.perf_counter()
                if remaining <= 0:
                    raise TimeoutError(
                        f"model {model} produced no first token within {per_model_timeout:.0f}s"
                    )
                ready, _, _ = select.select([proc.stdout], [], [], remaining)
                if not ready:
                    raise TimeoutError(
                        f"model {model} produced no first token within {per_model_timeout:.0f}s"
                    )
                raw_line = proc.stdout.readline()
                if raw_line == "":
                    if proc.poll() is not None:
                        break
                    continue
                line = raw_line.strip()
                if not line.startswith("data: "):
                    continue
                payload = line[6:]
                if payload == "[DONE]":
                    break
                event = json.loads(payload)
                for choice in event.get("choices", []):
                    delta = choice.get("delta") or {}
                    content = delta.get("content")
                    if content:
                        ttft = time.perf_counter() - start
                        raise StopIteration
            raise RuntimeError(f"model {model} produced no text delta")
        except StopIteration:
            pass
        finally:
            if proc.poll() is None:
                proc.terminate()
            try:
                stdout_rest, stderr = proc.communicate(timeout=5)
            except subprocess.TimeoutExpired:
                proc.kill()
                stdout_rest, stderr = proc.communicate(timeout=5)
            if ttft is None and proc.returncode not in (0, None):
                raise RuntimeError(f"ttft curl failed for {model}: {stderr or stdout_rest}")
        if ttft is None:
            raise RuntimeError(f"model {model} produced null ttft")
        results.append({"model": model, "ttft_seconds": round(ttft, 6)})
    except Exception as error:
        skipped.append({"model": model, "reason": str(error)})
        continue
if not results:
    raise RuntimeError(f"no callable model produced ttft; skipped={skipped}")

print(json.dumps(results, separators=(",", ":")))
PY
}

write_provider_fixture_config() {
  local home="$1"
  local extra_json="${2:-}"
  local source_config="/home/ubuntu/.saicode/config.json"
  mkdir -p "$home/.saicode"

  if [[ -n "$extra_json" ]]; then
    jq --slurp '
      (.[0] // {}) as $base
      | (.[1] // {}) as $extra
      | {
          profile: ($base.profile // "cliproxyapi"),
          providers: ($base.providers // {}),
          mcpServers: ($extra.mcpServers // {})
        }
    ' "$source_config" "$extra_json" > "$home/.saicode/config.json"
  else
    jq '
      {
        profile: (.profile // "cliproxyapi"),
        providers: (.providers // {})
      }
    ' "$source_config" > "$home/.saicode/config.json"
  fi
}

create_skill_fixture() {
  local home="$TMP_DIR/skill-home"
  local skill_dir="$home/.codex/skills/fixture"
  mkdir -p "$skill_dir/scripts" "$skill_dir/assets" "$skill_dir/templates"
  cat > "$skill_dir/SKILL.md" <<'EOF'
---
name: fixture
description: fixture skill description
---

# fixture

See [helper](scripts/helper.sh) and [template](templates/base.txt).
EOF
  printf '#!/bin/sh\necho helper\n' > "$skill_dir/scripts/helper.sh"
  printf 'asset\n' > "$skill_dir/assets/info.txt"
  printf 'template\n' > "$skill_dir/templates/base.txt"
  write_provider_fixture_config "$home"
  printf '%s\n' "$home"
}

create_mcp_fixture() {
  local root="$TMP_DIR/mcp-fixture"
  local home="$root/home"
  local server="$root/mcp_echo.py"
  mkdir -p "$root" "$home"
  cat > "$server" <<'PY'
import json, sys

def send(obj):
    payload = json.dumps(obj).encode()
    sys.stdout.write(f"Content-Length: {len(payload)}\r\n\r\n")
    sys.stdout.flush()
    sys.stdout.buffer.write(payload)
    sys.stdout.buffer.flush()

while True:
    headers = {}
    while True:
        line = sys.stdin.buffer.readline()
        if not line:
            raise SystemExit(0)
        if line in (b"\r\n", b"\n"):
            break
        key, value = line.decode().split(":", 1)
        headers[key.strip().lower()] = value.strip()
    length = int(headers["content-length"])
    message = json.loads(sys.stdin.buffer.read(length).decode())
    method = message.get("method")
    ident = message.get("id")
    if method == "initialize":
        send({"jsonrpc":"2.0","id":ident,"result":{"protocolVersion":"2024-11-05","capabilities":{},"serverInfo":{"name":"alpha","version":"1.0.0"}}})
    elif method == "notifications/initialized":
        pass
    elif method == "tools/list":
        send({"jsonrpc":"2.0","id":ident,"result":{"tools":[{"name":"echo","description":"Echo text","inputSchema":{"type":"object","properties":{"text":{"type":"string"}},"required":["text"]}}]}})
    elif method == "tools/call":
        text = ((message.get("params") or {}).get("arguments") or {}).get("text", "")
        send({"jsonrpc":"2.0","id":ident,"result":{"content":[{"type":"text","text":f"echo:{text}"}],"structuredContent":{"echoed":text}}})
    else:
        send({"jsonrpc":"2.0","id":ident,"error":{"code":-32601,"message":"method not found"}})
PY
  local extra_json="$root/extra.json"
  printf '{"mcpServers":{"alpha":{"command":"python3","args":["%s"]}}}\n' "$server" > "$extra_json"
  write_provider_fixture_config "$home" "$extra_json"
  printf '%s\n' "$home"
}

mkdir -p "$TMP_DIR"
rm -f "$TMP_DIR/tool_write.txt"
TOOL_FILE="$TMP_DIR/tool_write.txt"

log "building rust full cli if needed"
if [[ ! -x "$REPO_ROOT/rust/target/release/saicode-rust-cli" ]]; then
  (
    cd "$REPO_ROOT/rust"
    "$REPO_ROOT/scripts/rust-cargo.sh" build --release -q -p saicode-rust-cli
  )
fi

log "hard gates"

help_output="$(cd "$REPO_ROOT" && "$CLI" --help)"
[[ "$help_output" == *"Usage: saicode"* ]] || {
  printf 'help output missing Usage\n' >&2
  exit 1
}

status_output="$(cd "$REPO_ROOT" && "$CLI" status)"
[[ "$status_output" == *"Provider API"* && "$status_output" == *"Permission mode  danger-full-access"* ]] || {
  printf 'status output missing Provider API\n' >&2
  exit 1
}

route_output="$(cd "$REPO_ROOT" && SAICODE_NATIVE_DRY_RUN=1 "$CLI" status)"
[[ "$route_output" == *"route=full-cli target=$REPO_ROOT/rust/target/release/saicode-rust-cli"* ]] || {
  printf 'full-cli route is not rust: %s\n' "$route_output" >&2
  exit 1
}

stream_output="$(cd "$REPO_ROOT" && "$CLI" --bare --model "$ACCEPT_MODEL" -p --output-format stream-json 'Reply with exactly: ok')"
[[ "$stream_output" == *'"type":"content_delta"'* ]] || {
  printf 'stream-json output missing content_delta: %s\n' "$stream_output" >&2
  exit 1
}
[[ "$stream_output" == *'"type":"final_message"'* ]] || {
  printf 'stream-json output missing final_message: %s\n' "$stream_output" >&2
  exit 1
}

log "using acceptance model: $ACCEPT_MODEL"
log "using fast secondary model: $FAST_ACCEPT_MODEL"

ok_line="$(run_case ok timeout 45 "$(command -v bash)" -lc 'exec "$0" --model "$1" -p "$2"' "$CLI" "$ACCEPT_MODEL" 'Reply with exactly: ok')"
ok_output="${ok_line##*$'\t'}"
assert_eq "$ok_output" "ok" "ok prompt"

read_route_output="$(cd "$REPO_ROOT" && SAICODE_NATIVE_DRY_RUN=1 "$CLI" --bare -p --allowedTools Read -- 'Use Read to inspect rust/Cargo.toml and reply with only the workspace package version.')"
[[ "$read_route_output" == *"route=native-local-tools"* ]] || {
  printf 'expected restricted bare Read print to route to native-local-tools, got: %s\n' "$read_route_output" >&2
  exit 1
}

read_line="$(run_case read_allowed timeout 60 "$(command -v bash)" -lc 'exec "$0" --bare -p --allowedTools Read -- "$1"' "$CLI" 'Use Read to inspect rust/Cargo.toml and reply with only the workspace package version.')"
read_output="${read_line##*$'\t'}"
assert_eq "$read_output" "0.1.0" "read allowed"

free_read_line="$(run_case read_free timeout 60 "$(command -v bash)" -lc 'exec "$0" --bare -p --allowedTools Read -- "$1"' "$CLI" 'Use Read to inspect rust/Cargo.toml and reply with only the workspace package version.')"
free_read_output="${free_read_line##*$'\t'}"
assert_eq "$free_read_output" "0.1.0" "read free"

bash_line="$(run_case bash_default timeout 60 "$(command -v bash)" -lc 'exec "$0" --bare -p --allowedTools Bash -- "$1"' "$CLI" 'Use Bash to run pwd and reply with only the exact last path component.')"
bash_output="${bash_line##*$'\t'}"
assert_eq "$bash_output" "saicode" "bash default"

free_bash_line="$(run_case bash_free timeout 60 "$(command -v bash)" -lc 'exec "$0" --bare -p --allowedTools Bash -- "$1"' "$CLI" 'Use Bash to run pwd and reply with only the exact last path component.')"
free_bash_output="${free_bash_line##*$'\t'}"
assert_eq "$free_bash_output" "saicode" "bash free"

write_line="$(run_case write_allowed timeout 60 "$(command -v bash)" -lc 'exec "$0" --bare -p --allowedTools Write -- "$1"' "$CLI" "Use Write to create $TOOL_FILE with the exact content alpha and reply with only ok.")"
write_output="${write_line##*$'\t'}"
assert_eq "$write_output" "ok" "write allowed"

edit_line="$(run_case edit_allowed timeout 60 "$(command -v bash)" -lc 'exec "$0" --bare -p --allowedTools Edit -- "$1"' "$CLI" "Use Edit to replace alpha with beta in $TOOL_FILE and reply with only ok.")"
edit_output="${edit_line##*$'\t'}"
assert_eq "$edit_output" "ok" "edit allowed"

file_content="$(cat "$TOOL_FILE")"
assert_eq "$file_content" "beta" "edited file content"

read_file_line="$(run_case read_file_content timeout 60 "$(command -v bash)" -lc 'exec "$0" --bare -p --allowedTools Read -- "$1"' "$CLI" "Use Read to inspect $TOOL_FILE and reply with only the file content.")"
read_file_output="${read_file_line##*$'\t'}"
assert_eq "$read_file_output" "beta" "read edited file"

grep_line="$(run_case grep_allowed timeout 60 "$(command -v bash)" -lc 'exec "$0" --bare -p --allowedTools Grep -- "$1"' "$CLI" 'Use Grep to verify that rust/Cargo.toml contains a line matching members = and reply with exactly: ok')"
grep_output="${grep_line##*$'\t'}"
assert_eq "$grep_output" "ok" "grep"

webfetch_line="$(run_case webfetch timeout 60 "$(command -v bash)" -lc 'exec "$0" --bare -p --allowedTools WebFetch -- "$1"' "$CLI" 'Use WebFetch on https://example.com and reply with only the page title.')"
webfetch_output="${webfetch_line##*$'\t'}"
assert_eq "$webfetch_output" "Example Domain" "webfetch"

websearch_line="$(run_case websearch timeout 60 "$(command -v bash)" -lc 'exec "$0" --bare -p --allowedTools WebSearch -- "$1"' "$CLI" 'Use WebSearch to search for example.com and reply with only the top result domain.')"
websearch_output="${websearch_line##*$'\t'}"
websearch_first_line="$(printf '%s\n' "$websearch_output" | head -n 1)"
assert_eq "$websearch_first_line" "example.com" "websearch"

skill_home="$(create_skill_fixture)"
skill_line="$(run_case skill_runtime timeout 60 env HOME="$skill_home" "$(command -v bash)" -lc 'exec "$0" --bare --model "$1" -p --allowedTools Skill -- "$2"' "$CLI" "$ACCEPT_MODEL" 'Use Skill to load fixture and reply with only the referenced helper script relative path.')"
skill_output="${skill_line##*$'\t'}"
assert_eq "$skill_output" "scripts/helper.sh" "skill runtime"

mcp_home="$(create_mcp_fixture)"
mcp_line="$(run_case mcp_dynamic timeout 60 env HOME="$mcp_home" "$(command -v bash)" -lc 'exec "$0" --bare --model "$1" -p --allowedTools mcp__alpha__echo -- "$2"' "$CLI" "$ACCEPT_MODEL" 'Call mcp__alpha__echo with JSON arguments {"text":"acceptance"} and reply with only the returned text.')"
mcp_output="${mcp_line##*$'\t'}"
[[ "$mcp_output" == "echo:acceptance" || "$mcp_output" == 'echo:{"text":"acceptance"}' ]] || {
  printf 'dynamic mcp expected echoed acceptance but got %s\n' "$mcp_output" >&2
  exit 1
}

fast_ok_line="$(run_case fast_ok timeout 60 "$(command -v bash)" -lc 'exec "$0" --model "$1" -p "$2"' "$CLI" "$FAST_ACCEPT_MODEL" 'Reply with exactly: ok')"
fast_ok_output="${fast_ok_line##*$'\t'}"
assert_eq "$fast_ok_output" "ok" "fast ok prompt"

fast_read_line="$(run_case fast_read timeout 60 "$(command -v bash)" -lc 'exec "$0" --model "$1" -p --allowedTools Read -- "$2"' "$CLI" "$FAST_ACCEPT_MODEL" 'Use Read to inspect README.md and reply with only the first line.')"
fast_read_output="${fast_read_line##*$'\t'}"
assert_eq "$fast_read_output" "# saicode" "fast read"

task_before_line="$(run_case task_list_before timeout 60 "$(command -v bash)" -lc 'exec "$0" --bare --model "$1" -p --allowedTools TaskList -- "$2"' "$CLI" "$ACCEPT_MODEL" 'Use TaskList and reply with only the number of tasks.')"
task_before="${task_before_line##*$'\t'}"

task_create_line="$(run_case task_create timeout 60 "$(command -v bash)" -lc 'exec "$0" --bare --model "$1" -p --allowedTools TaskCreate -- "$2"' "$CLI" "$ACCEPT_MODEL" 'Use TaskCreate to create a background task with prompt ping and reply with only the created task_id.')"
task_create_output="${task_create_line##*$'\t'}"
[[ "$task_create_output" == task_* ]] || {
  printf 'expected task id, got: %s\n' "$task_create_output" >&2
  exit 1
}

task_create_free_line="$(run_case task_create_free timeout 60 "$(command -v bash)" -lc 'exec "$0" --bare --model "$1" -p --allowedTools TaskCreate -- "$2"' "$CLI" "$ACCEPT_MODEL" 'Use TaskCreate to create a background task with prompt ping and reply with only the created task_id.')"
task_create_free_output="${task_create_free_line##*$'\t'}"
[[ "$task_create_free_output" == task_* ]] || {
  printf 'expected free task id, got: %s\n' "$task_create_free_output" >&2
  exit 1
}

task_after_line="$(run_case task_list_after timeout 60 "$(command -v bash)" -lc 'exec "$0" --bare --model "$1" -p --allowedTools TaskList -- "$2"' "$CLI" "$ACCEPT_MODEL" 'Use TaskList and reply with only the number of tasks.')"
task_after="${task_after_line##*$'\t'}"
if ! [[ "$task_before" =~ ^[0-9]+$ && "$task_after" =~ ^[0-9]+$ ]]; then
  printf 'task counts are not numeric: before=%s after=%s\n' "$task_before" "$task_after" >&2
  exit 1
fi
if (( task_after < task_before + 1 )); then
  printf 'expected task count to increase: before=%s after=%s\n' "$task_before" "$task_after" >&2
  exit 1
fi

interactive_task_baseline="$(capture_saicode_pids "--allowedTools TaskCreate TaskList")"
interactive_task_bridge_baseline="$(capture_saicode_pids "ui-bridge")"
task_output="$(cd "$REPO_ROOT" && printf 'Use TaskCreate to create a background task with prompt ping and reply with only the created task_id.\nUse TaskList and reply with only the number of tasks.\n/exit\n' | timeout 120 "$CLI" --bare --model "$ACCEPT_MODEL" --allowedTools TaskCreate TaskList)"
wait_for_probe_processes_to_exit "$interactive_task_baseline" "--allowedTools TaskCreate TaskList"
wait_for_probe_processes_to_exit "$interactive_task_bridge_baseline" "ui-bridge"
interactive_count="$(printf '%s\n' "$task_output" | awk '/^[0-9]+$/ { value=$1 } END { print value }')"
if ! [[ "$interactive_count" =~ ^[0-9]+$ ]]; then
  printf 'task interactive output unexpected: %s\n' "$task_output" >&2
  exit 1
fi

interactive_read_baseline="$(capture_saicode_pids "--allowedTools Read")"
interactive_read_bridge_baseline="$(capture_saicode_pids "ui-bridge")"
interactive_tool_output="$(cd "$REPO_ROOT" && printf 'Use Read to inspect README.md and reply with its first line only.\n/exit\n' | timeout 120 "$CLI" --bare --model "$ACCEPT_MODEL" --allowedTools Read)"
wait_for_probe_processes_to_exit "$interactive_read_baseline" "--allowedTools Read"
wait_for_probe_processes_to_exit "$interactive_read_bridge_baseline" "ui-bridge"
[[ "$interactive_tool_output" == *"[tool] Read"* ]] || {
  printf 'interactive tool progress missing [tool] Read: %s\n' "$interactive_tool_output" >&2
  exit 1
}
[[ "$interactive_tool_output" == *"# saicode"* ]] || {
  printf 'interactive tool output missing README first line: %s\n' "$interactive_tool_output" >&2
  exit 1
}

ttft_line="$(run_text_case ttft_bench timeout 240 "$(command -v bash)" -lc "$(declare -f run_ttft_bench); run_ttft_bench")"
ttft_output="${ttft_line##*$'\t'}"
[[ "$ttft_output" == \[* ]] || {
  printf 'ttft output is not a JSON array: %s\n' "$ttft_output" >&2
  exit 1
}
[[ "$ttft_output" == *"ttft_seconds"* ]] || {
  printf 'ttft output missing ttft_seconds: %s\n' "$ttft_output" >&2
  exit 1
}
[[ "$ttft_output" != *"null"* ]] || {
  printf 'ttft output contains null: %s\n' "$ttft_output" >&2
  exit 1
}

log "observations"
cat <<EOF
hard_gate	ok
$(printf '%s\n' "$ok_line")
$(printf '%s\n' "$read_line")
$(printf '%s\n' "$free_read_line")
$(printf '%s\n' "$bash_line")
$(printf '%s\n' "$free_bash_line")
$(printf '%s\n' "$write_line")
$(printf '%s\n' "$edit_line")
$(printf '%s\n' "$read_file_line")
$(printf '%s\n' "$grep_line")
$(printf '%s\n' "$webfetch_line")
$(printf '%s\n' "$websearch_line")
$(printf '%s\n' "$skill_line")
$(printf '%s\n' "$mcp_line")
$(printf '%s\n' "$fast_ok_line")
$(printf '%s\n' "$fast_read_line")
$(printf '%s\n' "$task_before_line")
$(printf '%s\n' "$task_create_line")
$(printf '%s\n' "$task_create_free_line")
$(printf '%s\n' "$task_after_line")
$(printf '%s\n' "$ttft_line")
EOF
