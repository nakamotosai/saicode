# saicode Rust web tools phase Spec

## Goal

把 `saicode` 在 rewrite 前的 Rust fastpath 再往前推一层：让高频的一次性 `WebSearch` / `WebFetch` 不再默认只能走 Bun warm-headless，而是优先命中 native local-tools 路；同时保留 Bun fallback，避免语义倒退。

这轮不是做“看起来有 Rust 代码”的半成品，而是要把以下两件事都收口：

1. `WebSearch` 的真实搜索执行面下沉到 Rust。
2. `WebFetch` 的抓取与二次 prompt 处理进一步下沉到 Rust，而不只是把 URL fetch 一下就算完成。

## Scope

### In scope

- 扩展 native launcher / local-tools 路由：
  - `WebSearch`
  - `WebFetch`
- 在 Rust `native local-tools` 内实现：
  - `WebSearch` 的 DuckDuckGo HTML 搜索
  - 搜索结果解析、域名过滤、去重、top-K 控制
  - 搜索后 top page excerpt 自动抓取
  - `WebFetch` 的 URL 校验、http->https 升级、受限 redirect 处理
  - HTML/text 内容提取
  - `WebFetch.prompt` 的二次模型总结调用
- 保留 launcher-level Bun fallback：
  - native 明确不支持或高风险的输入
  - native 抓取失败但 Bun 仍可能成功的场景
- 补对应 Rust tests / dry-run 验证 / 真实 probe
- 回写计划、错题本、session ledger

### Out of scope

- `Write` / `Edit` 的 Rust 化
- 完整替换 Bun `QueryEngine`
- 完整复刻 TS `WebFetch` 的全部权限 / analytics / binary persistence / preflight 生态
- 引入高 MSRV 或重依赖的新 Rust 网络栈

## Constraints

- 当前 VPS 仍按旧 Rust toolchain 约束处理，优先复用系统 `curl`
- 不能把语义缩水实现伪装成“已完整支持”
- 遇到 native 暂不支持的能力时，优先 fallback 到 Bun，而不是直接让用户承担回归
- 真实验收必须包含 dry-run 命中、自动化回归、和真实联网探测

## Acceptance

1. `SAICODE_NATIVE_DRY_RUN=1 ./bin/saicode -p "hello" --tools WebSearch` 命中 `native-local-tools`
2. `SAICODE_NATIVE_DRY_RUN=1 ./bin/saicode -p "hello" --tools WebFetch` 命中 `native-local-tools`
3. native `WebSearch` 可真实返回正确搜索结果，且自动抓取 excerpt
4. native `WebFetch` 可真实抓取公开网页并按 `prompt` 返回处理后的结果
5. 对 native 明确不支持的情况，仍可自动 fallback 到 Bun
6. `cargo test --manifest-path native/saicode-launcher/Cargo.toml` 通过
7. `cargo build --release --manifest-path native/saicode-launcher/Cargo.toml` 通过
8. `bun run verify` 通过

## Risks

- 联网工具的 wall time 抖动大，不能只凭单次耗时判断成败
- `WebFetch` 的 TS 逻辑本身比 `WebSearch` 复杂，若 native 支持边界没定义清楚，容易误伤兼容性
- DuckDuckGo HTML 页结构变化可能影响直连搜索解析，因此必须保留 fallback
