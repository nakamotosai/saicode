export type FinalAgentDraft = {
  agentType: string
  whenToUse: string
  tools: string[]
  getSystemPrompt: () => string
  color?: string
  model?: string
  memory?: string
  source?: string
}

export type AgentWizardData = {
  name?: string
  type?: string
  prompt?: string
  description?: string
  model?: string
  tools?: string[]
  color?: string
  location?: string
  method?: string
  memory?: string
  wasGenerated?: boolean
  finalAgent?: FinalAgentDraft
  [key: string]: unknown
}
