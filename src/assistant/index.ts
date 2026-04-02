const assistantCommand: any = undefined

export default assistantCommand

export function isAssistantMode(): boolean {
  return false
}

export function markAssistantForced(): void {}

export function isAssistantForced(): boolean {
  return false
}

export async function initializeAssistantTeam(): Promise<undefined> {
  return undefined
}

export function getAssistantSystemPromptAddendum(): string {
  return ''
}

export function getAssistantActivationPath(): string | undefined {
  return undefined
}
