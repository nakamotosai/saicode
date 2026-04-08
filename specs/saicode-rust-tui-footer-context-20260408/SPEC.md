# saicode Rust TUI Footer / Context Spec

日期：
- 2026-04-08

## Goal

修复 Rust TUI 当前三个直接影响使用的问题：

1. `/model` 切换后，底部状态栏必须立即显示新模型与对应 wire/provider 状态。
2. 底部状态栏不能再被限制为一行，必须能完整显示核心运行状态。
3. TUI 必须显示当前会话上下文占用情况，并在达到 270000 上下文窗口的 80% 时自动 compact。

## Scope

### In scope

- Rust bridge 会话状态回传修复
- Rust TUI footer 多行状态展示
- 当前会话上下文 token 估算与百分比展示
- 80% 阈值自动 compact
- compact 摘要改为偏“文字关键信息保留”，减少代码操作/工具细节残留
- 最小回归测试与构建验收

### Out of scope

- 改 provider 计费逻辑
- 引入模型级不同上下文窗口
- 改动 cliproxyapi

## Constraints

1. 这轮以纯 Rust 实现收口，不引入 TS。
2. 上下文窗口统一按 270000 计算。
3. 自动 compact 阈值固定为 80%。
4. compact 后要尽量保留用户目标、文字类要求、待办、关键文件，不保留大段代码/工具操作细节。

## Acceptance

1. 在 Rust TUI 内执行 `/model` 后，footer 中 `model` / `wire` / `provider` 状态立即更新。
2. footer 至少支持多行显示，不再截断核心状态信息。
3. footer 显示 `当前上下文 tokens / 270000 / 百分比 / compact 阈值`。
4. 会话达到 80% 阈值后自动 compact，且前台能看到 compact 已触发。
5. compact 后摘要仍保留用户文字目标、待办、关键文件等信息，但不再塞入大段工具输入输出或代码操作细节。
6. 相关 Rust 测试通过，`saicode-rust-cli` 能成功构建。
