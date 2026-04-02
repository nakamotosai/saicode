import { z } from 'zod/v4'
import {
  AccountInfoSchema,
  AgentDefinitionSchema,
  AgentInfoSchema,
  AgentMcpServerSpecSchema,
  ApiKeySourceSchema,
  AsyncHookJSONOutputSchema,
  ConfigScopeSchema,
  FastModeStateSchema,
  HookEventSchema,
  HookInputSchema,
  HookJSONOutputSchema,
  McpClaudeAIProxyServerConfigSchema,
  McpHttpServerConfigSchema,
  McpSSEServerConfigSchema,
  McpSdkServerConfigSchema,
  McpServerConfigForProcessTransportSchema,
  McpServerStatusConfigSchema,
  McpServerStatusSchema,
  McpSetServersResultSchema,
  McpStdioServerConfigSchema,
  ModelInfoSchema,
  ModelUsageSchema,
  OutputFormatSchema,
  PermissionBehaviorSchema,
  PermissionDecisionClassificationSchema,
  PermissionModeSchema,
  PermissionResultSchema,
  PermissionRuleValueSchema,
  PermissionUpdateDestinationSchema,
  PermissionUpdateSchema,
  PromptRequestSchema,
  PromptResponseSchema,
  RewindFilesResultSchema,
  SDKAPIRetryMessageSchema,
  SDKAssistantMessageErrorSchema,
  SDKAssistantMessageSchema,
  SDKAuthStatusMessageSchema,
  SDKCompactBoundaryMessageSchema,
  SDKFilesPersistedEventSchema,
  SDKHookProgressMessageSchema,
  SDKHookResponseMessageSchema,
  SDKHookStartedMessageSchema,
  SDKLocalCommandOutputMessageSchema,
  SDKMessageSchema,
  SDKPartialAssistantMessageSchema,
  SDKPermissionDenialSchema,
  SDKPostTurnSummaryMessageSchema,
  SDKPromptSuggestionMessageSchema,
  SDKRateLimitEventSchema,
  SDKRateLimitInfoSchema,
  SDKResultErrorSchema,
  SDKResultMessageSchema,
  SDKResultSuccessSchema,
  SDKElicitationCompleteMessageSchema,
  SDKSessionInfoSchema,
  SDKSessionStateChangedMessageSchema,
  SDKStatusMessageSchema,
  SDKStatusSchema,
  SDKStreamlinedTextMessageSchema,
  SDKStreamlinedToolUseSummaryMessageSchema,
  SDKSystemMessageSchema,
  SDKTaskNotificationMessageSchema,
  SDKTaskProgressMessageSchema,
  SDKTaskStartedMessageSchema,
  SDKToolProgressMessageSchema,
  SDKToolUseSummaryMessageSchema,
  SDKUserMessageReplaySchema,
  SDKUserMessageSchema,
  SettingSourceSchema,
  SlashCommandSchema,
  SdkBetaSchema,
  SdkPluginConfigSchema,
  SyncHookJSONOutputSchema,
  ThinkingAdaptiveSchema,
  ThinkingConfigSchema,
  ThinkingDisabledSchema,
  ThinkingEnabledSchema,
} from './coreSchemas.js'

export type ModelUsage = z.infer<ReturnType<typeof ModelUsageSchema>>
export type OutputFormat = z.infer<ReturnType<typeof OutputFormatSchema>>
export type ApiKeySource = z.infer<ReturnType<typeof ApiKeySourceSchema>>
export type ConfigScope = z.infer<ReturnType<typeof ConfigScopeSchema>>
export type SdkBeta = z.infer<ReturnType<typeof SdkBetaSchema>>
export type ThinkingAdaptive = z.infer<ReturnType<typeof ThinkingAdaptiveSchema>>
export type ThinkingEnabled = z.infer<ReturnType<typeof ThinkingEnabledSchema>>
export type ThinkingDisabled = z.infer<ReturnType<typeof ThinkingDisabledSchema>>
export type ThinkingConfig = z.infer<ReturnType<typeof ThinkingConfigSchema>>
export type McpStdioServerConfig = z.infer<ReturnType<typeof McpStdioServerConfigSchema>>
export type McpSSEServerConfig = z.infer<ReturnType<typeof McpSSEServerConfigSchema>>
export type McpHttpServerConfig = z.infer<ReturnType<typeof McpHttpServerConfigSchema>>
export type McpSdkServerConfig = z.infer<ReturnType<typeof McpSdkServerConfigSchema>>
export type McpServerConfigForProcessTransport = any
export type McpClaudeAIProxyServerConfig = z.infer<ReturnType<typeof McpClaudeAIProxyServerConfigSchema>>
export type McpServerStatusConfig = z.infer<ReturnType<typeof McpServerStatusConfigSchema>>
export type McpServerStatus = any
export type McpSetServersResult = z.infer<ReturnType<typeof McpSetServersResultSchema>>
export type PermissionUpdateDestination = any
export type PermissionBehavior = z.infer<ReturnType<typeof PermissionBehaviorSchema>>
export type PermissionRuleValue = z.infer<ReturnType<typeof PermissionRuleValueSchema>>
export type PermissionUpdate = any
export type PermissionDecisionClassification = z.infer<ReturnType<typeof PermissionDecisionClassificationSchema>>
export type PermissionResult = any
export type PermissionMode = any
export type HookEvent = z.infer<ReturnType<typeof HookEventSchema>>
export type HookInput = z.infer<ReturnType<typeof HookInputSchema>>
export type AsyncHookJSONOutput = z.infer<ReturnType<typeof AsyncHookJSONOutputSchema>>
export type SyncHookJSONOutput = z.infer<ReturnType<typeof SyncHookJSONOutputSchema>>
export type HookJSONOutput = z.infer<ReturnType<typeof HookJSONOutputSchema>>
export type PromptRequest = z.infer<ReturnType<typeof PromptRequestSchema>>
export type PromptResponse = z.infer<ReturnType<typeof PromptResponseSchema>>
export type SlashCommand = z.infer<ReturnType<typeof SlashCommandSchema>>
export type AgentInfo = z.infer<ReturnType<typeof AgentInfoSchema>>
export type ModelInfo = any
export type AccountInfo = z.infer<ReturnType<typeof AccountInfoSchema>>
export type AgentMcpServerSpec = z.infer<ReturnType<typeof AgentMcpServerSpecSchema>>
export type AgentDefinition = z.infer<ReturnType<typeof AgentDefinitionSchema>>
export type SettingSource = z.infer<ReturnType<typeof SettingSourceSchema>>
export type SdkPluginConfig = z.infer<ReturnType<typeof SdkPluginConfigSchema>>
export type RewindFilesResult = any
export type SDKAssistantMessageError = z.infer<ReturnType<typeof SDKAssistantMessageErrorSchema>>
export type SDKStatus = any
export type SDKUserMessage = any
export type SDKUserMessageReplay = any
export type SDKRateLimitInfo = z.infer<ReturnType<typeof SDKRateLimitInfoSchema>>
export type SDKAssistantMessage = z.infer<ReturnType<typeof SDKAssistantMessageSchema>>
export type SDKRateLimitEvent = z.infer<ReturnType<typeof SDKRateLimitEventSchema>>
export type SDKStreamlinedTextMessage = z.infer<ReturnType<typeof SDKStreamlinedTextMessageSchema>>
export type SDKStreamlinedToolUseSummaryMessage = z.infer<ReturnType<typeof SDKStreamlinedToolUseSummaryMessageSchema>>
export type SDKPermissionDenial = z.infer<ReturnType<typeof SDKPermissionDenialSchema>>
export type SDKResultSuccess = z.infer<ReturnType<typeof SDKResultSuccessSchema>>
export type SDKResultError = z.infer<ReturnType<typeof SDKResultErrorSchema>>
export type SDKResultMessage = z.infer<ReturnType<typeof SDKResultMessageSchema>>
export type SDKSystemMessage = z.infer<ReturnType<typeof SDKSystemMessageSchema>>
export type SDKPartialAssistantMessage = any
export type SDKCompactBoundaryMessage = z.infer<ReturnType<typeof SDKCompactBoundaryMessageSchema>>
export type SDKStatusMessage = z.infer<ReturnType<typeof SDKStatusMessageSchema>>
export type SDKPostTurnSummaryMessage = z.infer<ReturnType<typeof SDKPostTurnSummaryMessageSchema>>
export type SDKAPIRetryMessage = z.infer<ReturnType<typeof SDKAPIRetryMessageSchema>>
export type SDKLocalCommandOutputMessage = z.infer<ReturnType<typeof SDKLocalCommandOutputMessageSchema>>
export type SDKHookStartedMessage = z.infer<ReturnType<typeof SDKHookStartedMessageSchema>>
export type SDKHookProgressMessage = z.infer<ReturnType<typeof SDKHookProgressMessageSchema>>
export type SDKHookResponseMessage = z.infer<ReturnType<typeof SDKHookResponseMessageSchema>>
export type SDKToolProgressMessage = z.infer<ReturnType<typeof SDKToolProgressMessageSchema>>
export type SDKAuthStatusMessage = z.infer<ReturnType<typeof SDKAuthStatusMessageSchema>>
export type SDKFilesPersistedEvent = z.infer<ReturnType<typeof SDKFilesPersistedEventSchema>>
export type SDKTaskNotificationMessage = z.infer<ReturnType<typeof SDKTaskNotificationMessageSchema>>
export type SDKTaskStartedMessage = z.infer<ReturnType<typeof SDKTaskStartedMessageSchema>>
export type SDKSessionStateChangedMessage = z.infer<ReturnType<typeof SDKSessionStateChangedMessageSchema>>
export type SDKTaskProgressMessage = z.infer<ReturnType<typeof SDKTaskProgressMessageSchema>>
export type SDKToolUseSummaryMessage = z.infer<ReturnType<typeof SDKToolUseSummaryMessageSchema>>
export type SDKElicitationCompleteMessage = z.infer<ReturnType<typeof SDKElicitationCompleteMessageSchema>>
export type SDKPromptSuggestionMessage = z.infer<ReturnType<typeof SDKPromptSuggestionMessageSchema>>
export type SDKSessionInfo = any
export type SDKMessage = any
export type FastModeState = z.infer<ReturnType<typeof FastModeStateSchema>>

export type APIUserMessage = unknown
export type APIAssistantMessage = unknown
export type RawMessageStreamEvent = unknown
export type UUID = string
