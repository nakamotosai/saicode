# saicode

`saicode` is now a pure Rust terminal client.

The repository policy has been cut over to:

- Rust owns the frontend, backend, CLI, streaming, tools, sessions, MCP, LSP, and plugins
- The old TypeScript/Bun frontend and runtime have been removed
- `./bin/saicode` now defaults to the Rust path only

## Current Status

- Default command entry: `./bin/saicode`
- Main implementation: `rust/`
- Native launcher: `native/`

## What Works Now

Current capability is whatever the Rust path exposes:

```bash
./bin/saicode --help
./bin/saicode status
```

If the local binaries are already built, these should also work:

```bash
./bin/saicode
./bin/saicode -p "hello"
```

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
```
