# saicode Rust native local tools（phase 1）Plan

## Step 1 - Scope lock

- 只做 `Read / Grep / Glob`
- 只做 `-p/--print`
- 只做无 session / 无 MCP / 无 plugin / 无 stream-json 的 one-shot
- `Bash`、`WebSearch`、`WebFetch`、warm worker 留到后续

## Step 2 - Native route & args

- 在 Rust launcher 增加 native local-tools route
- 只在满足安全条件时命中：
  - print 模式
  - tool restriction 存在
  - tool 子集属于 `Read/Grep/Glob`
  - 无 resume / continue / stream-json / agent / plugin / mcp 等重特性
- 增加 `SAICODE_DISABLE_NATIVE_LOCAL_TOOLS=1` fallback 开关

## Step 3 - Native tool loop

- 在 Rust 内实现最小 function-calling loop：
  - user prompt
  - provider request
  - tool call parse
  - local tool execute
  - tool result 回填
  - final text output
- 首批工具实现：
  - `Read`：文本文件读取、offset/limit、行号输出
  - `Grep`：调用系统 `rg`
  - `Glob`：调用系统 `rg --files`

## Step 4 - Verification

- `cargo test --manifest-path native/saicode-launcher/Cargo.toml`
- `cargo build --release --manifest-path native/saicode-launcher/Cargo.toml`
- `bun run verify`
- 真实 probe：
  - 命中 native local-tools dry-run
  - 至少一轮 `Read`
  - 至少一轮 `Grep` 或 `Glob`
- benchmark：
  - 对比 native local-tools 与 Bun lightweight headless 的耗时 / RSS

## Step 5 - Closeout

- 回写 plan outcome
- 回写 MISTAKEBOOK / SESSION_LEDGER
- 明确 phase 2 下一步：
  - `Bash`
  - 更强权限模型
  - warm worker / 更大规模 Rust 化

## Outcome

- Step 1 / 2 已完成：
  - Rust launcher 新增 `native-local-tools` route
  - 命中条件收紧为真正可安全支持的子集：
    - `-p/--print`
    - `Read / Grep / Glob`
    - 无 session / resume / stream-json / mcp / plugin / agent 等重特性
  - 新增 fallback 开关：
    - `SAICODE_DISABLE_NATIVE_LOCAL_TOOLS=1`
- Step 3 已完成：
  - 新增 `native/saicode-launcher/src/local_tools.rs`
  - Rust 内已具备最小 function-calling loop：
    - provider request
    - tool call parse
    - local tool execute
    - tool result 回填
    - final answer output
  - 首批原生工具已打通：
    - `Read`
    - `Grep`
    - `Glob`
  - 复用了 native recovery 已有的 provider/model/config 真相层，而不是重新分叉一套配置解释逻辑
- Step 4 已完成：
  - 验证通过：
    - `cargo test --manifest-path native/saicode-launcher/Cargo.toml`
    - `cargo build --release --manifest-path native/saicode-launcher/Cargo.toml`
    - `bun run verify`
  - dry-run 命中：
    - `SAICODE_NATIVE_DRY_RUN=1 ./bin/saicode -p "...\" --allowedTools Read`
    - `SAICODE_NATIVE_DRY_RUN=1 ./bin/saicode -p "...\" --allowedTools Grep`
    - `SAICODE_NATIVE_DRY_RUN=1 ./bin/saicode -p "...\" --allowedTools Glob`
    - 都已命中 `route=native-local-tools target=native-local-tools`
  - 真实 live probes：
    - `Read`：读取 `package.json` version -> `1.0.0`
    - `Grep`：搜索 `initLightweightHeadless` -> `src/entrypoints/initLightweightHeadless.ts`
    - `Glob`：查找包含 `headlessPrint` 的路径 -> `/home/ubuntu/saicode/src/entrypoints/headlessPrint.ts`
    - `Read + --output-format json` 正常
- benchmark：
  - 新增 `scripts/bench_native_local_tools.sh`
  - `package.json` 新增 `bench:native-local-tools`
  - `Read` probe 3x：
    - native local-tools：
      - `3.51s / 10696 KB`
      - `3.58s / 10524 KB`
      - `3.17s / 10696 KB`
    - Bun lightweight fallback：
      - `6.05s / 203148 KB`
      - `3.54s / 203464 KB`
      - `3.71s / 201580 KB`
- 当前结论：
  - wall time 仍会被远端模型波动覆盖，所以并非每次都压倒性更快
  - 但 RSS 已从 Bun lightweight 的约 `200MB` 级，压到 native 的约 `10MB` 级
  - 这说明 `Read / Grep / Glob` 这类 one-shot 本地工具请求，已经进入真正的更大规模 Rust 化阶段

## Boundary

- 仍未覆盖：
  - `Bash`
  - `WebSearch` / `WebFetch`
  - cwd 外权限模型
  - 图片 / PDF / notebook 级 `Read`
  - warm worker
- phase 2 最合适的下一步：
  - 先补 `Bash` 的 read-only 子集或更强权限模型
  - 再决定是否上 native warm worker / 更深的 QueryEngine Rust 化
