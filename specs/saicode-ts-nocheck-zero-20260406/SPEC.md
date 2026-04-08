# saicode `@ts-nocheck` 清零 Spec

日期：
- 2026-04-06

## Goal

把当前仓库 `src/` 与 `tests/` 下的 `227` 个 `@ts-nocheck` 文件全部清零。

本轮目标不是“把 budget 锁住就算完”，而是：

- 逐个移除文件级 `@ts-nocheck`
- 补齐缺失类型
- 修正历史遗留的无类型或错误类型写法
- 最终把 `ts-nocheck` 文件数降到 `0`

## Why Now

上一轮已经完成：

- active crate 树收口
- 默认门禁与当前主链对齐
- `@ts-nocheck` 增长门禁落地

这意味着当前剩余最显著、也最可量化的长期债，就是 `@ts-nocheck` 本身。

如果继续保留 `227` 个文件：

- 类型回归会继续藏在文件级豁免后面
- 默认 `typecheck` 的可信度仍然有限
- 后续更深层 Rust 化和主 runtime 治理会持续被 TS 盲区拖慢

## Scope

### In scope

- `src/` 与 `tests/` 下当前带 `@ts-nocheck` 的全部 `227` 个文件
- 为移除 `@ts-nocheck` 而必须补上的：
  - 类型定义
  - 显式返回类型
  - 缺失导入/导出类型
  - 缺失的窄化、断言、守卫
  - 旧的编译产物式写法在必要时回收为稳定 TS 写法
- 相关测试、门禁、文档与进度回写

### Out of scope

- 与移除 `@ts-nocheck` 无关的大规模功能改造
- 为了“风格统一”而进行的非必要重构
- 把类型问题外包成新的全局 `any`、全局声明污染或新的文件级豁免

## Constraints

- 不允许新增新的 `@ts-nocheck`
- 不允许用“改成 `any` 一路通关”冒充修复
- 允许在局部使用合理的 `unknown`、类型守卫、窄化和小范围断言
- 每一批移除后都必须过最小验证
- 进度必须回写到专项文档，而不是只留在聊天里

## Current Baseline

- 当前 `@ts-nocheck` 文件数：`227`
- 主要集中在：
  - `src/utils`: `75`
  - `src/components`: `53`
  - `src/services`: `31`
  - `src/tools`: `20`
  - `src/hooks`: `10`
  - `src/commands`: `10`

完整清单见：

- [INVENTORY.md](/home/ubuntu/saicode/specs/saicode-ts-nocheck-zero-20260406/INVENTORY.md)

## Strategy

按“低耦合先摘、核心重区后攻”的顺序推进：

1. 低风险叶子文件
2. hooks / types / 小型工具函数
3. 中小型组件与轻量服务
4. tools / services / commands 中等复杂文件
5. `src/utils`、`src/components`、`src/services/api` 等重区
6. 收尾校验，确认文件数归零

## Acceptance

1. `rg -l '@ts-nocheck' src tests -g '*.ts' -g '*.tsx' | wc -l` 返回 `0`
2. `bun run typecheck` 通过
3. `bun test` 通过
4. `bun run verify` 通过
5. 专项 `REPORT.md` 能说明每轮已清掉哪些批次、当前剩余多少
