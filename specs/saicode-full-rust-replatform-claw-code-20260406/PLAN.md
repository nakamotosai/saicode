# saicode 基于 claw-code 的全量 Rust 重平台化总计划

日期：
- 2026-04-06

## 当前状态

- Stage 0：已完成
- Stage 1：已完成
- Stage 2：已完成
- Stage 3：已完成
- Stage 4：已完成到可用 cutover
- Stage 5：已完成到可用 cutover
- Stage 6：已完成
- Stage 7：未开始，按用户要求暂不执行删除 / 归档

## 先纠偏

当前需要先纠正执行目标：

- `@ts-nocheck` 清理只能算迁移过程中的局部阻塞清理
- 不能再把它当作主线目标
- 主线必须重新回到“Rust 接管整个运行时”

因此后续执行顺序固定为：

1. 建立新的 Rust 母线
2. 用 parity/harness 锁住外观与 cliproxyapi 语义
3. 逐层把 TS/Bun 主运行时替换掉
4. 最后才清退旧主链

## Reference Conclusions from claw-code

结合 `claw-code` README / PARITY，可直接借鉴的不是某个具体产品 UI，而是 4 个工程做法：

1. `rust/` 成为 canonical workspace
2. parity 文档成为主线门禁，而不是“差不多能跑”
3. 任务 / MCP / LSP / permission 等采用 registry-backed runtime 逐层替换
4. companion/reference workspace 可以短期存在，但不是 primary runtime surface

## 总体阶段

### Stage 0：冻结错误主线

目标：
- 立即停止把 TS 清债作为主目标的继续扩张

动作：
- `ts-nocheck` 只在阻塞 Rust cutover 时顺手处理
- 新的里程碑与汇报统一改成 Rust 接管率口径

完成判定：
- 后续所有阶段都以“Rust 接管了什么”汇报，而不是“还剩多少 TS 类型债”

### Stage 1：建立 Rust canonical workspace

目标：
- 把当前仓库内 Rust 提升为真正唯一主底座

动作：
- 保留已有 `rust/` workspace
- 重新按全量主 runtime 目标审计当前 crates 缺口
- 按 `claw-code` 路线补齐或重建：
  - runtime
  - commands
  - tools
  - session/config persistence
  - provider/api
  - task/team/cron registry
  - MCP/LSP/permission registry

完成判定：
- `rust/` 不再只是高频 fastpath workspace，而是明确覆盖全 CLI 主能力的唯一增长面

### Stage 2：定义 saicode 不可变产品表面

目标：
- 锁定“必须保持不变”的外部行为

动作：
- 把当前不可变表面写成 parity target：
  - CLI 外观
  - 关键命令名与输出口径
  - 消息分组与权限展示体验
  - 模型 / effort / cliproxyapi 相关语义
- 为这些行为建立 snapshot/spec，而不是依赖记忆

完成判定：
- 可以清楚回答“哪些不能改”

### Stage 3：Provider / cliproxyapi Rust 接管

目标：
- 让 Rust 成为真实 provider/request 主链，同时完全保留当前 cliproxyapi 语义

动作：
- 在 Rust 中固化当前：
  - `cliproxyapi` base URL / auth / model alias / provider fallback
  - effort / reasoning_effort 映射
  - 模型 catalog / alias / default model 语义
- 通过真实 live probe 对齐当前 `saicode`

完成判定：
- 所有请求主链不再依赖 TS/Bun provider 逻辑
- `cliproxyapi` 行为保持不变

### Stage 4：Tools / commands / task / permission Rust 接管

目标：
- 让工具、命令、任务、权限、MCP/LSP 不再由 TS 主链驱动

动作：
- 以 `claw-code` 的 registry/runtime 思路接管：
  - tools
  - task/team/cron
  - permission enforcement
  - MCP lifecycle
  - LSP client/runtime
- 只在 UI 层保留外观，不保留旧执行内核

完成判定：
- 命令和工具的真实执行逻辑已 Rust 化

### Stage 5：Interactive UI runtime Rust 接管

目标：
- 保留当前 CLI 外观，但把交互式 runtime 也切到 Rust

动作：
- 先把当前 UI 行为抽成 parity contract
- 在 Rust 中重建：
  - REPL loop
  - prompt input state
  - messages rendering state machine
  - permission/task/tool progress 事件流
- 如果短期需要桥接，桥接层只能是过渡层，不能长期让 TS 继续做主 runtime

完成判定：
- 交互式会话默认也由 Rust binary 驱动
- 外观仍是用户熟悉的 `saicode`

### Stage 6：Parity harness & cutover

目标：
- 不靠主观感觉切主线

动作：
- 建立 `saicode` 自己的 parity harness：
  - one-shot
  - read/grep/write/edit/bash
  - web search/fetch
  - mcp/lsp/task
  - permission prompts
  - model/provider/effort
  - interactive session snapshots
- 让 Rust 主链通过 parity 后，再切默认入口

完成判定：
- Rust 默认入口上线
- 现有用户可见行为无明显漂移

### Stage 7：TS/Bun 主链退场

目标：
- 彻底完成“全量 Rust 改写”

动作：
- 删除或归档：
  - 旧 TS/Bun 主 runtime
  - 不再使用的 launcher fallback
  - 不再使用的 provider/tool/session 路径
- 只保留：
  - 必要参考资产
  - 已明确不在主路径的 archive

完成判定：
- TS/Bun 不再承担主运行时职责
- “全量 Rust 改写”不是口头目标，而是仓库结构事实

## 技术边界

### 必须保留不变

- CLI 外观与用户观感
- `cliproxyapi` 接入与语义
- 模型/effort 口径
- 关键命令名和主要交互文案

### 可以重做

- runtime/session
- tool execution
- commands registry
- provider pipeline
- MCP/LSP/task/permission 内核
- config/session persistence
- interactive state machine

## 里程碑口径

后续只看这 5 个数字，不再以 `@ts-nocheck` 作为主里程碑：

1. Rust 接管的请求主链比例
2. Rust 接管的工具/命令比例
3. Rust 接管的交互式会话比例
4. parity harness 通过率
5. 仍由 TS/Bun 承担主职责的模块数量

## 立即下一步

接下来真正应该做的不是继续单独清 TS 债，而是：

1. 把当前 `saicode` 的“不可变产品表面”写成 parity contract
2. 对照 `claw-code`，补一份 `saicode` 当前 Rust workspace 缺口清单
3. 重新定义 Stage 1-3 的 crate / 模块落点
4. 停止把 TS 清债当主线，转回 Rust 接管主链
