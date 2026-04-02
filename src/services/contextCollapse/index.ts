import type { AssistantMessage, Message } from '../../types/message.js'
import { isPromptTooLongMessage } from '../api/errors.js'
import { createSignal } from '../../utils/signal.js'

const changed = createSignal()

export function isContextCollapseEnabled(): boolean {
  return false
}

export function getStats(): {
  savedTokens: number
  savedPercent: number
  totalSpawns: number
  totalErrors: number
  totalEmptySpawns: number
  emptySpawnWarningEmitted: boolean
  health: {
    totalSpawns: number
    totalErrors: number
    totalEmptySpawns: number
    emptySpawnWarningEmitted: boolean
    lastError?: string
  }
  collapsedSpans: number
  collapsedMessages: number
  stagedSpans: number
  lastError?: string
} {
  return {
    savedTokens: 0,
    savedPercent: 0,
    totalSpawns: 0,
    totalErrors: 0,
    totalEmptySpawns: 0,
    emptySpawnWarningEmitted: false,
    health: {
      totalSpawns: 0,
      totalErrors: 0,
      totalEmptySpawns: 0,
      emptySpawnWarningEmitted: false,
    },
    collapsedSpans: 0,
    collapsedMessages: 0,
    stagedSpans: 0,
  }
}

export function maybeCollapseContext<T>(value: T): T {
  return value
}

export function collapseContext<T>(value: T): T {
  return value
}

export function initContextCollapse(): void {
  changed.emit()
}

export async function applyCollapsesIfNeeded<T extends Message[]>(
  messages: T,
  ..._args: unknown[]
): Promise<{ messages: T }> {
  return { messages }
}

export function isWithheldPromptTooLong(
  message: Message | undefined,
  predicate: (msg: AssistantMessage) => boolean = isPromptTooLongMessage,
  ..._args: unknown[]
): boolean {
  return message?.type === 'assistant' && predicate(message as AssistantMessage)
}

export function recoverFromOverflow<T extends Message[]>(
  messages: T,
  ..._args: unknown[]
): { messages: T; committed: number } {
  return { messages, committed: 0 }
}

export const subscribe = changed.subscribe

export function resetContextCollapse(): void {
  changed.emit()
}
