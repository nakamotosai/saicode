# saicode 基于 kcode 的 Rust 重平台化总计划

## 当前进度快照（截至 2026-04-05）

- Stage 1 已启动并拿到第一批实质进展：
  - 已在仓库内落地新的 Rust workspace：
    - [Cargo.toml](/home/ubuntu/saicode/rust/Cargo.toml)
    - [rust-toolchain.toml](/home/ubuntu/saicode/rust/rust-toolchain.toml)
  - 已把 donor workspace 导入到：
    - `/home/ubuntu/saicode/rust/crates/*`
  - 已新增 `saicode` 自有 adapter crate：
    - [Cargo.toml](/home/ubuntu/saicode/rust/crates/saicode-core-adapter/Cargo.toml)
    - [lib.rs](/home/ubuntu/saicode/rust/crates/saicode-core-adapter/src/lib.rs)
  - 已开始对 donor 核心 crates 做第一轮 `kcode -> saicode` 命名收口：
    - `SAICODE_*`
    - `.saicode`
    - `Saicode`
- 已完成验证：
  - `cd /home/ubuntu/saicode/rust && /home/ubuntu/.cargo/bin/cargo check -q`
  - `cd /home/ubuntu/saicode/rust && /home/ubuntu/.cargo/bin/cargo test -q -p saicode-core-adapter`
  - `cd /home/ubuntu/saicode/rust && /home/ubuntu/saicode/scripts/rust-cargo.sh check -q`
  - `cd /home/ubuntu/saicode/rust && /home/ubuntu/saicode/scripts/rust-cargo.sh test -q -p api`
  - `cd /home/ubuntu/saicode/rust && /home/ubuntu/saicode/scripts/rust-cargo.sh test -q -p runtime`
  - `cd /home/ubuntu/saicode/rust && /home/ubuntu/saicode/scripts/rust-cargo.sh test -q -p saicode-rust-one-shot`
  - `cd /home/ubuntu/saicode && cargo test --manifest-path native/saicode-launcher/Cargo.toml`
  - `cd /home/ubuntu/saicode && bun run typecheck:fast`
- 当前判断：
  - Rust donor base 已经不再停留在“外部候选仓库”，而是已经真实进入 `saicode` 仓库主工作树。
  - `reasoning_effort` 已开始在 Rust Core 中真实打通：
    - API 请求结构已支持 effort
    - `Saicode` OpenAI-compatible payload 已可发出 `reasoning_effort`
    - `max` 档在 `Saicode` provider 下已映射到最高档 wire value
    - adapter 已新增 `selection -> MessageRequest` 构建能力
  - 下一步应继续完成 Stage 1 的 donor 清洗和核心命名/配置面适配，然后再扩大到请求主链与 adapter 深化。
- 截至今晚的新进展：
  - Rust `openai_compat` 已补上真实上游响应兼容：
    - `tool_calls: null` 不再导致解码崩溃
    - live one-shot 已能真实返回 `ok`
  - Rust runtime 已补当前真实 `saicode` 设置兼容：
    - `~/.saicode/settings.json` 里的 `model = "cpa/gpt-5.4"` 会在 launch 前自动归一化成 `gpt-5.4`
    - provider 前缀模型别名不再把请求打到错误模型名
  - `saicode-rust-one-shot` 已从实验命令升级为兼容 `saicode -p` 的简单恢复/print 路径：
    - 支持 `-p/--print`
    - 支持 `--output-format json`
    - 支持 `--system-prompt` / `--system-prompt-file`
    - 支持 `--append-system-prompt`
  - native launcher 的 `Recovery` 路由已优先切到：
    - `/home/ubuntu/saicode/rust/target/release/saicode-rust-one-shot`
    - 若未构建或显式禁用，仍可回退旧恢复路径
  - 安装与收尾脚本已同步收口：
    - [install_vps_jp.sh](/home/ubuntu/saicode/scripts/install_vps_jp.sh) 会一起构建 Rust one-shot
    - [closeout_preflight.sh](/home/ubuntu/saicode/scripts/closeout_preflight.sh) 会检查并可 live probe Rust one-shot
  - 入口可用性已恢复：
    - `./bin/saicode --version` 正常
    - `./bin/saicode --help` 正常
    - `./bin/saicode` 已可真实进入交互界面，不再因 `MACRO is not defined` 崩溃
    - `SAICODE_CLOSEOUT_LIVE=1 ./scripts/closeout_preflight.sh` 已通过
  - 当前可量化速度结论：
    - 新 Rust recovery 路径：`./bin/saicode -p 'Reply with exactly: ok'` 约 `1.69s`
    - 旧 native recovery 路径：约 `2.69s`
    - Bun fallback recovery 路径：约 `1.98s`
  - 本轮继续收口后的高频 tool-loop 结果：
    - native / Rust 侧已统一支持标准 `--` prompt separator：
      - recovery
      - native local-tools
      - Rust one-shot
      - lightweight headless
    - native WebSearch 已改为：
      - 本地 `sai-search` HTTP 优先
      - SSH 仅做兜底
    - native local-tools 默认模型已切到小快模型：
      - 默认 `cpa/gpt-5.4-mini`
      - 显式 `--model` 仍然覆盖
    - `native local-tools` 已进一步从 launcher crate 内部逻辑提升为 `rust/` workspace 下的独立 Rust binary：
      - [Cargo.toml](/home/ubuntu/saicode/rust/crates/saicode-rust-local-tools/Cargo.toml)
      - [main.rs](/home/ubuntu/saicode/rust/crates/saicode-rust-local-tools/src/main.rs)
      - launcher 现已优先路由到：
        - `/home/ubuntu/saicode/rust/target/release/saicode-rust-local-tools`
    - recovery/local-tools 共享源头已正式提升为 workspace library：
      - [Cargo.toml](/home/ubuntu/saicode/rust/crates/saicode-frontline/Cargo.toml)
      - [lib.rs](/home/ubuntu/saicode/rust/crates/saicode-frontline/src/lib.rs)
      - [recovery.rs](/home/ubuntu/saicode/rust/crates/saicode-frontline/src/recovery.rs)
      - [local_tools.rs](/home/ubuntu/saicode/rust/crates/saicode-frontline/src/local_tools.rs)
      - `native/saicode-launcher/src/` 已不再保留 recovery/local-tools 源文件，launcher 与 Rust binaries 统一依赖 `saicode-frontline`
    - 当前高频命令实测：
      - `Read` native local-tools：约 `1.76s`
      - `Read` Bun fallback：约 `5.54s`
      - `Grep` native local-tools：约 `2.13s`
      - `WebSearch` native local-tools：约 `4.39s`
      - `WebSearch` Bun fallback：约 `9.35s`
    - 收到独立 Rust binary 后再次实测：
      - `Read` via `saicode-rust-local-tools`：约 `2.68s`
      - `WebSearch` via `saicode-rust-local-tools`：约 `5.35s`
      - 均已由 launcher trace 明确确认 target 为 `rust/target/release/saicode-rust-local-tools`
    - workspace library 收口后再次实测：
      - `Read` via `saicode-frontline -> saicode-rust-local-tools`：约 `1.48s`
      - `WebSearch` via `saicode-frontline -> saicode-rust-local-tools`：约 `7.48s`
      - `closeout_preflight` 现已要求 native local-tools probe 明确命中新 Rust binary target
  - 当前阶段判断：
    - Epic D 已拿到真实请求链可用性
    - Epic E 的 `E1. 让 -p/--print 默认走 Rust Core` 对简单恢复路径已初步达成
    - Epic E 的高频 tool-loop 也已经拿到一轮可量化收益
    - launcher 现已基本退化为路由壳与 warm-headless 壳，frontline 逻辑源头已迁入 workspace
    - 下一步的主要剩余项已集中到 interactive/runtime 主链本身，而不是高频 one-shot/tool-loop

## 0. 执行原则

- 这是一份母计划，不再按“下一步做什么”零散推进。
- 后续执行默认按本计划连续推进，不在每个子步骤重新请示。
- 内部仍然分阶段实施，但外部目标是“一次性把该做的任务全部定下来，然后持续做到收口”。
- 原则固定为：
  - `kcode` 作为 Rust donor base
  - `saicode` 保留自己的前端显示页与产品定制层
  - 先替换底座，再桥接表层，再切主入口，最后清理旧实现

## 1. 总体任务树

### Epic A：建立新的 Rust 主底座

目标：
- 在 `saicode` 仓库内落一套可持续演进的 Rust workspace，不再只剩 launcher/fastpath。

子任务：
- A1. 建立新的 Rust workspace 根目录与 crate 布局
- A2. 以 `kcode` 为 donor 导入基础 crates：
  - `api`
  - `runtime`
  - `tools`
  - `commands`
  - 必要时补：
    - `telemetry`
    - `plugins`
    - `bridge`
    - `adapters`
- A3. 统一命名与路径，去掉 `kcode` 自身品牌名残留
- A4. 固定本仓库的 Rust toolchain / lockfile 策略，解决当前环境可编译性问题
- A5. 补通本机 `cargo test` / `cargo build --release` / CI 级基础命令

验收：
- `saicode` 仓库内出现可独立编译的 Rust workspace
- donor crates 不再依赖原 `kcode` 仓库路径
- 本机可稳定跑 Rust 构建和至少一轮测试

### Epic B：清洗 donor，去掉不可直接继承项

目标：
- 把 `kcode` 里不适合直接当生产底座的半成品、占位逻辑和不符合 `saicode` 语义的部分先剥离清楚。

子任务：
- B1. 标记并处理 stub / 半实现能力：
  - task runtime
  - MCP auth
  - remote trigger
  - testing permission
- B2. 标记并处理 `plan_mode` 语义差异
- B3. 标记并处理 bridge / webhook 外部依赖差异
- B4. 标记并处理 `/effort` 仅命令面、未落请求体的问题
- B5. 对 donor 代码补 “可继承 / 需适配 / 暂不接” 清单

验收：
- donor 中的风险点不再被误写成“已完成能力”
- 新底座不会把 stub 逻辑直接带入 `saicode`

### Epic C：建立 saicode-core adapter 层

目标：
- 在 Rust Core 和 `saicode` 现有前端显示层之间建立稳定适配层，而不是让前端直接吃 donor 原始结构。

子任务：
- C1. 定义 `saicode` 需要的核心运行时协议：
  - session message model
  - tool use / tool result event model
  - thinking / redacted thinking 表示
  - usage / token / cost 表示
  - permission request / approval result 表示
- C2. 建立 Rust -> `saicode` UI 的消息映射
- C3. 建立 `saicode` UI -> Rust Core 的输入映射
- C4. 建立 slash command / local command / prompt command 的统一适配面
- C5. 建立 model / effort / fast-mode 状态映射
- C6. 建立 settings / config / profile 的统一读取口径

验收：
- 已明确 Rust Core 和现有前端之间的边界协议
- 前端不需要理解 donor 私有结构即可驱动新底座

### Epic D：替换 provider / request / model 主链

目标：
- 把模型请求真正从 Bun/TS runtime 切到 Rust Core，而不是只把本地工具做轻。

子任务：
- D1. 迁移 provider profile / model alias / base URL / api key 解析
- D2. 接入当前 `saicode` 真正在用的 provider 路线：
  - `cpa / cliproxyapi`
  - `nvidia`
  - 其他已保留模型面
- D3. 打通 OpenAI-compatible request builder
- D4. 把 `saicode` 当前模型 catalog / alias / 显示名同步到 Rust Core
- D5. 补真实 `reasoning_effort / effort` 打通：
  - 请求体字段
  - 模型支持矩阵
  - UI -> runtime -> provider 全链路
- D6. 补流式响应、tool call、usage 聚合

验收：
- `-p/--print` 的模型请求可由 Rust Core 直接完成
- `gpt-5.4` 路线与 effort 参数能真实生效

### Epic E：替换 one-shot / print / 高频工具执行链

目标：
- 先吃掉收益最大的非交互和工具调用路径，让“轻”和“快”先落到主用面。

子任务：
- E1. 让 `-p/--print` 默认走 Rust Core
- E2. 让当前已经 native 化的工具统一并入新 Rust Core，而不是继续分散在 launcher fastpath
- E3. 打通高频本地工具：
  - Read
  - Grep
  - Glob
  - Bash
  - Write
  - Edit
- E4. 打通 WebSearch / WebFetch
- E5. 打通 one-shot session persistence / resume 元数据
- E6. 跑探针矩阵并做性能基线回归：
  - 纯文本
  - Read
  - Grep
  - Write
  - Edit
  - WebSearch
  - WebFetch
  - effort 请求

验收：
- 高频 one-shot 请求不再依赖当前 Bun 主 runtime
- 端到端 wall time / RSS 至少维持当前 native fastpath 收益，不反向变重

### Epic F：保留现有前端显示层，替换交互式 runtime

目标：
- 用户继续看到熟悉的 `saicode`，但交互式会话底层执行换成 Rust Core。

子任务：
- F1. 明确前端保留面：
  - [REPL.tsx](/home/ubuntu/saicode/src/screens/REPL.tsx)
  - [PromptInput.tsx](/home/ubuntu/saicode/src/components/PromptInput/PromptInput.tsx)
  - [Messages.tsx](/home/ubuntu/saicode/src/components/Messages.tsx)
  - [ModelPicker.tsx](/home/ubuntu/saicode/src/components/ModelPicker.tsx)
  - `src/components/messages/*`
  - `src/components/permissions/*`
  - `src/components/tasks/*`
- F2. 把 REPL 的 query / message loop 改接 Rust Core
- F3. 把 tool progress / permission dialogs / task status 改接 Rust Core 事件
- F4. 把 ModelPicker / Effort UI 改接 Rust Core model state
- F5. 把 slash command palette / suggestions 改接 Rust Core command registry
- F6. 保证当前 UI 行为不因 donor 默认 TUI 语义而漂移

验收：
- 现有 `saicode` 交互页面仍保留
- 交互式主链已不再依赖旧 Bun runtime 完成核心执行

### Epic G：命令面与产品语义对齐

目标：
- Rust Core 能跑不够，还要让用户看到的命令面仍然是 `saicode` 的习惯语义。

子任务：
- G1. 对齐 `/model` 语义与当前 `saicode` 模型目录
- G2. 对齐 `/effort`、`ModelPicker`、`EffortCallout` 的产品语义
- G3. 对齐 `/plan` / plan mode / approval 语义
- G4. 对齐 `/doctor` / `/status` / `/config` 行为
- G5. 对齐 task / team / background 行为，只保留已真实可用面
- G6. 对齐 plugin / skill / MCP 的产品入口文案和约束

验收：
- 关键 slash command 不出现“看起来在，行为却不是当前 saicode 那套”的漂移

### Epic H：MCP / plugin / bridge / adapter 收口

目标：
- 对外围能力做分类处理，避免一上来全量迁移把主线拖死。

子任务：
- H1. MCP：
  - 先保留当前 `saicode` 可用 MCP 主面
  - donor 中未成熟的 auth / proxy / registry 只按需要吸收
- H2. plugin：
  - 保留 `saicode` 当前 plugin 行为语义
  - 只复用 donor 中稳定的 registry / metadata 结构
- H3. bridge / adapters：
  - Telegram 如当前确有高频使用，再收口
  - WhatsApp / Feishu 不默认前置
- H4. 对桥接面做“必接 / 可后接 / 暂不接”三档划分

验收：
- 外围能力迁移不阻塞主 CLI / REPL / one-shot 主线

### Epic I：入口切换与双轨过渡

目标：
- 在迁移期间维持可用，不搞一次性硬切。

子任务：
- I1. 保留旧入口作为 fallback
- I2. 新增 Rust Core 驱动入口
- I3. 做 feature flag / route 选择：
  - one-shot 先切
  - interactive 再切
- I4. 增加对照模式：
  - old runtime
  - new runtime
  - compare mode
- I5. 明确会话、日志、memory、config 的兼容策略

验收：
- 迁移期间用户仍可进入 `saicode`
- 任一阶段失败都有明确 fallback

### Epic J：验证、压测、切主、清理旧实现

目标：
- 完成最终切换，不让新旧长期并存造成维护负担。

子任务：
- J1. 补 Rust 测试门
- J2. 补 TypeScript/UI 适配层测试门
- J3. 跑真实端到端探针：
  - `--help`
  - `-p`
  - slash commands
  - tools
  - effort
  - model switching
  - resume/session
- J4. 跑性能对比：
  - cold start
  - first token
  - RSS
  - tool roundtrip
- J5. 切主入口到新底座
- J6. 删除或收口旧 Bun runtime 残留
- J7. 回写 docs / MISTAKEBOOK / SESSION_LEDGER / rollout 文档

验收：
- 默认入口由新 Rust 主底座接管
- 旧实现不再误导后续维护

## 2. 依赖关系

### 关键路径

- Epic A -> Epic B -> Epic C -> Epic D -> Epic E -> Epic F -> Epic I -> Epic J

### 可并行段

- Epic G 可在 Epic C/D 之后并行推进
- Epic H 可在 Epic D/E 之后按需并行推进

## 3. 默认执行顺序

### Stage 1：底座落地

- 完成 Epic A
- 完成 Epic B

### Stage 2：协议与主链

- 完成 Epic C
- 完成 Epic D
- 完成 Epic E

### Stage 3：保留表层，替换内核

- 完成 Epic F
- 完成 Epic G

### Stage 4：外围与切换

- 完成 Epic H
- 完成 Epic I
- 完成 Epic J

## 4. 明确哪些可以直接搬

- `kcode` Rust workspace / crate 分层
- `api`
- `runtime`
- `tools`
- `commands`
- provider profile / session persistence / memory 主干

## 5. 明确哪些不能原样搬

- `kcode` 自带默认 TUI 不能直接替代当前 `saicode` 前端显示页
- `plan_mode` 语义不能原样搬
- `/effort` 目前不能视为已完成实现
- donor 中的 stub / placeholder tool 不能原样搬
- Telegram webhook 等外部依赖链不能默认写成“已接好”

## 6. 本轮结论

- 可以不再一步一步零碎推进。
- 已经把后续该做的任务全部拆成：
  - `10` 个大任务
  - 每个大任务下的执行小任务
  - 明确的依赖关系
  - 清晰的切换顺序
- 后续默认就按这份总计划连续做，不再每轮重新定义任务树。
