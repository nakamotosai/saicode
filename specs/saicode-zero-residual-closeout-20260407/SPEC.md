# saicode Zero Residual Closeout Spec

## Goal

把本机 `saicode + cliproxyapi` 收口到“所有验收面清零”的状态，不再以“主功能可用但仍有剩余边界”作为完成。

## Scope

1. `saicode` CLI 主链
2. `saicode-frontline` 快路径
3. `tools` / MCP / Skill / Task / Web 工具链
4. 验收 fixtures 与隔离 `HOME` 场景
5. `closeout_preflight.sh`
6. `rust_tool_acceptance.sh`
7. TTFT 基准链路的稳定性与断言

## Non-Goals

- 不引入新的 provider
- 不改变现有 CLI 表面命令名
- 不做与当前验收失败无关的大范围重构

## Facts

- OpenAI GPT 与 NVIDIA Qwen 当前主功能和 `Read` 工具已通过 live probe
- `rust_tool_acceptance.sh` 已被推进到全通过，但仍存在 `ttft_bench` 结果中出现 `ttft_seconds: null` 的不稳定观测
- 用户要求以“所有验收面都清零”为完成标准，不能再用“剩余边界”交付

## Constraints

- 直接修改本机 `/home/ubuntu/saicode`
- 修改后必须落到持久代码与脚本，不接受一次性手工步骤
- 完成前必须运行真实验收，而不是只跑单元测试

## Acceptance

1. `./bin/saicode status` 正常
2. GPT / Qwen 纯文本与 `Read` live probe 通过
3. 流式文本输出可见且稳定
4. `cargo test -p saicode-frontline` 通过
5. `cargo test -p tools` 通过
6. `cargo test -p saicode-rust-one-shot` 通过
7. `cargo test -p saicode-rust-cli` 通过
8. `/home/ubuntu/saicode/scripts/closeout_preflight.sh` 通过
9. `/home/ubuntu/saicode/scripts/rust_tool_acceptance.sh` 通过
10. `rust_tool_acceptance.sh` 中所有已定义场景均无失败项
11. `ttft_bench` 输出结构稳定，且不再出现未解释的 `ttft_seconds: null`

## Done Definition

- 所有验收脚本和测试面通过
- 无“剩余边界”作为交付说明
- 如仍有无法消除的限制，必须转成显式失败项继续修复，直到消失
