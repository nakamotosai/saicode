# saicode Rust fastpath（rewrite 前）Spec

## Goal

在不直接重写整套 QueryEngine 的前提下，把 `saicode` 推到“Rust 重写之前尽量轻”的状态。已完成第 1 步 Rust launcher；本轮继续推进后续 4 步，优先把最常见的简单 `-p` 请求从 Bun/TS 世界里拿出来。

## Scope

### In scope

- 第 2 步：
  - native recovery print
  - native 配置/模型/provider 真相层
- 第 3 步（能做多少做多少）：
  - 高频本地工具链的进一步 native 化准备或落地
- 第 4 步（能做多少做多少）：
  - 重路径的 warm worker / daemon 化准备
- 回写 benchmark / docs / plan

### Out of scope

- 全量 Rust 重写 QueryEngine
- 一轮内把所有工具和所有交互模式都搬到 Rust
- 为了“看起来像重写了很多”而做大量无验收价值的搬运

## Constraints

- 任何 native 新路径都不能让 Bun fallback 失真
- 必须保留真实可用回退
- 必须用真实命令和探针验收，而不是只看 `cargo test`
- 不回退工作区已有脏改动

## Acceptance

1. 简单 `-p` print 请求可完全不进入 Bun 主链。
2. native 路对默认模型、provider、config 文件、关键 env alias 的判断与当前仓库真相一致。
3. 至少补一轮真实 benchmark，说明 simple print 的耗时/RSS 相对当前又下降了一层。
4. 若未能在本轮吃完第 3/4 步，必须明确做到哪一步、还差哪一层，不得含糊。
