# saicode usable closeout Spec

## Goal

把当前 `saicode` 从“仓库内入口看起来能跑，但用户真正敲的 `saicode` 命令会因为软链入口解析错误，或 full CLI Bun fallback 缺少 preload，而直接坏掉”的状态，收口到“已安装命令可正常进入、full CLI 也能正常起、安装脚本能提前发现坏链、收尾与 GitHub 提交流程有固定闸门”的状态。

## Scope

### In scope

- 修复 `bin/saicode` 通过软链执行时的 repo root 解析
- 修复 native launcher / Bun fallback 在仓库外 cwd 下缺少 `preload.ts` 的问题
- 恢复 `saicode --help`、已安装命令入口、以及 full CLI 进入面
- 给 wrapper 增加防回归测试
- 给安装脚本补命令入口 smoke test
- 新增 closeout preflight 脚本
- 新增 closeout 文档，覆盖本地收尾与 GitHub 提交流程
- 让 CI 至少执行 closeout preflight

### Out of scope

- 继续更大规模 Rust 化
- 新增复杂发布系统
- 一次性清理仓库所有历史文件与命名残留

## Constraints

- 不回退当前 native launcher / Bun fallback 的双入口设计
- 不把“仓库内 `./bin/saicode` 可跑”误当成“用户安装命令可用”
- closeout workflow 既要能本地执行，也要适合作为 GitHub 前的固定闸门

## Acceptance

1. `saicode --help` 正常输出
2. `SAICODE_DISABLE_NATIVE_LAUNCHER=1 saicode --help` 正常输出
3. `SAICODE_DISABLE_NATIVE_LAUNCHER=1 saicode mcp --help` 正常输出
4. 真实 `saicode` 交互式可成功进入 TUI
5. `bun test` 包含对软链入口和 full CLI fallback 的回归覆盖
6. `./scripts/install_vps_jp.sh` 在创建命令入口后会主动 smoke test
7. `bun run closeout:preflight` 通过
8. 配好 runtime config 的机器上，`bun run closeout:live` 返回 `ok`
9. 仓库内存在明确的 closeout / GitHub 提交流程文档
