# saicode `@ts-nocheck` 清零 Plan

## Spec Review

- 目标明确：
  - 清零当前 `227` 个 `@ts-nocheck` 文件
- 范围明确：
  - 只处理移除 `@ts-nocheck` 必须联动的类型修复
- 非目标明确：
  - 不借题发挥做无关重构
- 验收明确：
  - 文件数归零
  - `typecheck/test/verify` 通过

结论：
- Spec 可执行，进入实施。

## Phase 0 - Inventory

- 固定当前 `227` 文件清单
- 按目录和复杂度分波次
- 建立专项 `REPORT.md`

最小验证：
- inventory 文件与 `rg -l '@ts-nocheck'` 实际结果一致

## Phase 1 - 低风险叶子批

目标：
- 先清掉最小、低耦合、以类型声明/简单工具函数/轻量 UI 为主的文件

优先面：
- `src/skills`
- `src/types`
- `src/tasks/types`
- `src/utils/secureStorage`
- `src/utils/computerUse/*` 中 loader 类
- 小型 hooks / 小型组件 / 小型 util

最小验证：
- 每批移除后跑 `bun run typecheck`

## Phase 2 - 中小型批

目标：
- 清掉中等复杂度 hooks / components / tools / services

优先面：
- `< 220` 行的 `src/components`
- `< 220` 行的 `src/services`
- `< 220` 行的 `src/tools`
- `< 220` 行的 `src/utils`

最小验证：
- 每完成一组跑 `bun run typecheck`
- 关键 UI/工具改动补 `bun test`

## Phase 3 - 重区批

目标：
- 攻克大文件与核心耦合区

重点面：
- `src/utils`
- `src/components`
- `src/services/api`
- `src/commands`
- `src/ink`

最小验证：
- 每个子区完成后跑 `bun run typecheck`
- 必要时增补相关测试

## Phase 4 - 清零验收

- 再扫一遍 `@ts-nocheck`
- 更新 `REPORT.md`
- 跑：
  - `bun run typecheck`
  - `bun test`
  - `bun run verify`

## Batch Ordering

本轮执行顺序固定为：

1. 极小叶子文件
2. 小型 util / hooks / types
3. 小型 components / services / tools
4. 中型 files
5. 大型 commands / services / utils / components
6. 总验证

## Completion Rule

只有同时满足以下条件才算完成：

1. inventory 中所有文件都已处理
2. 仓库扫描结果为 `0`
3. 默认门禁通过
