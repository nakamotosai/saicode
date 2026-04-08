# saicode + cliproxyapi Provider Repair Spec

## Goal

修复本机 `saicode` 与 `cliproxyapi` 联动链路，使以下模型在 `saicode` 中可稳定使用：

- OpenAI GPT 路线：`gpt-5.4`、`gpt-5.4-mini`
- NVIDIA Qwen 路线：`qwen/qwen3.5-122b-a10b`、`qwen/qwen3.5-397b-a17b`

并移除当前不再需要的 `opencode zen` OAuth 聚合面。

## Scope

1. `cliproxyapi` 配置清理
2. NVIDIA key 健康排查与坏 key 剔除
3. `saicode` provider API 适配
4. `saicode` 上游异常识别与可用性探测
5. 端到端验收

## Non-Goals

- 不引入新的第三方 provider
- 不保留 `opencode zen`
- 不改变 `saicode` 的 CLI 外观

## Facts

- 本机 `cliproxyapi` 容器当前为 `cliproxyapi-local:qwen-oauth-fix`
- `cliproxyapi` 当前 `openai-responses` 面存在汇总异常：
  - 流式 delta 有正文
  - 最终 response 对象可能 `output: []`
- `openai-chat-completions` 流式面已验证对 GPT 与 Qwen 均可返回正文或 `tool_calls`
- NVIDIA Qwen 当前轮询池中至少存在一把会触发 `DEGRADED function cannot be invoked` 的坏 key

## Constraints

- 直接在本机同时修改 `saicode` 与 `cliproxyapi`
- 修改后必须落到持久配置，不依赖一次性环境变量
- 交付前必须完成 live probe

## Acceptance

1. `/opt/cliproxyapi/config.yaml` 中不再包含 `opencode-zen`
2. `cliproxyapi` 仅保留 OpenAI 与 NVIDIA 路线，且 NVIDIA 坏 key 被剔除
3. `saicode status` 显示的 `Provider API` 为可用面，且对应实测可工作
4. 以下 live probe 通过：
   - `./bin/saicode --model cpa/gpt-5.4-mini -p 'Reply with exactly: ok'`
   - `./bin/saicode --model cpa/gpt-5.4-mini -p --allowedTools Read -- 'Use Read to inspect README.md and reply with only the first line.'`
   - `./bin/saicode --model cpa/qwen/qwen3.5-122b-a10b -p 'Reply with exactly: ok'`
   - `./bin/saicode --model cpa/qwen/qwen3.5-122b-a10b -p --allowedTools Read -- 'Use Read to inspect README.md and reply with only the first line.'`
5. `rust_tool_acceptance.sh` 默认不再落到已知坏模型/API 组合上

## Done Definition

- 配置已更新并重启生效
- `saicode` 代码已适配
- live probe 通过
