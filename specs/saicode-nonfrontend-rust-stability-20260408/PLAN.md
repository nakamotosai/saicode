# Plan

## Phase 1. Baseline Inventory

- 盘点 `rust/` workspace、`native/` launcher、`bin/` wrapper、脚本与文档
- 标出当前 Rust cutover 的真实边界，以及仍可能依赖旧 TS/Bun 的位置
- 记录现有 worktree 与 README 口径是否一致

## Phase 2. Verification Sweep

- 跑最小必要的 build/test：
  - launcher
  - Rust workspace 关键 crate
- 跑真实命令探针：
  - `./bin/saicode --help`
  - `./bin/saicode status`
  - 非交互 `-p`
  - 必要时补内部会话链路或工具脚本 smoke
- 记录每项验证的通过、失败与失败位置

## Phase 3. Review And Risk Classification

- 审查核心实现：
  - 命令路由
  - runtime/session
  - local tools
  - wrapper 与 release/debug 路径
  - 测试和文档是否失真
- 输出阻塞性 bug、潜在风险、覆盖缺口和文档偏差

## Phase 4. Closeout

- 对可控阻塞问题直接修复
- 回写本轮 `SPEC.md` / `PLAN.md` 和必要文档
- 复跑对应验证
- 最终按“结果 -> 验证 -> 风险/边界”收口
