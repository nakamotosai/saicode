import { feature } from 'bun:bundle'
import { markPostCompaction } from 'src/bootstrap/state.js'
import { getUserContext } from '../../context.js'
import type { QuerySource } from '../../constants/querySource.js'
import type { Message, AssistantMessage } from '../../types/message.js'
import type { CacheSafeParams } from '../../utils/forkedAgent.js'
import { isMediaSizeErrorMessage, isPromptTooLongMessage } from '../api/errors.js'
import { notifyCompaction } from '../api/promptCacheBreakDetection.js'
import { setLastSummarizedMessageId } from '../SessionMemory/sessionMemoryUtils.js'
import {
  compactConversation,
  type CompactionResult,
  ERROR_MESSAGE_USER_ABORT,
} from './compact.js'
import { suppressCompactWarning } from './compactWarningState.js'
import { runPostCompactCleanup } from './postCompactCleanup.js'

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

type ReactiveCompactOptions = {
  customInstructions?: string
  trigger?: 'manual' | 'auto'
  querySource?: QuerySource
}

type TryReactiveCompactParams = {
  hasAttempted: boolean
  querySource?: QuerySource
  aborted: boolean
  messages: Message[]
  cacheSafeParams: CacheSafeParams
}

export function isWithheldPromptTooLong(
  msg: Message | AssistantMessage | undefined,
): msg is AssistantMessage {
  return msg?.type === 'assistant' && isPromptTooLongMessage(msg)
}

export function isWithheldMediaSizeError(
  msg: Message | AssistantMessage | undefined,
): msg is AssistantMessage {
  return msg?.type === 'assistant' && isMediaSizeErrorMessage(msg)
}

export async function reactiveCompactOnPromptTooLong(
  messages: Message[],
  cacheSafeParams: CacheSafeParams,
  options: ReactiveCompactOptions = {},
): Promise<ReactiveCompactResult> {
  if (cacheSafeParams.toolUseContext.abortController.signal.aborted) {
    return { ok: false, reason: 'aborted' }
  }
  if (messages.length < 2) {
    return { ok: false, reason: 'too_few_groups' }
  }

  try {
    const result = await compactConversation(
      messages,
      cacheSafeParams.toolUseContext,
      cacheSafeParams,
      true,
      options.customInstructions,
      options.trigger === 'auto',
    )
    return { ok: true, result }
  } catch (error) {
    if (
      cacheSafeParams.toolUseContext.abortController.signal.aborted ||
      (error instanceof Error && error.message === ERROR_MESSAGE_USER_ABORT)
    ) {
      return { ok: false, reason: 'aborted' }
    }
    return { ok: false, reason: 'error' }
  }
}

export async function tryReactiveCompact(
  params: TryReactiveCompactParams,
): Promise<CompactionResult | null> {
  if (params.hasAttempted || params.aborted) {
    return null
  }

  const outcome = await reactiveCompactOnPromptTooLong(
    params.messages,
    params.cacheSafeParams,
    {
      trigger: 'auto',
      querySource: params.querySource,
    },
  )

  if (!outcome.ok || !outcome.result) {
    return null
  }

  setLastSummarizedMessageId(undefined)
  runPostCompactCleanup(params.querySource)
  suppressCompactWarning()
  getUserContext.cache.clear?.()

  if (feature('PROMPT_CACHE_BREAK_DETECTION')) {
    notifyCompaction(
      params.querySource ?? 'repl_main_thread',
      params.cacheSafeParams.toolUseContext.agentId,
    )
  }
  markPostCompaction()
  return outcome.result
}
