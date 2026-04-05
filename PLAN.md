# saicode hardening Plan

## 当前进度快照（截至 2026-04-05）

- 当前主线已经从前期 hardening，进入“rewrite 前的 Rust fastpath”阶段。
- 如果按当前 Rust 主线继续细分到 explicit bypass `Bash` native 化，总共可以按 `8` 个阶段看：
  - 已完成 `8` 个阶段：
    - Stage 1：Rust launcher
    - Stage 2：native recovery print
    - Stage 3：高频本地工具链 native 化
    - Stage 4：重路径 warm 化 / warm worker
    - Stage 5：benchmark / 计划回写 / 阶段性总收口
    - Stage 6：WebSearch / WebFetch 更深层 native 化
    - Stage 7：Write / Edit native 化
    - Stage 8：显式 bypass one-shot Bash native 化
- 当前整体状态：
  - `8` 阶段已全部完成
  - rewrite 前这一段最重要的“让 saicode 真正变轻”的主收口，已经覆盖到高频 readonly、本地写改、web tools、以及显式 bypass 的 one-shot Bash

## 今天做了什么

- 在前面 3 个阶段的基础上，继续完成了：
  - Stage 4：`lightweight headless` 隔离式 warm worker
  - Stage 5：benchmark、计划回写、经验沉淀和阶段性总收口
  - Stage 6：`WebSearch / WebFetch` native local-tools 下沉
  - Stage 7：`Write / Edit` native local-tools 下沉
  - Stage 8：显式 bypass 的 one-shot `Bash` native local-tools 下沉
- 当前 warm 路不是危险的“同进程复用旧 runtime”，而是：
  - 常驻 Bun manager
  - 预热 one-shot child
  - child 单次执行后退出并预热下一只
- Rust launcher 现在对 `Route::LightweightHeadless` 的处理变成：
  - 先尝试 warm worker
  - 失败自动回退原 Bun `headlessPrint.ts`
- 新增脚本：
  - `scripts/bench_warm_headless.sh`
  - `package.json` -> `bench:warm-headless`
- 真实验证已经跑过：
  - `cargo test --manifest-path native/saicode-launcher/Cargo.toml`
  - `cargo build --release --manifest-path native/saicode-launcher/Cargo.toml`
  - `bun run verify`
  - `Write` warm path probe
  - `WebSearch` native path probe
  - `WebFetch` native path probe
  - `WebFetch(PDF)` native fallback-to-Bun probe
  - native `Write` create / update probes
  - native `Edit` probe
  - native bypass `Bash` dry-run probes：
    - `--dangerously-skip-permissions`
    - `--permission-mode bypassPermissions`
  - native bypass `Bash` 写文件 probe
  - native bypass `Bash` 读回 probe
  - Bun cold `Write` / `Edit` 对比 probe
  - Bun cold bypass `Bash` 对比 probe

## 今天做到什么程度

- Stage 1 已完成：
  - 最外层 launcher 已经 Rust 化
- Stage 2 已完成：
  - simple `-p/--print` recovery 已经不必进入 Bun
- Stage 3 已完成：
  - 高频本地工具链已经原生覆盖：
    - `Read`
    - `Grep`
    - `Glob`
    - readonly `Bash`
- Stage 4 已完成：
  - `lightweight headless` 的重路径已改成可复用 warm startup
- Stage 5 已完成：
  - benchmark / plan / ledger 的阶段性收口已完成
- Stage 6 已完成：
  - native local-tools 已继续覆盖：
    - `WebSearch`
    - `WebFetch`
  - `WebFetch` 的 native 路不只是 raw fetch，而是已经下沉到：
    - URL 校验 / redirect 处理
    - HTML/text 提取
    - 二次 prompt 小模型处理
  - 对 binary / native 不支持场景，仍保留自动 Bun fallback
- Stage 7 已完成：
  - native local-tools 已继续覆盖：
    - `Write`
    - `Edit`
  - `Read -> Write/Edit` 的关键安全语义也已经下沉：
    - full-read gate
    - stale 检测
    - `replace_all`
    - 多匹配拒绝
    - quote-normalized 匹配
- Stage 8 已完成：
  - native local-tools 已继续覆盖：
    - `Bash`（仅显式 `bypassPermissions` / `dangerously-skip-permissions`）
  - bypass 场景下已下沉到 native 的能力：
    - write-capable one-shot shell 执行
    - timeout
    - stdout / stderr / exit code 输出
    - Bash 执行后 read snapshot 清空
  - 仍保留 Bun fallback 的能力：
    - `run_in_background`
    - `Bash(...)` 规则匹配

## 当前真实状态

- 当前 one-shot 本地探测类请求，已经不再是“必须进 Bun/TS 世界”：
  - 本地文件读取
  - 本地内容搜索
  - 本地路径查找
  - 本地文件创建 / 全量覆盖
  - 本地精确文本替换
  - 本地只读 shell 探测
  - 显式 bypass 的本地 shell 写操作
  - Web 搜索
  - Web 页面抓取与摘要
  都已经有 Rust fastpath
- 这轮实测里，native readonly Bash 的 RSS 大约在 `10.7MB`，而 Bun fallback 大约在 `214MB`
- 这轮显式 bypass Bash 实测里：
  - native bypass `Bash`：
    - `3.93s / 10.5MB RSS`
  - Bun cold bypass `Bash`：
    - `5.83s / 209MB RSS`
  - `--permission-mode bypassPermissions` 读回 probe 也已正常返回 `bash-native-ok`
- 当前 rewrite 前还留在 Bun 世界、而且已经属于 rewrite 级别的重路径，主要只剩：
  - native background Bash / foreground task runtime
  - `Bash(...)` 规则与完整 permission engine
  - 更深层 `QueryEngine` / permission-heavy runtime Rust 化
- 这轮联网实测里：
  - native `WebSearch`：
    - `5.40s / 13MB RSS`
  - Bun cold `WebSearch`：
    - `6.13s / 227MB RSS`
  - native `WebFetch(example.com)`：
    - `3.14s / 13MB RSS`
  - Bun cold `WebFetch(example.com)`：
    - `7.90s / 215MB RSS`
  - `WebFetch(PDF)` 会从 native 自动回退 Bun，实测返回正常
- 这轮写改工具实测里：
  - native `Write` create：
    - `3.19s / 10.5MB RSS`
  - Bun cold `Write` create：
    - `5.25s / 205MB RSS`
  - native `Edit`：
    - `6.51s / 10.4MB RSS`
  - Bun cold `Edit`：
    - `6.70s / 214MB RSS`
- 结论不是“联网任务每次都会极限变快”，而是：
  - native local-tools 已真实扩展到写改链路
  - 本地负担显著下降
  - 对 `Write` 这条链，连端到端 wall time 也已有明显收益
  - 对 `Edit` 这条链，wall time 提升不大，但本地负担已经从 `214MB` 级压到 `10MB` 级

## 当前若继续推进，下一步是什么

- 这条 `rewrite 前 Rust fastpath` 主线，已经把高频 readonly + web tools + 轻量写改 + 显式 bypass 的 one-shot Bash 都吃下来了。
- 如果接着做，已经不该再按“再补一个小 fastpath”去估，而应直接按 rewrite-scale 处理：
  - native background Bash / foreground task runtime
  - `Bash(...)` 规则与完整 permission engine
  - Bun `QueryEngine` / permission-heavy tool runtime 分阶段 Rust 化

## 明天续跑前建议先看

- 顶层主进度：
  - `PLAN.md`
- 当前 Rust 主线详细进度：
  - `specs/saicode-rust-fastpath-20260405/PLAN.md`
- 今天新增的 Bash native 阶段记录：
  - `specs/saicode-rust-native-readonly-bash-20260405/PLAN.md`
- 今天新增的 Web tools native 阶段记录：
  - `specs/saicode-rust-web-tools-20260405/PLAN.md`
- 今天新增的 Write/Edit native 阶段记录：
  - `specs/saicode-rust-write-edit-20260405/PLAN.md`
- 今天新增的 Bash bypass native 阶段记录：
  - `specs/saicode-rust-bash-bypass-20260405/PLAN.md`

## Round 1 - 配置口径收口

- 更新 `SPEC.md` / `PLAN.md`
- 统一 `cpa` 的：
  - 环境变量名
  - 错误提示
  - README / `.env.example`
  - 运行时读取顺序
- 保留 `cliproxyapi/...` 与旧别名兼容
- 验证：
  - `saicode --help` 可跑
  - `cpa` 错误提示与真实读取逻辑一致

## Round 2 - 本机首请求打通

- 确认本机当前统一入口的真实配置源
- 先做 live probe，再落成 `saicode` 可复用配置
- 打通：
  - `saicode -p "Reply with exactly: ok"`
  - 基础 `cpa/...` 模型请求

## Round 3 - 回归门补齐

- 添加最小自动化验证：
  - 模型别名解析
  - `cpa` 配置解析或报错口径
- 补最小仓库脚本，避免以后只剩 `tsc`
- 验证：
  - 类型检查
  - 自动化测试

## Round 4 - 轻量提速

- 优先压缩最常用入口的冷启动绕路
- 不做大重构，只拿确定性收益
- 验证：
  - `-v`
  - `--help`
  - 非交互 print 请求

## Round 5 - 完整 CLI 默认模型修复

- 让 saicode mode 不只看环境变量，也能识别 `~/.saicode/config.json`
- 修正完整 CLI 在本机已有 provider 配置时仍退回旧 Claude 默认模型的问题
- 验证：
  - 不显式传 `--model` 的 `--print` 请求可真实返回
  - `stream-json` init 中的模型值落在 `cpa/...`

## Round 6 - 一次性 lean context 收口

- 让普通 `-p` 一次性请求默认进入 lean context
- 但不能把 `stream-json` / 指定工具集 / resume 等场景误送进 recovery 或误砍工具池
- 补 recovery 路由判断，避免探针场景测到假入口
- 验证：
  - 简单文本请求仍可走通
  - `--tools` + `stream-json` 进入完整 CLI

## Round 7 - 真实工具探针矩阵

- 跑一次性任务矩阵：
  - 纯文本
  - Read
  - Grep
  - Write
  - Edit
  - Telegram 工程桥包装输入
- 记录每条任务的：
  - 结果是否正确
  - 实际工具是否合理
  - wall time / duration

## Round 8 - 回写与沉淀

- 更新 `SPEC.md` / `PLAN.md`
- 补自动化测试覆盖：
  - 非交互入口路由
  - auto-bare 条件
  - config-file 驱动的 saicode mode 检测
- 把这轮排障经验写回错题本与 session ledger

## Round 9 - 默认模型健康收口

- 体检当前默认模型是否仍适合作为本机默认值
- 若默认模型已在真实 `-p` 路径上卡死：
  - 同步更新 catalog 默认值
  - 移除 recovery CLI 里的重复默认模型真相
  - 更新 `.env.example` / README 的默认示例
  - 补最小测试，避免默认值漂回失效模型

## Final Gate

- `saicode --help`
- `saicode -p "Reply with exactly: ok"`
- `bun test`
- Read / Grep / Write / Edit / Telegram wrapper 探针
- 当前机器后续无需临时手工注入即可继续使用
