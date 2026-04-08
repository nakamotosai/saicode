# saicode 全 Rust TUI Cutover Plan

## Plan Summary

本计划按“先做出可用 Rust TUI，再切默认入口，最后清理 TS 主链”的顺序推进。  
原则是：不再围绕 TS 前端修补，而是一次把最终目标态改对。

## Execution Plan

1. 建立 Rust TUI 前端骨架
   - 在 Rust workspace 中新增或启用独立 TUI 模块
   - 明确 app state、input state、dialog state、event queue
   - 先跑通基础布局：header / transcript / input / footer

2. 接入现有 Rust backend 事件流
   - 复用现有 frontend-bridge / conversation event 面
   - 把 `session_started`、`session_updated`、`content_delta`、`final_message`、`tool_*`、`permission_*` 映射到 TUI state
   - 去掉对 TS frontend 的依赖

3. 做出新的 Rust 前台视觉
   - 顶部改为轻量彩色 `saicode`
   - 底部状态栏展示 model / profile / permission / workspace
   - transcript 按 user / assistant / tool / system / error 上色
   - 不再使用大 `SAI` 三字块

4. 补齐流式与工具可视化
   - assistant delta 实时刷新
   - tool start/result/error 独立渲染
   - grep / search / bash / MCP 有过程反馈
   - 保证前台不会再次退化成整段静默后一次落结果

5. 实现通用 dialog/picker 基础设施
   - 建一个统一的 modal / picker 组件
   - 键盘支持：上下、回车、Esc、Tab
   - 后续 `/model`、`/permissions`、`/mcp` 复用同一套机制

6. 完成 `/model` 交互式切换
   - 候选来源先使用本地 model catalog
   - 前台输入 `/model` 时直接打开 picker，而不是打印 `model = xxx`
   - 选择后发出 `/model <name>` 或等价内部动作
   - 切换成功后刷新头部与状态栏

7. 补齐权限与能力面板
   - permission request dialog
   - slash command 输出面板
   - MCP / skills / LSP / plugins 至少具备可见状态与结果展示

8. 切默认入口并清理过渡物
   - `bin/saicode` 默认无参数进入 Rust TUI
   - TS frontend 从默认路由移除
   - 更新 `package.json` / README / 验证脚本
   - 视完成度决定 `frontend/` 是归档还是删除

## Verification Gates

每一阶段结束都必须过对应门禁：

1. Rust TUI 启动门禁
   - `saicode` 可进入 Rust 前端
   - 无闪退、无空白、无重复刷屏

2. 前台体验门禁
   - 有彩色 `saicode`
   - transcript / input / footer 布局稳定

3. 流式门禁
   - 普通 prompt 能看到增量输出

4. 工具门禁
   - grep/search 至少一条真实调用在前台可见

5. 模型切换门禁
   - `/model` 打开 picker
   - 可选、可确认、切换后生效

6. 权限门禁
   - permission request 可在前台交互完成

7. 能力门禁
   - `/status` `/mcp` `/skills` `/doctor` 至少做一轮真实前台验证

8. Cutover 门禁
   - 默认入口已改为 Rust TUI
   - TS frontend 不再是主链

## Risks

1. Rust backend 现有 slash surface 与 TUI picker 行为不完全匹配
   - 解法：TUI 先在前端侧拦截 `/model`，内部仍调用现有 Rust 命令面或直接改状态

2. 事件模型偏桥接导向，不够 TUI 原生
   - 解法：先兼容现有事件，后做内部统一 state/event 重构

3. 再次出现“能用但很难用”的前端回退
   - 解法：把 `/model picker`、流式、工具过程、权限弹层列为硬验收项

4. TS 退场过早导致应急路径丢失
   - 解法：在 Rust TUI 验收通过前，保留 TS 过渡物但不再扩展其能力

## Done Definition

以下全部成立才算本计划完成：

1. 默认入口是 Rust TUI。
2. 用户打开后看到的是彩色 `saicode` Rust 前端。
3. `/model` 是候选列表交互，而不是纯文本回显。
4. 流式、工具、权限、MCP/skills/LSP 的关键前台能力都能用。
5. TS frontend 已退出主链。
