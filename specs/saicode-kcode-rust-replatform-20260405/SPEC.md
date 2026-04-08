# saicode 基于 kcode 的 Rust 重平台化 Spec

## Goal

把当前 `saicode` 从“Bun/TypeScript 为主、Rust 只覆盖 launcher/fastpath”的架构，升级为“Rust 为主运行时”的新底座。

本轮不再把 `kcode` 仅作为参考仓库，而是明确把它视为：

- Rust 重写现成底座候选
- 可直接继承的大块核心模块来源
- `saicode` 全仓 Rust 化的加速器

同时，必须保留 `saicode` 现有已经形成用户习惯和产品差异的显示层与自定义层，尤其是：

- 当前 REPL/TUI 的主要前端显示页面与交互体验
- `saicode` 自己的消息展示/分组/权限弹层/UI 细节
- `saicode` 现有模型选择、effort 展示、自定义命令面和本机定制行为

目标不是做“另一个 kcode”，而是做“以 kcode Rust 内核为基础、保留 saicode 产品表面的新 saicode”。

## Scope

### In scope

- 评估并固化以下迁移原则：
  - 哪些 `kcode` Rust crate 可以直接继承
  - 哪些 `saicode` 现有表层必须保留
  - 哪些能力必须重写适配，不能原样继承
- 形成明确的新分层：
  - Rust 核心运行时层
  - Rust 工具/命令/会话层
  - `saicode` 自有 UI/TUI 表现层
  - `saicode` 自有产品行为适配层
- 以 `kcode` 为 donor base，规划后续迁移顺序，至少覆盖：
  - provider/api
  - runtime/session
  - tools/commands
  - bridge/adapters（按需）
  - `saicode` 现有 REPL 展示层对接点
- 给出“能否直接移植”的工程结论：
  - 哪些可以直接搬
  - 哪些只能参考后重写
  - 哪些暂时不接

### Out of scope

- 本轮直接完成整个 `saicode` Rust 重写
- 立即删除现有 Bun/TS 全量实现
- 一次性把 `kcode` 全仓原样塞进 `saicode`
- 在没有适配层前就切断现有 `saicode` 可用入口

## Constraints

- 允许直接吸收 `kcode` 的 MIT 代码，但不能把“仓库名替换”当成迁移方案
- 以 `saicode` 现有产品外观和用户习惯为主，不以 `kcode` 默认 UI 替代现有显示层
- 对 `kcode` 中仍为 stub / 半实现 / 占位的能力，不得写成“可直接继承”
- 新架构应优先减少：
  - Bun 启动负担
  - 宽 prompt / plan-mode 重路径
  - 高频本地工具调用的额外内存和冷启动成本
- 迁移必须分阶段可运行，不能走“一把梭哈切主线”

## 保留面

以下 `saicode` 表层视为默认保留资产，而不是迁移时默认抛弃：

- [REPL.tsx](/home/ubuntu/saicode/src/screens/REPL.tsx)
- [PromptInput.tsx](/home/ubuntu/saicode/src/components/PromptInput/PromptInput.tsx)
- [Messages.tsx](/home/ubuntu/saicode/src/components/Messages.tsx)
- [ModelPicker.tsx](/home/ubuntu/saicode/src/components/ModelPicker.tsx)
- `src/components/messages/*`
- `src/components/permissions/*`
- `src/components/tasks/*`
- `src/components/design-system/*`
- `src/commands/*` 中用户已形成习惯的命令行为与文案口径

这些层后续可以逐步 Rust 化或做桥接，但默认目标是“保留 saicode 的产品表面”，不是直接替换成 `kcode` 当前默认前端。

## Donor 继承优先级

优先考虑从 `kcode` 直接继承或重用设计的部分：

- Rust workspace / crate 分层方式
- provider / runtime / session / tool registry 主干
- Rust 原生命令解析与 slash command registry
- Rust TUI 渲染和 palette 的可复用部件
- memory / session persistence / provider profile

默认不直接继承、而是需要适配或保留 `saicode` 原版的部分：

- 当前 `saicode` 的 REPL 页面表现与消息 UI
- 现有 `saicode` 的模型/effort 展示交互
- 已有 `saicode` 的 plan mode、权限弹层、任务视图、定制命令口径
- `kcode` 中仍为占位或半实现的 task/MCP auth/remote trigger 等能力

## Acceptance

1. 仓库内有一份明确的迁移 Spec，说明 `kcode` 在新 `saicode` 中扮演什么角色。
2. 仓库内有一份可执行阶段计划，至少拆到“底座、桥接、表层保留、切换”级别。
3. 明确指出哪些 `kcode` 能直接继承，哪些不能。
4. 明确保留 `saicode` 的前端显示页和自定义层，不把它们含糊写成“后面再看”。
5. 最终结论必须能回答用户当前问题：
   - 可以直接借 `kcode` 大幅省步骤
   - 但应采用“Rust 底座直继承 + saicode 表层保留”的路径，而不是整仓平移。
