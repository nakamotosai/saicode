# saicode Rust web tools phase Plan

## 目标

- 把 `WebSearch` / `WebFetch` 从 warm-headless 再往 native local-tools 下沉一层。
- 维持 Bun fallback，不让这轮 Rust 化制造功能回归。

## 阶段

### Step 1 - 任务 Spec / 路由切口对齐

- 新建本任务 `SPEC.md` / `PLAN.md`
- 对齐当前 launcher、local-tools、TS web tool 真相
- 明确 native 与 fallback 的边界

验证：

- 影响面已确认：
  - `native/saicode-launcher/src/main.rs`
  - `native/saicode-launcher/src/local_tools.rs`

### Step 2 - native WebSearch

- 扩展 native 支持的工具集合，允许 `WebSearch` 命中 `native-local-tools`
- 在 Rust 内实现：
  - DuckDuckGo HTML 搜索
  - 搜索结果解析
  - allowed / blocked domains
  - top page excerpt 自动抓取
  - 结果格式与当前 one-shot tool loop 对齐
- 直连搜索拿不到结果时，回退 Bun

验证：

- Rust tests
- dry-run route hit
- 真实 `WebSearch` probe

### Step 3 - deeper native WebFetch

- 扩展 native 支持的工具集合，允许 `WebFetch` 命中 `native-local-tools`
- 在 Rust 内实现：
  - URL 校验
  - http->https 升级
  - same-host redirect 处理
  - HTML/text 提取
  - 按 `prompt` 调小模型做二次总结
- 遇到 binary / 不支持 redirect / 明显超边界内容时，回退 Bun

验证：

- Rust tests
- dry-run route hit
- 真实 `WebFetch` probe

### Step 4 - 回归 / 真实探测 / 收口

- 运行：
  - `cargo test --manifest-path native/saicode-launcher/Cargo.toml`
  - `cargo build --release --manifest-path native/saicode-launcher/Cargo.toml`
  - `bun run verify`
- 运行真实联网 probe
- 更新顶层计划、错题本、session ledger

## 当前状态

- Step 1 已完成
- Step 2 已完成
- Step 3 已完成
- Step 4 已完成

## 本轮已完成内容

- `native/saicode-launcher/src/main.rs`
  - `WebSearch / WebFetch` 已加入 `NATIVE_LOCAL_TOOL_NAMES`
  - dry-run 路由现在会命中 `native-local-tools`
- `native/saicode-launcher/src/local_tools.rs`
  - 新增 native `WebSearch`
    - DuckDuckGo HTML 搜索
    - generic link 兜底解析
    - allowed / blocked domains
    - top page excerpt 自动抓取
    - sai-search HTTP / SSH native fallback
  - 新增 deeper native `WebFetch`
    - URL 校验
    - http -> https 升级
    - same-host redirect 处理
    - HTML/text 提取
    - 小模型二次 prompt 处理
    - binary / 非文本内容自动回退 Bun
  - 新增对应 Rust tests

## 本轮验证结果

- 自动化：
  - `cargo test --manifest-path native/saicode-launcher/Cargo.toml`
  - `cargo build --release --manifest-path native/saicode-launcher/Cargo.toml`
  - `bun run verify`
- dry-run：
  - `SAICODE_NATIVE_DRY_RUN=1 ./bin/saicode -p "hello" --tools WebSearch`
    - `route=native-local-tools target=native-local-tools`
  - `SAICODE_NATIVE_DRY_RUN=1 ./bin/saicode -p "hello" --tools WebFetch`
    - `route=native-local-tools target=native-local-tools`
- 真实 probe：
  - native `WebSearch`
    - 题目：查询 Bun 最新稳定版
    - 结果正常返回，wall time `5.40s`，RSS `13248 KB`
  - Bun cold `WebSearch`
    - 同题 wall time `6.13s`，RSS `227324 KB`
  - native `WebFetch`
    - 题目：抓取 `https://example.com`
    - 返回 `Example Domain`
    - wall time `3.14s`，RSS `13176 KB`
  - Bun cold `WebFetch`
    - 同题 wall time `7.90s`，RSS `214968 KB`
  - native `WebFetch` fallback probe
    - 题目：抓取 PDF
    - native 自动回退 Bun
    - 返回 `ok`
    - wall time `7.33s`，RSS `210368 KB`

## 当前结论

- 这轮目标已达成：
  - `WebSearch / WebFetch` 已从 warm-headless 继续下沉到 native local-tools
  - Bun fallback 仍然保留
- 当前 rewrite 前剩余最值得继续吃掉的高频点：
  - `Write`
  - `Edit`
  - full Bash / write-capable tool loop
