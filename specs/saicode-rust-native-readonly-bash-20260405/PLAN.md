# saicode Rust native readonly Bash（phase 2）Plan

## Step 1 - Scope lock

- 只扩展现有 `native-local-tools` fastpath
- 只做 `-p/--print`
- 只做 readonly Bash 子集
- 不碰 session / agent / plugin / mcp / stream-json
- 不支持带细粒度 matcher 的 `Bash(...)` permission rule

## Step 2 - Route & fallback

- 收紧 native route 命中条件：
  - 工具子集属于 `Bash / Read / Grep / Glob`
  - `Bash(...)` 这类带规则后缀的限制直接不走 native
- native 执行过程中若遇到“不支持但 Bun 能支持”的 tool 能力：
  - 返回 launcher-level fallback signal
  - 自动切回 Bun headless，而不是硬报错

## Step 3 - Native readonly Bash

- 新增 `Bash` tool schema
- 在 Rust 内实现保守的 readonly shell validator：
  - 只允许 inspect / read / search / git-readonly 一类命令
  - 阻断重定向、命令替换、here-doc、后台运行、明显写操作
- 用系统 shell 实际执行，并限制超时与输出体积

## Step 4 - Verification

- `cargo test --manifest-path native/saicode-launcher/Cargo.toml`
- `cargo build --release --manifest-path native/saicode-launcher/Cargo.toml`
- `bun run verify`
- 真实 probe 至少覆盖：
  - `Bash` 当前目录/可执行探测
  - `Bash` git 只读状态
  - `Bash + Read/Grep/Glob` 混合请求
- benchmark：
  - native readonly Bash vs Bun fallback 的 wall time / RSS

## Step 5 - Closeout

- 回写 outcome
- 回写 `MISTAKEBOOK.md` / `SESSION_LEDGER.md`
- 更新 rewrite 前 Rust fastpath 总计划里的阶段结论

## Outcome

- Step 1 已完成：
  - 本轮边界锁定为“扩展现有 `native-local-tools` fastpath”，不是另起一套 Bash-only route
  - `Bash(...)` 这类带 permission matcher 后缀的规则不走 native，避免把细粒度权限误塌缩成裸 `Bash`
- Step 2 已完成：
  - `native-local-tools` route 现在支持工具子集：
    - `Read`
    - `Grep`
    - `Glob`
    - `Bash`
  - native 执行中如果遇到当前不支持但 Bun 应该能支持的能力，会返回 launcher-level fallback signal，再自动回退到 Bun headless
  - 当前已接入 fallback 的典型场景：
    - `Bash` 写操作 / shell 扩展 / glob / 重定向 / 背景任务 / 高风险 git flag
    - `Read.pages`
    - binary / unsupported native `Read`
- Step 3 已完成：
  - Rust 原生新增了 readonly Bash 子集：
    - `pwd`
    - `ls`
    - `command -v`
    - `which`
    - `git status/diff/log/show/rev-parse/ls-files/grep/blame/show-ref/describe`
    - 以及一批安全的 read/list/search 类命令
  - 通过保守 validator 阻断：
    - 重定向
    - shell 变量展开
    - glob expansion
    - 后台运行
    - `find -exec/-delete`
    - `git -c/--config-env/--exec-path/--output`
  - 实际 shell 执行加上了：
    - 超时上限
    - 输出截断
    - cwd 内执行
- Step 4 已完成：
  - 通过：
    - `cargo test --manifest-path /home/ubuntu/saicode/native/saicode-launcher/Cargo.toml`
    - `cargo build --release --manifest-path /home/ubuntu/saicode/native/saicode-launcher/Cargo.toml`
    - `bun run verify`
  - dry-run 命中：
    - `env SAICODE_NATIVE_DRY_RUN=1 ./bin/saicode -p "Use Bash to run 'command -v rg' ..." --allowedTools Bash`
    - 输出：`route=native-local-tools target=native-local-tools`
  - 真实 live probes：
    - `Bash`：`command -v rg` -> 返回真实 `rg` 路径
    - `Bash`：`git status --short` -> 返回真实首行 `M .env.example`
    - `Bash + Read`：`pwd + Read package.json` -> `saicode:1.0.0`
    - fallback probe：
      - prompt 明确要求 Bash glob expansion `src/entrypoints/*.ts`
      - 最终成功返回 `/home/ubuntu/saicode/src/entrypoints/fastCliHelp.ts`
      - 由于 native readonly Bash 明确不支持 glob expansion，这里可以合理推断：launcher 已成功自动回退到 Bun
  - benchmark：
    - `./scripts/bench_native_readonly_bash.sh`
    - native readonly Bash：
      - `3.75s / 10688 KB`
      - `4.30s / 10688 KB`
      - `3.59s / 10688 KB`
    - Bun lightweight fallback：
      - `4.92s / 214760 KB`
      - `14.41s / 213076 KB`
      - `6.86s / 214252 KB`
- 当前结论：
  - 这轮 Rust 化已经把 one-shot 本地读搜之外的一大块“只读 shell 探测”也纳入了 native fastpath
  - wall time 依然会受模型/网络波动影响，但 Bash 这条路的 RSS 已从约 `214MB` 压到约 `10.7MB`
  - 下一步如果继续扩大 Rust 面，优先级更适合是：
    - 更强的 Bash 权限模型
    - 更广的 `Read` 文件类型支持
    - native warm worker
