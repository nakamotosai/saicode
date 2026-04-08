# saicode P3 收口报告

日期：
- 2026-04-06

## 本轮目标

把完整体检剩余的 P3 项继续收口到：

- active crate 树不再混着 donor 历史目录
- `@ts-nocheck` 门禁与报告使用同一统计口径
- 文档与仓库现状一致

## 实际处理

### 1. donor crate 迁出 active 主树

已把以下历史 crate 从 `rust/crates` 迁到显式 archive：

- [adapters](/home/ubuntu/saicode/rust/archive/kcode-donor/adapters)
- [bridge](/home/ubuntu/saicode/rust/archive/kcode-donor/bridge)
- [kcode-cli](/home/ubuntu/saicode/rust/archive/kcode-donor/kcode-cli)

并新增：

- [README.md](/home/ubuntu/saicode/rust/archive/kcode-donor/README.md)

结果：

- `rust/crates` 现在只保留现役 saicode crate
- donor 资产仍可查，但不再占用活跃实现路径

### 2. `@ts-nocheck` 门禁口径对齐

已把 budget gate 从“按命中行数”改为“按文件数”。

当前默认基线：

- `227` 个文件

结果：

- 门禁口径与体检报告一致
- 后续新增 `@ts-nocheck` 文件会被默认门禁拦住

### 3. 文档同步

已同步更新：

- [REPORT.md](/home/ubuntu/saicode/specs/saicode-comprehensive-health-check-20260406/REPORT.md)
- [SPEC.md](/home/ubuntu/saicode/specs/saicode-p3-closeout-20260406/SPEC.md)
- [PLAN.md](/home/ubuntu/saicode/specs/saicode-p3-closeout-20260406/PLAN.md)

## 验收结果

已满足：

1. `rust/crates` 中不再有 `adapters / bridge / kcode-cli`
2. donor 历史代码已有 archive 落点和说明
3. `ts-nocheck:check` 与报告同口径，并以 `227` 为默认基线
4. 总报告与本轮 closeout 文档已同步

仍然保留但已受控：

- `@ts-nocheck` 总量本身仍偏高，属于长期压降债，不是本轮伪装成“已清零”的问题

## 结论

当前这批可直接收掉的 `P3` 已收口完成。

后续再做 `P3`，重点不再是清理活跃残留，而是持续压降类型债与继续推进更深层运行时治理。
