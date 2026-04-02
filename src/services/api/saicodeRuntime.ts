// @ts-nocheck
import { randomUUID } from 'crypto'
import { existsSync, readFileSync } from 'fs'
import { join } from 'path'
import type { BetaJSONOutputFormat } from '@anthropic-ai/sdk/resources/beta/messages/messages.mjs'
import type { AssistantMessage, Message, StreamEvent, SystemAPIErrorMessage } from '../../types/message.js'
import type { SystemPrompt } from '../../utils/systemPromptType.js'
import type { Tools } from '../../Tool.js'
import { getClaudeConfigHomeDir, isEnvTruthy } from '../../utils/envUtils.js'
import { createAssistantAPIErrorMessage, createAssistantMessage, createUserMessage } from '../../utils/messages.js'
import { toolToAPISchema } from '../../utils/api.js'
import { repairMalformedToolArguments } from '../../utils/toolInputRepair.js'
import { EMPTY_USAGE, type NonNullableUsage } from './logging.js'
import {
  getSaicodeSmallFastModelId,
  resolveSaicodeModel,
} from '../../utils/model/saicodeCatalog.js'

type WireAPI = 'openai-responses' | 'openai-chat-completions'

type ProviderConfig = {
  id: string
  api: WireAPI
  baseUrl: string
  apiKey?: string
  headers?: Record<string, string>
}

type RuntimeConfigFile = {
  providers?: Record<
    string,
    {
      api?: WireAPI
      baseUrl?: string
      apiKey?: string
      headers?: Record<string, string>
    }
  >
}

function getRuntimeConfigFile(): RuntimeConfigFile {
  const path = join(getClaudeConfigHomeDir(), 'config.json')
  if (!existsSync(path)) {
    return {}
  }
  try {
    return JSON.parse(readFileSync(path, 'utf8')) as RuntimeConfigFile
  } catch {
    return {}
  }
}

type LocalRateLimitReservation = {
  at: number
  estimatedInputTokens: number
}

const providerRateLimitWindows = new Map<string, LocalRateLimitReservation[]>()

function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms))
}

function parseEnvNumber(
  value: string | undefined,
  fallback: number,
): number {
  const parsed = Number(value)
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback
}

function pruneProviderWindow(
  key: string,
  windowMs: number,
): LocalRateLimitReservation[] {
  const now = Date.now()
  const existing = providerRateLimitWindows.get(key) ?? []
  const pruned = existing.filter(entry => now - entry.at < windowMs)
  providerRateLimitWindows.set(key, pruned)
  return pruned
}

function getLocalRateLimitConfig(provider: ProviderConfig): {
  windowMs: number
  maxRequests: number
  maxEstimatedInputTokens: number
} | null {
  if (provider.id === 'nvidia') {
    return {
      windowMs: parseEnvNumber(
        process.env.SAICODE_NVIDIA_RATE_LIMIT_WINDOW_MS,
        60_000,
      ),
      maxRequests: parseEnvNumber(
        process.env.SAICODE_NVIDIA_MAX_REQUESTS_PER_WINDOW,
        30,
      ),
      maxEstimatedInputTokens: parseEnvNumber(
        process.env.SAICODE_NVIDIA_MAX_ESTIMATED_INPUT_TOKENS_PER_WINDOW,
        90_000,
      ),
    }
  }

  if (provider.id === 'cliproxyapi' || provider.id === 'cpa') {
    return {
      windowMs: parseEnvNumber(
        process.env.SAICODE_CLIPROXY_RATE_LIMIT_WINDOW_MS,
        60_000,
      ),
      maxRequests: parseEnvNumber(
        process.env.SAICODE_CLIPROXY_MAX_REQUESTS_PER_WINDOW,
        20,
      ),
      maxEstimatedInputTokens: parseEnvNumber(
        process.env.SAICODE_CLIPROXY_MAX_ESTIMATED_INPUT_TOKENS_PER_WINDOW,
        120_000,
      ),
    }
  }

  return null
}

async function reserveLocalRateLimitSlot(args: {
  provider: ProviderConfig
  bodyText: string
}): Promise<void> {
  const config = getLocalRateLimitConfig(args.provider)
  if (!config) {
    return
  }

  const key = args.provider.id
  const estimatedInputTokens = Math.max(
    1,
    Math.round(args.bodyText.length / 4),
  )

  while (true) {
    const entries = pruneProviderWindow(key, config.windowMs)
    const requestCount = entries.length
    const tokenCount = entries.reduce(
      (sum, entry) => sum + entry.estimatedInputTokens,
      0,
    )

    if (
      requestCount < config.maxRequests &&
      tokenCount + estimatedInputTokens <= config.maxEstimatedInputTokens
    ) {
      entries.push({
        at: Date.now(),
        estimatedInputTokens,
      })
      providerRateLimitWindows.set(key, entries)
      return
    }

    const oldest = entries[0]
    const waitMs = oldest
      ? Math.max(500, config.windowMs - (Date.now() - oldest.at) + 250)
      : 1000
    await sleep(waitMs)
  }
}

function getRetryAfterMs(headers: Headers): number | undefined {
  const raw =
    headers.get('retry-after') ??
    headers.get('x-ratelimit-reset-after') ??
    headers.get('ratelimit-reset')
  if (!raw) {
    return undefined
  }

  const seconds = Number(raw)
  if (Number.isFinite(seconds) && seconds > 0) {
    return seconds < 1000 ? Math.round(seconds * 1000) : Math.round(seconds)
  }

  const absolute = Date.parse(raw)
  if (!Number.isNaN(absolute)) {
    return Math.max(0, absolute - Date.now())
  }

  return undefined
}

async function fetchWithLocalRateLimit(
  endpoint: string,
  provider: ProviderConfig,
  init: RequestInit,
  bodyText: string,
): Promise<Response> {
  await reserveLocalRateLimitSlot({
    provider,
    bodyText,
  })

  const maxRetries = parseEnvNumber(
    process.env.SAICODE_RATE_LIMIT_RETRIES,
    2,
  )

  for (let attempt = 0; ; attempt++) {
    const response = await fetch(endpoint, init)
    if (response.status !== 429) {
      return response
    }

    const body = await response.text()
    if (attempt >= maxRetries) {
      throw new Error(body)
    }

    const retryAfterMs =
      getRetryAfterMs(response.headers) ??
      parseEnvNumber(process.env.SAICODE_RATE_LIMIT_RETRY_MS, 20_000) *
        (attempt + 1)

    await sleep(retryAfterMs)
  }
}

function getProviderConfig(model: string | undefined): ProviderConfig {
  const runtimeConfig = getRuntimeConfigFile()
  const entry = resolveSaicodeModel(model)
  const providerKeys =
    entry.provider === 'cpa'
      ? ['cpa', 'cliproxyapi']
      : entry.provider === 'cliproxyapi'
        ? ['cliproxyapi', 'cpa']
        : [entry.provider]
  const fileProvider = providerKeys
    .map(key => runtimeConfig.providers?.[key])
    .find(Boolean)

  switch (entry.provider) {
    case 'cpa':
    case 'cliproxyapi':
      return {
        id: entry.provider === 'cpa' ? 'cpa' : 'cliproxyapi',
        api:
          (process.env.CLIPROXYAPI_API as WireAPI | undefined) ||
          fileProvider?.api ||
          'openai-responses',
        baseUrl:
          process.env.CLIPROXYAPI_BASE_URL ||
          fileProvider?.baseUrl ||
          'http://127.0.0.1:8317/v1',
        apiKey:
          process.env.CLIPROXYAPI_API_KEY ||
          fileProvider?.apiKey ||
          process.env.OPENAI_API_KEY,
        headers: fileProvider?.headers,
      }
    case 'nvidia':
    default:
      return {
        id: 'nvidia',
        api:
          (process.env.NVIDIA_API as WireAPI | undefined) ||
          fileProvider?.api ||
          'openai-chat-completions',
        baseUrl:
          process.env.NVIDIA_BASE_URL ||
          fileProvider?.baseUrl ||
          'https://integrate.api.nvidia.com/v1',
        apiKey: process.env.NVIDIA_API_KEY || fileProvider?.apiKey,
        headers: fileProvider?.headers,
      }
  }
}

function flattenSystemPrompt(systemPrompt: SystemPrompt): string | undefined {
  const combined = systemPrompt.join('\n\n').trim()
  return combined.length > 0 ? combined : undefined
}

function normalizeToolResultContent(content: unknown): string {
  if (typeof content === 'string') {
    return content
  }
  if (Array.isArray(content)) {
    return content
      .map(item => {
        if (typeof item === 'string') return item
        if (item && typeof item === 'object' && 'text' in item) {
          return String((item as { text?: unknown }).text ?? '')
        }
        return JSON.stringify(item)
      })
      .join('\n')
  }
  if (content === undefined) {
    return ''
  }
  return JSON.stringify(content)
}

function makeDataUrl(mediaType: string, data: string): string {
  return `data:${mediaType};base64,${data}`
}

function toResponseUserContent(
  blocks: unknown[],
): { content: unknown[]; toolOutputs: unknown[] } {
  const content: unknown[] = []
  const toolOutputs: unknown[] = []

  for (const block of blocks) {
    if (!block || typeof block !== 'object') {
      continue
    }

    const typed = block as Record<string, unknown>
    switch (typed.type) {
      case 'text':
        content.push({
          type: 'input_text',
          text: String(typed.text ?? ''),
        })
        break
      case 'image': {
        const source = typed.source as Record<string, unknown> | undefined
        if (
          source?.type === 'base64' &&
          typeof source.data === 'string' &&
          typeof source.media_type === 'string'
        ) {
          content.push({
            type: 'input_image',
            image_url: makeDataUrl(source.media_type, source.data),
          })
        }
        break
      }
      case 'tool_result':
        toolOutputs.push({
          type: 'function_call_output',
          call_id: String(typed.tool_use_id ?? ''),
          output: normalizeToolResultContent(typed.content),
        })
        break
      default:
        break
    }
  }

  return { content, toolOutputs }
}

function convertMessagesToResponsesInput(messages: Message[]): unknown[] {
  const input: unknown[] = []

  for (const message of messages) {
    if (message.type === 'user') {
      if (typeof message.message.content === 'string') {
        input.push({
          role: 'user',
          content: [{ type: 'input_text', text: message.message.content }],
        })
        continue
      }

      const { content, toolOutputs } = toResponseUserContent(
        message.message.content as unknown[],
      )
      if (content.length > 0) {
        input.push({ role: 'user', content })
      }
      input.push(...toolOutputs)
      continue
    }

    if (message.type === 'assistant') {
      const textParts: string[] = []
      for (const block of message.message.content as Array<Record<string, unknown>>) {
        switch (block.type) {
          case 'text':
            textParts.push(String(block.text ?? ''))
            break
          case 'tool_use':
            input.push({
              type: 'function_call',
              call_id: String(block.id ?? randomUUID()),
              name: String(block.name ?? ''),
              arguments: JSON.stringify(block.input ?? {}),
            })
            break
          default:
            break
        }
      }
      if (textParts.length > 0) {
        input.push({
          role: 'assistant',
          content: [{ type: 'output_text', text: textParts.join('\n\n') }],
        })
      }
    }
  }

  return input
}

function convertMessagesToChatCompletions(messages: Message[]): unknown[] {
  const out: unknown[] = []

  for (const message of messages) {
    if (message.type === 'user') {
      if (typeof message.message.content === 'string') {
        out.push({ role: 'user', content: message.message.content })
        continue
      }

      const content: unknown[] = []
      for (const block of message.message.content as Array<Record<string, unknown>>) {
        switch (block.type) {
          case 'text':
            content.push({ type: 'text', text: String(block.text ?? '') })
            break
          case 'image': {
            const source = block.source as Record<string, unknown> | undefined
            if (
              source?.type === 'base64' &&
              typeof source.data === 'string' &&
              typeof source.media_type === 'string'
            ) {
              content.push({
                type: 'image_url',
                image_url: {
                  url: makeDataUrl(source.media_type, source.data),
                },
              })
            }
            break
          }
          case 'tool_result':
            out.push({
              role: 'tool',
              tool_call_id: String(block.tool_use_id ?? ''),
              content: normalizeToolResultContent(block.content),
            })
            break
          default:
            break
        }
      }
      if (content.length > 0) {
        out.push({ role: 'user', content })
      }
      continue
    }

    if (message.type === 'assistant') {
      const textParts: string[] = []
      const toolCalls: unknown[] = []
      for (const block of message.message.content as Array<Record<string, unknown>>) {
        switch (block.type) {
          case 'text':
            textParts.push(String(block.text ?? ''))
            break
          case 'tool_use':
            toolCalls.push({
              id: String(block.id ?? randomUUID()),
              type: 'function',
              function: {
                name: String(block.name ?? ''),
                arguments: JSON.stringify(block.input ?? {}),
              },
            })
            break
          default:
            break
        }
      }
      out.push({
        role: 'assistant',
        ...(textParts.length > 0 ? { content: textParts.join('\n\n') } : { content: '' }),
        ...(toolCalls.length > 0 ? { tool_calls: toolCalls } : {}),
      })
    }
  }

  return out
}

async function buildOpenAITools(args: {
  tools: Tools
  model: string
  api: WireAPI
  getToolPermissionContext: () => Promise<unknown>
  agents: unknown[]
  allowedAgentTypes?: string[]
}): Promise<unknown[] | undefined> {
  if (args.tools.length === 0) {
    return undefined
  }

  const tools = await Promise.all(
    args.tools.map(async tool => {
      const schema = await toolToAPISchema(tool, {
        getToolPermissionContext: args.getToolPermissionContext as () => Promise<any>,
        tools: args.tools,
        agents: args.agents as any[],
        allowedAgentTypes: args.allowedAgentTypes,
        model: args.model,
      })

      if (args.api === 'openai-chat-completions') {
        return {
          type: 'function',
          function: {
            name: schema.name,
            description: schema.description,
            parameters: schema.input_schema,
          },
        }
      }

      return {
        type: 'function',
        name: schema.name,
        description: schema.description,
        parameters: schema.input_schema,
        ...(schema.strict ? { strict: true } : {}),
      }
    }),
  )

  return tools
}

function getHeaders(provider: ProviderConfig): HeadersInit {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...provider.headers,
  }
  if (provider.apiKey) {
    headers.Authorization = `Bearer ${provider.apiKey}`
  }
  return headers
}

function parseResponseUsage(usage: Record<string, unknown> | undefined): NonNullableUsage {
  const inputTokens = Number(usage?.input_tokens ?? usage?.prompt_tokens ?? 0)
  const outputTokens = Number(
    usage?.output_tokens ?? usage?.completion_tokens ?? 0,
  )
  return {
    ...EMPTY_USAGE,
    input_tokens: Number.isFinite(inputTokens) ? inputTokens : 0,
    output_tokens: Number.isFinite(outputTokens) ? outputTokens : 0,
  }
}

function parseResponsesAssistant(json: Record<string, any>): AssistantMessage {
  const content: Array<Record<string, unknown>> = []
  for (const item of json.output ?? []) {
    if (!item || typeof item !== 'object') continue
    if (item.type === 'message') {
      for (const c of item.content ?? []) {
        if (c?.type === 'output_text') {
          content.push({ type: 'text', text: String(c.text ?? '') })
        }
      }
    }
    if (item.type === 'function_call') {
      let input: unknown = {}
      try {
        input = item.arguments ? JSON.parse(String(item.arguments)) : {}
      } catch {
        input = repairMalformedToolArguments(String(item.arguments ?? ''))
      }
      content.push({
        type: 'tool_use',
        id: String(item.call_id ?? item.id ?? randomUUID()),
        name: String(item.name ?? ''),
        input,
      })
    }
  }

  if (content.length === 0 && typeof json.output_text === 'string') {
    content.push({ type: 'text', text: json.output_text })
  }

  return createAssistantMessage({
    content: content.length > 0 ? (content as any) : '',
    usage: parseResponseUsage(json.usage),
  })
}

function parseChatAssistant(json: Record<string, any>): AssistantMessage {
  const choice = json.choices?.[0]?.message ?? {}
  const content: Array<Record<string, unknown>> = []
  if (typeof choice.content === 'string' && choice.content.length > 0) {
    content.push({ type: 'text', text: choice.content })
  }
  for (const toolCall of choice.tool_calls ?? []) {
    let input: unknown = {}
    try {
      input = toolCall?.function?.arguments
        ? JSON.parse(String(toolCall.function.arguments))
        : {}
    } catch {
      input = repairMalformedToolArguments(
        String(toolCall?.function?.arguments ?? ''),
      )
    }
    content.push({
      type: 'tool_use',
      id: String(toolCall.id ?? randomUUID()),
      name: String(toolCall?.function?.name ?? ''),
      input,
    })
  }

  return createAssistantMessage({
    content: content.length > 0 ? (content as any) : '',
    usage: parseResponseUsage(json.usage),
  })
}

type SSEFrame = {
  event?: string
  data?: string
}

type StreamBlock =
  | { index: number; type: 'text'; text: string; closed: boolean; started: boolean }
  | {
      index: number
      type: 'tool_use'
      id: string
      name: string
      input: string
      closed: boolean
      started: boolean
    }

function parseSSEFrames(buffer: string): {
  frames: SSEFrame[]
  remaining: string
} {
  const frames: SSEFrame[] = []
  let pos = 0

  let idx: number
  while ((idx = buffer.indexOf('\n\n', pos)) !== -1) {
    const rawFrame = buffer.slice(pos, idx)
    pos = idx + 2
    if (!rawFrame.trim()) continue

    const frame: SSEFrame = {}
    for (const line of rawFrame.split('\n')) {
      if (line.startsWith(':')) continue
      const colonIdx = line.indexOf(':')
      if (colonIdx === -1) continue
      const field = line.slice(0, colonIdx)
      const value =
        line[colonIdx + 1] === ' '
          ? line.slice(colonIdx + 2)
          : line.slice(colonIdx + 1)
      if (field === 'event') frame.event = value
      if (field === 'data') {
        frame.data = frame.data ? `${frame.data}\n${value}` : value
      }
    }

    if (frame.data) {
      frames.push(frame)
    }
  }

  return { frames, remaining: buffer.slice(pos) }
}

function buildAssistantFromStreamBlocks(
  blocks: StreamBlock[],
  usage: NonNullableUsage,
): AssistantMessage {
  const content = blocks
    .sort((a, b) => a.index - b.index)
    .map(block => {
      if (block.type === 'text') {
        return { type: 'text', text: block.text }
      }

      let input: unknown = {}
      try {
        input = block.input ? JSON.parse(block.input) : {}
      } catch {
        input = repairMalformedToolArguments(block.input)
      }

      return {
        type: 'tool_use',
        id: block.id,
        name: block.name,
        input,
      }
    })

  return createAssistantMessage({
    content: content.length > 0 ? (content as any) : '',
    usage,
  })
}

function createMessageStartEvent(usage: NonNullableUsage): StreamEvent {
  return {
    type: 'stream_event',
    event: {
      type: 'message_start',
      message: {
        usage,
      },
    } as any,
  }
}

function createContentBlockStartEvent(block: StreamBlock): StreamEvent {
  const content_block =
    block.type === 'text'
      ? { type: 'text', text: '' }
      : {
          type: 'tool_use',
          id: block.id,
          name: block.name,
          input: '',
        }

  return {
    type: 'stream_event',
    event: {
      type: 'content_block_start',
      index: block.index,
      content_block,
    } as any,
  }
}

function createContentDeltaEvent(
  index: number,
  delta: { type: 'text_delta'; text: string } | { type: 'input_json_delta'; partial_json: string },
): StreamEvent {
  return {
    type: 'stream_event',
    event: {
      type: 'content_block_delta',
      index,
      delta,
    } as any,
  }
}

function createContentBlockStopEvent(index: number): StreamEvent {
  return {
    type: 'stream_event',
    event: {
      type: 'content_block_stop',
      index,
    } as any,
  }
}

function createMessageDeltaEvent(
  usage: NonNullableUsage,
  stopReason: string | null,
): StreamEvent {
  return {
    type: 'stream_event',
    event: {
      type: 'message_delta',
      delta: {
        stop_reason: stopReason,
      },
      usage,
    } as any,
  }
}

function createMessageStopEvent(): StreamEvent {
  return {
    type: 'stream_event',
    event: {
      type: 'message_stop',
    } as any,
  }
}

function getOrCreateResponsesTextBlock(
  blocks: Map<string, StreamBlock>,
  orderedBlocks: StreamBlock[],
  key: string,
): StreamBlock {
  const existing = blocks.get(key)
  if (existing) {
    return existing
  }

  const block: StreamBlock = {
    index: orderedBlocks.length,
    type: 'text',
    text: '',
    closed: false,
    started: false,
  }
  blocks.set(key, block)
  orderedBlocks.push(block)
  return block
}

function getOrCreateResponsesToolBlock(
  blocks: Map<string, StreamBlock>,
  orderedBlocks: StreamBlock[],
  key: string,
  name: string,
  id: string,
): StreamBlock {
  const existing = blocks.get(key)
  if (existing) {
    if (existing.type === 'tool_use') {
      if (!existing.name && name) {
        existing.name = name
      }
      if (!existing.id && id) {
        existing.id = id
      }
    }
    return existing
  }

  const block: StreamBlock = {
    index: orderedBlocks.length,
    type: 'tool_use',
    id,
    name,
    input: '',
    closed: false,
    started: false,
  }
  blocks.set(key, block)
  orderedBlocks.push(block)
  return block
}

function registerResponsesToolAliases(
  aliasMap: Map<string, string>,
  blockKey: string,
  ...aliases: Array<unknown>
): void {
  for (const alias of aliases) {
    if (typeof alias === 'string' && alias.length > 0) {
      aliasMap.set(alias, blockKey)
    } else if (typeof alias === 'number' && Number.isFinite(alias)) {
      aliasMap.set(String(alias), blockKey)
    }
  }
}

function resolveResponsesToolKey(
  aliasMap: Map<string, string>,
  ...candidates: Array<unknown>
): string | undefined {
  for (const candidate of candidates) {
    const normalized =
      typeof candidate === 'string'
        ? candidate
        : typeof candidate === 'number' && Number.isFinite(candidate)
          ? String(candidate)
          : undefined
    if (!normalized) {
      continue
    }
    const resolved = aliasMap.get(normalized)
    if (resolved) {
      return resolved
    }
  }
  for (const candidate of candidates) {
    const normalized =
      typeof candidate === 'string'
        ? candidate
        : typeof candidate === 'number' && Number.isFinite(candidate)
          ? String(candidate)
          : undefined
    if (normalized) {
      return `resp:tool:${normalized}`
    }
  }
  return undefined
}

function ensureBlockStarted(block: StreamBlock): StreamEvent | undefined {
  if (block.started) {
    return undefined
  }
  block.started = true
  return createContentBlockStartEvent(block)
}

async function* performStreamingRequest(args: {
  messages: Message[]
  systemPrompt: SystemPrompt
  tools: Tools
  options: any
}): AsyncGenerator<StreamEvent | AssistantMessage | SystemAPIErrorMessage, void> {
  const entry = resolveSaicodeModel(args.options.model)
  const provider = getProviderConfig(entry.alias)
  const system = flattenSystemPrompt(args.systemPrompt)
  const openAITools = await buildOpenAITools({
    tools: args.tools,
    model: entry.alias,
    api: provider.api,
    getToolPermissionContext:
      args.options.getToolPermissionContext ?? (async () => ({})),
    agents: args.options.agents ?? [],
    allowedAgentTypes: args.options.allowedAgentTypes,
  })

  if (!provider.apiKey && provider.id !== 'cliproxyapi') {
    throw new Error(
      `${provider.id} API key is missing. Set ${provider.id.toUpperCase()}_API_KEY or ~/.saicode/config.json`,
    )
  }

  if (args.options.outputFormat) {
    yield await performRequest(args)
    return
  }

  const body =
    provider.api === 'openai-chat-completions'
      ? {
          model: entry.model,
          messages: [
            ...(system ? [{ role: 'system', content: system }] : []),
            ...convertMessagesToChatCompletions(args.messages),
          ],
          ...(openAITools ? { tools: openAITools } : {}),
          tool_choice: openAITools ? 'auto' : undefined,
          max_tokens:
            args.options.maxOutputTokensOverride ?? entry.maxOutputTokens,
          stream: true,
          stream_options: { include_usage: true },
        }
      : {
          model: entry.model,
          ...(system ? { instructions: system } : {}),
          input: convertMessagesToResponsesInput(args.messages),
          ...(openAITools ? { tools: openAITools } : {}),
          ...(isEnvTruthy(process.env.SAICODE_PARALLEL_TOOL_CALLS)
            ? { parallel_tool_calls: true }
            : { parallel_tool_calls: false }),
          max_output_tokens:
            args.options.maxOutputTokensOverride ?? entry.maxOutputTokens,
          stream: true,
        }

  const endpoint =
    provider.api === 'openai-chat-completions'
      ? `${provider.baseUrl}/chat/completions`
      : `${provider.baseUrl}/responses`

  const bodyText = JSON.stringify(body)
  const response = await fetchWithLocalRateLimit(
    endpoint,
    provider,
    {
      method: 'POST',
      headers: getHeaders(provider),
      signal: args.options.signal,
      body: bodyText,
    },
    bodyText,
  )

  if (!response.ok) {
    throw new Error(await response.text())
  }
  if (!response.body) {
    throw new Error('Streaming response body is empty')
  }

  const orderedBlocks: StreamBlock[] = []
  const blockMap = new Map<string, StreamBlock>()
  const responsesToolAliasMap = new Map<string, string>()
  let usage = { ...EMPTY_USAGE }
  let stopReason: string | null = null
  let sentMessageStart = false

  const reader = response.body.getReader()
  const decoder = new TextDecoder()
  let buffer = ''

  const ensureMessageStart = (): StreamEvent | undefined => {
    if (sentMessageStart) return undefined
    sentMessageStart = true
    return createMessageStartEvent(usage)
  }

  const closeOpenBlocks = function* (): Generator<StreamEvent> {
    for (const block of orderedBlocks) {
      if (!block.closed) {
        block.closed = true
        yield createContentBlockStopEvent(block.index)
      }
    }
  }

  while (true) {
    const { value, done } = await reader.read()
    if (done) break
    buffer += decoder.decode(value, { stream: true })
    const parsed = parseSSEFrames(buffer)
    buffer = parsed.remaining

    for (const frame of parsed.frames) {
      const rawData = frame.data?.trim()
      if (!rawData || rawData === '[DONE]') continue

      let payload: Record<string, any>
      try {
        payload = JSON.parse(rawData) as Record<string, any>
      } catch {
        continue
      }

      if (payload.error) {
        throw new Error(
          typeof payload.error === 'string'
            ? payload.error
            : payload.error.message || rawData,
        )
      }

      const startEvent = ensureMessageStart()
      if (startEvent) {
        yield startEvent
      }

      if (provider.api === 'openai-chat-completions') {
        const choice = payload.choices?.[0]
        const delta = choice?.delta ?? {}

        if (typeof delta.content === 'string' && delta.content.length > 0) {
          const textBlock = getOrCreateResponsesTextBlock(
            blockMap,
            orderedBlocks,
            'chat:text',
          )
          const start = ensureBlockStarted(textBlock)
          if (start) {
            yield start
          }
          textBlock.text += delta.content
          yield createContentDeltaEvent(textBlock.index, {
            type: 'text_delta',
            text: delta.content,
          })
        }

        for (const toolCall of delta.tool_calls ?? []) {
          const key = `chat:tool:${toolCall.index ?? 0}`
          const name = String(toolCall.function?.name ?? '')
          const id = String(
            toolCall.id ?? `chat-tool-${toolCall.index ?? orderedBlocks.length}`,
          )
          const toolBlock = getOrCreateResponsesToolBlock(
            blockMap,
            orderedBlocks,
            key,
            name,
            id,
          )
          const start = ensureBlockStarted(toolBlock)
          if (start) {
            yield start
          }
          const partialJson = String(toolCall.function?.arguments ?? '')
          if (partialJson.length > 0 && toolBlock.type === 'tool_use') {
            toolBlock.input += partialJson
            yield createContentDeltaEvent(toolBlock.index, {
              type: 'input_json_delta',
              partial_json: partialJson,
            })
          }
        }

        usage = parseResponseUsage(
          (choice?.usage ?? payload.usage ?? {}) as Record<string, unknown>,
        )
        stopReason = choice?.finish_reason ?? stopReason
        continue
      }

      const eventType = String(payload.type ?? frame.event ?? '')
      switch (eventType) {
        case 'response.output_text.delta': {
          const key = `resp:text:${payload.item_id ?? payload.output_index ?? 0}:${payload.content_index ?? 0}`
          const textBlock = getOrCreateResponsesTextBlock(
            blockMap,
            orderedBlocks,
            key,
          )
          const start = ensureBlockStarted(textBlock)
          if (start) {
            yield start
          }
          const deltaText = String(payload.delta ?? '')
          textBlock.text += deltaText
          yield createContentDeltaEvent(textBlock.index, {
            type: 'text_delta',
            text: deltaText,
          })
          break
        }
        case 'response.output_text.done':
        case 'response.content_part.done': {
          const key = `resp:text:${payload.item_id ?? payload.output_index ?? 0}:${payload.content_index ?? 0}`
          const block = blockMap.get(key)
          if (block && !block.closed) {
            block.closed = true
            yield createContentBlockStopEvent(block.index)
          }
          break
        }
        case 'response.output_item.added': {
          const item = payload.item ?? payload.output_item ?? {}
          if (item.type === 'function_call') {
            const key =
              resolveResponsesToolKey(
                responsesToolAliasMap,
                item.call_id,
                item.id,
                payload.call_id,
                payload.item_id,
                payload.output_index,
              ) ?? `resp:tool:${randomUUID()}`
            const block = getOrCreateResponsesToolBlock(
              blockMap,
              orderedBlocks,
              key,
              String(item.name ?? ''),
              String(item.call_id ?? item.id ?? randomUUID()),
            )
            registerResponsesToolAliases(
              responsesToolAliasMap,
              key,
              item.call_id,
              item.id,
              payload.call_id,
              payload.item_id,
              payload.output_index,
            )
            const start = ensureBlockStarted(block)
            if (start) {
              yield start
            }
          }
          break
        }
        case 'response.function_call_arguments.delta': {
          const key =
            resolveResponsesToolKey(
              responsesToolAliasMap,
              payload.call_id,
              payload.item_id,
              payload.output_index,
            ) ?? `resp:tool:${randomUUID()}`
          const block = getOrCreateResponsesToolBlock(
            blockMap,
            orderedBlocks,
            key,
            String(payload.name ?? ''),
            String(payload.call_id ?? payload.item_id ?? randomUUID()),
          )
          registerResponsesToolAliases(
            responsesToolAliasMap,
            key,
            payload.call_id,
            payload.item_id,
            payload.output_index,
          )
          const start = ensureBlockStarted(block)
          if (start) {
            yield start
          }
          const partialJson = String(payload.delta ?? '')
          if (block.type === 'tool_use' && partialJson.length > 0) {
            block.input += partialJson
            yield createContentDeltaEvent(block.index, {
              type: 'input_json_delta',
              partial_json: partialJson,
            })
          }
          break
        }
        case 'response.function_call_arguments.done':
        case 'response.output_item.done': {
          const item = payload.item ?? payload.output_item ?? {}
          const key = resolveResponsesToolKey(
            responsesToolAliasMap,
            payload.call_id,
            item.call_id,
            item.id,
            payload.item_id,
            payload.output_index,
          )
          if (key) {
            registerResponsesToolAliases(
              responsesToolAliasMap,
              key,
              payload.call_id,
              item.call_id,
              item.id,
              payload.item_id,
              payload.output_index,
            )
          }
          const block = blockMap.get(key)
          if (block && !block.closed) {
            block.closed = true
            yield createContentBlockStopEvent(block.index)
          }
          break
        }
        case 'response.completed': {
          usage = parseResponseUsage(
            (payload.response?.usage ?? payload.usage ?? {}) as Record<string, unknown>,
          )
          stopReason = String(payload.response?.status ?? 'end_turn')
          break
        }
        default:
          break
      }
    }
  }

  for (const event of closeOpenBlocks()) {
    yield event
  }
  if (sentMessageStart) {
    yield createMessageDeltaEvent(usage, stopReason ?? 'end_turn')
    yield createMessageStopEvent()
  }
  yield buildAssistantFromStreamBlocks(orderedBlocks, usage)
}

async function performRequest(args: {
  messages: Message[]
  systemPrompt: SystemPrompt
  tools: Tools
  options: any
}): Promise<AssistantMessage> {
  const entry = resolveSaicodeModel(args.options.model)
  const provider = getProviderConfig(entry.alias)
  const system = flattenSystemPrompt(args.systemPrompt)
  const openAITools = await buildOpenAITools({
    tools: args.tools,
    model: entry.alias,
    api: provider.api,
    getToolPermissionContext:
      args.options.getToolPermissionContext ?? (async () => ({})),
    agents: args.options.agents ?? [],
    allowedAgentTypes: args.options.allowedAgentTypes,
  })

  if (!provider.apiKey && provider.id !== 'cliproxyapi') {
    throw new Error(
      `${provider.id} API key is missing. Set ${provider.id.toUpperCase()}_API_KEY or ~/.saicode/config.json`,
    )
  }

  if (provider.api === 'openai-chat-completions') {
    const requestBody = {
      model: entry.model,
      messages: [
        ...(system ? [{ role: 'system', content: system }] : []),
        ...convertMessagesToChatCompletions(args.messages),
      ],
      ...(openAITools ? { tools: openAITools } : {}),
      ...(args.options.outputFormat ? { response_format: args.options.outputFormat } : {}),
      tool_choice: openAITools ? 'auto' : undefined,
      max_tokens:
        args.options.maxOutputTokensOverride ??
        entry.maxOutputTokens,
    }
    const bodyText = JSON.stringify(requestBody)
    const response = await fetchWithLocalRateLimit(
      `${provider.baseUrl}/chat/completions`,
      provider,
      {
        method: 'POST',
        headers: getHeaders(provider),
        signal: args.options.signal,
        body: bodyText,
      },
      bodyText,
    )

    if (!response.ok) {
      throw new Error(await response.text())
    }

    return parseChatAssistant((await response.json()) as Record<string, any>)
  }

  const requestBody = {
    model: entry.model,
    ...(system ? { instructions: system } : {}),
    input: convertMessagesToResponsesInput(args.messages),
    ...(openAITools ? { tools: openAITools } : {}),
    ...(isEnvTruthy(process.env.SAICODE_PARALLEL_TOOL_CALLS)
      ? { parallel_tool_calls: true }
      : { parallel_tool_calls: false }),
    max_output_tokens:
      args.options.maxOutputTokensOverride ??
      entry.maxOutputTokens,
  }
  const bodyText = JSON.stringify(requestBody)
  const response = await fetchWithLocalRateLimit(
    `${provider.baseUrl}/responses`,
    provider,
    {
      method: 'POST',
      headers: getHeaders(provider),
      signal: args.options.signal,
      body: bodyText,
    },
    bodyText,
  )

  if (!response.ok) {
    throw new Error(await response.text())
  }

  return parseResponsesAssistant((await response.json()) as Record<string, any>)
}

export async function saicodeQueryModelWithoutStreaming(params: {
  messages: Message[]
  systemPrompt: SystemPrompt
  tools: Tools
  signal: AbortSignal
  options: any
}): Promise<AssistantMessage> {
  return performRequest({
    ...params,
    options: { ...params.options, signal: params.signal },
  })
}

export async function* saicodeQueryModelWithStreaming(params: {
  messages: Message[]
  systemPrompt: SystemPrompt
  tools: Tools
  signal: AbortSignal
  options: any
}): AsyncGenerator<StreamEvent | AssistantMessage | SystemAPIErrorMessage, void> {
  try {
    yield* performStreamingRequest({
      ...params,
      options: { ...params.options, signal: params.signal },
    })
  } catch (error) {
    const message =
      error instanceof Error ? error.message : 'saicode request failed'
    yield createAssistantAPIErrorMessage({
      content: message,
      apiError: 'api_error',
      errorDetails: message,
    })
  }
}

export async function saicodeQueryHaiku(args: {
  systemPrompt: SystemPrompt
  userPrompt: string
  outputFormat?: BetaJSONOutputFormat
  signal: AbortSignal
  options: any
}): Promise<AssistantMessage> {
  return saicodeQueryWithModel({
    ...args,
    options: {
      ...args.options,
      model:
        process.env.SAICODE_DEFAULT_HAIKU_MODEL ||
        process.env.SAICODE_SMALL_FAST_MODEL ||
        getSaicodeSmallFastModelId(),
    },
  })
}

export async function saicodeQueryWithModel(args: {
  systemPrompt: SystemPrompt
  userPrompt: string
  outputFormat?: BetaJSONOutputFormat
  signal: AbortSignal
  options: any
}): Promise<AssistantMessage> {
  const message = createUserMessage({ content: args.userPrompt })
  return saicodeQueryModelWithoutStreaming({
    messages: [message],
    systemPrompt: args.systemPrompt,
    tools: [],
    signal: args.signal,
    options: {
      ...args.options,
      outputFormat: args.outputFormat,
      getToolPermissionContext: async () => ({}),
    },
  })
}

export function saicodeVerifyApiKey(apiKey: string | undefined): boolean {
  return Boolean(apiKey && apiKey.trim())
}

export function saicodeGetAPIMetadata(): Record<string, never> {
  return {}
}

export function saicodeGetCacheControl(): undefined {
  return undefined
}

export function saicodeAdjustParamsForNonStreaming<
  T extends { max_tokens: number; thinking?: { budget_tokens?: number } },
>(params: T, maxTokensCap: number): T {
  const capped = Math.min(params.max_tokens, maxTokensCap)
  if (
    params.thinking &&
    typeof params.thinking.budget_tokens === 'number' &&
    params.thinking.budget_tokens >= capped
  ) {
    return {
      ...params,
      max_tokens: capped,
      thinking: {
        ...params.thinking,
        budget_tokens: capped - 1,
      },
    }
  }
  return {
    ...params,
    max_tokens: capped,
  }
}

export function saicodeGetMaxOutputTokensForModel(model: string): number {
  return resolveSaicodeModel(model).maxOutputTokens
}

export function saicodeUpdateUsage(
  usage: NonNullableUsage,
  update: Partial<NonNullableUsage> | undefined,
): NonNullableUsage {
  if (!update) return usage
  return {
    ...usage,
    input_tokens: usage.input_tokens + (update.input_tokens ?? 0),
    output_tokens: usage.output_tokens + (update.output_tokens ?? 0),
    cache_creation_input_tokens:
      usage.cache_creation_input_tokens +
      (update.cache_creation_input_tokens ?? 0),
    cache_read_input_tokens:
      usage.cache_read_input_tokens + (update.cache_read_input_tokens ?? 0),
  }
}

export function saicodeAccumulateUsage(
  total: NonNullableUsage,
  delta: NonNullableUsage,
): NonNullableUsage {
  return saicodeUpdateUsage(total, delta)
}
