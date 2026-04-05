# saicode Rust Bash bypass phase Spec

## Goal

把 `saicode` rewrite 前 Rust fastpath 的最后一个高频缺口再收掉一层：让 one-shot `Bash` 在用户已经显式进入 `bypassPermissions` / `dangerously-skip-permissions` 的场景下，不再白白回到 Bun，而是优先命中 native local-tools。

这轮目标不是完整复刻 Bun BashTool 的所有权限、后台任务、沙箱与 UI 行为，而是把“用户已明确给出不问权限的一次性 shell 执行”收成稳定的 native fastpath。

## Scope

### In scope

- 扩展 native launcher / local-tools：
  - 接受 `--permission-mode bypassPermissions`
  - 接受 `--dangerously-skip-permissions`
- 在 Rust `native local-tools` 内实现：
  - bypass 场景下的 write-capable `Bash`
  - 直接 shell 执行
  - timeout
  - stdout / stderr / exit code 输出
  - Bash 执行后 read snapshot 清空，避免后续 stale 状态失真
- 保留 native 已有 readonly Bash 能力
- 对仍不支持的情况保留 Bun fallback：
  - `run_in_background`
  - 复杂 Bash 规则匹配 `Bash(...)`
- 补对应 Rust tests / dry-run 验证 / 真实 probe
- 回写计划、错题本、session ledger

### Out of scope

- 复刻 Bun BashTool 的完整 permission rule engine
- native background task / foreground-task notification
- native shell sandbox
- 更深层 `QueryEngine` / permission runtime 全量 Rust 化

## Constraints

- 只在用户已经显式给出 `bypassPermissions` / `dangerously-skip-permissions` 时开放 write-capable native Bash
- 不把“没有沙箱”的 native Bash 假装成默认安全路径
- 对 `Bash(...)` 规则、后台任务等 native 暂不支持能力，宁可 fallback，不做半吊子兼容
- 真实验收必须包含 dry-run 命中、自动化回归、以及真实写场景 probe

## Acceptance

1. `SAICODE_NATIVE_DRY_RUN=1 ./bin/saicode -p "hello" --tools Bash --dangerously-skip-permissions` 命中 `native-local-tools`
2. `SAICODE_NATIVE_DRY_RUN=1 ./bin/saicode -p "hello" --tools Bash --permission-mode bypassPermissions` 命中 `native-local-tools`
3. native bypass `Bash` 可真实执行写文件命令
4. readonly Bash 现有行为不回归
5. `run_in_background` 仍不会被误当成 native 已支持
6. `cargo test --manifest-path native/saicode-launcher/Cargo.toml` 通过
7. `cargo build --release --manifest-path native/saicode-launcher/Cargo.toml` 通过
8. `bun run verify` 通过

## Risks

- native bypass Bash 没有复制 Bun sandbox 语义，因此必须严格限定在显式 bypass 场景
- 一旦 Bash 在同一 tool loop 内改了文件，旧 read snapshot 会失真，因此需要主动失效
- 如果 route 层不接受 permission flags，会出现“代码支持了但真实入口命不中”的假完成
