# saicode

`saicode` 是一个基于这份源码壳重构出来的自用型 coding agent CLI/TUI，默认不再走 Claude/Anthropic 登录链路，而是直接接你自己的 OpenAI-compatible provider。

当前这版已经同步到 GitHub 的新基线，日常默认类型检查可用 `bun run typecheck`，需要深度全量扫描时跑 `bun run typecheck:full`，最小回归可直接跑 `bun run verify`。

当前这一版的目标是：

- 品牌统一为 `saicode`
- 配置目录统一为 `~/.saicode`
- 默认模型切到 `cpa/gpt-5.4`
- 主 provider 统一为 `cpa`（`cliproxyapi` 的简写）
- 保留 CLI / TUI / MCP / plugins / skills 的主体能力
- 默认关闭旧的 `auth / setup-token / doctor / update / install` 命令面

## 当前内建模型

```text
cpa/gpt-5.4
cpa/gpt-5.4-mini
cpa/qwen/qwen3.5-122b-a10b
cpa/qwen/qwen3.5-397b-a17b
cpa/qwen3-coder-plus
cpa/vision-model
cpa/nvidia/nemotron-3-super-120b-a12b
cpa/openai/gpt-oss-120b
cpa/google/gemma-4-31b-it
cpa/opencode/qwen3.6-plus-free
cpa/opencode/mimo-v2-pro-free
cpa/opencode/mimo-v2-omni-free
```

## 快速开始

### 1. 安装依赖

```bash
bun install
```

### 1.1 运行类型检查

```bash
bun run typecheck
bun run typecheck:full
```

日常最小回归：

```bash
bun run verify
```

收尾前推荐再跑一遍：

```bash
bun run closeout:preflight
```

如果当前机器已经配好 `~/.saicode/config.json`，并且要在推 GitHub 前确认真实可用，再跑：

```bash
bun run closeout:live
```

### 2. 配置环境变量

```bash
cp .env.example .env
```

推荐最小配置：

```env
SAICODE_PROVIDER=cpa
SAICODE_MODEL=cpa/gpt-5.4
SAICODE_DEFAULT_MODEL=cpa/gpt-5.4
CPA_API_KEY=你的统一入口密钥
CPA_BASE_URL=http://127.0.0.1:8317/v1
CLIPROXYAPI_BASE_URL=http://127.0.0.1:8317/v1
API_TIMEOUT_MS=600000
DISABLE_TELEMETRY=1
SAICODE_DISABLE_LEGACY_COMMANDS=1
```

如果你主要走 `cliproxyapi`，在 `saicode` 里统一用 `cpa/...` 这套模型 ID 即可；`CPA_API_KEY` / `CPA_BASE_URL` 是推荐口径，`CLIPROXYAPI_API_KEY` / `CLIPROXYAPI_BASE_URL` 与 `OPENAI_API_KEY` 仍保留兼容。

当前默认值优先走 `gpt-5.4` / `gpt-5.4-mini` 这条已验证稳定的线路；Qwen 3.5 仍保留在内建模型目录里，但不再作为默认值。

### 3. 启动

macOS / Linux:

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

### 4. Deploy to Linux or VPS

For a clean Linux install such as `vps-jp`:

```bash
git clone https://github.com/nakamotosai/saicode.git ~/saicode
cd ~/saicode
./scripts/install_vps_jp.sh
```

The script will:

- install `bun` if it is missing
- clone or update the repo under `~/saicode`
- run `bun install --frozen-lockfile`
- create `.env` from `.env.example` if needed
- if `~/.saicode/config.json` is missing and the current machine already has a working OpenClaw `cliproxyapi` provider, bootstrap `saicode` runtime config from it
- create `~/.local/bin/saicode` as a direct command entry
- ensure `~/.local/bin` is on PATH through `~/.bashrc`

After the script finishes, review `~/saicode/.env`, then run:

```bash
source ~/.bashrc
saicode --help
bun run closeout:preflight
```

完整收尾与 GitHub 提交流程见：

```text
docs/closeout-workflow.md
```

## 目录与配置

```text
配置根目录: ~/.saicode
运行时配置: ~/.saicode/config.json
```

运行时常用环境变量：

```text
SAICODE_PROVIDER
SAICODE_MODEL
SAICODE_DEFAULT_MODEL
SAICODE_SMALL_FAST_MODEL
SAICODE_CONFIG_DIR
CPA_API_KEY / CPA_BASE_URL
CLIPROXYAPI_API_KEY / CLIPROXYAPI_BASE_URL
SAICODE_WEB_SEARCH_BASE_URL
SAICODE_SAI_SEARCH_BASE_URL
SAICODE_WEB_SEARCH_FETCH_TOP_K
```

## 当前实现边界

- 主要调用链已经切到 `saicodeRuntime`
- provider 统一走 OpenAI-compatible 适配层，当前默认只暴露 `cpa` 这一路
- `cliproxyapi` 已挂入 OpenCode free 模型：`qwen3.6-plus-free`、`mimo-v2-pro-free`、`mimo-v2-omni-free`
- 搜索已切到本地 `WebSearch` 实现，并支持 `sai-search` fallback 与结果页自动抓取
- 旧的 Claude 专属命令默认隐藏，但仓库里仍有大量历史命名尚未全量清洗
- 深度类型扫描仍可跑 `bun run typecheck:full`
- 默认 `bun run typecheck` 已切到高频快车道；完整总闸门保留在 `bun run typecheck:full`
- 当前仓库已补最小回归门：`bun run test`、`bun run check` 与 `bun run verify`
- 这是自用重构起点，不是干净从零设计的新仓库

## 项目结构

```text
bin/saicode                 新入口脚本
src/entrypoints/cli.tsx     主 CLI 入口
src/main.tsx                Commander + Ink 主程序
src/localRecoveryCli.ts     简化 recovery CLI
src/services/api/saicodeRuntime.ts
                            新 provider runtime
src/utils/model/            默认模型与 provider 判定
src/utils/theme.ts          saicode 主题颜色
SPEC.md                     本轮任务规格
PLAN.md                     本轮实施计划
```

## 说明

这份仓库来源特殊，适合拿来做自用重构、接口接入和交互层实验，不适合作为“未经处理直接上线”的正式产品底座。
