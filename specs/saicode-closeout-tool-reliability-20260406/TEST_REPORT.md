# saicode Stage 6 收口实测报告

日期：
- 2026-04-06

## 入口与门禁

- `./scripts/rust_parity_harness.sh`
  - 结果：通过
- `./scripts/closeout_preflight.sh`
  - 结果：通过
- `bun run rust:test:frontline`
  - 结果：通过
- `bun run verify`
  - 结果：通过

## 高级工具 acceptance script

脚本：
- `/home/ubuntu/saicode/scripts/rust_tool_acceptance.sh`

本次实测输出：

```text
hard_gate	ok
ok	elapsed=1.93	ok
read_allowed	elapsed=10.86	saicode
read_free	elapsed=4.76	saicode
bash_default	elapsed=10.93	saicode
bash_free	elapsed=10.92	saicode
write_allowed	elapsed=6.86	ok
edit_allowed	elapsed=9.53	ok
read_file_content	elapsed=4.15	beta
webfetch	elapsed=7.16	Example Domain
websearch	elapsed=4.30	example.com
task_list_before	elapsed=4.31	20
task_create	elapsed=5.05	task_19d620656b9
task_create_free	elapsed=8.53	task_19d62067704
task_list_after	elapsed=7.28	22
```

## 单独补充实测

### 默认入口是否已切到 Rust

- 命令：
  - `SAICODE_NATIVE_DRY_RUN=1 ./bin/saicode status`
- 结果：
  - 路由到 `rust/target/release/saicode-rust-cli`

### 同进程 Task 工具

- 命令：
  - 同一 interactive 进程中先 `TaskCreate` 再 `TaskList`
- 结果：
  - 创建成功
  - 随后 `TaskList` 返回 `1`
- 耗时：
  - 约 `12.42s`

### 跨进程 Task 工具

- 命令：
  - 单独一次 `-p --allowedTools TaskCreate`
  - 下一次新的 `-p --allowedTools TaskList`
- 结果：
  - `TaskCreate` 成功，返回 task id
  - 新进程中的 `TaskList` 返回增长后的任务数
- 判断：
  - 当前 Task registry 已是跨进程持久可见的本地 registry

### 默认权限模式

- 当前 `./bin/saicode status` 已显示：
  - `Permission mode  danger-full-access`
- `./bin/saicode -p --allowedTools Bash ...`
  - 已可直接通过，不再走 approval 文案

### 自由模式显式点名工具

- 当前实测通过：
  - `./bin/saicode -p 'Use Read to inspect package.json and reply with only the package name.'`
    - 返回：`saicode`
  - `./bin/saicode -p 'Use Bash to run pwd and reply with only the exact last path component.'`
    - 返回：`saicode`
  - `./bin/saicode -p 'Use TaskCreate to create a background task with prompt ping and reply with only the created task_id.'`
    - 返回：`task_*`
- 根因与修复：
  - 真正问题不是工具本身坏，而是 wrapper 会把这类 prompt 误路由到 recovery/one-shot
  - 修复后：
    - `Read/Bash/Write/Edit/Glob/Grep/Task*/MCP` 的显式工具请求会走 Full CLI
    - `WebFetch/WebSearch` 维持原来更稳的 recovery/轻路径

## 当前结论

### 已通过

- Rust 默认入口
- `cliproxyapi` live prompt
- 默认 `danger-full-access`
- `Read`（在硬门禁口径下）
- 自由模式 `Read`
- `Bash`（默认口径）
- 自由模式 `Bash`
- `Write`
- `Edit`
- `WebFetch`
- `WebSearch`
- cross-process `TaskCreate -> TaskList`
- same-process `TaskCreate -> TaskList`
- 自由模式 `TaskCreate`

### 未完全收口

在本收口 spec 的范围内，已无未关闭项。

更大的 Rust motherline 残项仍在：
- 旧 UI 逐像素 parity
- plugin tool 默认注入
- MCP / LSP 深水区能力扩展

## 对用户的解释

现在的 `saicode` 已经到了“主入口、高频工具、默认最高权限、跨进程本地任务 registry、自由模式显式点名工具”都能用的状态。

这一份 closeout spec 已经收完；后续如果继续推进，就该回到更大的 Rust motherline，而不是继续在这个收口包里打转。
