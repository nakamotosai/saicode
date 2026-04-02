declare module "@anthropic-ai/claude-agent-sdk" {
  export type PermissionMode = string
}

declare module "*assistant/index.js" {
  const value: any
  export = value
}

declare module "*services/oauth/types.js" {
  export type AuthMode = string
  export type OAuthAccountInfo = any
  export type OAuthState = any
  export type OAuthStatus = any
}

declare module "*services/compact/reactiveCompact.js" {
  export const maybeRunReactiveCompact: any
}

declare module "*services/contextCollapse/index.js" {
  export const maybeCollapseContext: any
  export const collapseContext: any
}

declare module "*services/contextCollapse/operations.js" {
  export const applyContextCollapseOperations: any
}

declare module "*utils/attributionHooks.js" {
  export const maybeRunAttributionHooks: any
}

declare module "*commands/workflows/index.js" {
  export const workflowsCommand: any
}

declare module "*services/skillSearch/localSearch.js" {
  export const searchLocalSkills: any
}

declare module "*commands/peers/index.js" {
  export const peersCommand: any
}

declare module "*commands/fork/index.js" {
  export const forkCommand: any
}

declare module "*commands/buddy/index.js" {
  export const buddyCommand: any
}

declare module "*tools/WorkflowTool/createWorkflowCommand.js" {
  export const createWorkflowCommand: any
}

declare module "*services/compact/snipProjection.js" {
  export const projectSnippedMessages: any
}

declare module "*services/compact/snipCompact.js" {
  export const compactSnippedMessages: any
}

declare module "*messages/SnipBoundaryMessage.js" {
  const SnipBoundaryMessage: any
  export { SnipBoundaryMessage }
}

declare module "*tools/SendUserFileTool/prompt.js" {
  export const SEND_USER_FILE_TOOL_NAME: string
}

declare module "*messages/UserGitHubWebhookMessage.js" {
  const UserGitHubWebhookMessage: any
  export { UserGitHubWebhookMessage }
}

declare module "*messages/UserForkBoilerplateMessage.js" {
  const UserForkBoilerplateMessage: any
  export { UserForkBoilerplateMessage }
}

declare module "@ant/computer-use-mcp/sentinelApps" {
  const value: any
  export = value
}

declare module "@ant/computer-use-mcp/types" {
  const value: any
  export = value
}

declare module "*tools/ReviewArtifactTool/ReviewArtifactTool.js" {
  const value: any
  export = value
}

declare module "*tools/WorkflowTool/WorkflowTool.js" {
  const value: any
  export = value
}

declare module "*tools/WorkflowTool/WorkflowPermissionRequest.js" {
  const value: any
  export = value
}

declare module "*tools/MonitorTool/MonitorTool.js" {
  const value: any
  export = value
}

declare module "*MonitorPermissionRequest/MonitorPermissionRequest.js" {
  const value: any
  export = value
}

declare module "*ui/option.js" {
  const value: any
  export = value
}

declare module "*types/statusLine.js" {
  export type StatusLineItem = any
}

declare module "*types/utils.js" {
  export type ValueOf<T> = any
  export type Optional<T> = any
}

declare module "*tasks/LocalWorkflowTask/LocalWorkflowTask.js" {
  const value: any
  export = value
}

declare module "*tasks/MonitorMcpTask/MonitorMcpTask.js" {
  const value: any
  export = value
}

declare module "*WorkflowDetailDialog.js" {
  const value: any
  export = value
}

declare module "*MonitorMcpDetailDialog.js" {
  const value: any
  export = value
}

declare module "*Spinner/types.js" {
  export type SpinnerFrame = any
  export type SpinnerVariant = any
  export type SpinnerState = any
}

declare module "*services/compact/cachedMCConfig.js" {
  export const getCachedMCConfig: any
}

declare module "*tools/DiscoverSkillsTool/prompt.js" {
  export const DISCOVER_SKILLS_TOOL_NAME: string
}

declare module "*services/skillSearch/featureCheck.js" {
  export const isSkillSearchEnabled: any
}

declare module "*services/skillSearch/signals.js" {
  export type DiscoverySignal = any
}

declare module "*services/skillSearch/prefetch.js" {
  export const prefetchSkills: any
}

declare module "*services/sessionTranscript/sessionTranscript.js" {
  export const sessionTranscript: any
}

declare module "*utils/bridge/webhookSanitizer.js" {
  const value: any
  export = value
}

declare module "*ink/events/paste-event.js" {
  const value: any
  export = value
}

declare module "*ink/events/resize-event.js" {
  const value: any
  export = value
}

declare module "*ink/frame.js" {
  const value: any
  export = value
}

declare module "*ink/devtools.js" {
  const value: any
  export = value
}

declare module "*memdir/memoryShapeTelemetry.js" {
  export const recordMemoryShapeTelemetry: any
}

declare module "*query/jobs/classifier.js" {
  export const classifyJob: any
}

declare module "*services/lsp/types.js" {
  export type LSPMessage = any
}

declare module "*services/tips/types.js" {
  export type Tip = any
}

declare module "*utils/udsMessaging.js" {
  const value: any
  export = value
}

declare module "cli-highlight" {
  export const highlight: any
  export const supportsLanguage: any
}

declare module "highlight.js" {
  export function getLanguage(name: string): { name: string } | undefined
  export function highlight(code: string, options: any): any
}

declare module "*assistant/sessionDiscovery.js" {
  const value: any
  export = value
}

declare module "*components/agents/SnapshotUpdateDialog.js" {
  const value: any
  export = value
}

declare module "*commands/assistant/assistant.js" {
  const value: any
  export = value
}

declare module "*daemon/workerRegistry.js" {
  const value: any
  export = value
}

declare module "*daemon/main.js" {
  const value: any
  export = value
}

declare module "*cli/bg.js" {
  const value: any
  export = value
}

declare module "*cli/handlers/templateJobs.js" {
  const value: any
  export = value
}

declare module "*environment-runner/main.js" {
  const value: any
  export = value
}

declare module "*self-hosted-runner/main.js" {
  const value: any
  export = value
}

declare module "*sdkUtilityTypes.js" {
  const value: any
  export = value
}

declare module "*types/fileSuggestion.js" {
  export type FileSuggestion = any
}

declare module "*transports/Transport.js" {
  export default interface Transport {
    send?(message: unknown): void | Promise<void>
    close?(): void | Promise<void>
  }
}

declare module "*types/tools.js" {
  export type ToolName = string
  export type ToolUseID = string
  export type ToolResult = any
  export type ToolUse = any
  export type ToolPermissionContext = any
  export type ToolCallParameterInfo = any
}

declare module "*keybindings/types.js" {
  export type Keybinding = any
  export type KeybindingScope = string
}

declare module "*components/mcp/types.js" {
  export type McpServerItem = any
  export type McpToolItem = any
}

declare module "*wizard/types.js" {
  export type WizardStep = any
  export type WizardState = any
}

declare module "*new-agent-creation/types.js" {
  export type AgentCreateOption = any
  export type AgentCreateState = any
}

declare module "*commands/plugin/types.js" {
  export type PluginRow = any
  export type PluginInstallState = any
  export type PluginAction = any
}

declare module "*commands/plugin/unifiedTypes.js" {
  export type UnifiedPluginMetadata = any
  export type UnifiedPluginVersion = any
}

declare module "*commands/install-github-app/types.js" {
  export type InstallGithubAppState = any
}
