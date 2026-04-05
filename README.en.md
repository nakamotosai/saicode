# saicode

`saicode` is a self-hosted coding agent CLI/TUI rebuilt from the original source tree and re-centered on OpenAI-compatible providers. The current baseline uses `cpa` as the main provider namespace.

This repository is now synced to the GitHub main branch baseline. Use the fast daily typecheck by default, and keep the full scan for the deeper gate:

```bash
bun run typecheck
bun run typecheck:full
```

For the day-to-day regression gate, use:

```bash
bun run verify
```

## Status

- Main provider namespace: `cpa`
- Default model: `cpa/gpt-5.4`
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
SAICODE_MODEL=cpa/gpt-5.4
SAICODE_DEFAULT_MODEL=cpa/gpt-5.4
CPA_API_KEY=your-provider-key
CPA_BASE_URL=http://127.0.0.1:8317/v1
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
bun run verify
bun run typecheck
bun run typecheck:full
./bin/saicode --help
./bin/saicode -p "Reply with exactly: ok"
```

## Notes

- `CPA_API_KEY` / `CPA_BASE_URL` is the preferred config surface for the `cpa` provider. `CLIPROXYAPI_API_KEY`, `CLIPROXYAPI_BASE_URL`, and `OPENAI_API_KEY` remain supported as compatibility inputs.
- The current default path is `gpt-5.4` / `gpt-5.4-mini`, which is the live-validated stable baseline on this host. Qwen 3.5 remains available in the built-in catalog, but it is no longer the default.
- `cliproxyapi/...` and legacy `nvidia/...` model IDs are kept as compatibility aliases.
- The built-in `cpa/...` catalog includes `cpa/google/gemma-4-31b-it` after the NVIDIA NIM rollout.
- `bun run typecheck` now targets the fast day-to-day lane, while `bun run typecheck:full` remains the full gate.
- The repository now includes a minimal automated gate via `bun run test`, `bun run check`, and `bun run verify`.
- The repository still contains historical names in some modules, but the public runtime is centered on the `saicode` provider/model surface.
- This is a personal rebuild and operational baseline, not a clean greenfield codebase.
