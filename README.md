# saicode

`saicode` 现在是纯 Rust 终端客户端。

当前仓库策略已经切换为：

- 前端、后端、CLI、流式输出、工具系统、会话、MCP、LSP、插件能力统一由 Rust 承担
- TypeScript/Bun 前端和旧 TS 运行时已经删除
- 默认入口 `./bin/saicode` 直接进入 Rust 路径

## 当前状态

- 默认命令入口：`./bin/saicode`
- 当前配置默认模型：`cpa/qwen/qwen3.5-122b-a10b`
- 主实现位置：`rust/`
- 原生 launcher：`native/`

当前运行真相：

- `status` / `config show` 看到的默认模型仍是 `cpa/qwen/qwen3.5-122b-a10b`
- 为了避免当前 provider 在函数调用和部分流式链路上出现 degraded / 无 token / 不收尾，工具型请求、one-shot、recovery 等稳定性优先路径会自动切到 `cpa/gpt-5.4-mini`
- 这不是第二入口，也不是第二套配置；只是同一条 Rust 运行时里的稳定执行回退

## 现在什么还能用

当前非前端可用面以 Rust 为准：

```bash
./bin/saicode --help
./bin/saicode status
./bin/saicode config show
./bin/saicode -p "Reply with exactly: ok"
printf '/help\n/exit\n' | ./bin/saicode --bare
./scripts/rust_tool_acceptance.sh
SAICODE_CLOSEOUT_LIVE=1 SAICODE_CLOSEOUT_ACCEPTANCE=1 ./scripts/closeout_preflight.sh
```

其中：

- `./bin/saicode` 是唯一入口
- builtin tools、skill、MCP、plugin、LSP 的实测入口以 `./scripts/rust_tool_acceptance.sh` 为准
- closeout 入口以 `./scripts/closeout_preflight.sh` 为准

## 验证口径

当前推荐的非前端收口命令：

```bash
cargo test --manifest-path native/saicode-launcher/Cargo.toml
(cd rust && ../scripts/rust-cargo.sh test -q -p api -p runtime -p commands -p tools -p saicode-frontline -p saicode-rust-cli -p saicode-rust-one-shot -p saicode-rust-local-tools)
./scripts/rust_tool_acceptance.sh
SAICODE_CLOSEOUT_LIVE=1 SAICODE_CLOSEOUT_ACCEPTANCE=1 ./scripts/closeout_preflight.sh
```

`rust_tool_acceptance.sh` 里的 TTFT bench 只统计真正拿到首 token 的可调用模型。模型如果虽然能出现在 `/v1/models`，但真实 `/chat/completions` 不出 token 或直接 degraded，会被跳过，不算假通过。

## 开发

Rust 构建：

```bash
cargo build --release --manifest-path native/saicode-launcher/Cargo.toml
(cd rust && ../scripts/rust-cargo.sh build --release -p saicode-rust-cli)
```

Rust 测试：

```bash
cargo test --manifest-path native/saicode-launcher/Cargo.toml
(cd rust && ../scripts/rust-cargo.sh test -q -p api -p runtime -p commands -p tools -p saicode-frontline -p saicode-rust-cli -p saicode-rust-one-shot -p saicode-rust-local-tools)
SAICODE_CLOSEOUT_LIVE=1 SAICODE_CLOSEOUT_ACCEPTANCE=1 ./scripts/closeout_preflight.sh
```

## 当前边界

- 当前仓库默认展示模型仍是 `cpa/qwen/qwen3.5-122b-a10b`，这是配置口径，不等于每条运行时请求都必须直打 qwen
- 若外部 provider 连 `gpt-5.4-mini` 也不可用，工具型和 one-shot 稳定性会退回到上游可用性问题，不在仓库内可单独消除
- 本 README 只描述非前端真相；浏览器前端不在本轮收口范围内

## 清理原则

这次切换是刻意的破坏性收敛：

- 不再保留 TS 与 Rust 双实现
- 不再保留 Bun/TS 前端或后端入口
- 不再维持“表面还能跑、实际多入口漂移”的兼容层

## 目录

```text
bin/saicode                 Rust 统一入口包装器
rust/                       Rust 主工作区
native/                     Rust launcher
scripts/                    仅保留 Rust 相关脚本
```
