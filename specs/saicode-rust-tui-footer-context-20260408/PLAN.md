# saicode Rust TUI Footer / Context Plan

## Execution Plan

1. 修 bridge 状态真相源
   - 让 `/model` 切换后重新加载当前 `RuntimeSurface`
   - 让 `session_updated` 回传真实的 model / wire / provider / profile

2. 补会话上下文字段
   - 在 bridge session payload 中加入 estimated context tokens、窗口大小、占比、compaction 次数等字段

3. 改 auto compact 触发条件
   - 从旧的累计 input token 阈值切到基于会话估算 token 的 80%/270000 阈值
   - turn 完成后把 auto compact 结果透传到前台

4. 收紧 compact 摘要内容
   - timeline 以 user/assistant 文字信息为主
   - tool/code 操作降噪为摘要化占位，不保留大段细节

5. 改 TUI footer
   - footer 改为多行
   - 展示 model / wire / provider / usage / context / compaction 状态
   - 处理高占比颜色提醒

6. 验证
   - 相关单测
   - `cargo test -p runtime -p saicode-rust-cli`
   - `cargo build -p saicode-rust-cli`
