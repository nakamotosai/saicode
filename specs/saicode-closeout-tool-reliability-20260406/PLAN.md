# saicode Stage 6 收口与工具可靠性 Plan

日期：
- 2026-04-06

## 总策略

按 4 段收口：

1. 固定当前真实状态
2. 跑真实工具验收
3. 修掉本轮能直接修掉的低风险问题
4. 输出剩余残项与后续动作，不进入 Stage 7

## Phase 1：状态冻结

目标：
- 先确认当前默认入口、门禁和 parity 资产是否都还是对的

动作：
- 读当前 Stage 状态、gap audit、parity contract
- 复核：
  - `./bin/saicode`
  - `./bin/saicode status`
  - `./scripts/rust_parity_harness.sh`
  - `bun run verify`

完成判定：
- 当前“Rust 默认入口已切换”是事实，不是口头描述

## Phase 2：前台工具实测

目标：
- 用真实 `./bin/saicode -p` 和同进程 interactive 验证高频工具

动作：
- 建 acceptance script：
  - `scripts/rust_tool_acceptance.sh`
- 硬门禁：
  - `ok`
  - `Read`
  - `Bash`
  - `Write`
  - `Edit`
  - `Read after Edit`
  - `WebFetch`
  - `WebSearch`
  - cross-process `TaskCreate -> TaskList`
  - same-process `TaskCreate -> TaskList`
- 观察项：
  - free-form `Read`

完成判定：
- 有统一脚本能重复给出工具验收结果

## Phase 3：直接修复本轮命中的低风险问题

目标：
- 不把明显可修的前台问题留到“以后再说”

本轮已修：
- `--allowedTools` / `--disallowedTools` 解析吞掉 prompt
- `auto-dream` 后台日志泄露到前台
- `mcp --help` wrapper 兼容口径
- Rust Full CLI 默认权限固定为 `danger-full-access`
- `Task/Team/Cron` 本地落盘持久化，跨进程可见
- 显式工具请求路由收口：
  - `Read/Bash/Write/Edit/Glob/Grep/Task*/MCP` 显式请求走 Full CLI
  - `WebFetch/WebSearch` 维持原稳定路径

## Phase 4：完整 closeout 输出

目标：
- 给用户一份不是流水账、而是可执行的收口方案

输出物：
- `SPEC.md`
- `PLAN.md`
- `TEST_REPORT.md`
- `scripts/rust_tool_acceptance.sh`

## 当前已关闭残项

- `Bash` 默认权限非交互不稳定
- `Task*` 跨进程不可见
- 显式工具请求掉到 recovery / one-shot

## 停止条件

本轮到此为止，不继续做：

- Stage 7 删除 / 归档旧 TS/Bun
- UI 逐像素 parity
- MCP / LSP 深水区扩建

因为这些已经超出“当前收口方案”的边界

## 本收口任务结论

- 本 closeout plan 已完成
- 后续如果继续推进，应回到更大的 Rust motherline 文档，而不是继续在这个 closeout plan 下无限追加
