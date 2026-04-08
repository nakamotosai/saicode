# Archived kcode donor crates

这里存放的是从 kcode donor workspace 带来的历史 Rust crate：

- `adapters`
- `bridge`
- `kcode-cli`

这些目录保留的目的只有两个：

1. 作为迁移过程中的只读参考资产。
2. 避免继续和当前 saicode 现役 crate 混在同一棵 `rust/crates` 主树里。

当前约定：

- `rust/crates/*` 只放现役、可被 active workspace 直接构建/测试的 saicode crate。
- `rust/archive/kcode-donor/*` 明确视为归档资产，不参与默认 workspace 构建、测试和入口判断。
- 若后续其中某段实现需要复用，应先 rebrand / 拆分 / 重新接入，再回到 active crate 树。

P3 closeout on 2026-04-06 moved these donor crates here to eliminate active-tree ambiguity.
