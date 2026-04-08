# saicode Rust delete-readiness plan

日期：
- 2026-04-06

步骤：
1. 入口与回退链路核对
   - 检查 `bin/saicode` 与 native launcher 是否仍保留 Bun/TS handoff。
   - 判断 TS 是否纯备份，还是仍承担回退与部分功能表面。

2. Rust 主链基础可用性实测
   - 跑 `status`、简单 `-p`、明确工具调用 prompt。
   - 观察输出是否正确、是否进入 Rust 路由。

3. Rust 工具能力实测
   - 测 `Read`、`Bash`、`TaskCreate`、`WebSearch/WebFetch` 等高频面。
   - 特别关注“已开最高权限但仍说无权限/无法执行”的历史问题。

4. Rust 模型测速代理能力实测
   - 让 Rust 版自己用本地 CLI/脚本思路完成模型 TTFT 测速。
   - 观察是否能自己读配置、列模型、发请求、给结果。

5. 删除 readiness 结论
   - 输出“可删 / 不可删”。
   - 若不可删，列最小阻塞清单。

执行结果：
- 结论：当前还不能直接删除 TS/Bun 旧主链。
- 原因不是 Rust 主链不可用；相反，Rust 主链在禁用 Bun 条件下已经通过了核心实测。

已通过的 Rust 实测：
- `SAICODE_BUN_BIN=/bin/false ./bin/saicode status`
- `SAICODE_BUN_BIN=/bin/false ./bin/saicode doctor`
- `SAICODE_BUN_BIN=/bin/false ./bin/saicode profile list`
- `SAICODE_BUN_BIN=/bin/false ./bin/saicode plugins`
- `SAICODE_BUN_BIN=/bin/false` 交互 REPL 下的 `/help`、`/status`、`/plugins`、`/mcp`、`/skills`
- `bun run accept:rust-tools`
- `bun run accept:rust-tools:no-shadow`
- Rust 版已能完成“读取配置 + 列本地模型 + 用 Bash 做 TTFT bench”的代理任务，无需人工补 `--allowed-tools`

不可直接删除的阻塞点：
1. 启动器仍把 TS 文件当 repo root 锚点
   - `native/saicode-launcher/src/main.rs` 的 `looks_like_repo_root()` 仍要求：
     - `package.json`
     - `src/entrypoints/router.ts`
   - 实测把 `src/`、`preload.ts`、`package.json` 临时移走后，`./bin/saicode status` 直接失败：
     - `Could not locate saicode repo root for native launcher`

2. 启动器仍保留 Bun handoff 活跃代码
   - `bin/saicode` 末尾仍会 fallback 到：
     - `bun --preload preload.ts bin/saicode-bun`
   - `native/saicode-launcher/src/main.rs` 仍保留：
     - `hand_off_to_bun()`
     - route -> `src/localRecoveryCli.ts`
     - route -> `src/entrypoints/headlessPrint.ts`
     - route -> `src/entrypoints/cli.tsx`
   - `native/saicode-launcher/src/warm_headless.rs` 仍直接依赖：
     - `src/entrypoints/headlessPrintWarmWorker.ts`
     - `preload.ts`

3. 构建链仍读取 `package.json`
   - `native/saicode-launcher/build.rs` 仍从仓库根的 `package.json` 提取版本号。
   - 这意味着即使运行时多数路径已 Rust 化，删掉 `package.json` 仍会让当前构建/版本注入链失真。

最小修复方案：
1. 把 launcher 的 repo root 识别从 `package.json + src/entrypoints/router.ts` 改成 Rust 锚点
   - 建议改为优先识别：
     - `rust/Cargo.toml`
     - `native/saicode-launcher/Cargo.toml`
     - 或项目级专用标记文件

2. 删掉 launcher 与 wrapper 的 Bun fallback 主链
   - `bin/saicode` 改成只允许：
     - native launcher
     - rust full cli
   - `native/saicode-launcher/src/main.rs` 删除 `hand_off_to_bun()` 和所有 TS entrypoint 映射
   - `native/saicode-launcher/src/warm_headless.rs` 要么 Rust 化，要么整个 route/能力下线

3. 把版本源从 `package.json` 移到 Rust 真相源
   - 建议统一改为 workspace `Cargo.toml` 或单独的 `VERSION` 文件
   - 然后删除 launcher 对 `package.json` 的 build-time 依赖

4. 做一轮“无 TS 影子”验收
   - 临时移走：
     - `src/`
     - `preload.ts`
     - `bin/saicode-bun`
   - 保留最小必要非 TS 元数据后，重跑：
     - `help`
     - `status`
     - `doctor`
     - `profile list`
     - REPL slash commands
     - `read/bash/web/task/ttft-bench`

5. 通过后再删旧主链
   - 删除 TS entrypoints 与 Bun wrapper
   - 清理 package scripts 里的 Bun-only 流程
   - 更新 install / closeout / acceptance 文档与脚本
