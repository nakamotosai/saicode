export type MCPViewState =
  | { type: 'list' }
  | { type: string; [key: string]: unknown }

export type ServerInfo = {
  name: string
  type?: string
  client?: any
  config?: any
  [key: string]: unknown
}

export type AgentMcpServerInfo = ServerInfo & {
  agentName?: string
}

export type StdioServerInfo = ServerInfo & {
  command?: string
  args?: string[]
}

export type SSEServerInfo = ServerInfo & {
  url?: string
}

export type HTTPServerInfo = ServerInfo & {
  url?: string
}

export type ClaudeAIServerInfo = ServerInfo & {
  claudeAi?: boolean
}

