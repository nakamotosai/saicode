# saicode Zero Residual Closeout Plan

## Phase 1

Inspect:
- 当前 `ttft_bench` 逻辑
- `rust_tool_acceptance.sh` 现状
- TTFT 相关调用链与输出来源

Verify:
- 明确最后未清零项的真实根因

## Phase 2

Modify:
- TTFT 基准链路
- 相关工具 / CLI / 脚本断言

Verify:
- 单点 live probe
- 针对性测试

## Phase 3

Verify:
- `cargo test -p saicode-frontline`
- `cargo test -p tools`
- `cargo test -p saicode-rust-one-shot`
- `cargo test -p saicode-rust-cli`
- `closeout_preflight.sh`
- `rust_tool_acceptance.sh`

## Exit Gate

- 所有验收面通过
- 无剩余边界
