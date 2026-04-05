# saicode 全仓重新审视 Plan

## Round 1 - 基线

- 读取错题本与当前仓库脚本/入口
- 建立本轮 Spec/Plan

## Round 2 - 仓库健康检查

- 跑：
  - `bun test`
  - `bun run typecheck`
  - `bun run typecheck:full`
  - `bun run verify`
  - `./bin/saicode --help`
  - 必要的真实 `-p` 探针
- 记录：
  - 是否通过
  - wall time
  - RSS
  - 明显异常

## Round 3 - 结构审视

- 扫描：
  - 测试覆盖面
  - `@ts-nocheck` / 旁路类型安全
  - 文档与脚本一致性
  - 快路径与重路径是否仍有明显割裂

## Round 4 - 结论

- 输出问题分级：
  - P0/P1：已影响可用性或高概率回归
  - P2：重要但不阻塞
  - P3：技术债
- 说明是否建议继续同轮修

## Outcome

- 当前结论：
  - 未发现新的 P0 可用性故障。
  - 默认模型、基础非交互 `-p`、`Grep`、`WebSearch`、`--help` 都已可用。
  - 剩余问题主要集中在“质量门禁覆盖不足”和“整体仍偏重”。
- 已完成验证：
  - `bun test`
  - `bun run typecheck`
  - `bun run typecheck:full`
  - `bun run verify`
  - `./bin/saicode --help`
  - `./bin/saicode -p "Reply with exactly: ok"`
  - `./bin/saicode -p ... --tools Grep`
  - `./bin/saicode -p ... --tools WebSearch`
- 关键证据：
  - `src_files=1968`
  - `test_files=6`
  - `ts_nocheck=256`
  - `typecheck:full`：`elapsed=7.92s`, `rss_kb=1041872`
  - `verify`：`elapsed=8.24s`, `rss_kb=984620`
  - 默认 `-p`：`elapsed=1.95s`, `rss_kb=182084`
  - `Grep`：`elapsed=5.69s`, `rss_kb=212900`
  - `WebSearch`：`elapsed=8.93s`, `rss_kb=223612`
- 分级判断：
  - P1：
    - 默认快门禁覆盖过窄，不能代表整仓类型健康。
  - P2：
    - 自动化测试覆盖面过薄。
    - 类型旁路过多（大量 `@ts-nocheck`）。
    - 显式工具/搜索路径的端到端仍明显偏重。
  - P3：
    - 仓库里仍有残留 `.bak` 文件。
