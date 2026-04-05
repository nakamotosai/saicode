# saicode Rust native readonly Bash（phase 2）Spec

## Goal

把 `saicode` 从“原生只覆盖 `Read / Grep / Glob`”继续推进到“高频的一次性只读 Bash 探测也能走 Rust fastpath”，进一步减少 Bun/TS 冷启动负担。

本轮不是把 TS 里的整套 Bash 权限系统搬到 Rust，而是新增一个保守的 native readonly Bash 子集，并在遇到 native 不支持但 Bun 支持的能力时自动回退到 Bun，保证减重和兼容性同时成立。

## Scope

### In scope

- 扩展现有 Rust native local-tools fastpath
- 新增原生 `Bash` 工具，但只支持 readonly / inspect 型命令
- 支持 `Bash` 与 `Read / Grep / Glob` 组合出现在同一个 one-shot print 请求里
- 对 native 不支持的工具能力提供“自动回退到 Bun”的收口
- 新增 benchmark / probe / plan 回写

### Out of scope

- 全量重写 TS Bash 权限系统
- 写操作、后台任务、sandbox 覆盖、权限提升
- `WebSearch` / `WebFetch`
- 多轮 session / resume / mcp / plugin / agent
- warm worker / daemon

## Constraints

- 不能把“native 只读 Bash”误当成“完整 Bash”
- 不能错误解释 `Bash(git:*)` 这类带细粒度 permission matcher 的规则；本轮不支持就必须直接回退
- 不能破坏现有 `Read / Grep / Glob` native 路
- 必须保留 Bun fallback，且要真实验证 fallback 仍可用
- 必须用真实 CLI probe 验证，而不是只跑单元测试

## Acceptance

1. 对满足条件的 `-p/--print` + `Bash` 或 `Bash + Read/Grep/Glob` one-shot 请求，Rust fastpath 可以直接命中。
2. native `Bash` 至少能真实跑通几类常见只读探测：
   - 当前目录/文件列表类
   - 可执行文件/版本探测类
   - git 只读状态类
3. 当模型请求 native 不支持的 Bash 能力时，launcher 会自动回退到 Bun，而不是把任务卡死在半支持状态。
4. `cargo test`、`cargo build --release`、`bun run verify`、真实 probe、benchmark 都通过。
5. 文档能明确写清：
   - 这轮 Bash native 化到底覆盖了什么
   - 哪些 Bash 仍会回退到 Bun
   - 为什么这样切边界
