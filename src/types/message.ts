// Local recovery type surface for the leaked source tree.
// Runtime code only imports these as types, so broad aliases are sufficient.

export type Message = any
export type AssistantMessage = any
export type UserMessage = any
export type MessageOrigin = any
export type NormalizedUserMessage<T = any> = T
export type NormalizedAssistantMessage<T = any> = T
export type NormalizedMessage = any
export type RenderableMessage = any
export type ProgressMessage<T = any> = T
export type HookResultMessage = any
export type AttachmentMessage<T = any> = T
export type GroupedToolUseMessage = any
export type CollapsedReadSearchGroup = any
export type SystemMessage = any
export type SystemAPIErrorMessage = any
export type RequestStartEvent = any
export type StopHookInfo = any
export type TombstoneMessage = any
export type ToolUseSummaryMessage = any
export type SystemAgentsKilledMessage = any
export type SystemApiMetricsMessage = any
export type SystemAwaySummaryMessage = any
export type SystemStopHookSummaryMessage = any
export type SystemBridgeStatusMessage = any
export type SystemCompactBoundaryMessage = any
export type SystemLocalCommandMessage = any
export type SystemTurnDurationMessage = any
export type SystemThinkingMessage = any
export type SystemMemorySavedMessage = any
export type SystemInformationalMessage = any
export type SystemMessageLevel = 'info' | 'warn' | 'error' | string
export type SystemMicrocompactBoundaryMessage = any
export type SystemPermissionRetryMessage = any
export type SystemScheduledTaskFireMessage = any
export type StreamEvent = any
export type PartialCompactDirection = string
