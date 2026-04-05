# saicode Rust fastpath（rewrite 前）Plan

## 当前状态 - 截至 2026-04-05

- 这是当前 Rust fastpath 主线的收口文件。
- 当前 Rust 主线总共 `5` 个阶段：
  - 已完成 `5` 个：
    - Step 1：Rust launcher
    - Step 2：native recovery print
    - Step 3：高频本地工具链 native 化
    - Step 4：重路径 warm 化
    - Step 5：最终总收口
- 当前结论：
  - `5` 步已经全部完成
  - rewrite 前这一段“先把最重的 Bun 冷启动压掉”的主线已经收口

## 今天新增完成的内容

- 完成了 Step 4：
  - 为 `lightweight headless` 新增了隔离式 warm worker
  - 不是同进程复用旧 runtime，而是：
    - 常驻 Bun manager
    - 预热 one-shot child
    - child 只处理单次请求，结束后重建下一只 warm child
  - 这样既能吃到 warm path，又不把 session / hook / global state 串到下一次请求
- Rust launcher 已接上 warm path：
  - 当前 `Route::LightweightHeadless` 会优先尝试 `warm-headless-worker`
  - 失败时自动回退到原 Bun `src/entrypoints/headlessPrint.ts`
- `headlessPrint` 已抽成可复用执行面：
  - 轻量请求解析
  - lightweight runtime 初始化
  - 单请求执行函数
  现在都能被常规 CLI 与 warm child 共用
- 为了让非 TTY benchmark / 脚本也能稳定命中 warm path，新增：
  - `SAICODE_FORCE_WARM_HEADLESS=1`
- 新增 benchmark 脚本：
  - `scripts/bench_warm_headless.sh`
  - `package.json` -> `bench:warm-headless`

## 本轮真实验证

- 构建 / 回归门：
  - `cargo test --manifest-path native/saicode-launcher/Cargo.toml`
  - `cargo build --release --manifest-path native/saicode-launcher/Cargo.toml`
  - `bun run verify`
- 路由验证：
  - TTY 下 `SAICODE_NATIVE_DRY_RUN=1 ./bin/saicode -p "hello" --tools WebSearch`
    -> `route=lightweight-headless target=warm-headless-worker`
- 真实 probe：
  - `Write` warm path 实测成功，目标文件已真实写入 `/tmp/saicode-warm-worker-check-7.txt`
  - `WebSearch` warm path 实测成功，返回 `Paris`
  - `WebFetch` warm path 实测成功，返回 `ok`
- 真实测速观察：
  - `Write` 同场景复测：
    - warm：`4.80s`
    - cold fallback：`10.77s`
  - `WebSearch` 复测：
    - warm：`5.98s`
    - cold fallback：`6.44s`
  - `WebFetch` 复测：
    - warm：`5.99s`
    - cold fallback：`5.56s`

## 当前应怎么解读这些结果

- Step 4 已真实落地，不再只是“方案”。
- 对本地重路径，warm path 已经可以把“每次冷起 Bun”改成“复用预热 child”。
- 对联网工具，端到端 wall time 仍会被远端模型 / 网络抖动覆盖，所以：
  - `WebSearch` 这次略快
  - `WebFetch` 这次略慢
  都不能单次绝对化
- 更稳的结论是：
  - warm path 已真实命中
  - fallback 正常
  - 本地重链已经从“纯冷启动”变成“可复用 warm startup”

## Step 1 - Rust launcher

- 已完成

## Step 2 - Native recovery print

- 在 Rust launcher 内直接处理 recovery print
- 不再 exec `localRecoveryCli.ts`
- 支持：
  - `-p/--print`
  - `--model`
  - `--system-prompt`
  - `--system-prompt-file`
  - `--append-system-prompt`
  - `--output-format text|json`
- 补 provider/config/model/env precedence 测试

## Step 3 - 高频本地工具链

- 评估 `Read/Grep/Glob/Bash` 的 native 化切口
- 若当前轮风险过高，至少补 shared truth / warm path 准备

## Step 4 - 重路径 warm 化

- 评估 recovery 之外的 Bun 重链如何改成 warm worker
- 若当前轮不直接实现，至少落设计和接缝

## Step 5 - 总收口

- benchmark
- docs / plan 回写
- mistakebook / ledger 沉淀

## Outcome So Far

- Step 2 已完成：
  - Rust launcher 现在会对 recovery route 先尝试 native handling，而不是直接 `exec localRecoveryCli.ts`
  - native recovery 已支持：
    - `-p/--print`
    - `--model`
    - `--system-prompt`
    - `--system-prompt-file`
    - `--append-system-prompt`
    - `--output-format text|json`
  - native recovery 内部已具备：
    - model alias 解析
    - provider / config / env precedence 解析
    - `cpa` / `cliproxyapi` / `nvidia` provider config 收口
  - 若设置 `SAICODE_DISABLE_NATIVE_RECOVERY=1`，会回退到原 Bun recovery CLI
  - 为兼容当前 VPS 上的 `rustc/cargo 1.75.0`，native recovery 的 HTTP 传输本轮采用系统 `curl`，避免被新 crate 的 MSRV 漂移卡死；但整个 simple print 主链仍已脱离 Bun
- 新验证门：
  - `package.json` 的 `verify/check` 已加入 `native:test:optional`
- 验证通过：
  - `cargo test --manifest-path native/saicode-launcher/Cargo.toml`
  - `cargo build --release --manifest-path native/saicode-launcher/Cargo.toml`
  - `bun run verify`
  - `./bin/saicode -p "Reply with exactly: ok"` -> `ok`
  - `./bin/saicode -p "Reply with exactly: ok" --output-format json` 正常
  - `SAICODE_NATIVE_DRY_RUN=1 ./bin/saicode -p "hello"` -> `route=recovery target=native-recovery`
- benchmark：
  - native recovery 3x：
    - `0.67s / 10748 KB`
    - `0.88s / 10752 KB`
    - `0.87s / 10744 KB`
  - Bun recovery fallback 3x：
    - `1.81s / 182148 KB`
    - `2.12s / 182652 KB`
    - `2.26s / 181260 KB`
- 当前判断：
  - simple print 这条链已经明显更轻，且不再进入 Bun recovery 主链
  - 剩余大头在 Step 3 / Step 4：
    - 工具型 one-shot 还在 lightweight headless / QueryEngine 世界
    - 重路径仍是冷启动 Bun，而不是 warm worker

## Step 3 Outcome

- 已落地的减重：
- native local-tools phase 1：
  - Rust 已直接接住 `Read / Grep / Glob` one-shot print
  - RSS 从 Bun lightweight 的约 `200MB` 级压到约 `10MB` 级
- native readonly Bash phase 2：
  - `native-local-tools` fastpath 已继续扩展到 readonly Bash 子集
  - 现在高频的本地 inspect 类 shell 探测也能不进入 Bun/TS 主链
  - 对 native 不支持但 Bun 支持的能力，已补 launcher-level 自动回退，而不是把“native 子集”误装成“完整 Bash”
- `src/QueryEngine.ts`
    - lightweight / bare 路现在不再静态吃进 `commands.js` 和 `pluginLoader.js`
    - skills / plugins 改成仅在非 lightweight 路按需 import
  - `src/cli/print.ts`
    - Grove / GrowthBook / settings sync / managed settings wait 改成按需 import
    - lightweight headless 不再默认订阅 settings change detector
  - `src/entrypoints/headlessPrint.ts`
    - 保留一个实验性的 `initLightweightHeadless.ts`，但默认不启用
    - 默认仍走 full `init()`，因为真实 profile 证明“更小的 init 函数体”不等于更快的整条 lightweight 链
  - benchmark / scripts：
    - 新增 `scripts/bench_lightweight_headless.sh`
    - `package.json` 新增 `bench:lightweight`
- 真实结论：
  - 真正有效的是“把 lightweight 路不会用到的重模块改成按需加载”
  - 单独把 init 拆小虽然看起来更轻，但会把后续共享依赖推迟到更痛的冷路径，导致 `headless_turn_start` 波动和尾部变差
- profile 结果：
  - 变更前（此前 baseline）：
    - lightweight headless `headless_turn_start = 2792.006ms`
  - 当前优化后的默认路径（full init + lazy imports）：
    - `headless_turn_start = 1991.378ms`
    - 复测稳定值约 `2042.802ms / 2048.578ms`
  - 实验性 lean init：
    - 有时可到 `1914ms / 2146ms`
    - 但也出现 `3327ms / 4329ms` 级坏尾
    - 因而不作为默认路径
- 当前判断：
  - Step 3 已经收口到“默认路径更快、实验慢路不再误开”的状态
  - lightweight 工具路不再是“所有本地工具都必须进 Bun/TS 世界”：
    - `Read / Grep / Glob`
    - readonly `Bash`
    已经进入 Rust fastpath
  - 剩余没吃掉的大头更多集中在：
    - write-capable / full-permission Bash
    - `WebSearch` / `WebFetch`
    - warm worker / 更深层 QueryEngine 状态复用

## Step 4 Outcome

- 已完成：
  - `src/entrypoints/headlessPrintWarmWorker.ts`
    - 常驻 manager 负责接本地 Unix socket 请求
    - 每次请求使用预热好的 one-shot child
    - child 只处理一单，处理完就退出并预热下一只
- 为什么这样做：
  - 不需要给 `runHeadless()` 硬补“全量 reset”
  - 避免同进程复用导致的 listener / session / global state 串味
  - 仍然能把最重的 Bun 冷启动前段前移到空闲时间完成
- launcher 行为：
  - TTY 下的 lightweight headless one-shot 请求，优先走 warm worker
  - 失败自动回退原 Bun entrypoint
  - 非 TTY 默认仍保守关闭，避免误吞 stdin；benchmark/脚本可显式设 `SAICODE_FORCE_WARM_HEADLESS=1`
- 当前边界：
  - warm child 目前按“最近使用的 `mode + cwd`”维持一条预热线
  - 切换 cwd 或从 `simple` 切到 `lightweight` 时，第一下仍可能付一次冷启动

## Step 5 Outcome

- 已完成：
  - 构建 / 自动化回归已重新跑通
  - 新增 `bench:warm-headless`
  - 顶层与专项计划文件已回写
  - 错题本与 session ledger 已沉淀
- 最终判断：
  - rewrite 前这一段最重要的 5 个阶段已经全部收口
  - 现在剩下如果还要继续“更大规模 Rust 化”，就不再是补 warm worker 这种 bridge，而是进入更深层的 native rewrite 范围
