# Execution Plan

## Phase 0: Freeze The Correct Product Shape

### Create

- 任务级 spec 与 plan

### Modify

- `specs/saicode-ts-frontend-rust-backend-bridge-20260407/SPEC.md`
- `specs/saicode-ts-frontend-rust-backend-bridge-20260407/PLAN.md`

### Verify

- 方案明确写出：`TS 前端 + Rust 后端`，并排除“删除 TS 前端”的误解。

## Phase 1: Define The Rust Frontend Bridge Contract

### Create

- Rust bridge protocol types
- TS bridge protocol types
- event / command schema 文档

### Modify

- `rust/crates/saicode-rust-cli/src/main.rs` 或新 bridge entrypoint
- `rust/crates/runtime/*`
- `frontend/*` 或 `src/cli/*` 中的 bridge 类型定义

### Deliver

- 一个无终端 UI 的 Rust frontend bridge mode
- stdin/stdout NDJSON contract

### Verify

- 能用 shell 管道喂入 `user_turn`
- 能收到结构化 `content_delta / final_message / session` 事件

## Phase 2: Extract Rust Interactive Logic Into Headless Runtime Surface

### Create

- bridge session runner
- event emitter
- permission callback plumbing

### Modify

- `rust/crates/saicode-rust-cli/src/main.rs`
- `rust/crates/runtime/*`
- `rust/crates/commands/*`
- `rust/crates/tools/*`

### Deliver

- Rust runtime 从“自己打印 UI”改成“发事件给前端”
- slash / session / tools / permissions 可通过同一 bridge surface 使用

### Verify

- bridge mode 下：
  - 普通 prompt 可流式返回
  - tool_start / tool_result 有结构化事件
  - permission request 可被发出

## Phase 3: Build The TS RustBridgeClient

### Create

- `RustBridgeClient`
- child process lifecycle manager
- stdin writer / stdout parser
- reconnect / shutdown handling

### Modify

- `frontend/index.tsx`
- `src/cli/*`
- `src/state/*`
- `src/screens/REPL*`

### Deliver

- TS 启动时拉起 Rust backend
- TS 把用户输入发给 Rust
- TS 消费 Rust 事件并驱动 UI

### Verify

- 前端真实可启动
- 输入 prompt 有实时流
- 退出时能正确回收 backend 子进程

## Phase 4: AppState And UI Event Mapping

### Create

- Rust event -> AppState adapter
- tool / usage / session / status reducer
- permission dialog bridge adapter

### Modify

- `src/state/*`
- `src/components/*`
- `src/screens/REPL*`
- `src/cli/structuredIO.ts` 相关复用点

### Deliver

- 旧 UI 组件继续工作，但数据源改为 Rust

### Verify

- conversation pane 正常更新
- tool panel / spinner / status line 正常更新
- permission dialog 可闭环

## Phase 5: Slash / Session / Process Command Unification

### Create

- slash dispatch adapter
- session control adapter

### Modify

- `rust/crates/commands/*`
- `src/commands/*` 中仍需保留的前端壳
- `bin/saicode`

### Deliver

- `/help` `/status` `/model` `/mcp` `/skills` `/agents` `/plugins` 在 TS 前端统一走 Rust
- `saicode --help` / `status` 等直接命令保持可用

### Verify

- 前台 slash 命令逐项 smoke
- 直连命令面 smoke

## Phase 6: Capability Closure

### Close

- grep / 基础搜索
- MCP
- LSP
- skill
- plugin
- provider/profile/model route

### Modify

- Rust 相关 runtime / tools / commands / plugins / api
- TS 前端显示层与状态层

### Verify

- grep 在真实前台链路可用
- MCP 可列出并调用
- LSP 查询有前端反馈
- skill/plugin 至少可列出和触发

## Phase 7: Cutover And Cleanup

### Modify

- `bin/saicode`
- `package.json`
- `README*`
- 验收脚本

### Deliver

- 默认产品入口正式是 TS frontend bridge
- Rust 原 interactive REPL 只保留为调试面或显式子命令

### Verify

- `saicode`
- `saicode --help`
- `saicode status`
- `saicode -p ...`
- 真实前台 prompt + tool + permission + slash

## Phase 8: Zero-Residual Validation

### Verify

- `bun run frontend:typecheck`
- Rust 相关 cargo tests
- tool acceptance
- closeout preflight
- 真实 TTY 人工验收

### Exit Criteria

- TS 前端不再承载核心业务逻辑
- Rust 成为唯一核心运行时
- 用户可在 TS 前端中正常使用完整 saicode
