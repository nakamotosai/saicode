# saicode pre-Rust 轻量化拆层 Spec

## Goal

在不进行 Rust 重写的前提下，把 `saicode` 的高频 one-shot 轻量路径再往前推进一层：让显式轻量工具请求尽量不再掉进 full CLI，并减少这些请求对整套工具世界的装载成本。

## Scope

### In scope

- 重新划分 lite / full 路由：
  - 保留现有 `--help` / `--version` / recovery / simple-tool 路径
  - 扩大 lightweight headless 路径，使其支持一小组“headless 友好工具”
- 为 lightweight headless 提供按需工具装载，而不是依赖整份 `tools.ts`
- 尽量避免 lightweight headless 仅因权限初始化而顶层拉入 `tools.ts`
- 补最小回归测试
- 跑真实探针验证：
  - `--tools Read/Grep`
  - `--tools WebSearch` / `--tools WebFetch`

### Out of scope

- 重写 TUI / main CLI / agent runtime
- MCP / plugin / skill 全量轻量化
- 常驻 daemon 或 Rust 实现

## Constraints

- 不破坏现有 full CLI 能力
- 不破坏 stream-json 和会话相关重路径判定
- 只把明确属于 headless 友好集合的工具放进 lite 路径

## Acceptance

1. `--tools`/`--allowedTools` 落在 lite 集合内时，应优先命中 lightweight headless 路由。
2. lightweight headless 路径不再依赖整份 `tools.ts` 来构造这类请求的工具列表。
3. 相关测试通过，且真实 `Read/Grep/WebSearch/WebFetch` 探针仍正确。
4. 至少给出一轮实测结果，证明这步改造已落地而非停留在代码层。
