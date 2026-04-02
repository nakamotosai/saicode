import type { Message } from '../../types/message.js'

export const SNIP_NUDGE_TEXT =
  'Long history snip runtime is unavailable in this local recovery build.'

export function isSnipRuntimeEnabled(): boolean {
  return false
}

export function shouldNudgeForSnips(): boolean {
  return false
}

export function snipCompactIfNeeded<T>(value: T, _options?: unknown): T {
  return value
}

export function compactSnippedMessages<T extends Message[]>(messages: T): T {
  return messages
}
