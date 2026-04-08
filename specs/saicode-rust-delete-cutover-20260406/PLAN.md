# saicode Rust delete-cutover plan

日期：
- 2026-04-06

步骤：
1. 写任务级 Spec / Plan
   - 固定 6 步目标、范围、约束、验收。

2. 改 repo-root 与版本源
   - launcher 改用 Rust 锚点识别 repo root。
   - launcher 版本源不再读取 `package.json`。

3. 切断 Bun fallback
   - `bin/saicode` 只允许 native launcher / rust full cli。
   - native launcher 删除 `hand_off_to_bun()` 与 TS entrypoint 映射。
   - `warm_headless` 改为 Rust 路径或整条路线下线。

4. 更新脚本与验收流
   - closeout / acceptance 脚本改成 Rust-only 前提。
   - 增加“临时移走 TS runtime 文件”的无影子验收。

5. 跑无 TS 影子验收
   - 临时移走 `src/`、`preload.ts`、`bin/saicode-bun`。
   - 重跑 `help/status/doctor/profile/repl/read/bash/web/task/ttft-bench`。

6. 删除旧 runtime 主链并复测
   - 删除 TS/Bun runtime 入口文件。
   - 重跑关键验收并回写结论。
