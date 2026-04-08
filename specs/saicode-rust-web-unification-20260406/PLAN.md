# saicode Rust web and no-shadow closeout plan

日期：
- 2026-04-06

步骤：
1. 修文档与定位
   - 记录本轮目标、范围、验收。
   - 核对 `full-cli` 与 `native-recovery` 的 Web 工具分叉点。

2. 修复 full-cli 多工具显式识别
   - 让 `Use Read and Bash ...`、`Use Read and Bash if needed ...` 这类提示能同时识别多个显式工具。
   - 显式命中多个工具时，收窄工具面到这组工具，并强化“这些工具可用”提示。

3. 统一 WebSearch/WebFetch Rust 执行面
   - 让 full-cli Web 工具走与 recovery 相同的 Rust 实现。
   - 消除 `recovery 能用 / full-cli 坏掉` 的分叉。

4. 加 no-shadow 验收
   - 新增或更新脚本，令 Bun handoff 一旦发生就直接失败。
   - 覆盖 `help/status/repl/read/bash/web/task/ttft-bench`。

5. 运行验收并回写结果
   - 常规验收。
   - no-shadow 验收。
   - 回写结论与残留。

执行结果：
- 已完成步骤 1-5。
- full-cli 显式多工具识别已修复：`Use Read and Bash ...` 不再需要人工补 `--allowed-tools`。
- `WebSearch/WebFetch` 已统一到与 recovery 相同的 Rust 执行面，移除了旧的 `tools/web.rs` 分叉实现。
- 验收脚本已支持显式模型钉死：默认使用 `SAICODE_ACCEPT_MODEL`，未设置时走 `cpa/gpt-5.4-mini`，避免默认 `gpt-5.4` cooldown 造成假失败。

实测：
- `bun run accept:rust-tools` 通过。
- `bun run accept:rust-tools:no-shadow` 通过。
- no-shadow 条件：`SAICODE_BUN_BIN=/bin/false`。

关键观测：
- 常规验收通过项覆盖：`help/status/read/bash/write/edit/webfetch/websearch/task/ttft-bench`。
- no-shadow 验收通过项覆盖：`help/status/repl/read/bash/write/edit/webfetch/websearch/task/ttft-bench`。
- `ttft-bench` 在无需人工喂 `--allowed-tools` 的条件下通过，样例输出：
  - `[{\"model\":\"gpt-5.4\",\"ttft_seconds\":1.360959},{\"model\":\"gpt-5.4-mini\",\"ttft_seconds\":2.035501}]`
  - `[{\"model\":\"gpt-5.4-mini\",\"ttft_seconds\":0.531},{\"model\":\"gpt-5.4\",\"ttft_seconds\":0.87}]`

速度备注：
- 一次 no-shadow 验收中的 `bash_free` 出现过 39.65s 尖峰，但随后连续 3 次同模型复测均为 10-12s，暂未复现为稳定慢点。

残留边界：
- 本轮已满足当前 spec，但仍未进行“删除 TS/Bun 旧版”的最终切换。
- TTFT bench 结果依赖本地 `/models` 实时返回的模型集合，因此样例中的两模型顺序可能波动。
