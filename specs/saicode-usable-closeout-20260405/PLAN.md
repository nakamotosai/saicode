# saicode usable closeout Plan

## 目标

- 修复已安装 `saicode` 命令的软链入口故障。
- 把本地收尾、真实验收、GitHub 提交前闸门固化成不遗漏流程。

## 步骤

### Step 1 - 复现与定因

- 复现：
  - `saicode --help`
  - `./bin/saicode --help`
- 继续复现：
  - `saicode`
  - `SAICODE_DISABLE_NATIVE_LAUNCHER=1 saicode mcp --help`
- 确认 `PATH` 里的 `saicode` 是软链到仓库 wrapper
- 确认 wrapper 用 `BASH_SOURCE[0]` 直接推 repo root，导致通过软链执行时把 repo root 误判成 `~/.local`
- 确认 full CLI 走 Bun 时，当前 cwd 不在仓库里，`bunfig.toml` 的 `preload.ts` 不会自动生效，于是 `main.tsx` 在 `MACRO.VERSION` 处直接炸掉

验证：

- 根因能解释：
  - 为什么 `./bin/saicode` 正常
  - 为什么 `saicode` 会去找 `/home/ubuntu/.local/bin/saicode-bun`
  - 为什么 `saicode --help` 修好后，`saicode` 交互式仍会在 full CLI 里报 `MACRO is not defined`

### Step 2 - 入口修复

- 修改 `bin/saicode`
- 让 wrapper 先解析软链真路径，再计算 repo root
- 修改 native launcher / Bun handoff
- 让 Bun fallback 显式带上 `preload.ts`
- 保持 native launcher 优先、Bun fallback 保底不变

验证：

- `saicode --help`
- `SAICODE_DISABLE_NATIVE_LAUNCHER=1 saicode --help`
- `SAICODE_DISABLE_NATIVE_LAUNCHER=1 saicode mcp --help`
- `saicode` 可进入 TUI

### Step 3 - 防回归与安装防呆

- 新增软链入口测试
- 新增 full CLI fallback 测试
- 修改 `scripts/install_vps_jp.sh`
- 在创建 `~/.local/bin/saicode` 后立刻跑 help smoke
- 同时从仓库外 cwd 跑一次 full CLI fallback smoke

验证：

- `bun test`
- 直接跑安装命令入口 smoke

### Step 4 - closeout workflow 固化

- 新增 `scripts/closeout_preflight.sh`
- README 补 closeout 命令
- 新增 `docs/closeout-workflow.md`
- CI 增加 closeout preflight
- 把 full CLI fallback 验收也收进 closeout preflight

验证：

- `bun run closeout:preflight`
- 当前机器配置存在时：
  - `bun run closeout:live`

## 当前完成判定

- `saicode` 命令本身恢复可用
- closeout workflow 已固定成“文档 + 脚本 + CI”三层
- 本轮结果可直接作为 GitHub 提交前流程使用
