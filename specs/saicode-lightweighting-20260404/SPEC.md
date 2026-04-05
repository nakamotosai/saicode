# saicode lightweighting Spec

## Goal

把 `saicode` 当前“能用但明显偏重”的常用链路，收敛到“单次启动更轻、常见轻命令更快、开发自检更适合高频迭代”的状态。

本轮目标不是重写整套 CLI，而是优先拿下用户真实高频面上最明显的无谓开销：

- `--help` / `--version`
- 简单 `-p` 一次性调用
- 指定本地轻工具集的 `-p` 一次性调用
- 日常开发中的 typecheck 回路

在第一阶段拿下 `--help` / `--version` / recovery 快路径后，第二阶段继续聚焦：

- 明确把“指定本地轻工具集的 `-p`”从完整 `main.tsx` 启动链里拆出来
- 让日常默认 `typecheck` 更偏向高频开发快车道

## Scope

### In scope

- 去掉当前 `bin/saicode` 的双 Bun 启动
- 增加真正轻量的 `--help` / `--version` 快路径
- 让路由在单进程内决定：
  - recovery CLI
  - 轻量 simple-tools headless print CLI
  - 完整 CLI
- 对明确限定为本地轻工具集的 `-p` 请求，提前打开 simple mode，而不是等到主 CLI 全量 import 之后
- 为 typecheck 增加适合高频开发的快车道
- 把默认 `typecheck` 指向高频开发快车道，并保留 `typecheck:full` 作为总闸门
- 保留全量 typecheck，避免“只剩快车道、没有总闸门”
- 增加对应最小自动化覆盖

### Out of scope

- 全面拆分 `src/main.tsx`
- 一轮内把所有 5k 行级大文件全部模块化
- 重写 `print.ts` 的完整 QueryEngine
- 改变现有 provider / 模型 / MCP 主体行为

## Constraints

- 不能破坏现有完整 CLI 能力
- 不能把带 MCP、session、resume、stream-json 的重场景误判成轻路径
- 不能把 slash command、agent/plugin/session 注入这类依赖完整上下文的请求误送进轻量 headless
- 不能为了快而删除全量校验入口
- 现有脏工作区中的用户改动必须保留

## Acceptance

1. `./bin/saicode --help` 走轻路径，不再加载完整主 CLI。
2. `./bin/saicode --version` 走轻路径，不再绕双 Bun。
3. 简单 `./bin/saicode -p "Reply with exactly: ok"` 仍能正常返回。
4. 显式限制为本地轻工具集的 `-p` 请求，会在 import 主 CLI 前提前打开 simple mode。
5. 全量 `bun run typecheck` 仍可用。
6. 新增 `bun run typecheck:fast`，用于高频开发回路。
7. 指定本地轻工具集的 `-p` 请求，在不触发 MCP / resume / stream-json / plugin / agent / settings 等重场景时，可绕开 `src/main.tsx`。
8. 默认 `bun run typecheck` 指向快车道，同时保留 `bun run typecheck:full`。
9. 至少有最小自动化验证覆盖：
   - simple-tools headless 路由判定
   - 轻量 headless 入口的关键参数解析
10. 本轮改动后，simple-tools `-p` 请求相比当前基线有可见下降。
11. `--help` 的 wall time 和 RSS 相比当前基线保持已拿到的明显下降。

## Baseline

- `./bin/saicode --help`: 约 `1.16s`，约 `182MB RSS`
- `./bin/saicode -p "Reply with exactly: ok"`: 约 `2.89s`，约 `181MB RSS`
- `./bin/saicode -p "Reply with exactly: ok" --allowedTools Read,Grep`: 约 `3.18s`，约 `206MB RSS`
- `bun run typecheck`: 约 `23.66s`，约 `1.44GB RSS`

## Risks

- `--help` 文本若改成快路径，必须避免和主 CLI 常用帮助信息严重漂移
- simple mode 若提前开启条件过宽，可能误伤需要完整工具面的请求
- 轻量 headless 若和完整 CLI 参数口径漂移，可能造成“看似更快、实际少能力”的假优化
- 只做启动层瘦身，无法一轮消灭整个仓库的历史体量
