# PLAN

1. 修 system prompt 体积
- 检查 project context 中 git status/diff 注入逻辑。
- 改为摘要/截断，避免脏仓库把整份 diff 注入请求。
- 用真实 `./bin/saicode -p` 验证首包和成功返回。

2. 补 bridge/TUI 进度事件
- 找到 turn 执行入口和可观察阶段。
- 在模型请求开始、等待首包、工具开始/结束等阶段发短状态。
- TUI 显示最近 1-2 行动态状态，并在新事件到来时更新。

3. 修文字选择/鼠标交互
- 重新设计 mouse capture 行为，避免默认把选择彻底吞掉。
- 保留回看历史能力，同时给出可操作的选择模式或默认可选行为。
- 做真实 TTY smoke 验证。
