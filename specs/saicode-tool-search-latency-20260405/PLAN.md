# saicode tool/search latency Plan

## Round 1 - 基线探针

- 跑本地工具探针：
  - `Read`
  - `Grep`
  - 必要时 `Write` / `Edit`
- 跑 WebSearch 探针
- 记录：
  - 输出是否正确
  - wall time
  - RSS

## Round 2 - 归因

- 判断慢点主要来自：
  - CLI 启动
  - 工具调用链
  - WebSearch / WebFetch / 搜索 fallback
- 若有必要，增加带 `stream-json` / debug 的二次探针确认链路

## Round 3 - 修复

- 仅在热点明确时改代码
- 优先做低风险、单点、可复测的性能修复

## Round 4 - 验证

- 相关测试 / typecheck
- 前后台复测同一批探针
- 记录修复前后变化

## Outcome

- 已完成真实探针：
  - `Read`
  - `Grep`
  - `WebSearch`
  - `WebFetch`
- 已确认并修复的热点：
  - `--tools WebSearch` / `--tools WebFetch` 这类显式非 simple 工具，之前会被 auto-bare/simple-mode 误判，导致工具未正确挂载；现已修复并通过真实 `tool_use` 复测。
  - `Read` 工具对 `pages: ""` 会先报错再让模型重试，造成额外一轮；现已改为将空白 `pages` 视为未传，并补回归测试。
- 关键复测结果：
  - `Read` stream-json 端到端从 `5183ms / 3 turns` 降到 `3913ms / 2 turns`
  - `Grep` 正常返回 `k9t7-marker`
  - `WebSearch` 正常返回 `openai.com`
  - `WebFetch` 正常返回 `Example Domain`
- 自动化验证：
  - `bun test tests/file-read-tool.test.ts tests/non-interactive-mode.test.ts tests/headless-print.test.ts`
  - `bun run typecheck`
