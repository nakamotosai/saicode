# saicode repair Plan

## Round 1 - 主链路止血

- 更新 `SPEC.md` / `PLAN.md`
- 统一模型面到 `cpa/...`
- 保留旧模型 ID 兼容解析
- 封死已删除实现对应的命令面和入口面：
  - `direct connect`
  - `ssh remote`
  - `server mode`
  - `ccshare resume`
- 验证：
  - `/model` 读回
  - 旧 ID 解析到新 ID
  - `saicode --help` 可跑

## Round 2 - 高优先级热区修复

- 聚焦 4 个高频热区：
  - `src/screens/REPL.tsx`
  - `src/ink/components/App.tsx`
  - `src/utils/messages.ts`
  - `src/utils/hooks.ts`
- 先修三类问题：
  - 已删除文件的残留引用
  - ant/claude 残留分支
  - 直接阻断基础对话链路的类型漂移
- 每修一簇就做一次定向类型检查

## Round 3 - 运行面回归

- 验证：
  - `saicode --help`
  - `saicode -p "Reply with exactly: ok"`
  - `/model` 菜单读回
  - 主要旧入口不再误导性暴露
- 复盘剩余错误：
  - 区分“当前必须继续修”与“可以留到下一轮的深层残留”

## Round 4 - 残余封口

- 对已删除旧功能、外围 UI、分析和调试残留做封口处理
- 优先保持运行语义不变，不恢复已明确删掉的旧模块
- 以全量 `tsc` 清零和最小 CLI 冒烟作为完成门槛

## Multi-Agent 分工

- 分身 A：
  - 调查 `main.tsx` 和命令面还残留哪些死入口
  - 给出最小封口建议
- 分身 B：
  - 调查 `REPL/App/messages/hooks` 的高频错误簇
  - 给出“最小修复路径”而不是泛泛 review
- 主控：
  - 维护 Spec/Plan
  - 集成两份勘察结果
  - 控制每轮写入范围
  - 执行最终验证
