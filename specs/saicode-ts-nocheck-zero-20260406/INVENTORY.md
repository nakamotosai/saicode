# saicode `@ts-nocheck` Inventory

日期：
- 2026-04-06

总数：
- `227`

## Directory Summary

- `src/utils`: `75`
- `src/components`: `53`
- `src/services`: `31`
- `src/tools`: `20`
- `src/hooks`: `10`
- `src/commands`: `10`
- `src/ink`: `6`
- `src/types`: `2`
- `src/tasks`: `2`
- `src/skills`: `2`
- `src/screens`: `2`
- `src/entrypoints`: `2`
- `src/upstreamproxy`: `1`
- `src/Tool.ts`: `1`
- `src/setup.ts`: `1`
- `src/query.ts`: `1`
- `src/QueryEngine.ts`: `1`
- `src/query`: `1`
- `src/memdir`: `1`
- `src/interactiveHelpers.tsx`: `1`
- `src/dialogLaunchers.tsx`: `1`
- `src/constants`: `1`
- `src/cli`: `1`
- `src/buddy`: `1`

## Full File List

### src/buddy

- `src/buddy/useBuddyNotification.tsx`

### src/cli

- `src/cli/print.ts`

### src/commands

- `src/commands/insights.ts`
- `src/commands/mcp/mcp.tsx`
- `src/commands/plugin/BrowseMarketplace.tsx`
- `src/commands/plugin/DiscoverPlugins.tsx`
- `src/commands/plugin/ManageMarketplaces.tsx`
- `src/commands/remote-setup/remote-setup.tsx`
- `src/commands/review/reviewRemote.ts`
- `src/commands/terminalSetup/terminalSetup.tsx`
- `src/commands/thinkback/thinkback.tsx`
- `src/commands/ultraplan.tsx`

### src/components

- `src/components/agents/new-agent-creation/wizard-steps/ConfirmStep.tsx`
- `src/components/agents/new-agent-creation/wizard-steps/ConfirmStepWrapper.tsx`
- `src/components/agents/new-agent-creation/wizard-steps/MemoryStep.tsx`
- `src/components/agents/ToolSelector.tsx`
- `src/components/AutoUpdater.tsx`
- `src/components/ConsoleOAuthFlow.tsx`
- `src/components/CustomSelect/select.tsx`
- `src/components/DevBar.tsx`
- `src/components/FeedbackSurvey/useMemorySurvey.tsx`
- `src/components/Feedback.tsx`
- `src/components/HistorySearchDialog.tsx`
- `src/components/LogoV2/feedConfigs.tsx`
- `src/components/LogoV2/LogoV2.tsx`
- `src/components/Markdown.tsx`
- `src/components/mcp/ElicitationDialog.tsx`
- `src/components/mcp/MCPAgentServerMenu.tsx`
- `src/components/mcp/MCPListPanel.tsx`
- `src/components/mcp/MCPRemoteServerMenu.tsx`
- `src/components/MemoryUsageIndicator.tsx`
- `src/components/messages/AssistantToolUseMessage.tsx`
- `src/components/messages/AttachmentMessage.tsx`
- `src/components/MessageSelector.tsx`
- `src/components/messages/nullRenderingAttachments.ts`
- `src/components/Messages.tsx`
- `src/components/messages/UserTextMessage.tsx`
- `src/components/Message.tsx`
- `src/components/NativeAutoUpdater.tsx`
- `src/components/PackageManagerAutoUpdater.tsx`
- `src/components/Passes/Passes.tsx`
- `src/components/permissions/AskUserQuestionPermissionRequest/AskUserQuestionPermissionRequest.tsx`
- `src/components/permissions/BashPermissionRequest/bashToolUseOptions.tsx`
- `src/components/permissions/ComputerUseApproval/ComputerUseApproval.tsx`
- `src/components/permissions/ExitPlanModePermissionRequest/ExitPlanModePermissionRequest.tsx`
- `src/components/permissions/NotebookEditPermissionRequest/NotebookEditToolDiff.tsx`
- `src/components/permissions/PermissionDecisionDebugInfo.tsx`
- `src/components/permissions/PermissionRequest.tsx`
- `src/components/permissions/rules/PermissionRuleList.tsx`
- `src/components/PromptInput/PromptInputFooterLeftSide.tsx`
- `src/components/PromptInput/PromptInputFooter.tsx`
- `src/components/PromptInput/PromptInput.tsx`
- `src/components/SentryErrorBoundary.ts`
- `src/components/Settings/Config.tsx`
- `src/components/Settings/Settings.tsx`
- `src/components/skills/SkillsMenu.tsx`
- `src/components/Spinner/TeammateSpinnerTree.tsx`
- `src/components/Spinner.tsx`
- `src/components/Stats.tsx`
- `src/components/TaskListV2.tsx`
- `src/components/tasks/BackgroundTasksDialog.tsx`
- `src/components/tasks/taskStatusUtils.tsx`
- `src/components/ThemePicker.tsx`
- `src/components/ui/TreeSelect.tsx`
- `src/components/wizard/WizardProvider.tsx`

### src/constants

- `src/constants/prompts.ts`

### src/entrypoints

- `src/entrypoints/cli.tsx`
- `src/entrypoints/mcp.ts`

### src/hooks

- `src/hooks/notifs/useCanSwitchToExistingSubscription.tsx`
- `src/hooks/notifs/useInstallMessages.tsx`
- `src/hooks/useCanUseTool.tsx`
- `src/hooks/useChromeExtensionNotification.tsx`
- `src/hooks/useClaudeCodeHintRecommendation.tsx`
- `src/hooks/useDirectConnect.ts`
- `src/hooks/useRemoteSession.ts`
- `src/hooks/useReplBridge.tsx`
- `src/hooks/useTypeahead.tsx`
- `src/hooks/useUpdateNotification.ts`

### src/ink

- `src/ink/components/App.tsx`
- `src/ink/events/event-handlers.ts`
- `src/ink/frame.ts`
- `src/ink/ink.tsx`
- `src/ink/reconciler.ts`
- `src/ink/render-to-screen.ts`

### src/memdir

- `src/memdir/findRelevantMemories.ts`

### src/query

- `src/query/stopHooks.ts`

### src/screens

- `src/screens/REPL.tsx`
- `src/screens/ResumeConversation.tsx`

### src/services

- `src/services/analytics/firstPartyEventLogger.ts`
- `src/services/analytics/firstPartyEventLoggingExporter.ts`
- `src/services/analytics/metadata.ts`
- `src/services/api/bootstrap.ts`
- `src/services/api/claude.ts`
- `src/services/api/client.ts`
- `src/services/api/filesApi.ts`
- `src/services/api/logging.ts`
- `src/services/api/referral.ts`
- `src/services/api/saicodeRuntime.ts`
- `src/services/autoDream/autoDream.ts`
- `src/services/compact/compact.ts`
- `src/services/compact/microCompact.ts`
- `src/services/compact/postCompactCleanup.ts`
- `src/services/extractMemories/extractMemories.ts`
- `src/services/lsp/config.ts`
- `src/services/lsp/LSPClient.ts`
- `src/services/lsp/LSPServerInstance.ts`
- `src/services/lsp/LSPServerManager.ts`
- `src/services/lsp/passiveFeedback.ts`
- `src/services/mcp/client.ts`
- `src/services/mcp/config.ts`
- `src/services/mcp/useManageMCPConnections.ts`
- `src/services/notifier.ts`
- `src/services/oauth/client.ts`
- `src/services/plugins/pluginOperations.ts`
- `src/services/tips/tipRegistry.ts`
- `src/services/tips/tipScheduler.ts`
- `src/services/tokenEstimation.ts`
- `src/services/tools/toolExecution.ts`
- `src/services/voice.ts`

### src/skills

- `src/skills/bundled/claudeApiContent.ts`
- `src/skills/bundled/verifyContent.ts`

### src/tasks

- `src/tasks/RemoteAgentTask/RemoteAgentTask.tsx`
- `src/tasks/types.ts`

### src/tools

- `src/tools/AgentTool/AgentTool.tsx`
- `src/tools/AgentTool/builtInAgents.ts`
- `src/tools/AgentTool/UI.tsx`
- `src/tools/BashTool/bashPermissions.ts`
- `src/tools/BashTool/sedValidation.ts`
- `src/tools/BashTool/UI.tsx`
- `src/tools/FileReadTool/FileReadTool.ts`
- `src/tools/FileReadTool/imageProcessor.ts`
- `src/tools/MCPTool/UI.tsx`
- `src/tools/NotebookEditTool/NotebookEditTool.ts`
- `src/tools/PowerShellTool/pathValidation.ts`
- `src/tools/PowerShellTool/UI.tsx`
- `src/tools/SendMessageTool/SendMessageTool.ts`
- `src/tools/SkillTool/SkillTool.ts`
- `src/tools/SkillTool/UI.tsx`
- `src/tools/TaskOutputTool/TaskOutputTool.tsx`
- `src/tools/TaskStopTool/UI.tsx`
- `src/tools/testing/TestingPermissionTool.tsx`
- `src/tools/ToolSearchTool/prompt.ts`
- `src/tools/WebSearchTool/UI.tsx`

### src/types

- `src/types/hooks.ts`
- `src/types/plugin.ts`

### src/upstreamproxy

- `src/upstreamproxy/relay.ts`

### src/utils

- `src/utils/attachments.ts`
- `src/utils/attribution.ts`
- `src/utils/autoRunIssue.tsx`
- `src/utils/autoUpdater.ts`
- `src/utils/aws.ts`
- `src/utils/cleanup.ts`
- `src/utils/collapseReadSearch.ts`
- `src/utils/computerUse/executor.ts`
- `src/utils/computerUse/gates.ts`
- `src/utils/computerUse/hostAdapter.ts`
- `src/utils/computerUse/inputLoader.ts`
- `src/utils/computerUse/mcpServer.ts`
- `src/utils/computerUse/setup.ts`
- `src/utils/computerUse/swiftLoader.ts`
- `src/utils/computerUse/wrapper.tsx`
- `src/utils/config.ts`
- `src/utils/context.ts`
- `src/utils/conversationRecovery.ts`
- `src/utils/cronScheduler.ts`
- `src/utils/deepLink/protocolHandler.ts`
- `src/utils/doctorDiagnostic.ts`
- `src/utils/dxt/helpers.ts`
- `src/utils/dxt/zip.ts`
- `src/utils/effort.ts`
- `src/utils/envUtils.ts`
- `src/utils/execFileNoThrowPortable.ts`
- `src/utils/filePersistence/filePersistence.ts`
- `src/utils/fingerprint.ts`
- `src/utils/generators.ts`
- `src/utils/groupToolUses.ts`
- `src/utils/heapDumpService.ts`
- `src/utils/hooks.ts`
- `src/utils/ide.ts`
- `src/utils/imagePaste.ts`
- `src/utils/logoV2Utils.ts`
- `src/utils/log.ts`
- `src/utils/messageQueueManager.ts`
- `src/utils/messages/mappers.ts`
- `src/utils/messages.ts`
- `src/utils/model/bedrock.ts`
- `src/utils/model/modelOptions.ts`
- `src/utils/model/model.ts`
- `src/utils/nativeInstaller/download.ts`
- `src/utils/nativeInstaller/installer.ts`
- `src/utils/notebook.ts`
- `src/utils/permissions/classifierDecision.ts`
- `src/utils/permissions/filesystem.ts`
- `src/utils/permissions/pathValidation.ts`
- `src/utils/plans.ts`
- `src/utils/plugins/lspPluginIntegration.ts`
- `src/utils/plugins/marketplaceManager.ts`
- `src/utils/plugins/mcpbHandler.ts`
- `src/utils/plugins/zipCache.ts`
- `src/utils/processUserInput/processSlashCommand.tsx`
- `src/utils/processUserInput/processUserInput.ts`
- `src/utils/queryHelpers.ts`
- `src/utils/releaseNotes.ts`
- `src/utils/secureStorage/fallbackStorage.ts`
- `src/utils/secureStorage/index.ts`
- `src/utils/secureStorage/macOsKeychainStorage.ts`
- `src/utils/secureStorage/plainTextStorage.ts`
- `src/utils/sessionFileAccessHooks.ts`
- `src/utils/sessionStorage.ts`
- `src/utils/settings/validateEditTool.ts`
- `src/utils/sideQuery.ts`
- `src/utils/stats.ts`
- `src/utils/status.tsx`
- `src/utils/streamlinedTransform.ts`
- `src/utils/telemetryAttributes.ts`
- `src/utils/telemetry/instrumentation.ts`
- `src/utils/teleport/gitBundle.ts`
- `src/utils/teleport.tsx`
- `src/utils/thinking.ts`
- `src/utils/user.ts`
- `src/utils/worktree.ts`

### single files at root of src

- `src/dialogLaunchers.tsx`
- `src/interactiveHelpers.tsx`
- `src/QueryEngine.ts`
- `src/query.ts`
- `src/setup.ts`
- `src/Tool.ts`
