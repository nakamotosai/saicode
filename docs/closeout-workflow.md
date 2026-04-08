# saicode Closeout Workflow

这份流程用于把一次改动从“本地改完”收口到“命令能用、回归已过、可以推 GitHub”。

先统一一个运行时事实：

- `./bin/saicode` 是唯一入口
- 当前配置默认模型仍是 `cpa/qwen/qwen3.5-122b-a10b`
- 为了稳定性，工具型请求、one-shot、recovery，以及 qwen 函数调用 degraded 的重试路径，会自动回退到 `cpa/gpt-5.4-mini`
- closeout 要验证的是“当前真实运行链路可用”，不是“每一次都必须直打 qwen 才算通过”

## 1. 入口可用性

先确认最容易坏的几个入口都正常：

```bash
./bin/saicode --help
SAICODE_DISABLE_NATIVE_LAUNCHER=1 ./bin/saicode --help
saicode --help
SAICODE_DISABLE_NATIVE_LAUNCHER=1 saicode mcp --help
```

如果 `saicode` 是通过软链安装的，还要确认软链入口不是把仓库根目录误判成 `~/.local`，并且 full CLI fallback 不会因为缺少旧前端文件而失效。

## 2. 本地收尾预检

先跑自动化收尾预检：

```bash
./scripts/closeout_preflight.sh
```

这一步会检查：

- repo wrapper 可用
- full CLI fallback 可用
- symlink 入口可用
- 已安装命令入口可用
- native release 路径是否存在
- 交互工具进度 probe 正常结束且不遗留额外 `saicode` 进程

## 3. 真实可用性验收

在当前机器已经配好 `~/.saicode/config.json` 时，再跑真实请求：

```bash
SAICODE_CLOSEOUT_LIVE=1 ./scripts/closeout_preflight.sh
```

这一步要求 `saicode -p "Reply with exactly: ok"` 真正返回 `ok`。

若要把全量工具面也一起验收，直接跑：

```bash
SAICODE_CLOSEOUT_LIVE=1 SAICODE_CLOSEOUT_ACCEPTANCE=1 ./scripts/closeout_preflight.sh
```

当前 acceptance 默认模型应与仓库默认模型一致，默认走 `cpa/qwen/qwen3.5-122b-a10b`；若需要额外比对小模型，可显式设置 `SAICODE_FAST_ACCEPT_MODEL`。

这里的含义是：

- `SAICODE_ACCEPT_MODEL` 决定 acceptance 的配置默认模型口径
- 运行时如果遇到当前已知的 qwen degraded function invocation / 无 token / 不收尾问题，仍会在 Rust 代码里自动回退到 `cpa/gpt-5.4-mini`
- `SAICODE_FAST_ACCEPT_MODEL` 用于显式覆盖这条稳定回退模型
- acceptance 内的 TTFT bench 只统计真正拿到首 token 的模型；能列出但不可调用的模型会被跳过

## 4. 代码与文档收口

提交前至少再看一遍：

- `git status --short`
- `git diff --check`
- 本轮 `SPEC.md` / `PLAN.md`
- README 是否已经反映新的真实入口与验证方式
- 是否补了对应测试或防回归脚本

## 5. GitHub 提交流程

建议按这个顺序：

```bash
git status --short
git diff --check
cargo test --manifest-path native/saicode-launcher/Cargo.toml
(cd rust && ../scripts/rust-cargo.sh test -q -p api -p runtime -p commands -p tools -p saicode-frontline -p saicode-rust-cli -p saicode-rust-one-shot -p saicode-rust-local-tools)
./scripts/closeout_preflight.sh
SAICODE_CLOSEOUT_LIVE=1 ./scripts/closeout_preflight.sh
SAICODE_CLOSEOUT_LIVE=1 SAICODE_CLOSEOUT_ACCEPTANCE=1 ./scripts/closeout_preflight.sh
git add <本轮文件>
git commit -m "fix: close out non-frontend runtime and acceptance"
git push origin <current-branch>
```

推送后再看 GitHub Actions 的 `ci` 是否通过。

## 6. 停止条件

只有下面这些都满足，才算本轮真正可以收：

- `saicode --help` 正常
- 真实 `-p` 请求正常
- `./scripts/rust_tool_acceptance.sh` 正常
- 自动化回归正常
- closeout preflight 正常
- 文档和计划已回写
- `git status --short` 干净
- GitHub 远端校验正常
