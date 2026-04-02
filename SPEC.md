# saicode repair Spec

## Goal

把当前 `saicode` 从“主链路混着已删除旧功能、类型面大面积失真”的状态，收敛到“主用 CLI/TUI 能稳定工作、模型面统一为 `cpa/...`、死入口不再拖垮运行面”的状态。

## Scope

### In scope

- 统一模型面：
  - `/model`、默认值、说明文案统一为 `cpa/...`
  - 保留旧 `cliproxyapi/...` 与旧直连 `nvidia/...` 作为兼容别名
- 封死已删除实现对应的旧入口：
  - `direct connect`
  - `ssh remote`
  - `server mode`
  - `ccshare resume`
  - 其他底层文件已删除但命令面仍暴露的旧分支
- 修复当前最高优先级错误热区：
  - `src/main.tsx`
  - `src/screens/REPL.tsx`
  - `src/ink/components/App.tsx`
  - `src/utils/messages.ts`
  - `src/utils/hooks.ts`
- 优先处理会直接影响以下能力的错误：
  - 启动
  - `--help`
  - `-p/--print`
  - `/model`
  - 基础对话主链路

### Out of scope

- 全仓 `tsc` 清零
- 恢复你已经明确删掉的旧模块
- 恢复 Claude/Anthropic 私有产品旁支
- 新增新的 provider/runtime 架构

## Constraints

- 不补回已删除模块；只能删入口、封口或改成显式不可用
- 保留新加的：
  - `cpa/qwen3-coder-plus`
  - `cpa/vision-model`
- 原有直连 NVIDIA 模型不再作为主可见模型
- 若必须改变功能，优先收口旧入口，不做伪兼容
- 优先以真实 CLI/TUI 可用性为验收，不以单纯类型数字下降为完成标准

## Acceptance

至少满足以下条件才算这一轮修复达标：

1. `/model` 真实读回时可见模型使用 `cpa/...`
2. 旧 `cliproxyapi/...` / `nvidia/...` 配置仍能解析到新模型 ID
3. `saicode --help` 可正常输出，且不再把已删除实现当作可用主入口
4. `saicode -p "Reply with exactly: ok"` 能走通当前主 provider 链路
5. `main.tsx` 不再强依赖已删除文件而导致启动/命令面直接炸掉
6. 第一优先级热区错误数明显下降，且下降来自主链路而不是无关区域

## Risks

- 当前仓库不是单一根因，而是“旧产品壳残留 + 类型漂移 + 缺文件”叠加
- `REPL.tsx` 和 `App.tsx` 仍可能藏有第二层残留引用，需要分轮修
- 全仓 `tsc` 即使显著下降，也不代表所有前台行为都恢复，仍要做真实冒烟
