# saicode P3 收口 Plan

## Spec Review

- 目标明确：
  - 收掉当前仍可直接处理的 P3 残留
- 范围明确：
  - 结构归档
  - 门禁口径对齐
  - 文档同步
- 非目标明确：
  - 不假装本轮消灭全部类型债

结论：
- Spec 可执行，直接实施。

## Phase 1

- 把 donor crate 从 `rust/crates` 主树迁到 `rust/archive/kcode-donor`
- 补 archive README
- 更新 `rust/Cargo.toml` 说明

最小验证：
- `find rust/crates -maxdepth 1 -mindepth 1 -type d | sort`
- `find rust/archive/kcode-donor -maxdepth 2 -type f | sort`

## Phase 2

- 调整 `scripts/check_ts_nocheck_budget.sh`
  - 统计口径从命中行数改为文件数
  - 默认基线改为当前 `227`

最小验证：
- `bun run ts-nocheck:check`

## Phase 3

- 更新：
  - 完整体检 `REPORT.md`
  - 本轮 `REPORT.md`

最小验证：
- 文档中的路径、数量、结论与当前仓库真实状态一致

## Phase 4

- 运行最小门禁：
  - `bun run ts-nocheck:check`
  - `bun test`
  - `bun run rust:test:frontline`

完成判定：
- active crate 树已收干净
- 门禁口径与报告一致
- P3 closeout 文档和总报告一致
