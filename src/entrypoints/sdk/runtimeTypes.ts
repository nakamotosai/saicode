// Local recovery types for the leaked source tree.
// These are intentionally minimal and exist to restore local type imports.

export type AnyZodRawShape = Record<string, unknown>
export type InferShape<Schema> = Schema extends Record<string, unknown>
  ? { [K in keyof Schema]: unknown }
  : Record<string, unknown>

export type EffortLevel = 'low' | 'medium' | 'high' | 'max'

export type SdkMcpToolDefinition<Schema = AnyZodRawShape> = {
  name?: string
  description?: string
  inputSchema?: Schema
  call?: (...args: unknown[]) => Promise<unknown>
}

export type McpSdkServerConfigWithInstance = {
  type: 'sdk'
  name: string
  instance?: unknown
}

export type Options = Record<string, unknown>
export type InternalOptions = Record<string, unknown>
export type Query = AsyncIterable<unknown>
export type InternalQuery = AsyncIterable<unknown>

export type SDKSessionOptions = Record<string, unknown>
export type SDKSession = {
  id?: string
}

export type ListSessionsOptions = {
  dir?: string
  limit?: number
  offset?: number
}

export type GetSessionInfoOptions = {
  dir?: string
}

export type GetSessionMessagesOptions = {
  dir?: string
  limit?: number
  offset?: number
  includeSystemMessages?: boolean
}

export type SessionMutationOptions = {
  dir?: string
}

export type ForkSessionOptions = {
  dir?: string
  upToMessageId?: string
  title?: string
}

export type ForkSessionResult = {
  sessionId: string
}

export type SessionMessage = unknown
