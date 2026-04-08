# saicode Rust 流式输出 / LSP / 工具契约 / 插件与 MCP 完整化 Spec

日期：
- 2026-04-07

## Goal

在当前“Rust 已成为主入口，但关键能力仍存在半成品或占位实现”的基础上，把以下四条主链补到可作为默认产品面的完成态：

- Rust 流式输出
- Rust LSP 真实现
- Rust 工具契约统一
- Rust 插件 / MCP 真注入与真执行

目标不是继续保留“能跑但不一致”的兼容层，而是把 Rust 主链收敛到：

- 对外帮助文案与真实能力一致
- 对模型暴露的工具 schema 与真实执行一致
- 对用户可见的流式 / 工具 / LSP / MCP 行为稳定且可验收

## Current Reality

基于当前仓库事实：

- 默认入口已切到 Rust / native launcher。
- Rust CLI 当前仍只正式支持 `text|json`，`stream-json` 没有完整产品面。
- Rust `LSP` 当前仍是 fallback search，而不是 language-server orchestration。
- Rust tool surface 仍存在 display schema、runtime dispatch、prompt guidance 三套真相并行。
- plugin 管理已部分 Rust 化，但 plugin tools 还没有完整进入默认会话 tool pool。
- MCP 已具备部分 stdio manager / resource 能力，但 transport、auth、tool 注入、会话绑定还不完整。

## Donor Reuse Decision: claw-code

已核对 GitHub 仓库：

- 仓库：`ultraworkers/claw-code`
- 顶层 README 明确 `rust/` 为 canonical workspace
- `rust/Cargo.toml` 标记 workspace package `license = "MIT"`

本轮 donor 结论分三类：

### 可以直接复制

1. `rust/crates/runtime/src/permission_enforcer.rs`
   - 适合作为权限执行层的直接 donor
   - 原因：
     - 与产品表面耦合低
     - 结构简单
     - 责任边界清楚
     - 测试覆盖完整
   - 允许改名与接线，但逻辑可直接迁入 `saicode`

2. `rust/crates/rusty-claude-cli/tests/mock_parity_harness.rs`
   - 适合作为 parity harness 主骨架 donor
   - 原因：
     - 与模型/工具/权限/插件/MCP 的行为验收组织方式成熟
     - 与 `saicode` 当前缺少的“真实闭环验收”高度匹配
   - 复制后需要把：
     - binary name
     - provider env
     - scenario 文案
     - tool names
     - session/config 路径
     替换成 `saicode` 版本

3. `rust/MOCK_PARITY_HARNESS.md`
   - 可直接借其 harness 文档组织方式与场景结构

### 可以复制骨架，但必须做 saicode 语义适配

1. `rust/crates/runtime/src/mcp_tool_bridge.rs`
   - 可复制 registry 结构、连接状态模型、tool/resource/auth bridge 组织方式
   - 必须适配：
     - 现有 `saicode` runtime manager 类型
     - tool naming 规则
     - event surface
     - provider / session / config 语义

2. `rust/crates/tools/src/lib.rs`
   - 不能整段复制
   - 只能复制：
     - `GlobalToolRegistry` 组织模式
     - builtin/runtime/plugin tool 三层装配思路
     - `normalize_allowed_tools()` 这类工具契约收口模式
   - 不能直接搬的原因：
     - 该文件高度产品化
     - donor 默认 tool names、provider、command surface 与 `saicode` 不同

3. `rust/crates/rusty-claude-cli/src/main.rs`
   - 可参考其 REPL / CLI / streaming / runtime-plugin-state 装配方式
   - 不建议整段复制
   - 原因：
     - 它的 CLI surface、provider、command taxonomy 与 `saicode` 已明显分叉

### 只能参考，不应直接复制

1. `rust/crates/runtime/src/lsp_client.rs`
   - 当前实现只是 registry + dispatch 占位，不是真正 language-server orchestration
   - 可借其：
     - action enum
     - result structs
     - registry 边界
   - 不应直接复制为最终实现
   - 原因：
     - 它仍然是“LSP registry facade”，不是本轮所需的真 LSP runtime

## Copy Policy

本轮严格按以下策略使用 donor：

- 可直接复制：
  - 优先保留测试与模块边界
  - 只改命名、接线和 `saicode` 运行时适配
- 可复制骨架：
  - 只借结构，不借产品表面
  - 必须经过 `saicode` 语义重写
- 参考-only：
  - 只用来设计接口与阶段，不直接搬代码

## Scope

### In scope

- Rust CLI / launcher 的流式输出完整化
- Rust LSP runtime 真实现
- Rust tool contract 单一真相源重建
- Rust plugin / MCP 主链完整化
- 相关帮助文案、参数、错误面、测试与验收

### Out of scope

- 旧 TS/Ink 界面逐像素复刻
- 新增非必要产品能力
- 更换 provider / model / cliproxyapi 语义
- 大规模 UI 美化

## Constraints

- 默认入口必须继续保持 Rust-only。
- 不能再引入新的 TS 后端回退。
- 不能让帮助页、schema、真实执行再次漂移。
- 所有阶段都必须有最小验收命令。
- 若某能力尚未完成，不允许继续对外假装支持。

## Required End State

### 1. Streaming

- `./bin/saicode -p --output-format stream-json ...` 真正可用。
- interactive Rust CLI 具备逐步输出，不再只在完成后整段打印。
- help、参数解析、runtime、测试对 `stream-json` 的认知一致。
- 非流式与流式的错误面区分清楚，不把内部事件裸露给用户。

### 2. LSP

- Rust 侧不再使用 fallback grep 冒充 LSP。
- 至少支持：
  - definition
  - references
  - hover
  - document symbols
  - workspace symbols
- tool schema、prompt、dispatch、结果格式完全一致。
- 无法连接 LSP server 时给出明确、可操作、非误导的错误信息。

### 3. Tool Contract

- tool 名称、输入 schema、权限要求、display name、runtime canonical name 共用单一真相源。
- 不再出现“帮助里说支持、运行时报不支持”或“schema 允许、dispatch 失败”的情况。
- 模型收到的 tool definitions 与实际 dispatcher 保持 1:1。

### 4. Plugin / MCP

- plugin tools 可被发现、过滤、注入默认会话并执行。
- MCP 至少完整覆盖：
  - stdio
  - remote SSE / WebSocket（若现有 runtime 已有基础）
  - 资源列举 / 读取
  - tool 注入
  - auth 生命周期
- plugin / MCP 错误不再直接污染主回复，而是通过结构化事件或明确提示呈现。

## Non-goals Clarified

- 本轮不要求把所有前端 UI 用 Rust 重做。
- 本轮不要求把所有历史脚本都重写，只要求脚本口径不再误导。
- 本轮不追求所有 donor 能力全量搬运，只交付当前 `saicode` 默认产品面所需的闭环。

## Acceptance

### A. Streaming acceptance

1. `./bin/saicode --help` 明确展示并真实支持 `stream-json`。
2. `./bin/saicode -p --output-format stream-json --verbose "Reply with exactly: ok"` 输出 NDJSON 流。
3. 流式模式下不会再出现“最终整段一次性输出、看不到过程”的现象。
4. 非法组合参数会被明确拒绝，错误文案与实际限制一致。

### B. LSP acceptance

1. Rust `LSP` tool schema 与 dispatcher 完全一致。
2. 至少一门真实语言 server 探针可通过。
3. 不再出现 `LSP query is empty` 这类由契约设计导致的伪错误。
4. 帮助文案不再把 fallback search 描述成 LSP。

### C. Tool contract acceptance

1. 工具定义源头唯一。
2. display name / canonical name 映射统一。
3. `allowedTools` / `disallowedTools` / tool prompt guidance 与 runtime 行为一致。
4. 所有现有内建高频工具都通过契约一致性测试。

### D. Plugin / MCP acceptance

1. 至少一个 bundled plugin tool 可被默认会话调用。
2. 至少一个 MCP server 的 tool 与 resource 路径可在默认会话中工作。
3. MCP auth 可完成最小成功 / 失败路径验证。
4. plugin / MCP 加载失败不会污染主答案正文。

## Risks

- 当前 Rust CLI 已成为主入口，补能力时会直接影响默认产品面。
- LSP 真实现会引入 server lifecycle、workspace sync、超时与重试复杂度。
- MCP transport 完整化涉及 auth、session、错误传播和 reconnect。
- tool contract 收口会波及 CLI 帮助、prompt guidance、permission policy、tests。

## Rollback

- 若某子链路无法在当轮做完，应先下线帮助文案和入口暴露，不能半支持。
- 保持 `text|json`、基础会话、基础工具可用，禁止为了补高级能力把基础路径打坏。
