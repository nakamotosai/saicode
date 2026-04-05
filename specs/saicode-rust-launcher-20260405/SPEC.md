# saicode Rust launcher 第一阶段 Spec

## Goal

正式开始 `saicode` 的 Rust 化，但把第一刀收窄到“最外层 launcher”。目标不是一轮重写 QueryEngine，而是先把 CLI 入口判定从 Bun/TS 挪到原生二进制：让 `help/version/recovery/lite/full` 的分流不再依赖 JS router 才能开始工作，并为下一阶段继续把 recovery 或更深链路挪到 Rust 打好地基。

## Scope

### In scope

- 新增一个 Rust 原生 launcher：
  - 处理 standalone `--help` / `--version`
  - 判定 `recovery` / `lightweight headless` / `full CLI`
  - 直接把请求转发到对应 Bun 入口，而不是先进入 JS router
- 把 `bin/saicode` 改成：
  - 优先使用 native launcher
  - native 不存在或显式禁用时，自动回退到现有 Bun router
- 补最小 Rust 构建与测试脚本
- 在安装脚本中加入“有 cargo 就构建 native launcher”的步骤
- 跑真实命令验证 native path 和 fallback path

### Out of scope

- Rust 重写 QueryEngine / tool runtime / TUI
- Rust 直连 provider 完整实现
- Windows 专项打包
- 把 Rust 校验强行并入所有默认 Bun 验证闸门

## Constraints

- 不破坏当前 Bun 路由的可用性
- native launcher 必须是“增强层”，不是单点故障
- 保持当前关键路由语义：
  - `SAICODE_FORCE_RECOVERY_CLI`
  - `SAICODE_AUTO_BARE_PRINT`
  - simple/lightweight tool routing
- 这一步优先做窄切口和可验证，不因为“已经用了 Rust”就顺手扩大成半重写

## Acceptance

1. 仓库内存在可构建的 Rust launcher，能独立通过 `cargo test`。
2. `bin/saicode` 默认优先走 native launcher，但 native 不存在或被显式禁用时，仍能回退到 Bun 路由。
3. native launcher 至少能原生处理：
   - `saicode --help`
   - `saicode --version`
4. native launcher 至少能正确分流三类路径：
   - recovery print
   - lightweight headless print
   - full CLI
5. 本机完成一轮真实探针，证明 native launcher 已落地而不是只停留在代码层。
