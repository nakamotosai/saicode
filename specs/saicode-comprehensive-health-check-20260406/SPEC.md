# saicode 完整体检 Spec

## Goal

为当前 `saicode` 建立一套“仓库级 + 运行级 + 用户可见级”的完整体检规格，重新检查整个项目是否还存在新的可用性问题、结构性风险、脚本/文档漂移、性能瓶颈或高概率回归点。

本轮目标不是立刻重写全部剩余架构，而是把“完整体检”这件事先收成一套明确可执行的规格，后续所有巡检、修复、收尾和发布前检查都统一按这套规格走。

## Why Now

当前项目已经完成多轮 Rust 化与入口收口，最近几轮已拿到以下积极信号：

- `saicode` installed command 可直接使用
- `--help` / `--version` / 简单 `-p` 已恢复
- 高频 `Read/Grep/WebSearch/WebFetch/Bash/Write/Edit` 已可走新 Rust fastpath
- `closeout_preflight`、`live probe`、`verify` 已通过

但这并不等于“整仓健康已被完整证明”。当前仍然需要一轮正式体检来回答：

- 现在到底还有没有新的 P0/P1 问题
- 哪些只是技术债，哪些是高概率回归点
- 哪些路径虽然能跑，但仍然偏重、偏慢或覆盖不足
- 现在的发布前/日常使用前检查是否足够

## Current Baseline

截至 2026-04-06，基于当前仓库与最近实测：

- 代码面规模：
  - `src` 下 TS/TSX 文件约 `1950`
  - `rust` 下 Rust 源文件约 `208`
  - `src/tests/native/rust` 合计代码文件约 `2174`
- 测试面：
  - `tests/` 文件数约 `9`
  - `src + tests` 中 `@ts-nocheck` 文件数约 `227`
- 运行面：
  - installed command 正常
  - repo wrapper 正常
  - 简单 `-p` 正常
  - 高频 local-tools/native-tools 正常
- 结构面：
  - recovery/local-tools 共享逻辑已迁入 `rust/crates/saicode-frontline`
  - launcher 主要退化为路由壳
  - interactive 主 runtime 仍主要由 Bun/TS 承担

## Scope

### In scope

- 仓库级健康检查：
  - 测试、类型检查、脚本门禁、构建、启动面
- 用户级可见健康检查：
  - installed command
  - repo wrapper
  - 非交互 `-p`
  - 交互启动
  - 高频任务链路
- 运行路径检查：
  - recovery
  - native local-tools
  - lightweight headless
  - full CLI fallback
- 配置/模型/入口一致性检查：
  - 默认模型
  - provider/profile 解析
  - `--` prompt separator
  - installed command 与 repo 内命令是否一致
- 结构性风险检查：
  - 测试覆盖薄弱点
  - `@ts-nocheck` 旁路面
  - 新旧双路径并存残留
  - 文档/脚本/真实行为漂移
- 性能与卡顿检查：
  - 慢任务分类
  - 正常网络耗时与异常阻塞区分
  - 冷启动/高频 one-shot/tool-loop 的相对耗时

### Out of scope

- 本轮直接完成整个交互主 runtime 的全面 Rust 重写
- 无证据的大规模性能推测
- 与当前体检目标无关的美化、重构或风格整理
- 为了“整洁”而回退用户工作区已有改动

## Constraints

- 必须以真实命令、真实入口、真实工作区、真实 installed command 为准
- 必须区分：
  - 已影响可用性的故障
  - 高概率回归的结构性风险
  - 可延后的技术债
- 体检结论必须可复核：
  - 每个重要判断都对应命令、代码证据或前台行为
- 不能只测 repo 内 wrapper；必须包含 `~/.local/bin/saicode`
- 不能只测成功路径；必须明确：
  - fallback 是否合理
  - 错误是否可理解
  - 是否存在“静默掉回更重路径”的现象

## Audit Surfaces

### Surface A - Build & Gate

- `bun test`
- `bun run typecheck`
- `bun run typecheck:full`
- `bun run verify`
- Rust workspace tests/builds
- native launcher tests/builds

### Surface B - Entrypoints

- `saicode --help`
- `saicode --version`
- `saicode -p ...`
- `saicode` 交互启动
- repo wrapper / installed command / symlink / home cwd

### Surface C - Task Execution

- 纯文本 one-shot
- Read / Grep / Glob
- Write / Edit
- Bash readonly / bypass
- WebSearch / WebFetch
- 多工具组合任务

### Surface D - Runtime Semantics

- 路由是否命中预期 target
- `--` separator 是否全链路一致
- 默认模型是否合理
- fallback 是否显式、可理解、可接受

### Surface E - Structural Health

- 测试覆盖薄弱区
- 类型旁路数量与集中区
- 旧路径残留与双实现风险
- 文档/脚本/实际行为是否一致

## Severity Model

- `P0`
  - 已影响当前可用性，用户现在就会撞上
- `P1`
  - 当前虽然未必每次爆，但高概率回归或会在常见使用中造成严重误导/阻塞
- `P2`
  - 重要但不阻塞当前使用，适合纳入下一轮收敛
- `P3`
  - 技术债或质量改进项

## Deliverables

- 一份体检执行计划 `PLAN.md`
- 一份体检结论报告：
  - 通过项
  - 问题分级清单
  - 每项证据
  - 建议修复顺序
- 必要时生成专项子 Spec / 子 Plan，挂回母计划

## Acceptance

1. 体检计划必须覆盖 build、entrypoint、task execution、runtime semantics、structural health 五大面。
2. 至少包含 installed command、repo wrapper、`-p`、交互启动和多类任务的真实探针矩阵。
3. 最终必须输出 P0/P1/P2/P3 分级结论，而不是笼统说“差不多可用”。
4. 必须明确哪些问题已经会挡住当前使用，哪些只是应继续收敛的风险。
5. 计划应能直接作为后续修复闭环的依据，而不是一次性口头说明。
