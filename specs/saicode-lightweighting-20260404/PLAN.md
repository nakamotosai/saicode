# saicode lightweighting Plan

## Round 1 - 落 Spec

- 新建本轮轻量化 `SPEC.md` / `PLAN.md`
- 明确：
  - 快路径目标
  - 非目标
  - 验收命令

## Round 2 - 启动入口减重

- 把 `bin/saicode` 改成单跳 launcher
- 新增单进程 router entrypoint
- 验证：
  - `./bin/saicode --version`
  - `./bin/saicode --help`

## Round 3 - 轻 help / version 快路径

- 为单独的 `--help` / `-h` 提供静态快路径
- 为单独的 `--version` / `-v` / `-V` 提供静态快路径
- 避免为了 help 继续 import `src/main.tsx`

## Round 4 - print/simple-tools 提前轻模式

- 在 raw args 阶段识别“本地轻工具集 one-shot 请求”
- 在 import `main.tsx` 之前打开 `CLAUDE_CODE_SIMPLE`
- 保持以下场景继续走完整 CLI：
  - MCP
  - session / continue / resume
  - stream-json
  - agents / plugins
  - 非轻工具集

## Round 5 - typecheck 快车道

- 新增 `tsconfig.fast.json`
- 新增 `typecheck:fast`
- 保留全量 `typecheck`
- 尽量让快车道适合频繁本地迭代

## Round 6 - 自动化验证

- 为路由和轻模式判断补最小测试
- 覆盖：
  - recovery 路由
  - help/version 快路径判定
  - simple-tools 提前轻模式判定

## Round 7 - 真实复测

- 复测并记录：
  - `./bin/saicode --help`
  - `./bin/saicode --version`
  - `./bin/saicode -p "Reply with exactly: ok"`
  - `bun run typecheck:fast`
  - `bun run typecheck`

## Round 8 - simple-tools 轻量 headless

- 新增 simple-tools 专用 headless print 入口
- 支持范围收口在：
  - `-p` / `--print`
  - 本地 simple tools 限定
  - `text` / `json`
  - `model` / `system-prompt` / `append-system-prompt`
  - `permission-mode` / `--dangerously-skip-permissions`
  - `max-turns` / `max-budget-usd` / `task-budget`
- 明确以下继续走完整 CLI：
  - `stream-json`
  - `mcp-config`
  - `resume` / `continue` / `fork-session`
  - `agent` / `agents`
  - `plugin-dir` / `add-dir` / `settings`
  - 非 simple tools

## Round 9 - 路由与脚本收口

- 在 router 中增加 simple-tools headless 路由
- 让 `typecheck` 默认指向快车道
- 新增 `typecheck:full`
- 保持 `check` / `verify` 继续挂全量闸门

## Round 10 - 第二轮自动化验证

- 补最小测试：
  - simple-tools headless 路由判定
  - 轻量 headless 参数解析
  - typecheck 脚本口径

## Round 11 - 第二轮真实复测

- 复测并记录：
  - `./bin/saicode -p "Reply with exactly: ok" --allowedTools Read,Grep`
  - `./bin/saicode -p "Reply with exactly: ok" --tools Read Grep`
  - `bun run typecheck`
  - `bun run typecheck:full`

## Final Gate

- 快路径功能正确
- recovery 与完整 CLI 路由未串线
- typecheck 双车道都可运行
- simple-tools `-p` 相比基线至少拿到一轮确定性减重收益
- 相比基线至少拿到一轮确定性减重收益
