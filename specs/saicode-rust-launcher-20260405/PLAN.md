# saicode Rust launcher 第一阶段 Plan

## Round 1 - Spec 收口

- 新建本轮 Rust task Spec / Plan
- 把范围收窄为“原生 launcher + Bun fallback”

## Round 2 - Native launcher

- 新建 `native/saicode-launcher`
- 实现：
  - help/version 快路径
  - recovery / lightweight / full 路由判定
  - Bun entrypoint 直达执行
  - repo root 解析
  - trace / dry-run 调试口

## Round 3 - Wrapper 与安装

- 把 `bin/saicode` 改成 shell wrapper
- 默认优先 native，保留显式禁用 native 的回退开关
- 更新 `package.json` 与安装脚本
- 补 `.gitignore`

## Round 4 - 验证

- `cargo test --manifest-path native/saicode-launcher/Cargo.toml`
- `bun test`
- `bun run typecheck`
- 真实探针：
  - native `--help`
  - native `--version`
  - native dry-run recovery / lightweight / full 路由
  - fallback Bun `--help`

## Round 5 - 回写

- 把本轮结果回写到任务 Plan
- 必要时再压缩到错题本 / session ledger

## Outcome

- 本轮已完成：
  - 新增 `native/saicode-launcher` Rust 原生 launcher
  - native launcher 已接管：
    - standalone `--help`
    - standalone `--version`
    - recovery / lightweight headless / full CLI 路由判定
  - `bin/saicode` 改为 shell wrapper：
    - 默认优先 native
    - `SAICODE_DISABLE_NATIVE_LAUNCHER=1` 时回退 Bun launcher
  - 补充：
    - `native:build`
    - `native:test`
    - 安装脚本里的 native build
    - `.gitignore` 对 Rust target 的收口
- 验证通过：
  - `cargo test --manifest-path native/saicode-launcher/Cargo.toml`
  - `cargo build --release --manifest-path native/saicode-launcher/Cargo.toml`
  - `bun test`
  - `bun run typecheck`
  - `./bin/saicode --help`
  - `./bin/saicode --version`
  - `SAICODE_NATIVE_DRY_RUN=1 ./bin/saicode -p "hello"`
  - `SAICODE_NATIVE_DRY_RUN=1 ./bin/saicode -p "hello" --tools WebSearch`
  - `SAICODE_NATIVE_DRY_RUN=1 ./bin/saicode -p "hello" --resume session-id`
  - `SAICODE_DISABLE_NATIVE_LAUNCHER=1 ./bin/saicode --help`
  - `./bin/saicode -p "Reply with exactly: ok"` -> `ok`
  - `./bin/saicode -p "...QueryEngine..." --tools Grep` -> `29`
- 实测补充：
  - native `--help`: `run=0.01`, `rss=3116`
  - Bun fallback `--help`: `run=0.04`, `rss=47872`
- 本轮结论：
  - `saicode` 已经正式进入 Rust 第一阶段，但仍只切到了“最外层 launcher”。
  - 真正更重的 query / QueryEngine / tool runtime 还在 Bun/TS 世界里，下一阶段要想继续显著降首请求成本，就该考虑把 recovery print 或更深的 headless 主链继续下沉到 Rust。
