# saicode Closeout Workflow

这份流程用于把一次改动从“本地改完”收口到“命令能用、回归已过、可以推 GitHub”。

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

## 3. 真实可用性验收

在当前机器已经配好 `~/.saicode/config.json` 时，再跑真实请求：

```bash
SAICODE_CLOSEOUT_LIVE=1 ./scripts/closeout_preflight.sh
```

这一步要求 `saicode -p "Reply with exactly: ok"` 真正返回 `ok`。

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
git add <本轮文件>
git commit -m "fix: restore saicode installed entry and add closeout workflow"
git push origin <current-branch>
```

推送后再看 GitHub Actions 的 `ci` 是否通过。

## 6. 停止条件

只有下面这些都满足，才算本轮真正可以收：

- `saicode --help` 正常
- 真实 `-p` 请求正常
- 自动化回归正常
- closeout preflight 正常
- 文档和计划已回写
- GitHub 远端校验正常
