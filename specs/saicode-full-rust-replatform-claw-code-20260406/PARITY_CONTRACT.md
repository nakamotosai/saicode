# saicode Rust parity contract

日期：
- 2026-04-06

## 不可变表面

这轮 Rust cutover 必须保持不变的，不是 donor `claw-code` 的产品表层，而是当前 `saicode` 用户已经习惯的这些点：

### 1. 命令入口

- `saicode`
- `saicode --help`
- `saicode --version`
- `saicode -p <prompt>`
- 默认不带参数进入交互式会话

### 2. provider / cliproxyapi 语义

- 继续读取 `~/.saicode/config.json` / 本地 config 里的 `cpa` / `cliproxyapi` provider
- 模型 alias 不能把 `cpa/...` 前缀原样错误地下发给 upstream
- `cliproxyapi` base URL / apiKey / provider fallback 不变

### 3. 高频工具表面

- 用户可见的高频工具名保持：
  - `Bash`
  - `Read`
  - `Write`
  - `Edit`
  - `Glob`
  - `Grep`
  - `WebSearch`
  - `WebFetch`
- permission prompt 必须能在 Rust 主链里工作

### 4. 高频 slash / process 命令

- `/help`
- `/status`
- `/model`
- `/permissions`
- `/cost`
- `/resume`
- `/config`
- `/mcp`
- `/agents`
- `/skills`
- `/plugins`
- `/doctor`
- `/sandbox`
- `/exit`

### 5. session 行为

- 新会话默认可持久化
- `--resume` / `/resume` 可恢复会话
- `/clear` 能开启新 session

## 本轮 Rust 侧的对齐策略

### CLI 外观

- `--help` 保持 saicode 风格文本帮助，不换成 donor/claw 的产品文案。
- 交互式 runtime 用 Rust 自己驱动，但先保持“轻量文本 REPL + slash commands + permission prompts”的稳定面。

### tool surface mapping

- Rust runtime 内部 canonical tool 名仍允许用 donor 风格；
- 对用户和模型暴露时，映射回 saicode 高频显示名。

### provider normalization

- provider config 走 saicode frontline model catalog。
- wire model 用 `ResolvedModel.model`。
- display model 保留 `ResolvedModel.alias` 便于用户理解当前所选模型语义。

## 本轮 parity smoke gate

至少要通过：

1. `./bin/saicode --help`
2. `SAICODE_NATIVE_DRY_RUN=1 ./bin/saicode status`
   - 必须路由到 `rust/target/release/saicode-rust-cli`
3. `./rust/target/release/saicode-rust-cli status`
4. `./rust/target/release/saicode-rust-cli -p 'Reply with exactly: ok'`
   - 必须返回 `ok`
5. `printf '/help\n/exit\n' | ./rust/target/release/saicode-rust-cli`
   - 必须能进入 Rust interactive loop 并显示 slash command help

## 明确暂不承诺的 parity

- 旧 TS/Ink 界面逐像素一致
- plugin tools 自动注入默认会话
- 远程 MCP transport 全量统一 manager
- 真 language-server 级别的 LSP runtime
