# saicode Stage 6 收口与工具可靠性 Spec

日期：
- 2026-04-06

## Goal

在不进入 Stage 7 删除旧 TS/Bun 主链的前提下，把当前 `saicode` 的 Rust cutover 收口到“用户现在就能放心用”的状态。

这里的“收口”不是继续扩大重写范围，而是把当前默认入口、前台行为、工具可靠性、权限表现、安装/验证工作流全部拉到可重复验收的水平。

## Scope

### In scope

- 复盘当前默认入口是否真的已切到 Rust
- 对高频前台工具做真实链路验收：
  - `Read`
  - `Bash`
  - `Write`
  - `Edit`
  - `WebFetch`
  - `WebSearch`
  - `TaskCreate/TaskList`
- 识别并分级“工具经常出错”的真实来源：
  - tool dispatch 真坏
  - 权限链不稳定
  - 模型不稳定地不选工具
  - 进程内状态和跨进程状态语义不一致
- 把收口方案固化成：
  - 任务级 spec
  - 任务级 plan
  - 实测报告
  - 可复跑 acceptance script

### Out of scope

- Stage 7 删除 / 归档旧 TS/Bun 主链
- 旧 Ink/TS UI 的逐像素 parity
- 全 transport MCP 最终形态
- 真 language-server 级 LSP 完整实现

## Constraints

- 保持现有 `cliproxyapi` 语义不变
- 保持当前命令入口与高频 CLI 外观不大改
- 默认中文汇报
- 结论必须以真实前台可见行为为准，不能只靠单元测试宣布收口

## Current Reality

截至本 spec 建立时，已确认：

- `FullCli` 默认入口已路由到 Rust binary
- `./bin/saicode -p 'Reply with exactly: ok'` 已打通
- Rust parity smoke 已通过
- `bun run verify` 已通过
- 默认权限已固定为 `danger-full-access`
- `Task/Team/Cron` 已具备本地落盘后的跨进程可见性

本收口轮新增关闭的问题：

### 已关闭问题：自由模式下，显式点名工具落错执行面

- 修复前：
  - `Use Read ...` 这类请求会被 wrapper 误判成 simple print，掉到 recovery/one-shot
  - 前台表现成：
    - `unknown`
    - 空输出
    - “I can’t access ... here”
- 修复后：
  - `Read/Bash/Write/Edit/Glob/Grep/Task*/MCP` 的显式工具请求会路由到 Full CLI
  - `WebFetch/WebSearch` 保持原本更稳的 recovery/轻路径，不被这轮修复误伤
  - 自由模式前台实测已通过：
    - `Read`
    - `Bash`
    - `TaskCreate`

### 已关闭问题：`Bash` 默认权限非确定性

- 当前默认权限已固定为 `danger-full-access`
- `./bin/saicode -p --allowedTools Bash ...`
  - 已可稳定作为硬门禁口径

### 已关闭问题：`Task*` 只在进程内有效

- 当前 `TaskCreate` 后，新的 `TaskList` 进程已可看到增长后的任务数
- 当前同一交互进程内的 `TaskCreate -> TaskList` 也仍通过
- 说明：
  - 这一层已经不再是“纯内存假后台”
  - 但它仍是本地持久 registry，不是完整 worker/scheduler 系统

## Acceptance

收口完成至少需要满足：

1. 默认入口仍为 Rust：
   - `SAICODE_NATIVE_DRY_RUN=1 ./bin/saicode status`
   - 路由到 `rust/target/release/saicode-rust-cli`
2. 基础前台链路通过：
   - `./bin/saicode --help`
   - `./bin/saicode status`
   - `./bin/saicode doctor`
   - `./bin/saicode -p 'Reply with exactly: ok'`
3. 高级工具硬门禁通过：
   - `Read`
   - `Bash`（默认口径）
   - `Write`
   - `Edit`
   - `WebFetch`
   - `WebSearch`
4. 状态型工具必须同时通过：
   - 跨进程 `TaskCreate -> TaskList`
   - 同一交互进程中的 `TaskCreate -> TaskList`
5. 必须有一条仓库内可复跑的 acceptance script
6. 必须有一份“哪些是已通过 / 哪些是残项 / 为什么还没收完”的测试报告

## Scope Verdict

在“Stage 6 收口与工具可靠性”这个任务范围内，当前已经达到收口完成：

- 默认入口正确
- 默认权限正确
- 高频工具硬门禁通过
- 自由模式显式点名工具已通过关键前台实测
- task state 具备跨进程可见性

更大的 Rust motherline 残项仍然存在，但不属于本收口 spec 的完成判定。

## Closeout Principle

这轮真正的完成标准不是“Rust 已经很多了”，而是：

- 用户从 `./bin/saicode` 进去后
- 常用前台工具链路有稳定验收口径
- 已知不稳定点被明确写进 closeout plan
- 后续要继续收的只剩清晰的残项，而不是模糊的“再看看还有什么问题”
