# saicode Rust Skill/MCP/Tool Surface Closeout Plan

## Phase 1

Create:
- 任务级 `SPEC.md`
- 任务级 `PLAN.md`

Verify:
- 文档覆盖五项任务、约束和验收面

## Phase 2

Modify:
- `/home/ubuntu/saicode/rust/crates/tools/src/types.rs`
- `/home/ubuntu/saicode/rust/crates/tools/src/todo_skill.rs`
- `/home/ubuntu/saicode/rust/crates/tools/src/tests_agent.rs`

Work:
- 扩展 skill 输出结构
- 解析 `SKILL.md` 描述
- 发现 `scripts/`、`assets/`、`templates/`、相邻引用文件
- 返回执行就绪的结构化技能元数据

Verify:
- `cargo test -p tools skill_...`

## Phase 3

Modify:
- `/home/ubuntu/saicode/rust/crates/saicode-rust-cli/src/main.rs`

Work:
- 在默认工具面注入动态 MCP 工具定义
- 统一 builtin/plugin/MCP 的显示名、权限和执行映射
- 让执行器直接路由动态 MCP 工具

Verify:
- `cargo test -p saicode-rust-cli`
- MCP 注入相关单测

## Phase 4

Modify:
- `/home/ubuntu/saicode/native/saicode-launcher/src/main.rs`

Work:
- 对齐快速帮助文本
- 对齐当前 Rust CLI 输出格式、命令和说明
- 核对 route probe 假设

Verify:
- `./bin/saicode --help`
- `SAICODE_NATIVE_DRY_RUN=1 ./bin/saicode ...`

## Phase 5

Modify:
- `/home/ubuntu/saicode/scripts/rust_tool_acceptance.sh`
- `/home/ubuntu/saicode/scripts/closeout_preflight.sh`

Work:
- 增加流式输出、交互过程、LSP、skill、MCP、launcher/help、基础搜索工具验收
- 将 closeout 与 acceptance 对齐到当前 Rust 能力面

Verify:
- `cargo test -p tools -- --nocapture`
- `cargo test -p saicode-rust-cli -- --nocapture`
- `cargo build --release -p saicode-rust-cli`
- `scripts/closeout_preflight.sh`
- 针对新增 acceptance 能力的最小脚本实跑
