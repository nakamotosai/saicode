# saicode lite 裁剪 Plan

## 1. 规格对齐

- 更新 `SPEC.md` / `PLAN.md`
- 明确当前任务是把 6 类旧旁支从主运行面摘掉

## 2. 命令与入口裁剪

- 从 `commands.ts` 摘掉旧命令入口
- 从 `main.tsx` 摘掉旧 CLI flags、assistant/remote-control 兼容入口
- 关闭 Chrome 和 Remote Control 的默认启用逻辑

## 3. 设置与提示面收口

- 从 Settings 页移除 Usage / Chrome / Remote Control 相关设置
- 清掉仍会误导用户走旧旁支的提示文案

## 4. 运行面回归

- 验证 `saicode -v`
- 验证 `saicode --help`
- 验证 `saicode -p "Reply with exactly: saicode lite ok"`
- 验证旧入口命令不再出现在帮助或 slash commands 中

## 5. 经验沉淀

- 把这轮“Anthropic 私有产品旁支应优先摘入口，而不是继续兼容”的经验回写到错题本和会话账本
