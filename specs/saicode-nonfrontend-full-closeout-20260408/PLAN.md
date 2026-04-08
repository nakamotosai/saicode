# Plan

## Spec Review

- 目标清楚：本轮不是泛泛“再检查一下”，而是把非前端能力收口到真实可用、可验证、不卡顿。
- 范围清楚：只做非前端，不再混入浏览器/视觉前端。
- 最大风险已识别：
  - 工具面列出不等于可用
  - 交互工具调用可能收尾卡住
  - acceptance 默认模型与当前真实默认模型漂移
- 验收可执行：每一类能力都要求真实命令探针，不只看单一 smoke。

结论：Spec 可以进入实施。

## Phase 1. Baseline Inventory

目标：
- 先拿到当前非前端面的真实完成度和缺口清单

Modify / Inspect
- `bin/saicode`
- `native/saicode-launcher/**`
- `rust/crates/**`
- `scripts/closeout_preflight.sh`
- `scripts/rust_tool_acceptance.sh`
- `README.md`
- `README.en.md`
- 当前相关 specs / docs

动作
- 盘点命令面、工具面、launcher 路由、默认模型、closeout 脚本、acceptance 脚本
- 形成“已稳定 / 未稳定 / 假完成 / 缺验证”四类清单
- 明确哪些工具当前宣称可用，哪些只是代码存在但不应纳入本轮默认验收

最小验证
- `./bin/saicode --help`
- `./bin/saicode status`
- `./bin/saicode config show`
- `rg` 检查当前默认模型、closeout、acceptance 是否一致

## Phase 2. Runtime Truth And Entry Consistency

目标：
- 统一当前唯一入口、默认模型、路由和帮助口径

Modify
- `bin/saicode`
- `native/saicode-launcher/src/main.rs`
- `rust/crates/saicode-rust-cli/src/main.rs`
- `rust/crates/saicode-frontline/**`
- `README.md`
- `README.en.md`
- `docs/closeout-workflow.md`

动作
- 清理入口/帮助/状态输出中的过时假设
- 统一默认模型与 acceptance 默认模型
- 明确 launcher、full CLI、local-tools、one-shot 的真实路由规则
- 把 README / closeout workflow 对齐到当前真链路

最小验证
- launcher tests
- `./bin/saicode status`
- `./bin/saicode --help`
- `saicode --help`

## Phase 3. Core Tool Surface Hardening

目标：
- 让 builtin 工具面真实可用，而不是“能列出来但不可靠”

Modify
- `rust/crates/tools/**`
- `rust/crates/runtime/**`
- `rust/crates/saicode-frontline/**`
- `rust/crates/saicode-rust-cli/**`
- `scripts/rust_tool_acceptance.sh`

动作
- 逐类核对 builtin 工具：
  - `Read`
  - `Grep`
  - `Glob`
  - `Write`
  - `Edit`
  - `Bash`
  - `WebSearch` / `WebFetch`
- 修工具选择、执行、权限、输出格式、最终答复收尾中的断链或假成功
- 把工具验收从“单点脚本”扩展为按类别分组的稳定门

最小验证
- 每类工具至少一条 `-p --allowedTools ...` 真探针
- 至少一条 `--bare` 交互探针
- 对应 crate tests

## Phase 4. Extended Surface Hardening

目标：
- 让非 builtin 能力也进入“默认可验收”状态

Modify
- `rust/crates/commands/**`
- `rust/crates/runtime/**`
- `rust/crates/plugins/**`
- `scripts/rust_tool_acceptance.sh`

动作
- 核对并修复：
  - skill
  - MCP
  - plugin
  - LSP
  - session / resume / compact / status / doctor / config
- 明确哪些能力需要 fixture，哪些需要 live service，哪些只能在条件满足时运行
- 为每一类能力增加稳定夹具或降级说明，避免 acceptance 假绿

最小验证
- skill fixture probe
- MCP fixture probe
- plugin probe
- LSP probe
- session / resume / compact / status / doctor / config probes

## Phase 5. No-Hang And Latency Closure

目标：
- 收掉“功能做完但前台卡住 / 退出不干净 / 子进程残留 / 明显卡顿”的问题

Modify
- `rust/crates/saicode-rust-cli/src/main.rs`
- `rust/crates/saicode-rust-cli/src/tui.rs`
- `rust/crates/runtime/**`
- `native/saicode-launcher/src/main.rs`
- `scripts/closeout_preflight.sh`
- `scripts/rust_tool_acceptance.sh`

动作
- 针对交互工具调用、流式输出、`/exit`、子进程生命周期做收尾治理
- 为“工具事件已发生但最终答复/退出卡住”补回归
- 增加最小性能门：
  - timeout 不超时
  - 交互收尾不残留进程
  - 基础工具调用 wall time 在可接受区间内

最小验证
- `closeout_preflight`
- `SAICODE_CLOSEOUT_LIVE=1 ./scripts/closeout_preflight.sh`
- `SAICODE_CLOSEOUT_ACCEPTANCE=1 ./scripts/closeout_preflight.sh`
- 进程残留检查

## Phase 6. Final Verification And Closeout

目标：
- 形成一套真正能复跑的非前端交付门

Modify
- `README.md`
- `README.en.md`
- `docs/closeout-workflow.md`
- 本轮 `SPEC.md`
- 本轮 `PLAN.md`

动作
- 复跑快速回归、全量 acceptance、live probes、性能/不卡顿门
- 同步文档与实际命令
- 若本轮有修复，确保对应测试、脚本、README 一起收口
- 最终保持 `git status --short` 干净

最终验证矩阵
- `cargo test --manifest-path native/saicode-launcher/Cargo.toml`
- `(cd rust && ../scripts/rust-cargo.sh test -q -p api -p runtime -p commands -p tools -p saicode-frontline -p saicode-rust-cli -p saicode-rust-one-shot -p saicode-rust-local-tools)`
- `cargo build --release --manifest-path native/saicode-launcher/Cargo.toml`
- `(cd rust && ../scripts/rust-cargo.sh build --release -q -p saicode-rust-cli -p saicode-frontline -p commands -p saicode-rust-local-tools -p saicode-rust-one-shot)`
- `./bin/saicode status`
- `./bin/saicode config show`
- `./bin/saicode -p 'Reply with exactly: ok'`
- 交互 `/help`
- 交互工具进度 probe
- skill / MCP / plugin / LSP probes
- `./scripts/rust_tool_acceptance.sh`
- `SAICODE_CLOSEOUT_LIVE=1 ./scripts/closeout_preflight.sh`
- `SAICODE_CLOSEOUT_LIVE=1 SAICODE_CLOSEOUT_ACCEPTANCE=1 ./scripts/closeout_preflight.sh`

## Plan Review

- 计划逐项映射 Spec：入口真相、builtin 工具、extended surface、不卡顿、closeout 都有对应阶段。
- 粒度可执行：每阶段都写了影响面和最小验证。
- 没混入前端扩张：所有步骤都限定在非前端代码和文档。

结论：Plan 可以进入实施。
