# saicode 完整体检 Plan

## Spec Review

- 目标明确：
  - 建立一套覆盖“可用性 + 结构风险 + 性能 + 文档脚本一致性”的完整体检流程
- 范围明确：
  - 不只测 `-p`
  - 不只测 repo 内 wrapper
  - 不只看 build/test 成功
- 非目标明确：
  - 本轮不把交互 runtime 全量 Rust 化作为体检前置
- 验收明确：
  - 最终必须能输出带证据的问题分级清单

结论：
- Spec 可执行，进入 Plan。

## Plan Review

- 每阶段都映射到 Spec 中的一个或多个 audit surface
- 每阶段都带最小验证
- 当前计划默认先“体检 -> 分级 -> 再决定专项修复”，避免再陷入只修眼前一处

结论：
- Plan 可执行，后续可直接按阶段推进。

## Phase 0 - Baseline Snapshot

目标：
- 固定当前真实状态，避免体检时混淆“旧问题 / 新问题 / 已修问题”。

动作：
- 记录当前工作树状态
- 记录当前 installed command 指向
- 记录当前 Rust/Bun 双栈分工
- 记录当前已知高频路径命中面

最小验证：
- `git status --short`
- `readlink -f ~/.local/bin/saicode`
- `closeout_preflight`
- 关键 trace probe

产物：
- 基线快照
- 体检起点说明

## Phase 1 - Gate Health

目标：
- 确认现有自动化门禁是否可信，是否已经足以发现明显回归。

动作：
- 跑：
  - `bun test`
  - `bun run typecheck`
  - `bun run typecheck:full`
  - `bun run verify`
  - Rust workspace关键 crates tests/build
  - native launcher tests/build
- 记录耗时与失败点

最小验证：
- 所有 gate 命令结果、wall time、显著异常

关注点：
- 是否存在“快门禁绿，但全量门禁有问题”
- 是否存在“脚本名叫 verify，但覆盖面不足”

## Phase 2 - Entrypoint Health

目标：
- 检查所有关键入口是否一致、是否真的可用。

动作：
- 实测：
  - repo wrapper：`./bin/saicode`
  - installed command：`saicode`
  - `--help`
  - `--version`
  - `-p`
  - `saicode` 交互启动
- 覆盖 cwd：
  - repo 内
  - `~`
  - symlink 目录

最小验证：
- 能启动
- 不崩
- 输出合理
- route/target 符合预期

关注点：
- installed command 和 repo wrapper 是否漂移
- home cwd 下是否还能用
- 是否有 `MACRO` / preload / cwd 假设类问题回潮

## Phase 3 - Task Matrix

目标：
- 用一组日常真实任务检查“会不会卡、会不会做错、会不会特别慢”。

动作：
- 至少覆盖：
  - 纯文本
  - Read
  - Grep
  - Glob + Read
  - Write
  - Read + Edit
  - Bash readonly
  - Bash bypass
  - WebSearch
  - WebFetch
- 必要时补：
  - 多工具组合任务
  - 失败/兜底路径

最小验证：
- 正确性：
  - 输出是否符合预期
  - 文件副作用是否正确
- 速度：
  - 每项 wall time
- 稳定性：
  - 是否 timeout
  - 是否静默 fallback
  - 是否出现明显卡死

产物：
- 任务矩阵报告
- 慢项清单

## Phase 4 - Runtime Semantics

目标：
- 检查当前路由与语义面是否存在隐藏漂移。

动作：
- 检查：
  - recovery 命中
  - native local-tools 命中
  - lightweight headless fallback
  - full CLI fallback
  - `--` separator 一致性
  - 默认模型与 provider/profile 解析

最小验证：
- trace 输出
- 配置解析
- 失败文案是否能让人理解

关注点：
- 是否还有“其实掉回 Bun 了但用户不知道”
- fallback 是否比主路径更慢且经常误触发

## Phase 5 - Structural Health

目标：
- 在“能用”之外找出最关键的结构性风险。

动作：
- 扫描：
  - 测试文件数量与覆盖面
  - `@ts-nocheck` 分布
  - scripts/package.json/真实行为一致性
  - 新旧双路径残留
  - Rust workspace 中的 donor 残留与中间态

最小验证：
- 形成具体问题清单，而不是抽象评价

关注点：
- 哪些路径仍只有脚本测，没有行为测
- 哪些模块虽然能跑，但未来极易回归

## Phase 6 - Synthesis

目标：
- 形成最终体检结论与后续动作。

输出格式：
- `Passed`
  - 当前已确认健康的面
- `Findings`
  - P0 / P1 / P2 / P3
  - 每项附证据
- `Recommendation`
  - 是否建议继续同轮修复
  - 如果继续，先修哪一类

## 当前初始判断

基于 2026-04-06 前的现状，本轮体检开始前的初始判断是：

- 当前未见新的 `P0` 阻塞故障
- 当前用户级可用性已经过一轮恢复和收尾
- 当前最值得重点检查的剩余面是：
  - 交互主 runtime 深层路径
  - fallback 误触发与语义漂移
  - 测试/类型门禁覆盖不足
  - 文档/脚本/真实行为一致性

## 执行顺序

1. Baseline Snapshot
2. Gate Health
3. Entrypoint Health
4. Task Matrix
5. Runtime Semantics
6. Structural Health
7. Synthesis
