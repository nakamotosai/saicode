# saicode TS Frontend + Rust Backend Recovery

## Goal

恢复并保留 TypeScript 前端交互层作为 `saicode` 默认入口，同时保留 Rust `saicode` 作为后端执行内核与命令面。

## Scope

- 恢复被误删的旧 TS 前端启动链路与所需源码。
- 保持 `saicode` 无参数默认进入 TS 前端。
- 保持带参数命令可继续走 Rust CLI / native launcher。
- 修复当前仅显示占位文案的前端壳问题。
- 完成最小可运行验证。

## Non-Goals

- 本轮不把全部 TS 业务逻辑完全迁移到 Rust。
- 本轮不重做 UI 设计。
- 本轮不改动已通过验收的 Rust provider / MCP / skill / grep / streaming 后端能力。

## Constraints

- 用户真实目标是“TS 前端 + Rust 后端”，不是“删除 TS 前端”。
- 不回滚无关改动。
- 交付必须有启动与命令面验证。

## Acceptance

1. `saicode` 默认进入可用的 TS 前端，而不是占位壳或 Rust 前端。
2. `saicode --help` 仍可正常输出帮助。
3. `saicode status` 仍可走 Rust 命令面。
4. 仓库中不再保留把 TS 前端当成“UI-only placeholder”的错误入口实现。
