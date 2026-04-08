# saicode 全 Rust TUI Cutover Spec

日期：
- 2026-04-08

## Goal

把 `saicode` 的最终形态重新收敛为：

- `Rust 前端 + Rust 后端`
- `Rust 成为唯一主运行时`
- `saicode` 默认入口进入 Rust TUI，而不是 Bun/TS 前端
- 新 Rust 前端保留“好用、可见过程、可交互选择”的体验，不退回到简陋文本壳

这次不再以“保留 TS 前端”为目标态。当前 TS frontend bridge 仅视为过渡物，用于在 Rust TUI 完成前维持可用性。

## Current State

当前仓库的真实状态：

1. Rust backend 主链已经打通：
   - session
   - provider 调用
   - streaming
   - tools
   - grep
   - slash command
   - permission prompt bridge
   - MCP / skill / doctor / config 等命令面
2. 默认入口目前仍会在无参数时进入 TS frontend bridge。
3. Rust 现有前台仍是较粗糙的 readline 风格，不满足用户对交互体验的要求。
4. 用户新的最终要求已经改变：
   - 不再保留 TS 前端
   - 要 Rust 前端也具备可用、彩色、可交互的体验
   - `/model` 不能只是打印当前值，必须能在前端里弹出候选列表供选择

## Product Direction

新的目标不是“复刻旧 TS 全量内部实现”，而是：

- 用 Rust 做出一个新的终端前端
- 保留用户真正关心的体验面
- 删除 TS 在主链上的职责

用户明确关心的体验面包括：

1. 顶部用较小但明显的彩色 `saicode` 字样，而不是 `SAI` 三个大块字。
2. 打开后能看到前台过程，而不是整段静默后一次性落结果。
3. `/model` 要弹出候选列表并支持键盘选择。
4. 工具调用、grep、搜索、权限请求要在前台可见。
5. MCP、skill、LSP、插件等能力仍然要能在 Rust 主链里用。

## Scope

### In scope

- Rust TUI 前端主框架
- Rust 事件循环、输入态、UI 状态机
- Rust 版 transcript / tool event / status line / prompt 区
- Rust 版 `/model` picker
- Rust 版权限请求面板
- Rust 版 slash command 交互层
- Rust 版流式输出呈现
- Rust 版 MCP / skill / LSP / plugin 入口可见化
- `bin/saicode` 默认入口切换到 Rust TUI
- TS frontend 退场与清理

### Out of scope

- 继续维护 TS frontend 作为长期入口
- 恢复旧 `src/main.tsx` 运行时
- 在本轮把所有历史 TS 参考源码立即全部删除干净
  - 允许先退主链、后归档/删除
- 改动 `cliproxyapi` / `cpa` 的 provider 语义

## Non-Goals

以下不作为本轮目标：

1. 不追求和旧 TS 前端 100% 像素级一致。
2. 不把“有 UI”误当成完成，必须做到可实际使用。
3. 不保留双前端长期共存。
4. 不接受 Rust 前端再次退化成只有一行提示和输入框的简陋壳。

## Constraints

1. 用户要的是全 Rust 最终态。
2. 默认要保留完整前台可见过程。
3. `cliproxyapi/cpa` 和 NVIDIA/Qwen 可调用能力不能回退。
4. 已经打通的 Rust backend 能力不能因为重做前端而丢失。
5. 迁移必须允许过渡，但最终默认入口必须切到 Rust TUI。
6. 交付不能只靠代码分析，必须走真实前台验收。

## Target UX

Rust TUI 完成态至少要包含：

### 1. Header

- 一行或两行轻量彩色 `saicode`
- 当前 model / profile / permission / workspace 状态
- 不再使用现在这个 `SAI` 三个大字 logo

### 2. Transcript

- 用户消息
- assistant 流式消息
- tool start / result / error
- system / slash output
- permission request / resolved

### 3. Input

- 常规 prompt 输入
- slash 命令输入
- 上下历史
- Esc 清空或关闭弹层

### 4. Pickers / Dialogs

- `/model` 触发模型选择器
- 后续可扩展 `/permissions`、`/mcp`、`/skills`、`/agents`

### 5. Runtime Feedback

- streaming 中明确可见“正在生成”
- tool 执行期间有前台事件
- permission request 有明确等待状态
- 错误在前台单独渲染，不埋在 stderr

## Architecture Decision

### Final shape

- `bin/saicode`
  - 无参数默认进入 Rust TUI
  - 带 `--help`、`status`、`doctor`、`-p` 等命令继续走 Rust CLI/command surfaces
- Rust TUI 直接复用已有 Rust backend/runtime 能力
- 统一使用 Rust 内部事件模型，不再依赖 TS bridge UI

### Transitional shape

在 Rust TUI 完成前，当前 TS bridge 可继续存在，但只作为短期过渡物，不再被视为目标态。

## Implementation Principle

优先采用：

1. 现有 Rust CLI / session / tool / permission / MCP 主链
2. donor 中已验证过的 Rust TUI 结构与 model picker 思路
3. `saicode` 自己当前已经打通的 backend event 面

不要采用：

1. 再补一层更复杂的 TS 前端
2. 继续把 UI 能力放在 TS、把执行能力放在 Rust 的长期双栈结构
3. 把 `/model` 做成仅打印文本提示而无选择器

## Required Deliverables

1. Rust TUI 前端入口与状态机
2. Rust 版彩色 header
3. Rust 版 transcript renderer
4. Rust 版 prompt input
5. Rust 版 `/model` picker
6. Rust 版 permission dialog
7. Rust 版 tool / grep / search / streaming 可视化
8. 默认入口切换与 TS frontend 退主链

## Eight Stages

### Stage 1: Runtime Inventory

- 梳理现有 Rust backend 事件和前台需求
- 确认 TUI 所需状态模型

### Stage 2: Rust TUI Shell

- 建立 Rust 前端 app / event loop / layout
- 做出基础 header + transcript + input 三段布局

### Stage 3: Streaming Transcript

- 接入 `content_delta`
- 接入 final message
- 接入 usage / prompt cache / system output

### Stage 4: Tool Surface

- 接入 tool start/result/error
- 让 grep / search / bash / MCP 工具在前台有可见过程

### Stage 5: Interactive Pickers

- `/model` picker
- 后续通用 picker/dialog 基础设施

### Stage 6: Permission / MCP / LSP Surface

- permission request dialog
- MCP / skill / LSP / plugin 相关 slash surface 与状态面板

### Stage 7: Default Cutover

- `bin/saicode` 默认改进 Rust TUI
- TS frontend 退为备用或直接移除默认路由

### Stage 8: Closeout

- 删除或归档 TS frontend 主链
- 文档与脚本收口
- 完成真实前台验收

## Acceptance

必须同时满足以下条件才算完成：

1. `saicode` 无参数默认进入 Rust TUI。
2. 打开后能看到彩色 `saicode` header，而不是大 `SAI` 三字块，也不是简陋文本壳。
3. 普通 prompt 能流式显示。
4. tool / grep / 搜索过程前台可见。
5. `/model` 会弹出候选列表，可用上下键与回车选择。
6. 模型切换后真实生效，并在状态栏/头部可见。
7. permission request 有前台交互，不需要盲输。
8. `/status`、`/mcp`、`/skills`、`/doctor` 等命令在 Rust 主链仍可工作。
9. `frontend/` TS 主入口不再承担默认运行时职责。
10. 至少完成一次真实 TTY 验收，而不是只靠单元测试或日志判断。

## Exit Criteria

只有在以下条件成立时，才允许宣布“回到全 Rust 版本”：

- 默认入口已切到 Rust TUI
- 用户关键体验面已补齐
- TS frontend 已退主链
- 核心验收面都已真实验证通过
