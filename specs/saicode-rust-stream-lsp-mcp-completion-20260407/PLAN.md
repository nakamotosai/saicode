# saicode Rust 流式输出 / LSP / 工具契约 / 插件与 MCP 完整化 Plan

日期：
- 2026-04-07

## Strategy

执行顺序固定为：

1. 先收口工具契约真相源
2. 再补流式输出
3. 再补 LSP 真实现
4. 再补 plugin / MCP 真注入与真执行
5. 最后统一做帮助文案、脚本、验收与 cutover 清理

原因：

- tool contract 是其它三条链路的共同底座
- streaming 是最直接的用户缺陷
- LSP 与 plugin/MCP 都依赖稳定的 tool/runtime/event surface

## Donor Reuse Matrix

### Direct copy first

优先直接引入 `claw-code` 的以下实现：

1. `permission_enforcer.rs`
2. `mock_parity_harness.rs`
3. `MOCK_PARITY_HARNESS.md` 的场景组织方式

要求：

- 先保留 donor 测试
- 再做 `saicode` 命名与接线适配
- 不要先重写再猜等价性

### Copy skeleton, then adapt

以下 donor 只复制骨架：

1. `mcp_tool_bridge.rs`
2. `tools/src/lib.rs` 中的 registry / normalization 组织方式
3. `rusty-claude-cli/src/main.rs` 中的 runtime plugin state / clean-env harness 装配方式

要求：

- 不直接继承 donor 的 CLI 外观、tool names、provider 语义
- 只保留结构和测试组织法

### Reference only

`lsp_client.rs` 只做接口参考，不作为最终 LSP 实现 donor。

## Phase 1: Tool Contract Unification

目标：
- 建单一工具定义真相源，消除 schema / help / dispatch / prompt 漂移

修改面：
- `rust/crates/tools`
- `rust/crates/saicode-rust-cli`
- `rust/crates/commands`
- `native/saicode-launcher`（仅当帮助页受影响）

动作：
- 抽出统一 `ToolSpec` / `DisplayToolSpec` 装配层
- 借 `claw-code` `GlobalToolRegistry` 的装配结构重写本地 registry
- 让以下内容从同一份定义生成：
  - model-facing tool definitions
  - CLI 帮助文本
  - allowed/disallowed tool normalization
  - permission requirements
  - display/canonical 映射
- 删除临时硬编码列表，尤其是：
  - `TOOL_OPTION_NAMES`
  - prompt guidance 中手写的工具名清单
  - 与 dispatch 不一致的 schema

验证：
- 单元测试：schema 与 dispatcher 枚举一致
- 集成测试：`--allowedTools Read`、`--allowedTools LSP`、`--allowedTools MCP` 均按新契约生效

## Phase 2: Streaming Completion

目标：
- 让 Rust CLI 真正提供可用流式输出

修改面：
- `rust/crates/api`
- `rust/crates/runtime`
- `rust/crates/saicode-rust-cli`
- `bin/saicode`
- `native/saicode-launcher`

动作：
- 扩展 Rust CLI `OutputFormat` 到 `text | json | stream-json`
- 参考 `claw-code` CLI 的事件流处理和 clean-env 测试组织方式
- 增加对应参数解析、帮助文案、错误校验
- 建立 NDJSON 事件协议：
  - message_start
  - content delta
  - tool_start / tool_progress / tool_result
  - permission_request
  - session_state
  - final_message
  - error
- interactive 模式增加渐进输出，不再等完整总结后一次性 `println!`
- 明确区分：
  - `stream-json` 给 SDK / 上层宿主
  - interactive streaming 给终端用户

验证：
- `./bin/saicode -p --output-format stream-json --verbose "Reply with exactly: ok"`
- 至少一个工具调用过程的增量事件探针
- 非流式 print 仍保持兼容
- 引入 donor parity harness 的 `streaming_text` 场景并改写为 `saicode` 版本

## Phase 3: LSP True Runtime

目标：
- 用真实 language-server manager 替换 fallback grep

修改面：
- `rust/crates/tools`
- `rust/crates/runtime`
- 可能新增 `rust/crates/lsp` 或在 runtime 下建 `lsp` 模块

动作：
- 设计统一 LSP input schema：
  - `operation`
  - `file_path`
  - `line`
  - `character`
  - 可选 `query`
- 对齐支持操作集合
- 仅参考 donor `lsp_client.rs` 的 action enum / result structs
- 实现：
  - server discovery / registration
  - didOpen / didChange / didSave / didClose
  - request timeout / retry / startup readiness
  - 错误格式化
- 删除当前 fallback search 的“伪 LSP”描述
- 若保留 fallback，必须明确叫 `symbol search fallback`，不能继续叫 LSP

验证：
- 以 Rust 项目自身或一个最小 fixture 做 definition / references / hover 探针
- LSP 不可用时，报错清楚且无内部泄漏

## Phase 4: Plugin and MCP Completion

目标：
- 让 plugin / MCP 不只是“能管理”，而是能进入默认会话执行面

修改面：
- `rust/crates/plugins`
- `rust/crates/runtime`
- `rust/crates/tools`
- `rust/crates/saicode-rust-cli`

动作：
- plugin：
  - 统一 plugin manifest 解析
  - 注入 plugin tools 到 tool pool
  - hooks / permissions / lifecycle 接入会话主链
- MCP：
  - 以 donor `mcp_tool_bridge.rs` 为骨架重建本地 bridge
  - 收口 stdio / SSE / WebSocket transport 接入面
  - tool 注入统一走 tool pool
  - resources / auth / reconnect / timeout 策略统一
- 在事件层区分：
  - 加载失败
  - auth required
  - tool 调用失败
  - server transport 失败

验证：
- 一个 bundled plugin tool 探针
- 一个 stdio MCP server 探针
- 若仓库已有 remote MCP transport 基础，再补一条 remote transport 探针
- 引入 donor parity harness 的 `plugin_tool_roundtrip` 场景并改写为 `saicode` 版本

## Phase 5: Surface Cleanup and Cutover Gate

目标：
- 所有对外暴露面与真实能力一致

修改面：
- `README.md`
- `README.en.md`
- `scripts/closeout_preflight.sh`
- `scripts/rust_tool_acceptance.sh`
- `bin/saicode`
- `bin/saicode` 单入口包装与验收面

动作：
- 帮助文案与参数校验统一
- 清理已不成立的脚本假设
- 增加四类验收门：
  - streaming
  - LSP
  - tool contract
  - plugin/MCP

验证：
- `closeout_preflight`
- 一组真实探针报告

## Test Matrix

### Streaming

- text print
- json print
- stream-json print
- interactive incremental output
- stream + tool call

### LSP

- definition
- references
- hover
- document symbols
- unavailable server

### Tool contract

- display/canonical mapping
- allowed/disallowed filtering
- permission mode mapping
- help/schema/dispatch 一致性

### Plugin / MCP

- plugin discovery
- plugin tool injection
- stdio MCP
- resource list / read
- auth required / auth success / auth failure
- plugin tool roundtrip parity scenario

## Donor Import Work Order

1. 先导入 `permission_enforcer.rs`
2. 再导入并改写 mock parity harness
3. 再用 `mcp_tool_bridge.rs` 骨架重建本地 MCP bridge
4. 最后才推进 LSP 真 runtime

原因：

- 这条顺序能先用 donor 的低耦合资产把验证门和权限门建起来
- 避免一开始就陷入 LSP 复杂度

## Delivery Sequence

推荐分三轮交付，而不是一锅炖：

1. Round 1
   - Tool contract
   - Streaming
2. Round 2
   - LSP true runtime
3. Round 3
   - Plugin / MCP completion
   - Surface cleanup

## Exit Criteria

- 默认 Rust CLI 对外不再存在“宣称支持但实际失败”的主路径
- 用户能看到真实流式过程
- LSP 不再是假实现
- plugin / MCP 能真正进入默认会话
- 所有关键能力有命令级验收
