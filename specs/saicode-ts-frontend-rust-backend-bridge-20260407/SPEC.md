# saicode TS Frontend Direct-Drive Rust Backend

## Goal

让 `saicode` 形成清晰、稳定、可持续迁移的双层结构：

- `TS 前端` 只负责终端 UI、输入交互、状态展示、权限弹窗和用户体验。
- `Rust 后端` 成为唯一核心运行时，负责会话、模型调用、流式输出、工具执行、LSP、MCP、skill/plugin、provider、session 持久化与命令处理。

最终目标不是“TS 和 Rust 各跑一套”，而是 `TS 前端直接驱动 Rust 后端`，让整个 `saicode` 正常运作。

## Current Facts

### 现有 TS 面

- 当前默认 `saicode` 已恢复到旧 TS 前端界面。
- TS 侧仍保留完整的 UI/状态层基础设施，包括：
  - `src/cli/structuredIO.ts`
  - `src/cli/remoteIO.ts`
  - `src/state/*`
  - `src/components/*`
  - `src/screens/REPL.js`
- 这些能力说明 TS 原本就有“消费结构化流、驱动 UI、处理权限请求”的成熟路径。

### 现有 Rust 面

- Rust 侧已经具备核心运行时能力：
  - provider / model / stream
  - session persistence
  - tool execution
  - grep / web / local tools
  - LSP
  - MCP
  - skill / plugin / command registry
  - slash command
- `rust/crates/saicode-rust-cli/src/main.rs` 已有：
  - `AssistantEvent`
  - `--output-format stream-json`
  - session metadata 输出
  - interactive loop
- 当前 Rust 的主要问题不是“能力缺失”，而是“前后端协议层没有正式抽离”，导致：
  - Rust 自己带了一套难用的前端 REPL
  - TS 前端仍默认绑定旧 TS 运行时
  - 二者没有统一的桥接协议与 ownership

## Problem Statement

当前最核心的结构性问题是：

`UI 层和运行时层没有完成真正解耦。`

具体表现为：

1. TS 前端和 Rust 后端之间没有正式的前端桥接协议。
2. Rust 把“运行时能力”和“自己的交互前端”绑在一起。
3. TS 前端目前恢复的是旧运行链路，不是“直接驱动 Rust”。
4. 流式输出、工具事件、权限请求、slash、session、MCP/LSP 状态尚未统一到同一协议。
5. 没有一条完整验收链路证明“TS 前端 + Rust 后端”已经闭环。

## Target Architecture

## Layer 1: TS Frontend

TS 前端保留以下责任：

- Ink 终端 UI
- prompt 输入
- conversation pane / tool pane / status line
- permission dialog
- slash 输入体验
- 本地短生命周期 UI 状态
- 用户交互事件转发

TS 前端不再负责：

- 模型请求
- tool dispatch
- session 真相源
- provider/profile 解析
- MCP/LSP/skill/plugin 核心逻辑
- grep/search/runtime capability 执行

## Layer 2: Rust Backend

Rust 后端成为唯一真相源，负责：

- session state
- conversation runtime
- model/provider invocation
- stream lifecycle
- tool registry / contracts
- grep/search/web/local tools
- LSP
- MCP
- skill/plugin
- permission policy / execution
- slash commands
- status / doctor / config / profile / sandbox

## Bridge Protocol

TS 与 Rust 之间新增正式桥接协议，优先采用：

- `child_process stdio + NDJSON`

原因：

- 与现有 TS `structuredIO` 设计一致
- 与现有 Rust `stream-json` 能力接近
- 首轮最容易落地和调试
- 终端交互延迟低
- 不必一开始引入 socket / daemon / transport 复杂度

### Rust -> TS 事件

至少统一这些事件：

- `session_started`
- `session_resumed`
- `content_delta`
- `message_stop`
- `final_message`
- `tool_start`
- `tool_result`
- `tool_error`
- `usage`
- `permission_request`
- `permission_resolved`
- `slash_result`
- `status_snapshot`
- `mcp_update`
- `lsp_update`
- `error`

### TS -> Rust 命令

至少统一这些输入：

- `user_turn`
- `slash_command`
- `permission_response`
- `session_new`
- `session_resume`
- `session_compact`
- `interrupt`
- `shutdown`
- `ui_ready`

## Scope

本轮完整方案覆盖：

1. TS 前端直接驱动 Rust 后端
2. 流式输出闭环
3. session / resume / compact / status 闭环
4. slash command 统一到 Rust
5. permission request / response 闭环
6. 工具事件与工具结果渲染闭环
7. grep / search / 基础工具可用
8. MCP / LSP / skill / plugin 在前端可见可用
9. 默认入口和命令路由收敛
10. 验收脚本与手工验收面收口

## Non-Goals

- 本轮不追求把所有 TS UI 组件都立刻重写为更优设计。
- 本轮不追求网络远程 transport 统一到同一阶段完成。
- 本轮不追求把所有历史 feature gate 一次性清掉。
- 本轮不保留 Rust 自己那套终端 REPL 作为主要产品界面。

## Design Decisions

### 决策 1：Rust 必须拆出 Frontend Bridge Mode

不能继续复用当前 `run_interactive_mode()` 作为产品前端。

需要新增一个 Rust 前端桥接模式，例如：

- `saicode-rust-cli frontend-bridge`

历史上也讨论过独立 bridge entrypoint，但当前收口已确定只保留单入口 `saicode` 路线，不再增加第二个用户入口或独立 bridge crate。

职责：

- 接收 TS 的结构化输入
- 输出稳定 NDJSON 事件
- 内部复用现有 runtime / tools / commands / session
- 完全不做终端文本 UI 渲染

### 决策 2：TS 只保留 UI 编排，不再调用旧 TS 运行时

TS 前端入口不能再走当前 `src/entrypoints/cli.tsx -> src/main.tsx` 这条全功能旧链路。

需要改成：

- TS 启动 UI
- 拉起 Rust backend child process
- 通过 bridge protocol 发送用户输入
- 把 Rust 事件映射到 AppState / UI components

### 决策 3：slash command 的真相源迁移到 Rust

`/help`、`/status`、`/model`、`/mcp`、`/skills`、`/agents`、`/plugins` 等命令，以 Rust 为唯一执行面。

TS 前端只负责：

- 输入捕获
- 自动补全 / 展示
- 渲染返回结果

### 决策 4：权限请求必须走显式 request/response 协议

Rust 后端不能直接在 stdout 上弹自己的人类可读提示。

必须统一为：

- Rust 发 `permission_request`
- TS 渲染权限对话框
- TS 回 `permission_response`
- Rust 决定继续 / 拒绝 / 永久更新规则

### 决策 5：流式输出必须是事件流，不是最终大段文本

用户已经明确不接受“最后一下子输出整段”。

因此桥接协议必须以流式事件为一等公民：

- token / delta 连续推送
- tool 生命周期实时推送
- usage / stop / final message 明确分帧

## Migration Strategy

采用“前端协议切换优先”的迁移，而不是“大量先删 TS 再补”。

### Phase A: Protocol Extraction

- 从 Rust CLI 提炼无 UI 的 frontend bridge mode
- 固化 NDJSON schema
- 把当前 `stream-json` 扩展为 UI 可消费的完整事件流

### Phase B: TS Frontend Adapter

- 新建 TS `RustBridgeClient`
- child process 生命周期管理
- stdin/stdout 帧编解码
- event -> AppState 映射
- user input -> Rust command 映射

### Phase C: Feature Closure

- streaming
- tool events
- permission
- slash
- session/new/resume/compact
- grep/search
- MCP/LSP/skill/plugin 展示与操作

### Phase D: Routing Cleanup

- `saicode` 默认进 TS frontend bridge
- `saicode --help` / `status` / 其他 process commands 仍可直走 Rust
- Rust 原始 interactive REPL 降级为 debug/maintenance surface，而不是默认产品面

### Phase E: Validation

- 自动化验收
- 真实 TTY 前台验收
- provider live probe
- grep/MCP/LSP/skill/tool smoke

## Deliverables

1. Rust frontend bridge mode
2. TS Rust bridge client
3. TS AppState bridge adapter
4. slash / permission / session / tool event 映射
5. 默认入口切换
6. 验收脚本与 closeout 更新

## Acceptance

以下验收全部通过，才算达标：

1. `saicode` 无参数默认进入 TS 前端，且该前端由 Rust backend 驱动。
2. 输入普通 prompt 时，前端能看到持续流式输出，而不是最终整段落地。
3. 工具调用过程中，前端能实时看到工具开始、执行和结束结果。
4. permission 请求由 TS 对话框承接，用户响应后 Rust 继续执行。
5. `/help`、`/status`、`/model`、`/mcp`、`/skills`、`/agents`、`/plugins` 等命令可从 TS 前端使用，并由 Rust 返回结果。
6. session `new/resume/compact` 在 TS 前端可用，且 Rust session 文件是真相源。
7. grep / 基础搜索类工具可在真实前台链路中使用。
8. MCP / LSP / skill/plugin 至少达到“可列出、可调用、前端有反馈”的标准。
9. `saicode --help`、`saicode status` 等命令面保持可用。
10. 至少一条真实前台验收链路证明 `TS frontend + Rust backend` 已闭环。

## Risks

### 风险 1：直接复用旧 TS AppState 会过重

处理：

- 先做 adapter 层，不先重写全 UI。
- 用事件映射而不是一次性重构全部状态模型。

### 风险 2：Rust 当前 stream-json 事件不够细

处理：

- 扩展事件 schema，而不是让 TS 去猜文本。

### 风险 3：permission / MCP elicitation 交互复杂

处理：

- 先统一协议，后补富交互细节。
- 所有“需要用户响应”的流程都必须变成显式 control request。

### 风险 4：slash command 和 process command 容易双轨漂移

处理：

- Rust commands crate 成为唯一真相源。
- TS 只做输入和展示，不保留另一份业务实现。

## Out of Scope Cleanup After Closure

当桥接闭环完成后，应继续清理：

- TS 旧运行时逻辑
- Rust 原产品级前端 REPL
- 重复的帮助文案和路由
- 多余入口和历史占位壳
