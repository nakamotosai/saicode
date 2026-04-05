# saicode Rust Write/Edit phase Plan

## 目标

- 把 `Write / Edit` 从 Bun headless 再往 native local-tools 下沉一层。
- 保住“先读后写 / stale 检测 / replace_all 语义”，不做缩水版伪支持。

## 阶段

### Step 1 - Spec / 现状对齐

- 新建本任务 `SPEC.md` / `PLAN.md`
- 对齐当前 launcher、local-tools、TS `Write/Edit` 真相
- 锁定 native 与 fallback 的边界

验证：

- 影响面已确认：
  - `native/saicode-launcher/src/main.rs`
  - `native/saicode-launcher/src/local_tools.rs`

### Step 2 - native Write / Edit

- 扩展 native 支持的工具集合，允许 `Write / Edit` 命中 `native-local-tools`
- 在 Rust 内实现：
  - read snapshot 状态
  - full-read gate
  - stale 检测
  - `Write`
  - `Edit`
  - `replace_all`
  - 多匹配拒绝
  - 新建文件路径
- 遇到 native 明确不支持的情况，回退 Bun

验证：

- Rust tests
- dry-run route hit
- 真实 `Write` / `Edit` probe

### Step 3 - 回归 / 验证 / 收口

- 运行：
  - `cargo test --manifest-path native/saicode-launcher/Cargo.toml`
  - `cargo build --release --manifest-path native/saicode-launcher/Cargo.toml`
  - `bun run verify`
- 用真实入口做 dry-run 和实写探针
- 更新顶层计划、错题本、session ledger

## 当前状态

- Step 1 已完成
- Step 2 已完成
- Step 3 已完成

## 本轮已完成内容

- `native/saicode-launcher/src/main.rs`
  - `Write / Edit` 已加入 `NATIVE_LOCAL_TOOL_NAMES`
  - dry-run 路由现在会命中 `native-local-tools`
- `native/saicode-launcher/src/local_tools.rs`
  - 新增 native `Write`
    - 新建文件
    - 已有文件 full-read gate
    - stale 检测
    - UTF-8 / UTF-16LE 文本写入
  - 新增 native `Edit`
    - `old_string / new_string / replace_all`
    - 多匹配拒绝
    - quote-normalized 匹配
    - 基本引号风格保留
    - stale 检测
  - `Read` 现在会在 native tool loop 内记录 read snapshot，供后续 `Write / Edit` 使用
  - 新增对应 Rust tests

## 本轮验证结果

- 自动化：
  - `cargo test --manifest-path native/saicode-launcher/Cargo.toml`
  - `cargo build --release --manifest-path native/saicode-launcher/Cargo.toml`
  - `bun run verify`
- dry-run：
  - `SAICODE_NATIVE_DRY_RUN=1 ./bin/saicode -p "hello" --tools Write`
    - `route=native-local-tools target=native-local-tools`
  - `SAICODE_NATIVE_DRY_RUN=1 ./bin/saicode -p "hello" --tools Edit`
    - `route=native-local-tools target=native-local-tools`
- 真实 probe：
  - native `Write` create
    - 创建 `.tmp-native-probes/native-write.txt`
    - 文件内容正确
    - `3.19s / 10.5MB RSS`
  - native `Write` update
    - 先 `Read` 再全量覆盖 `.tmp-native-probes/native-write-update.txt`
    - 文件内容正确
    - `4.69s / 10.5MB RSS`
  - native `Edit`
    - 先 `Read` 再编辑 `.tmp-native-probes/native-edit.txt`
    - 曲引号风格保留，结果为 `say “goodbye” now`
    - `6.51s / 10.4MB RSS`
- Bun fallback 对比：
  - Bun cold `Write` create
    - `5.25s / 205MB RSS`
  - Bun cold `Edit`
    - `6.70s / 214MB RSS`

## 当前结论

- 这轮目标已达成：
  - `Write / Edit` 已从 Bun headless 继续下沉到 native local-tools
  - `Read -> Write/Edit` 的关键安全语义也已一起下沉
- 当前 rewrite 前剩余最值得继续吃掉的高频点：
  - full-permission / write-capable `Bash`
  - 更深层 permission/runtime / QueryEngine Rust 化
