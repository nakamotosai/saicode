# saicode Rust Bash bypass phase Plan

## 目标

- 把 `bypassPermissions` 下的 one-shot `Bash` 从 Bun 再往 native local-tools 下沉一层。
- 明确边界：只收显式 bypass 场景，不伪装成完整 Bash runtime Rust 化。

## 阶段

### Step 1 - Spec / 路由边界对齐

- 新建本任务 `SPEC.md` / `PLAN.md`
- 对齐当前 native Bash、route flags、TS Bash 权限模式真相
- 锁定 native 与 fallback 的边界

验证：

- 影响面已确认：
  - `native/saicode-launcher/src/main.rs`
  - `native/saicode-launcher/src/local_tools.rs`

### Step 2 - native bypass Bash

- 扩展 native route，接受：
  - `--dangerously-skip-permissions`
  - `--permission-mode bypassPermissions`
- 在 Rust 内实现：
  - bypass Bash policy
  - write-capable shell 执行
  - timeout / output 格式
  - Bash 执行后 read snapshot 清空
- 对 `run_in_background` 和 `Bash(...)` 规则继续 fallback

验证：

- Rust tests
- dry-run route hit
- 真实 bypass `Bash` probe

### Step 3 - 回归 / 验证 / 收口

- 运行：
  - `cargo test --manifest-path native/saicode-launcher/Cargo.toml`
  - `cargo build --release --manifest-path native/saicode-launcher/Cargo.toml`
  - `bun run verify`
- 用真实入口做 dry-run 和写文件 probe
- 更新顶层计划、错题本、session ledger

## 当前状态

- Step 1 已完成
- Step 2 已完成
- Step 3 已完成

## 本轮已完成内容

- `native/saicode-launcher/src/main.rs`
  - native route 现在接受：
    - `--dangerously-skip-permissions`
    - `--permission-mode bypassPermissions`
  - 这两条 Bash bypass 场景现在都会命中 `native-local-tools`
- `native/saicode-launcher/src/local_tools.rs`
  - 新增 native `Bash` policy：
    - readonly
    - unrestricted（仅显式 bypass 场景）
  - bypass 场景下 `Bash` 已支持：
    - write-capable shell 执行
    - timeout
    - stdout / stderr / exit code 输出
  - Bash 执行后会清空 read snapshots，避免后续 stale 状态失真
  - `run_in_background` 与 `Bash(...)` 规则仍保留 Bun fallback
  - 新增对应 Rust tests

## 本轮验证结果

- 自动化：
  - `cargo test --manifest-path native/saicode-launcher/Cargo.toml`
  - `cargo build --release --manifest-path native/saicode-launcher/Cargo.toml`
  - `bun run verify`
- dry-run：
  - `SAICODE_NATIVE_DRY_RUN=1 ./bin/saicode -p "hello" --tools Bash --dangerously-skip-permissions`
    - `route=native-local-tools target=native-local-tools`
  - `SAICODE_NATIVE_DRY_RUN=1 ./bin/saicode -p "hello" --tools Bash --permission-mode bypassPermissions`
    - `route=native-local-tools target=native-local-tools`
- 真实 probe：
  - native bypass `Bash`
    - 用 `printf ... > .tmp-native-probes/bash-native.txt` 真实写文件
    - 返回 `done`
    - `3.93s / 10.5MB RSS`
    - 文件内容正确：`bash-native-ok`
  - native bypass `Bash` + `--permission-mode bypassPermissions`
    - `cat .tmp-native-probes/bash-native.txt`
    - 正常返回 `bash-native-ok`
- Bun cold 对比：
  - 同类写文件 Bash probe
    - `5.83s / 209MB RSS`
    - 文件内容正确：`bash-bun-ok`

## 当前结论

- 这轮目标已达成：
  - 显式 bypass 场景下的 one-shot `Bash` 已从 Bun headless 继续下沉到 native local-tools
  - rewrite 前剩余“高频 yet practical”的 heavy path 已基本吃完
- 当前 rewrite 前真正还剩的，只是更深层、已接近重写级别的部分：
  - native background Bash / foreground task runtime
  - `Bash(...)` 规则与完整 permission engine
  - 更深层 `QueryEngine` / permission-heavy runtime Rust 化
