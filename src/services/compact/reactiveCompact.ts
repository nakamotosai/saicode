import type { CompactionResult } from './compact.js'

type ReactiveCompactResult = {
  ok: boolean
  result?: CompactionResult
  reason?:
    | 'too_few_groups'
    | 'aborted'
    | 'exhausted'
    | 'error'
    | 'media_unstrippable'
}

export function isReactiveOnlyMode(): boolean {
  return false
}

export async function reactiveCompactOnPromptTooLong(
  ..._args: unknown[]
): Promise<ReactiveCompactResult> {
  return { ok: false, reason: 'error' }
}
