export type KeybindingAction = string
export type KeybindingContextName = string
export type ParsedKeystroke = {
  key: string
  ctrl?: boolean
  shift?: boolean
  alt?: boolean
  meta?: boolean
  super?: boolean
}
export type Chord = ParsedKeystroke[]
export type Keybinding = {
  key?: string
  action?: KeybindingAction
  context?: KeybindingContextName
  [key: string]: unknown
}
export type KeybindingBlock = Keybinding
export type ParsedBinding = {
  action: KeybindingAction
  chord: Chord
  context?: KeybindingContextName
  [key: string]: unknown
}
export type KeybindingScope = string
