# saicode Rust web and no-shadow closeout spec

日期：
- 2026-04-06

目标：
- 收口当前 Rust 主链里剩余的 3 个删除阻塞：
  - `WebSearch/WebFetch` 在 recovery 与 full-cli 之间行为分叉；
  - full-cli 下复杂 `Read + Bash` 任务会误判工具不可用；
  - 缺少“禁用 TS/Bun fallback”条件下的无影子验收。

范围：
- 修复 full-cli 的显式多工具识别与工具面收窄逻辑。
- 让 full-cli 的 `WebSearch/WebFetch` 走与 recovery 相同的 Rust 执行实现。
- 增加并执行 no-shadow 验收，覆盖：
  - `help`
  - `status`
  - `repl`
  - `read`
  - `bash`
  - `web`
  - `task`
  - `ttft-bench`

非目标：
- 本轮不删除 TS/Bun 旧版。
- 本轮不做 UI parity 或大规模剩余 TS 迁移。

约束：
- 必须保持当前 `saicode` 外观与 `cliproxyapi` 语义不变。
- 必须用真实命令验收，不以“理论上应该可用”替代实测。
- 若保留旧 TS/Bun fallback，则必须确认当前无影子验收在禁用 Bun shadow 时通过。

验收：
- 显式 `WebSearch/WebFetch` 请求在 full-cli 下不再比 recovery 更差。
- `Use Read and Bash ...` 类复杂任务无需人工喂 `--allowed-tools` 也能完成。
- 在 `SAICODE_BUN_BIN=/bin/false` 条件下，`help/status/repl/read/bash/web/task/ttft-bench` 全部通过。
