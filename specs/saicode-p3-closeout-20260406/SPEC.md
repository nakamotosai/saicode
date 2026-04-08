# saicode P3 收口 Spec

## Goal

把 2026-04-06 完整体检里剩余的 `P3` 项从“仍挂在主树里的残留问题”收成：

- active 树结构清晰
- 门禁口径和报告口径一致
- 文档与当前真实状态一致

本轮不追求把长期类型债一次清零，而是要把当前还能直接收掉的 `P3` 全部收掉。

## Scope

### In scope

- 把 donor 历史 crate 从 `rust/crates` 主树迁到明确 archive 路径
- 为 archive 补说明，避免后续继续误判为现役 crate
- 把 `@ts-nocheck` budget gate 调整为“按文件数统计”，和体检报告口径一致
- 把基线收准到当前仓库真实值
- 更新体检报告与本轮 P3 closeout 文档

### Out of scope

- 本轮删除全部 donor 历史代码
- 本轮把 `@ts-nocheck` 文件数直接大幅压到很低
- 本轮推进新的运行时大改

## Constraints

- 不回退现有可用入口
- 不把 archive 资产继续留在 active crate 树里
- 不编造“类型债已经解决”；必须明确它只是进入长期受控状态

## Acceptance

1. `rust/crates` 目录中不再出现 `adapters / bridge / kcode-cli`。
2. donor 历史 crate 有明确 archive 落点和说明文件。
3. `ts-nocheck:check` 改为按文件数统计，并以当前 `227` 作为默认基线。
4. 体检报告与 P3 closeout 文档能准确反映当前状态。
5. 最小验证命令通过，证明收口没有破坏现有主链。
