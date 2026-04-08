# saicode

`saicode` 现在是纯 Rust 终端客户端。

当前仓库策略已经切换为：

- 前端、后端、CLI、流式输出、工具系统、会话、MCP、LSP、插件能力统一由 Rust 承担
- TypeScript/Bun 前端和旧 TS 运行时已经删除
- 默认入口 `./bin/saicode` 直接进入 Rust 路径

## 当前状态

- 默认命令入口：`./bin/saicode`
- 主实现位置：`rust/`
- 原生 launcher：`native/`

## 现在什么还能用

当前可用面以 Rust 为准：

```bash
./bin/saicode --help
./bin/saicode status
```

若本机已构建对应二进制，也可直接用：

```bash
./bin/saicode
./bin/saicode -p "hello"
```

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
```

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
