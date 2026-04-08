# saicode 基于 claw-code 的全量 Rust 重平台化 Spec

日期：
- 2026-04-06

## Goal

把当前 `saicode` 从“Rust 只覆盖部分高频路径、主运行时仍由 Bun/TypeScript 承担”的状态，推进到：

- **Rust 成为唯一主运行时**
- **Rust 成为唯一主 CLI/会话/工具/命令执行面**
- **现有 CLI 外观与交互体验保持不变**
- **现有 `cliproxyapi` 接入、模型路由、provider 语义保持不变**

目标不是继续把旧 TS 体系打磨得更健康，而是最终让旧 TS 退出主运行时位置。

## External Constraints

以下两块视为强约束，不作为迁移时的改造目标：

1. `saicode` 当前 CLI 外观、消息呈现风格、交互观感不改。
2. `cliproxyapi` 接入、模型/provider 语义、已有别名和调用口径不改。

除了这两块之外，其它运行时、命令、工具、会话、权限、任务、MCP、配置读取实现都允许重写。

## Reference Baseline: claw-code

本轮参考的上游是：

- `ultraworkers/claw-code`

已确认到 2026-04-06 的公开状态：

- 仓库主语言是 `Rust`
- README 明确把 `rust/` 视为 canonical workspace
- README 明确把 `src/ + tests/` 视为 companion/reference workspace，而不是 primary runtime surface
- `PARITY.md` 明确采用“Rust 主 workspace + parity 文档 + mock parity harness”的推进方式
- `PARITY.md` 记录了任务、team/cron、MCP、LSP、权限等 registry-backed runtime 的逐步替换方式

这给 `saicode` 的价值不是“照搬它的产品表面”，而是：

- 借它验证过的 Rust 主 runtime 分层
- 借它的 parity/harness 推进方式
- 借它已经踩过的 registry/runtime/tool/permission/MCP/LSP 路线

## Current Reality

当前 `saicode` 的真实状态是：

- 高频路径已有一部分 Rust 化
- 但主交互 runtime、大量工具 UI、命令面、设置面、消息展示面仍在 `src/**/*.ts|tsx`
- 当前仓库还存在大体量 TS 资产与 `@ts-nocheck` 债

这说明：

- 之前的 Rust 化属于“关键路径 Rust 化”
- 还远远不是“全量 Rust 重写完成”

## Scope

### In scope

- 以 `claw-code` 为参考，重建 `saicode` 的 Rust 主 runtime
- 让 Rust 接管：
  - CLI entrypoint
  - session/runtime
  - provider/request pipeline
  - tools/commands/task/MCP/LSP/permission execution
  - config/session persistence
  - interactive rendering/runtime state machine
- 保留 `saicode` 现有外观和 cliproxyapi 语义
- 建立新的 parity harness，保证 Rust 新实现对齐当前 `saicode` 用户可见行为
- 在最终 cutover 后，逐步删除旧 TS 主链和不再使用的 Bun runtime 路径

### Out of scope

- 继续把 TS `@ts-nocheck` 清零作为独立主线目标
- 重设计 CLI 外观
- 更换 `cliproxyapi` 接入方式或模型/provider 语义
- 在没有 parity/harness 的情况下直接一次性硬切主入口

## Migration Principle

采用：

- `claw-code` 的 **Rust 主 runtime / registry / parity-harness 思路**
- `saicode` 的 **外观 / 交互 / cliproxyapi 语义**

即：

- 不是做“另一个 claw-code”
- 而是做“内核按 claw-code 路线 Rust 化、外表保持 saicode 的新 saicode”

## Required End State

最终完成态必须满足：

1. `saicode` 默认入口由 Rust binary 驱动，而不是 Bun/TS。
2. 常用会话、命令、工具、权限、任务、MCP、LSP 全部走 Rust 主链。
3. `cliproxyapi` 与相关 provider/model 行为保持与当前用户习惯一致。
4. 用户看到的 CLI 外观、提示、消息体验不发生产品级漂移。
5. 旧 TS 运行时不再承担主路径职责，只允许保留为短期过渡适配或参考资产，最终应删除或归档。

## Acceptance

1. 有一份新的母计划，明确以 `claw-code` 为参考对象，而不是继续把 TS 清债当主线。
2. 计划明确区分：
   - 哪些 Rust donor/runtime 思路可借鉴
   - 哪些 `saicode` 表层必须保持不变
3. 计划必须覆盖：
   - runtime
   - provider
   - tools
   - MCP/LSP/task/permission
   - interactive UI runtime
   - parity harness
   - cutover
   - TS/Bun 退场
4. 最终完成标准必须是“Rust 主运行时接管”，不是“TS 类型更干净”。
