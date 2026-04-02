type BaseShellProgress = {
  output: string
  fullOutput: string
  elapsedTimeSeconds?: number
  totalLines?: number
  totalBytes?: number
  timeoutMs?: number
  taskId?: string
  [key: string]: unknown
}

export type ShellProgress = BaseShellProgress
export type BashProgress = BaseShellProgress
export type PowerShellProgress = BaseShellProgress

export type AgentToolProgress = {
  stage?: string
  message?: string
  [key: string]: unknown
}

export type WebSearchProgress = {
  query?: string
  completedSearches?: number
  resultCount?: number
  [key: string]: unknown
}

export type MCPProgress = {
  serverName?: string
  toolName?: string
  [key: string]: unknown
}

export type SkillToolProgress = {
  skillName?: string
  message?: string
  [key: string]: unknown
}

export type TaskOutputProgress = {
  taskId?: string
  message?: string
  [key: string]: unknown
}

export type SdkWorkflowProgress = {
  step?: string
  message?: string
  percent?: number
  [key: string]: unknown
}

