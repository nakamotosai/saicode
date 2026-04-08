# saicode P2 收口 Plan

## Phase 1

- 修改 `package.json`
  - `typecheck` -> full
  - 新增 `ts-nocheck` 门禁
  - 保持 `verify/check` 覆盖 Rust frontline

## Phase 2

- 新增自动化测试：
  - 锁定关键脚本语义
  - 补 `--` separator 在 TS 路由侧的一致性测试

## Phase 3

- 调整 Rust workspace：
  - 移除 `adapters`
  - 移除 `bridge`
  - 移除 `kcode-cli`

## Phase 4

- 跑：
  - `bun test`
  - `bun run typecheck`
  - `bun run verify`
  - `cargo test --manifest-path native/saicode-launcher/Cargo.toml`
  - `./scripts/closeout_preflight.sh`

## 完成判定

- 所有命令通过
- 默认门禁与当前主链一致
- P2 项从“活跃风险”收敛为“受控技术债”或已关闭
