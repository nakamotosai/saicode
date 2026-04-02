# saicode lite 裁剪 Spec

## Goal

把当前 `saicode` 收口到“主链路保留、Anthropic 私有产品旁支从实际运行面摘除”的状态。

## Scope

### In scope

- 从实际可达运行面裁掉这 6 类旧入口：
  - `chrome`
  - `computer use`
  - `bridge / remote-control`
  - `assistant`
  - `login/logout/usage/upgrade/passes/privacy-settings` 一类 first-party 账号与计费面
  - `desktop/mobile/share/teleport/remote-setup` 一类远端产品壳
- 保留并不破坏当前主力：
  - CLI / TUI
  - provider runtime
  - model registry
  - Bash / Read / Edit / Write / Grep / Glob / WebSearch / WebFetch
  - sessions / permissions / MCP / plugins / skills
- 同步清掉相关高可见设置项、帮助入口和误导性文案

### Out of scope

- 收缩或删除现有模型列表
- 新增 GUI / WebUI / fallback 路由
- 全仓彻底删除所有历史源码文件
- 重写浏览器自动化或 computer-use 为新实现

## Constraints

- 不破坏当前已经跑通的 `cliproxyapi / NVIDIA / WebSearch / WebFetch` 主链路
- 优先摘除入口、关闭默认行为、清理高可见设置项；不强求本轮物理删除所有深层文件
- 保持 `saicode` 品牌和 `.saicode / SAICODE.md` 语义不回退
- 验收以真实 CLI/TUI 行为优先

## Acceptance

任务完成时至少满足：

1. `SPEC.md` / `PLAN.md` 与当前“lite 裁剪”目标一致
2. `saicode -v`、`saicode --help`、`saicode -p "Reply with exactly: ..."` 真实通过
3. 以下入口从主使用面消失：
   - `/chrome`
   - `/assistant`
   - `/remote-control`
   - `/login` `/logout`
   - `/usage` `/upgrade` `/passes` `/privacy-settings`
   - `/desktop` `/mobile`
4. `--chrome`、`--assistant`、`--teleport`、`--remote`、`--remote-control` 不再作为可用入口
5. Chrome / Remote Control 相关设置项和 Usage 页不再出现在设置主界面
6. 主链路能力不回归

## Risks

- 这轮主要是“从运行面摘入口”，不是全仓物理删除，深层文件与类型残留仍可能存在
- `teleport / remote / bridge / chrome / computer-use` 深层实现仍可能被少量内部模块引用，但不应再对外暴露
- 未来若要浏览器自动化或 computer use，应该新建 `saicode` 自己的实现，而不是恢复这些旧分支
