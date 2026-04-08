# SPEC

## Goal
修复 saicode Rust TUI 的三类前台问题：
1. 文本无法选中。
2. `rust backend working…` 过于空泛，看不到真实执行阶段。
3. 普通 prompt（如写诗）长时间卡住，最终失败。

## Scope
- Rust TUI 事件与交互。
- Rust system prompt 构建与体积控制。
- 真实 `./bin/saicode` 验证。

## Constraints
- 保留全屏 TUI 默认入口。
- 不引入会改变用户主交互范式的绕行方案。
- 不回退 UTF-8 输入修复。

## Acceptance
- `./bin/saicode -p "Reply with exactly: ok" --output-format stream-json` 在合理时间内返回，不再因为超大 prompt 失败。
- TUI 在首个文本 delta 之前能显示 1-2 行动态状态，而不是只显示固定 `rust backend working…`。
- TUI 鼠标/选择行为不再像现在这样完全锁死；至少提供明确、可用的文字选择路径，并保留历史回看能力。
