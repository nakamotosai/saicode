# saicode Rust canonical workspace gap audit

日期：
- 2026-04-06

## 当前 active Rust 覆盖面

- `crates/api`
  - 已具备 OpenAI-compatible 请求、stream 事件、tool use block 解析。
- `crates/runtime`
  - 已具备 session persistence、conversation loop、permission policy、hooks、memory、sandbox、MCP stdio manager。
- `crates/tools`
  - 已具备 built-in tool schema、dispatch、web、shell、sub-agent runtime。
- `crates/commands`
  - 已具备 slash/process command spec、registry、help、catalog handler。
- `crates/saicode-frontline`
  - 已具备 one-shot / native local tools / recovery 模型与 provider 解析。

## 审计时发现的核心缺口

### Gap 1：缺少真正的 Rust Full CLI 主入口

- 现象：
  - `rust/` 内没有负责默认 `saicode` 交互入口的完整 binary。
  - `native/saicode-launcher` 的 `Route::FullCli` 仍直接 handoff 到 Bun/TS。
- 影响：
  - Rust 只能覆盖 one-shot / local tools / recovery，无法完成 Stage 5-6 cutover。

### Gap 2：provider/request 主链仍缺 saicode 自身模型语义适配

- 现象：
  - donor `tools::agent_runtime::ProviderRuntimeClient` 偏向通用 OpenAI/XAI/Anthropic 模式。
  - `cliproxyapi` / `cpa/...` alias 需要额外的 saicode model catalog 归一化。
- 影响：
  - 直接拿 donor runtime 会把 `cpa/gpt-5.4-mini` 原样送给 upstream，导致 unknown provider。

### Gap 3：`tools::dispatch` 里仍有多处 stub

- 审计命中：
  - `Task*`
  - `Team*`
  - `Cron*`
  - `LSP`
  - `ListMcpResources`
  - `ReadMcpResource`
  - `McpAuth`
  - `MCP`
  - `RemoteTrigger`
- 影响：
  - 即使聊天主链切到 Rust，tool/runtime 仍不是完整的 native execution surface。

### Gap 4：交互式外观需要 parity，但 donor 默认是 lower-case tool surface

- 现象：
  - donor tool names 默认是 `bash/read_file/edit_file/...`
  - saicode 现有高频观感是 `Bash/Read/Edit/Glob/Grep/...`
- 影响：
  - 不做 surface mapping 会在高频工具面直接产生用户可见漂移。

## 本轮已完成的补齐

### Stage 0

- 停止把 TS `@ts-nocheck` 清债当成母线。
- 母计划、错题本、session ledger 已回写“全量 Rust 重平台化”为主线。

### Stage 1

- 新增 `rust/crates/saicode-rust-cli`，把 `rust/` 从“高频 fastpath 集合”推进到“默认 Full CLI 可执行底座”。

### Stage 2

- 在新的 Rust CLI 中保留 saicode 高频表面：
  - `--help` / `--version`
  - `status` / `sandbox` / `doctor` / `mcp` / `agents` / `skills` / `plugins`
  - interactive REPL
  - `Bash/Read/Write/Edit/Glob/Grep` 的显示名映射

### Stage 3

- Rust Full CLI 现已通过 `saicode-frontline::recovery` model catalog 解析 `cpa/...` / `cliproxyapi` 语义。
- live probe 已验证：
  - `./rust/target/release/saicode-rust-cli -p 'Reply with exactly: ok'`
  - 返回：`ok`

### Stage 4

- `tools::dispatch` 已把以下路径从 stub 换成 native runtime：
  - `TaskCreate/TaskGet/TaskList/TaskStop/TaskUpdate/TaskOutput`
  - `TeamCreate/TeamDelete`
  - `CronCreate/CronDelete/CronList`
  - `RemoteTrigger`
  - `LSP`（当前为 Rust fallback search，不再是固定 stub）
  - `ListMcpResources`
  - `ReadMcpResource`
  - `McpAuth`
  - `MCP`

### Stage 5

- Rust interactive runtime 已接管默认 REPL loop、permission prompt、slash command 处理、session persistence。
- 当前外观仍是“saicode 风格的轻量文本 REPL”，不是旧 Ink TUI 的逐像素重现。

### Stage 6

- 默认入口已切换到 Rust：
  - `native/saicode-launcher` 的 `FullCli` 现优先路由 `rust/target/release/saicode-rust-cli`
  - `bin/saicode` 在 native launcher 不存在时，也会优先直启 `saicode-rust-cli`

## 剩余非 Stage 7 残项

- `AskUserQuestion` 仍是 pending payload，不是完整用户问答 tool runtime。
- MCP manager 当前主打 stdio server；remote/SSE/WS transport 还没有完全进入统一 manager。
- LSP 目前是 Rust fallback search，不是完整 language-server orchestration。
- interactive UI 已经 Rust 化，但还没有做到旧 TS/Ink 界面的逐像素 parity。
- plugin tools 还没有被自动注入会话 tool pool；目前 `/plugins` 管理已 Rust 化，但 plugin tool execution surface 还没并入默认会话。

## 判断

- 就 Stage 0-6 的“默认入口切到 Rust、cliproxyapi 主链可用、常用工具 loop 回到 native、具备 parity smoke”目标来说，已经达到可用 cutover。
- 就“满分 UI parity / 全 transport MCP / 真 LSP / plugin tool 全注入”来说，还没到 Stage 7 之前的最终形态。
