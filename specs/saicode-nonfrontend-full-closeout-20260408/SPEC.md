# Saicode Non-Frontend Full Closeout

## Goal

在不处理浏览器/视觉前端的前提下，把 `saicode` 的整个非前端面收口到“默认入口可直接用、核心工具面可真实调用、常见交互与一次性任务不卡顿、回归与 closeout 可重复执行”的状态。

本轮完成标准不是“Rust 已经替代 TS”，而是“用户在当前机器上使用 `saicode` 做非前端工作时，不再频繁遇到断链、假帮助、工具不可调用、调用后卡住、退出不干净、验证脚本失真”等问题。

## Scope

### In scope

- `bin/saicode`
- `native/saicode-launcher`
- `rust/` workspace 下所有非前端 crate
- 非前端脚本、closeout、acceptance、安装链路
- 工具与命令面：
  - builtin tools
  - skill
  - MCP
  - plugin
  - LSP
  - session / resume / compact / status / config / doctor
- 一次性 `-p`、交互 `--bare`、默认 REPL、launcher fallback、已安装软链入口
- 与以上能力直接相关的 README / SPEC / PLAN / 验收脚本口径

### Out of scope

- 浏览器前端、视觉布局、样式、前端交互体验
- 新产品能力扩张
- 远端机器协同、分布式调度、与本轮“本机非前端可用性”无关的运维工程

## Constraints

- 不把“单次 `ok` 请求成功”误判成“整套非前端稳定”
- 不把“工具能被列出来”误判成“工具可真实执行且不会卡住”
- 不把“有 acceptance 脚本”误判成“脚本已覆盖当前真实默认模型和真实工具面”
- 不引入第二入口、第二套运行时真相或新的过渡兼容层
- 发现阻塞稳定使用的问题时，优先修当前真链路，而不是继续堆文档解释
- 没有真实命令探针和自动化验证，不算完成

## Acceptance

1. `./bin/saicode` 作为唯一入口，以下链路都能稳定通过：
   - `--help`
   - `status`
   - `config show`
   - `-p`
   - 默认交互模式
   - `--bare`
   - 已安装命令软链入口
2. 核心 builtin 工具至少以下类别全部有真实探针通过，且输出符合任务意图：
   - `Read`
   - `Grep` / 搜索
   - `Glob`
   - `Write`
   - `Edit`
   - `Bash`
   - `WebSearch` / `WebFetch`（若当前默认工具面声明可用）
3. 非 builtin 能力至少以下面全部有真实探针通过：
   - skill
   - MCP
   - plugin
   - LSP
4. 交互式工具调用不再出现“工具事件已发生但前台卡住不收尾”的已知不稳定行为；至少覆盖：
   - 工具进度输出
   - 最终答复输出
   - `/exit` 正常退出
   - 子进程不残留
5. closeout / acceptance 体系与当前真实默认模型、默认入口、当前支持面一致，不再依赖过时模型或过时命令假设
6. 至少一套完整自动化验收可在本机复跑，并明确区分：
   - 快速回归
   - 全量工具面验收
   - live probe
   - 性能/不卡顿门
7. README 与 closeout 文档能准确说明：
   - 当前唯一入口
   - 当前默认模型
   - 当前验证命令
   - 当前已知边界

## Non-Goals

- 不承诺“所有未来扩展工具零成本接入”
- 不承诺解决与前端体验直接相关的问题
- 不承诺绕过外部 provider / 网络 / 第三方服务本身的故障

## Risks

- 当前 acceptance 默认模型仍与仓库主默认模型存在漂移风险
- 工具面覆盖范围大，若只修单点 hang，容易遗漏同类交互收尾问题
- MCP / LSP / Web 这类能力依赖额外环境或外部服务，若验收夹具不足，容易形成假通过
- 交互链路可能存在“功能正确但退出/收尾/子进程管理不稳定”的隐蔽问题
