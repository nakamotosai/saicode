# saicode pre-Rust 轻量化拆层 Plan

## Round 1 - 基线

- 审视当前 router / headlessPrint / permissionSetup / tools.ts
- 确认 full CLI 误吸轻量工具请求的具体点

## Round 2 - 路由改造

- 定义 lite headless 允许的工具集合
- 让 lightweight headless 的命中条件不再只限于 simple tools
- 仍保留 stream-json / session / MCP / agent 等重路径在 full CLI

## Round 3 - 装载改造

- 为 lightweight headless 增加按需工具装载
- 避免仅为解析 `base tools` 预设而在顶层导入整份 `tools.ts`

## Round 4 - 验证

- 补测试
- 跑：
  - `bun test`
  - `bun run typecheck`
  - `bun run typecheck:full`
  - 真实 one-shot 工具探针

## Round 5 - 结论

- 记录本轮真正减掉了哪一层重负担
- 明确离 Rust/daemon 化还差什么

## Outcome

- 本轮已完成：
  - 把 lightweight headless 的适用范围从 simple local tools 扩大到一小组 headless 友好工具：
    - `Bash`
    - `Glob`
    - `Grep`
    - `Read`
    - `Edit`
    - `Write`
    - `WebFetch`
    - `WebSearch`
  - 让 lightweight headless 在命中上述工具集时，按需动态装载这些工具，而不是先依赖整份 `tools.ts`
  - 在 `permissionSetup` 中加入 `toolUniverseCli`，避免 lightweight headless 仅为处理 `--tools` 限制就顶层导入 `tools.ts`
  - 补充路由与 simple-mode 边界测试
- 验证通过：
  - `bun test`
  - `bun run typecheck`
  - `bun run typecheck:full`
  - `bun run verify`
- 真实探针结果：
  - `Grep`
    - 第一阶段 lite 路：`elapsed=13.60`, `rss_kb=217984`
    - 第二阶段 lite 路：`elapsed=6.25`, `rss_kb=215324`
    - 强制 full 路：`elapsed=6.66`, `rss_kb=230552`
  - `WebSearch`
    - 第一阶段 lite 路：`elapsed=8.62`, `rss_kb=213936`
    - 第二阶段 lite 路：`elapsed=7.14`, `rss_kb=208112`
    - 强制 full 路：`elapsed=6.61`, `rss_kb=222232`
  - `WebFetch`
    - 第一阶段 lite 路：`elapsed=7.77`, `rss_kb=217664`
    - 第二阶段 lite 路：`elapsed=7.85`, `rss_kb=212292`
    - 强制 full 路：`elapsed=5.71`, `rss_kb=224424`
  - startup profile（`WebSearch` lite 路）：
    - 第一阶段：`headless_turn_start = 8587.204ms`
    - 第二阶段：`headless_turn_start = 6621.080ms`
- 结论：
  - 这一步已经把“显式 `WebSearch/WebFetch` 一定会走 full CLI”改掉了，且第二阶段进一步给 lite 路加上了最小 setup 与后台初始化裁剪，说明 lite/full 拆层已经不只是路由判定，而是独立运行面。
  - 当前最稳定的收益体现在：
    - RSS 持续低于强制 full 路
    - startup profile 的 `headless_turn_start` 明显前移
    - `WebSearch` / `Grep` 的端到端时间已出现实质下降
  - 继续往下做会进入更高风险区域：`runHeadless` / `QueryEngine` / query 主链重构，适合作为下一阶段，而不是继续在当前轮零碎加 if。
