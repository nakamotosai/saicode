# saicode

`saicode` is now a pure Rust terminal client.

The repository policy has been cut over to:

- Rust owns the frontend, backend, CLI, streaming, tools, sessions, MCP, LSP, and plugins
- The old TypeScript/Bun frontend and runtime have been removed
- `./bin/saicode` now defaults to the Rust path only

## Current Status

- Default command entry: `./bin/saicode`
- Default configured model: `cpa/qwen/qwen3.5-122b-a10b`
- Main implementation: `rust/`
- Native launcher: `native/`

Current runtime truth:

- `status` and `config show` still report `cpa/qwen/qwen3.5-122b-a10b` as the default model
- To avoid degraded function invocation, empty streaming, and hanging closeout on the current provider path, tool-capable requests, one-shot, and recovery flows may automatically execute on `cpa/gpt-5.4-mini`
- This is not a second entrypoint or a second config surface; it is a stability fallback inside the same Rust runtime

## What Works Now

Current non-frontend capability is whatever the Rust path exposes:

```bash
./bin/saicode --help
./bin/saicode status
./bin/saicode config show
./bin/saicode -p "Reply with exactly: ok"
printf '/help\n/exit\n' | ./bin/saicode --bare
./scripts/rust_tool_acceptance.sh
SAICODE_CLOSEOUT_LIVE=1 SAICODE_CLOSEOUT_ACCEPTANCE=1 ./scripts/closeout_preflight.sh
```

In practice:

- `./bin/saicode` is the only supported entrypoint
- builtin tools, skill, MCP, plugin, and LSP acceptance are covered by `./scripts/rust_tool_acceptance.sh`
- closeout is covered by `./scripts/closeout_preflight.sh`

## Verification Surface

Recommended non-frontend closeout commands:

```bash
cargo test --manifest-path native/saicode-launcher/Cargo.toml
(cd rust && ../scripts/rust-cargo.sh test -q -p api -p runtime -p commands -p tools -p saicode-frontline -p saicode-rust-cli -p saicode-rust-one-shot -p saicode-rust-local-tools)
./scripts/rust_tool_acceptance.sh
SAICODE_CLOSEOUT_LIVE=1 SAICODE_CLOSEOUT_ACCEPTANCE=1 ./scripts/closeout_preflight.sh
```

The TTFT bench inside `rust_tool_acceptance.sh` only counts models that produce a real first token. A model that appears in `/v1/models` but fails or hangs on `/chat/completions` is skipped instead of being treated as a pass.

## Development

Build the Rust side:

```bash
cargo build --release --manifest-path native/saicode-launcher/Cargo.toml
(cd rust && ../scripts/rust-cargo.sh build --release -p saicode-rust-cli)
```

Run Rust tests:

```bash
cargo test --manifest-path native/saicode-launcher/Cargo.toml
(cd rust && ../scripts/rust-cargo.sh test -q -p api -p runtime -p commands -p tools -p saicode-frontline -p saicode-rust-cli -p saicode-rust-one-shot -p saicode-rust-local-tools)
SAICODE_CLOSEOUT_LIVE=1 SAICODE_CLOSEOUT_ACCEPTANCE=1 ./scripts/closeout_preflight.sh
```

## Current Boundaries

- The repository still displays `cpa/qwen/qwen3.5-122b-a10b` as the default configured model; that does not mean every runtime path must call qwen directly
- If the upstream provider also cannot serve `gpt-5.4-mini`, tool-capable and one-shot stability fall back to upstream availability limits
- This README documents the non-frontend surface only; browser frontend work is out of scope for this closeout
