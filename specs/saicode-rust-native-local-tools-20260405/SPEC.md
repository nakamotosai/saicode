# saicode Rust native local tools（phase 1）Spec

## Goal

把 `saicode` 从“只有简单 `-p` 才能原生化”推进到“本地读搜索 one-shot 也能原生化”。

本轮目标不是重写整套 QueryEngine，而是在 Rust launcher 内新增一个真正可工作的本地工具回路，让高频的 `Read / Grep / Glob` print 请求尽量不再进入 Bun/TS 主链。

## Scope

### In scope

- 新增 Rust native local-tools 路由
- 支持 `-p/--print` 下的原生工具回路
- 首批支持工具：
  - `Read`
  - `Grep`
  - `Glob`
- 支持 OpenAI-compatible provider 的非流式 function calling / tool loop
- 保留并验证 Bun fallback
- 补 benchmark / plan / ledger / mistakebook

### Out of scope

- 全量 Rust 重写 QueryEngine
- 会话恢复、MCP、plugin、agent、多轮 session 状态
- `WebSearch` / `WebFetch`
- `Bash` 原生化
- 直接做同进程 warm worker

## Constraints

- 任何原生新路径都不能破坏现有 native recovery 和 Bun fallback
- 不允许把高风险权限/沙箱逻辑“先假装支持”
- 必须真实跑命令验证工具回路，而不是只看单元测试
- 当前 VPS Rust toolchain 较老，优先少依赖 / 低 MSRV 实现

## Acceptance

1. 对满足条件的 `-p` + `Read/Grep/Glob` one-shot 请求，native launcher 能直接命中 Rust local-tools 路，而不是 Bun lightweight headless。
2. Rust local-tools 路能至少完整跑通一轮真实工具调用，不只是无工具回答。
3. 不支持的特性或高风险场景会明确回退到 Bun，而不是半支持。
4. `cargo test`、`bun run verify`、真实 probe 都通过。
5. benchmark / profile / 文档回写能说明：这轮 Rust 化覆盖到了哪类请求、还没覆盖哪类请求。
