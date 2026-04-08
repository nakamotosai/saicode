# Saicode Non-Frontend Remaining Boundary Closeout

## Goal

在前一轮非前端 closeout 已完成的基础上，把当前仅剩的“模型配置口径、实际稳定执行口径、用户可见状态口径未完全统一”这一条边界收口到可验证、可解释、可长期维护的状态。

本轮目标不是再做一次全量大扫除，而是把 `saicode` 的 provider/runtime 真相面收拢成单一、稳定、无隐藏漂移的行为：

- 用户能看清当前配置默认模型是什么
- 用户能看清当前实际稳定执行会用什么模型
- runtime 的回退规则是显式、可探针、可验收的
- 若 upstream 已恢复到 qwen 全面可用，则优先把回退移除并统一到 qwen
- 若 upstream 仍不稳定，则把“隐藏回退”升级成“健康感知的显式路由”，使仓库内部不再留未收口边界

## Scope

### In scope

- `./bin/saicode`、`status`、`doctor`、`config show`
- `native/saicode-launcher`
- `rust/crates/saicode-rust-cli`
- `rust/crates/saicode-rust-one-shot`
- `rust/crates/saicode-frontline`
- 与 provider / model routing / fallback 直接相关的 runtime 代码
- `scripts/rust_tool_acceptance.sh`
- `scripts/closeout_preflight.sh`
- `README.md`
- `README.en.md`
- 本轮任务级 `SPEC.md` / `PLAN.md`

### Out of scope

- 浏览器前端和视觉前端
- 与 provider/runtime 真相面无关的功能扩张
- 更换上游 provider 产品策略本身

## Current Boundary

截至基线提交 `808c1b8ed77704140c4af026d96a959635906483`：

- `config show` 与 `status` 仍展示 `cpa/qwen/qwen3.5-122b-a10b`
- 但工具型请求、one-shot、recovery 的稳定执行会在 runtime 内回退到 `cpa/gpt-5.4-mini`
- 当前行为对熟悉代码的人是可解释的，但对 CLI 使用者仍然偏“隐式”
- 因而仓库内部还残留一条产品化边界：配置真相、执行真相、状态真相没有完全合一

## Constraints

- 不触碰前端范围
- 不把“外部 provider 自己不稳定”误写成“仓库内部还没做完”
- 不再引入第二入口、第二份主配置或新的临时兼容层
- 没有 live probe 和真实命令验证，不算完成
- 若要继续保留 fallback，必须让 fallback 可观测、可诊断、可验收，而不是只在实现里偷偷发生

## Acceptance

1. `status` 或 `doctor` 至少有一个主入口能明确展示：
   - 配置默认模型
   - 当前 plain chat 有效执行模型
   - 当前 tool-capable / one-shot 有效执行模型
   - 若发生回退，能看到回退原因或健康状态摘要
2. runtime 的模型选择不再只是硬编码隐藏行为，而是满足以下二选一：
   - qwen 在当前 provider 上通过 plain chat 与 tool-capable live probe，统一直接执行
   - qwen 仍不稳定时，fallback 由显式健康探针/能力判定驱动，并被状态面准确暴露
3. `rust_tool_acceptance.sh` 与 `closeout_preflight.sh` 能覆盖：
   - qwen plain chat probe
   - tool-capable probe
   - fallback 行为 probe
   - 无首 token / degraded / hang 的防卡死门
4. README 与 README.en 明确说明当前最终口径：
   - 若已统一到 qwen，则不再写隐式 fallback
   - 若仍需 fallback，则明确这是健康感知的稳定路由，而不是隐藏残留
5. 本轮如有代码改动，相关测试、脚本、README、状态输出一起收口，并完成 commit / push / 干净工作树

## Non-Goals

- 不承诺让任意第三方模型都变成工具稳定面
- 不承诺解决外部 provider 全部可用性问题
- 不重新开启多入口或 TS/Bun 回退面

## Risks

- 上游 provider 可能在今天和明天表现不同，因此所有结论都必须绑定真实 probe 结果
- 若只改 README 或 status 文案，不改运行时判定逻辑，边界不会真正消失
- 若只改运行时，不补状态输出和脚本验证，后续仍会再次回到“实现知道、用户不知道”的隐式状态
