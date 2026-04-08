# saicode P2 收口 Spec

## Goal

把 2026-04-06 完整体检里仍然活跃的 `P2` 问题直接收口到“默认使用不会再被这些问题误导或拖累”的状态。

本轮重点不是扩张新功能，而是收：

- 默认门禁口径误导
- 活跃 workspace 中的 donor 残留
- 自动化覆盖与 `@ts-nocheck` 风险缺少约束

## Scope

### In scope

- 把默认 `typecheck` 收到 full 口径
- 把 `verify/check` 继续维持与当前 Rust frontline 主链一致
- 为 `@ts-nocheck` 增加增长门禁，避免继续失控
- 补测试锁定：
  - `package.json` 关键脚本语义
  - `--` separator 路由一致性
- 把当前已不参与主用链路的 donor 重残留 crate 从 active workspace 隔离

### Out of scope

- 本轮把 256 个 `@ts-nocheck` 全部消灭
- 本轮重写交互主 runtime
- 本轮清完所有 donor 历史代码文本

## Acceptance

1. `bun run typecheck` 默认即 full。
2. `verify/check` 包含 Rust frontline 门禁且继续通过。
3. 存在 `@ts-nocheck` 增长门禁，后续新增不会静默溢出。
4. 至少新增一组测试锁定脚本/路由关键语义。
5. `adapters / bridge / kcode-cli` 不再作为 active workspace 成员参与主项目默认构建/测试。
