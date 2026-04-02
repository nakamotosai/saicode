# saicode

`saicode` is a self-hosted coding agent CLI/TUI rebuilt from the original source tree and re-centered on OpenAI-compatible providers. The current baseline uses `cpa` as the main provider namespace.

This repository is now synced to the GitHub main branch baseline, and the local TypeScript check is clean:

```bash
bun run check
```

That command is equivalent to:

```bash
bun run typecheck
```

## Status

- Main provider namespace: `cpa`
- Default model: `cpa/qwen/qwen3.5-397b-a17b`
- Config directory: `~/.saicode`
- TypeScript diagnostics: `0`

## Install

```bash
bun install
cp .env.example .env
```

Recommended minimal environment:

```env
SAICODE_PROVIDER=cpa
SAICODE_MODEL=cpa/qwen/qwen3.5-397b-a17b
SAICODE_DEFAULT_MODEL=cpa/qwen/qwen3.5-397b-a17b
CLIPROXYAPI_BASE_URL=http://127.0.0.1:8317/v1
API_TIMEOUT_MS=600000
DISABLE_TELEMETRY=1
SAICODE_DISABLE_LEGACY_COMMANDS=1
```

## Run

```bash
./bin/saicode
./bin/saicode -p "hello"
./bin/saicode --help
```

Windows:

```powershell
bun --env-file=.env ./src/entrypoints/cli.tsx
bun --env-file=.env ./src/entrypoints/cli.tsx -p "hello"
bun --env-file=.env ./src/localRecoveryCli.ts --help
```

## Verification

Use the following commands when you want to confirm the current baseline:

```bash
bun run check
./bin/saicode --help
./bin/saicode -p "Reply with exactly: ok"
```

## Notes

- `cliproxyapi/...` and legacy `nvidia/...` model IDs are kept as compatibility aliases.
- The repository still contains historical names in some modules, but the public runtime is centered on the `saicode` provider/model surface.
- This is a personal rebuild and operational baseline, not a clean greenfield codebase.
