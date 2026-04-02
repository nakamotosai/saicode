export type KeybindingAction = string
export type KeybindingContextName = string
export type Keybinding = {
  key?: string
  action?: KeybindingAction
  context?: KeybindingContextName
  [key: string]: unknown
}
export type KeybindingScope = string

