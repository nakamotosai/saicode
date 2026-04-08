# saicode + cliproxyapi Provider Repair Plan

## Phase 1

Create:
- `SPEC.md`
- `PLAN.md`

Verify:
- 明确目标、范围、验收

## Phase 2

Inspect:
- `/opt/cliproxyapi/config.yaml`
- `cliproxyapi` 容器与日志
- 单 key NVIDIA 可用性

Modify:
- 清理 `opencode zen`
- 剔除坏的 NVIDIA key

Verify:
- 直接对 `cliproxyapi` 做 `curl` live probe

## Phase 3

Modify:
- `/home/ubuntu/.saicode/config.json`
- 如有必要，`/home/ubuntu/saicode` 下 provider 适配代码与验收脚本

Work:
- 默认切到 `openai-chat-completions`
- 识别空响应/坏上游
- 避免 acceptance 默认撞上坏组合

Verify:
- `./bin/saicode status`
- `./bin/saicode -p ...`

## Phase 4

Modify:
- `/home/ubuntu/saicode/scripts/rust_tool_acceptance.sh`
- 其他相关验证脚本

Verify:
- GPT live probe
- Qwen live probe
- tool-calling live probe
- acceptance 脚本
