# saicode Rust Skill/MCP/Tool Surface Closeout Spec

## Goal

完成 saicode Rust 主运行时的五项收口能力，使默认会话可直接使用技能、MCP、统一工具面、同步 launcher/help，并具备覆盖流式输出、LSP、基础搜索工具的验收链路。

## Scope

1. Skill runtime
2. MCP bridge into default tool pool
3. Unified builtin/plugin/MCP tool assembly
4. Native launcher/help sync
5. Acceptance and closeout coverage

## Non-Goals

- 不恢复 TS 后端运行时
- 不在本轮重做前端 TS 界面
- 不扩展到与这五项无关的新工具能力

## Constraints

- 以 Rust 主运行时为唯一后端真相源
- 兼容当前已有 builtin、plugin、LSP、stream-json 行为
- 不回退用户现有脏工作区中的其他改动
- 验收必须覆盖基础工具可用性，至少包含 `Read`、`Grep`/搜索、流式输出、LSP、Skill、MCP

## Acceptance

1. `Skill` 工具不再只是返回原始文件内容，能够返回执行所需的结构化技能元数据，至少包括：
   - skill 文件路径
   - 描述
   - prompt
   - 相邻脚本入口
   - 相邻资源/模板索引
   - 相对路径解析结果
2. 默认会话工具池能够自动注入 MCP server 暴露的工具，模型可直接调用动态 MCP 工具名，而不是只能经由泛化 `MCP` 工具间接调用。
3. builtin / plugin / MCP 三类工具在默认工具面、权限规则、显示名映射、执行路由上保持一致，不出现一处可见一处不可执行的漂移。
4. native launcher 的快速帮助文本与当前 Rust CLI 对齐，帮助文案和关键路由假设不再过时。
5. closeout 脚本与 acceptance 脚本能覆盖并验证：
   - `stream-json`
   - 交互式工具过程输出
   - LSP
   - skill runtime
   - MCP bridge
   - launcher/help
   - 基础读取/搜索类工具

## Done Definition

- 代码实现完成
- 相关自动化测试通过
- 至少一轮脚本级 closeout / acceptance 验证通过
