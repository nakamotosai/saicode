# Plan

## Spec Review

- 目标清楚：不是重复全量 closeout，而是直收当前唯一剩余边界。
- 范围清楚：只处理 provider/runtime/status/acceptance 这一条链路，不扩张到前端和无关功能。
- 验收可执行：每个收口点都要求 live probe、状态面展示、脚本覆盖和最终 git closeout。

结论：Spec 可以进入实施。

## Phase 1. Baseline Probe And Truth Capture

目标：
- 先确认 2026-04-09 当天真实的 provider 能力面，避免按旧印象设计

Inspect
- `./bin/saicode status`
- `./bin/saicode doctor`
- `./bin/saicode config show`
- `scripts/rust_tool_acceptance.sh`
- `scripts/closeout_preflight.sh`
- provider 相关 runtime 路由代码

动作
- 记录 qwen plain chat probe 是否稳定
- 记录 qwen tool-capable probe 是否稳定
- 记录 `gpt-5.4-mini` plain/tool probe 是否稳定
- 对比当前状态输出和实际路由是否一致

最小验证
- `./bin/saicode status`
- `./bin/saicode doctor`
- `./bin/saicode -p 'Reply with exactly: ok'`
- 至少一条 `--allowedTools Read` probe

## Phase 2. Collapse Hidden Routing Into Explicit Runtime Contract

目标：
- 把“代码里偷偷 fallback”改成“runtime 有明确定义的能力判定与状态合同”

Modify
- `rust/crates/saicode-rust-cli/src/main.rs`
- `rust/crates/saicode-rust-one-shot/src/main.rs`
- `rust/crates/saicode-frontline/src/recovery.rs`
- 必要时相关 runtime/provider 模块

动作
- 如果 qwen 已通过 plain/tool live probe：
  - 去掉对应 fallback
  - 统一 runtime 到 qwen
- 如果 qwen 仍未通过：
  - 收敛成显式 capability routing
  - 把 plain chat 与 tool-capable / recovery / one-shot 的有效模型选择抽成统一规则
  - 为回退记录健康原因、判定来源和可展示摘要

最小验证
- 代码级单测或集成测
- plain/tool 两类 probe 行为与规则一致

## Phase 3. Expose Runtime Truth To Users

目标：
- 让 CLI 用户无需看源码，也能知道当前到底怎么跑

Modify
- `./bin/saicode` 对应 Rust status/doctor 输出
- `native/saicode-launcher/src/main.rs`
- `rust/crates/saicode-rust-cli/src/main.rs`

动作
- 在 `status` 或 `doctor` 中补充：
  - configured model
  - effective plain model
  - effective tool-capable model
  - fallback / health summary
- 统一帮助文案，避免继续把“配置默认值”误读成“所有路径的执行真相”

最小验证
- `./bin/saicode status`
- `./bin/saicode doctor`
- 文案与真实 probe 结果一致

## Phase 4. Acceptance And No-Hang Guard

目标：
- 把这条边界纳入自动化门，避免以后再退回“靠 README 解释”

Modify
- `scripts/rust_tool_acceptance.sh`
- `scripts/closeout_preflight.sh`
- 相关测试

动作
- 新增或强化：
  - qwen plain chat probe
  - qwen tool-capable probe
  - fallback 触发与收尾 probe
  - 无 token / degraded / 卡住时的超时和失败判定
- 让 acceptance 输出能直接说明当前最终策略是“全 qwen”还是“健康感知 fallback”

最小验证
- `./scripts/rust_tool_acceptance.sh`
- `SAICODE_CLOSEOUT_LIVE=1 SAICODE_CLOSEOUT_ACCEPTANCE=1 ./scripts/closeout_preflight.sh`

## Phase 5. README Status Writeback And Git Closeout

目标：
- 把结果写回唯一真相源，并把本轮文档与代码一次提交干净

Modify
- `README.md`
- `README.en.md`
- 本轮 `SPEC.md`
- 本轮 `PLAN.md`

动作
- 回写当前最终状态：
  - 若 qwen 已统一直跑，则删除相应 fallback 边界口径
  - 若仍保留 fallback，则明确它已从“隐式残留”升级为“显式健康路由”
- 提交并推送到 `main`
- 保持 `git status --short` 为空

最小验证
- `git status --short`
- `git log -1 --oneline`
- `git push`

## Final Verification Matrix

- `cargo test --manifest-path native/saicode-launcher/Cargo.toml`
- `(cd rust && ../scripts/rust-cargo.sh test -q -p api -p runtime -p commands -p tools -p saicode-frontline -p saicode-rust-cli -p saicode-rust-one-shot -p saicode-rust-local-tools)`
- `./bin/saicode status`
- `./bin/saicode doctor`
- `./bin/saicode config show`
- `./bin/saicode -p 'Reply with exactly: ok'`
- `./bin/saicode -p --allowedTools Read -- 'Use Read to inspect README.md and reply with only the first line.'`
- `./scripts/rust_tool_acceptance.sh`
- `SAICODE_CLOSEOUT_LIVE=1 SAICODE_CLOSEOUT_ACCEPTANCE=1 ./scripts/closeout_preflight.sh`

## Plan Review

- 计划直接映射当前唯一剩余边界，没有重新扩成全仓库大扫除。
- 每阶段都绑定了文件落点和最小验证，不会变成只写方案不落地。
- Git closeout 被纳入正式阶段，满足 README / commit / push / clean 的仓库要求。

结论：Plan 可执行，且能覆盖“争取把剩余边界真正收掉，否则至少把它从隐式残留变成显式、可验收的健康路由”。
