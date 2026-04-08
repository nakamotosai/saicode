# saicode `@ts-nocheck` 清零报告

日期：
- 2026-04-06

## Baseline

- 起始数量：`227`
- 当前数量：`197`

## 当前状态

- 已建立专项 Spec / Plan / Inventory
- 正在执行 Phase 1：低风险叶子批

## 批次记录

### Phase 0

- 已完成 inventory 固定
- 已完成分波次策略

### Phase 1

- 已完成第一批低风险叶子文件，`227 -> 212`
- 已清理文件：
  - `src/skills/bundled/verifyContent.ts`
  - `src/hooks/notifs/useInstallMessages.tsx`
  - `src/components/SentryErrorBoundary.ts`
  - `src/utils/secureStorage/index.ts`
  - `src/utils/computerUse/swiftLoader.ts`
  - `src/utils/computerUse/inputLoader.ts`
  - `src/components/MemoryUsageIndicator.tsx`
  - `src/tasks/types.ts`
  - `src/hooks/useUpdateNotification.ts`
  - `src/utils/settings/validateEditTool.ts`
  - `src/utils/secureStorage/fallbackStorage.ts`
  - `src/utils/secureStorage/plainTextStorage.ts`
  - `src/utils/aws.ts`
  - `src/utils/fingerprint.ts`
  - `src/utils/generators.ts`
- 为支撑首批清理，已同步补齐：
  - `src/globals.d.ts`
  - `src/utils/secureStorage/types.ts`
  - `src/services/mcp/auth.ts`
  - `src/services/mcp/xaaIdpLogin.ts`
  - `src/tasks/pillLabel.ts`
  - `src/hooks/useBackgroundTaskNavigation.ts`

### Phase 2

- 已完成第二批低风险文件，`212 -> 203`
- 已清理文件：
  - `src/skills/bundled/claudeApiContent.ts`
  - `src/services/compact/postCompactCleanup.ts`
  - `src/services/lsp/config.ts`
  - `src/services/tips/tipScheduler.ts`
  - `src/tools/AgentTool/builtInAgents.ts`
  - `src/utils/dxt/helpers.ts`
  - `src/utils/execFileNoThrowPortable.ts`
  - `src/utils/permissions/classifierDecision.ts`
  - `src/utils/telemetryAttributes.ts`
- 已补齐：
  - `src/services/lsp/types.ts`
  - `src/services/tips/types.ts`
  - `src/missing-module-shims.d.ts`

### Phase 3

- 已完成第三批清理，`203 -> 197`
- 已清理文件：
  - `src/hooks/notifs/useCanSwitchToExistingSubscription.tsx`
  - `src/hooks/useChromeExtensionNotification.tsx`
  - `src/utils/computerUse/gates.ts`
  - `src/utils/computerUse/hostAdapter.ts`
  - `src/utils/computerUse/setup.ts`
  - `src/components/messages/nullRenderingAttachments.ts`
- 已同步增强：
  - `src/globals.d.ts`

### Current

- `bun run typecheck` 通过
- `bun test` 通过
- 当前剩余 `@ts-nocheck` 文件数：`197`

## 验收门禁

最终必须通过：

- `bun run typecheck`
- `bun test`
- `bun run verify`
