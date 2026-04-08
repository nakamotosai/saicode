# claw-code donor reuse matrix for saicode

日期：
- 2026-04-07

donor 仓库：
- `https://github.com/ultraworkers/claw-code`

## 结论

不是“整个 claw-code Rust 工作区都能拿来复制”。

真正建议直接复制的只有两类：

1. 低产品耦合的 runtime enforcement 模块
2. parity harness 与测试夹具

## Direct copy

### Permission enforcement

文件：
- `rust/crates/runtime/src/permission_enforcer.rs`

建议：
- 直接复制进 `saicode` 对应 runtime 模块
- 保留测试
- 只改：
  - crate path
  - permission enum/path
  - tool name 别名映射

原因：
- 该实现责任边界清楚
- 几乎不绑定 donor 产品 surface
- 直接提升 `saicode` 当前权限执行质量

### Mock parity harness

文件：
- `rust/crates/rusty-claude-cli/tests/mock_parity_harness.rs`
- `rust/MOCK_PARITY_HARNESS.md`

建议：
- 直接复制场景组织方式
- 改成 `saicode` 的：
  - binary name
  - env names
  - provider
  - tool display names
  - config/session paths

原因：
- 这是现成的“真实行为回归闸门”
- 正好覆盖 `streaming / file tools / bash / permission / plugin tool`

## Copy skeleton only

### MCP bridge

文件：
- `rust/crates/runtime/src/mcp_tool_bridge.rs`

建议：
- 复制 registry 结构和测试夹具
- 重写：
  - runtime manager 接口层
  - tool exposure
  - event surface
  - auth wiring

原因：
- 架构值得借
- 但和本地 runtime 接线强相关

### Tool registry

文件：
- `rust/crates/tools/src/lib.rs`

建议：
- 不整段复制
- 只借：
  - `GlobalToolRegistry`
  - builtin/runtime/plugin 三层装配
  - allow-list normalization
  - permission spec assembly

原因：
- donor 文件太产品化
- 直接复制会把 `claw` 的工具面和 provider 语义一起带进来

### CLI composition

文件：
- `rust/crates/rusty-claude-cli/src/main.rs`

建议：
- 只借：
  - REPL loop 组织
  - runtime plugin state build
  - clean-env run/test 结构
- 不直接复制：
  - 参数面
  - help surface
  - top-level commands

## Reference only

### LSP client

文件：
- `rust/crates/runtime/src/lsp_client.rs`

结论：
- 只做接口参考
- 不作为最终 LSP donor

原因：
- 当前实现仍偏 registry facade
- 不是真正 language-server orchestration
- 不能满足 `saicode` 本轮“真 LSP”目标

## Recommended import order

1. `permission_enforcer.rs`
2. `mock_parity_harness.rs`
3. `mcp_tool_bridge.rs` skeleton
4. `tools/src/lib.rs` registry pattern
5. donor LSP 仅做接口参考
