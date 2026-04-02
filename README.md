# saicode

`saicode` 是一个基于这份源码壳重构出来的自用型 coding agent CLI/TUI，默认不再走 Claude/Anthropic 登录链路，而是直接接你自己的 OpenAI-compatible provider。

当前这一版的目标是：

- 品牌统一为 `saicode`
- 配置目录统一为 `~/.saicode`
- 默认模型切到 `cliproxyapi/qwen/qwen3.5-122b-a10b`
- 主 provider 支持 `nvidia` 与 `cliproxyapi`
- 保留 CLI / TUI / MCP / plugins / skills 的主体能力
- 默认关闭旧的 `auth / setup-token / doctor / update / install` 命令面

## 当前内建模型

```text
nvidia/qwen/qwen3.5-122b-a10b
nvidia/qwen/qwen3.5-397b-a17b
nvidia/nvidia/nemotron-3-super-120b-a12b
nvidia/openai/gpt-oss-120b
cliproxyapi/gpt-5.4
cliproxyapi/gpt-5.4-mini
cliproxyapi/qwen/qwen3.5-122b-a10b
cliproxyapi/qwen/qwen3.5-397b-a17b
cliproxyapi/nvidia/nemotron-3-super-120b-a12b
cliproxyapi/openai/gpt-oss-120b
cliproxyapi/opencode/qwen3.6-plus-free
cliproxyapi/opencode/mimo-v2-pro-free
cliproxyapi/opencode/mimo-v2-omni-free
```

## 快速开始

### 1. 安装依赖

```bash
bun install
```

### 2. 配置环境变量

```bash
cp .env.example .env
```

推荐最小配置：

```env
SAICODE_PROVIDER=cliproxyapi
SAICODE_MODEL=cliproxyapi/qwen/qwen3.5-122b-a10b
SAICODE_DEFAULT_MODEL=cliproxyapi/qwen/qwen3.5-122b-a10b
NVIDIA_API_KEY=your_nvidia_key_here
NVIDIA_BASE_URL=https://integrate.api.nvidia.com/v1
CLIPROXYAPI_BASE_URL=http://127.0.0.1:8317/v1
API_TIMEOUT_MS=600000
DISABLE_TELEMETRY=1
SAICODE_DISABLE_LEGACY_COMMANDS=1
```

如果你主要走 `cliproxyapi`，把 `SAICODE_PROVIDER` 和 `SAICODE_MODEL` 改成对应模型即可。

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
- create `~/.local/bin/saicode` as a direct command entry

After the script finishes, review `~/saicode/.env`, then run:

```bash
source ~/.bashrc
saicode --help
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
NVIDIA_API_KEY / NVIDIA_BASE_URL
CLIPROXYAPI_API_KEY / CLIPROXYAPI_BASE_URL
SAICODE_WEB_SEARCH_BASE_URL
SAICODE_SAI_SEARCH_BASE_URL
SAICODE_WEB_SEARCH_FETCH_TOP_K
```

## 当前实现边界

- 主要调用链已经切到 `saicodeRuntime`
- provider 统一走 OpenAI-compatible 适配层，已支持 `Responses` 与 `Chat Completions`
- `cliproxyapi` 已挂入 OpenCode free 模型：`qwen3.6-plus-free`、`mimo-v2-pro-free`、`mimo-v2-omni-free`
- 搜索已切到本地 `WebSearch` 实现，并支持 `sai-search` fallback 与结果页自动抓取
- 旧的 Claude 专属命令默认隐藏，但仓库里仍有大量历史命名尚未全量清洗
- 这份源码壳本身的 TypeScript 构建面仍不干净；日常验收应优先看真实 CLI/TUI 运行与目标功能实测
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
