# Saicode Non-Frontend Rust Stability

## Goal

在不处理浏览器/视觉前端的前提下，审计并收口 `saicode` 当前 Rust 重写后的非前端能力，确保后端、CLI、native launcher、session/runtime、工具链与相关脚本达到“可构建、可验证、关键链路可稳定使用”的状态。

本轮不以 README 或阶段性计划口径作为完成标准，只以本地代码事实、自动化验证和真实命令探针为准。

## Scope

### In scope

- `rust/` workspace 下所有非前端 crate
- `native/saicode-launcher`
- `bin/saicode` 包装入口及其相关安装链路
- 非前端脚本、构建链路、测试链路、安装/验收脚本
- 与上述链路直接相关的 README / SPEC / PLAN / 文档口径
- Rust cutover 后残留的旧 TS/Bun 运行时耦合、假切换或断链风险

### Out of scope

- 浏览器前端、视觉布局、样式、前端交互体验
- 纯界面层美观问题
- 与本轮稳定性无关的新功能扩张

## Constraints

- 不把“能编译部分 crate”误判成“整个非前端面稳定”
- 不把“README 已宣称纯 Rust”误判成“运行时已经完全切断旧链路”
- 不以单一 `cargo test` 或单一 smoke 替代端到端判断
- 发现问题优先分级：阻塞性 bug、稳定性风险、覆盖缺口、文档失真
- 若存在可控且局部的阻塞问题，直接修；若属于大范围设计缺口，先明确最小收口动作

## Acceptance

1. 能清楚给出 Rust 重写非前端范围的真实完成度，而不是口头判断
2. 至少完成一轮静态审计，覆盖 launcher、workspace crate、入口脚本、测试与文档口径
3. 至少完成一轮动态验证，覆盖 build/test 和真实 CLI 主链路 smoke
4. 所有发现按严重度分类，并标明对应文件或验证面
5. 若发现阻塞“稳定使用”的问题，能给出已修复结果或最小后续收口动作
6. 最终结论必须回答：现在非前端哪些已稳定、哪些未稳定、哪些是假完成

## Risks

- 当前 worktree 处于大规模 cutover 中，README 与真实代码可能脱节
- 旧 TS/Bun 代码已大面积删除，任何遗漏的运行时依赖都会在真实命令中暴露为硬失败
- Rust workspace 看起来完整，不代表 crate 之间的集成、内部会话/桥接协议和 wrapper 已一致
- 若验证面只覆盖 `--help` 之类快路径，会漏掉真正的请求、工具和 runtime 问题
