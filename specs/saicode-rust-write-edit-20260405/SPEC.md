# saicode Rust Write/Edit phase Spec

## Goal

把 `saicode` rewrite 前的 Rust fastpath 再往前推进一层：让高频的一次性 `Write` / `Edit` 不再默认落回 Bun，而是优先命中 native local-tools；同时保住最关键的安全语义，不把“能写文件”误当成“已经可用”。

这轮必须同时收口两件事：

1. `Write` 支持新建文件与已有文件全量覆盖。
2. `Edit` 支持已有文件的精确替换语义。

## Scope

### In scope

- 扩展 native launcher / local-tools 路由：
  - `Write`
  - `Edit`
- 在 Rust `native local-tools` 内实现：
  - 读快照状态缓存
  - 先读后写 / 先读后改 的安全门
  - 基于 mtime + 内容快照的 stale 检测
  - `Edit.old_string / new_string / replace_all`
  - 多匹配拒绝
  - quote-normalized 匹配与基本引号风格保留
  - 新建文件场景
- 对 native 暂不支持或高风险的情况保留 Bun fallback
- 补对应 Rust tests / dry-run 验证 / 真实 probe
- 回写计划、错题本、session ledger

### Out of scope

- 完整替换 Bun `QueryEngine`
- 完整复刻 TS 侧所有 analytics / LSP / git diff / settings guard
- full-permission Bash 或更大规模 permission runtime Rust 化
- 引入高 MSRV 或重依赖的新 Rust 文本处理栈

## Constraints

- 当前 VPS 仍按旧 Rust toolchain 约束处理，优先复用标准库与现有 native local-tools 架构
- 已有文件的 `Write` / `Edit` 不能静默绕过“先读后写”安全门
- native 处理不了的情况，优先 fallback 到 Bun，而不是伪支持
- 真实验收必须包含 dry-run 命中、自动化回归、以及真实写入/编辑探针

## Acceptance

1. `SAICODE_NATIVE_DRY_RUN=1 ./bin/saicode -p "hello" --tools Write` 命中 `native-local-tools`
2. `SAICODE_NATIVE_DRY_RUN=1 ./bin/saicode -p "hello" --tools Edit` 命中 `native-local-tools`
3. native `Write` 可真实创建新文件
4. native `Write` 对已有文件在未先完整 `Read` 时不会静默写入
5. native `Edit` 可真实完成唯一匹配替换
6. native `Edit` 在多匹配且 `replace_all=false` 时会拒绝
7. stale 文件场景可被检测，不会把旧快照直接覆盖到新文件
8. `cargo test --manifest-path native/saicode-launcher/Cargo.toml` 通过
9. `cargo build --release --manifest-path native/saicode-launcher/Cargo.toml` 通过
10. `bun run verify` 通过

## Risks

- native `Read` 默认仍有行数上限，因此“先读后写”必须识别 full read 与 partial read
- 文本编码 / 行尾差异如果处理不好，容易造成“功能看似成功但文件被悄悄改坏”
- `Edit` 的 quote-normalized 匹配若实现过粗，会出现误替换，因此不确定情况宁可 fallback
