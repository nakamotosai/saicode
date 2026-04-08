# saicode Rust delete-cutover spec

日期：
- 2026-04-06

目标：
- 按 6 步把当前 saicode 运行主链彻底切到 Rust，并删除 TS/Bun 旧 runtime 主链。

范围：
- 修正 launcher 的 repo-root 识别，不再依赖 `src/` 与 `package.json`。
- 删除 `bin/saicode` 与 native launcher 中的 Bun fallback。
- 处理 `warm_headless` 的 TS 依赖，要求运行时不再需要 TS worker。
- 把版本源改到 Rust 真相源。
- 做“无 TS 影子”验收：临时移走 TS runtime 文件后，核心命令仍通过。
- 删除旧 runtime 入口文件与相关脚本残留。

非目标：
- 本轮不要求删除仓库里所有 TypeScript 源码。
- 本轮不做剩余 TS 业务/界面的全量 Rust 迁移。
- 本轮不重做 CLI 外观与 `cliproxyapi` 语义。

约束：
- 必须保持当前 `saicode` 命令入口、CLI 外观和 provider 行为不变。
- 必须用真实命令验收，不以 dry-run 代替最终结论。
- 删除旧主链前，必须先通过“无 TS 影子”验收。

验收：
- `bin/saicode` 不再执行 Bun。
- native launcher 不再 handoff 到 `src/*.ts(x)`。
- launcher 可在 `src/`、`preload.ts`、`bin/saicode-bun` 缺失时正常找到 repo root 并运行。
- `help/status/doctor/profile/repl/read/bash/web/task/ttft-bench` 在无 TS runtime 影子条件下通过。
- 旧 TS/Bun runtime 主链文件被删除或下线，项目仍可正常使用。
