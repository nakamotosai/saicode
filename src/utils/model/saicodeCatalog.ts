export type SaicodeModelEntry = {
  id: string
  provider: string
  model: string
  label: string
  description: string
  maxOutputTokens: number
  aliases: readonly string[]
}

const DEFAULT_MODEL_ID = 'nvidia/qwen/qwen3.5-397b-a17b'
const DEFAULT_BEST_MODEL_ID = 'nvidia/qwen/qwen3.5-397b-a17b'
const DEFAULT_SMALL_FAST_MODEL_ID = 'cliproxyapi/gpt-5.4-mini'

const SAICODE_MODELS: readonly SaicodeModelEntry[] = [
  {
    id: 'nvidia/qwen/qwen3.5-122b-a10b',
    provider: 'nvidia',
    model: 'qwen/qwen3.5-122b-a10b',
    label: 'Qwen 3.5 122B',
    description: '默认模型，适合大多数 coding agent 场景',
    maxOutputTokens: 65536,
    aliases: ['qwen-fast', 'qwen-122b', 'default'],
  },
  {
    id: 'nvidia/qwen/qwen3.5-397b-a17b',
    provider: 'nvidia',
    model: 'qwen/qwen3.5-397b-a17b',
    label: 'Qwen 3.5 397B',
    description: '更强的 Qwen 选项，适合更复杂的推理和长任务',
    maxOutputTokens: 81920,
    aliases: ['qwen-max', 'qwen-397b', 'qwen397', 'best'],
  },
  {
    id: 'nvidia/nvidia/nemotron-3-super-120b-a12b',
    provider: 'nvidia',
    model: 'nvidia/nemotron-3-super-120b-a12b',
    label: 'Nemotron 120B',
    description: 'NVIDIA Nemotron，适合技术问答和代码分析',
    maxOutputTokens: 32768,
    aliases: ['nemotron'],
  },
  {
    id: 'nvidia/openai/gpt-oss-120b',
    provider: 'nvidia',
    model: 'openai/gpt-oss-120b',
    label: 'GPT-OSS 120B',
    description: '开放权重路线的高质量通用模型',
    maxOutputTokens: 32768,
    aliases: ['gpt-oss'],
  },
  {
    id: 'cliproxyapi/gpt-5.4',
    provider: 'cliproxyapi',
    model: 'gpt-5.4',
    label: 'GPT-5.4',
    description: 'cliproxyapi 上的 Codex / GPT-5.4 路线',
    maxOutputTokens: 32768,
    aliases: ['codex', 'gpt-5.4'],
  },
  {
    id: 'cliproxyapi/gpt-5.4-mini',
    provider: 'cliproxyapi',
    model: 'gpt-5.4-mini',
    label: 'GPT-5.4 Mini',
    description: '更快更轻的 Codex 线路',
    maxOutputTokens: 32768,
    aliases: ['codex-mini', 'gpt-5.4-mini'],
  },
  {
    id: 'cliproxyapi/qwen/qwen3.5-122b-a10b',
    provider: 'cliproxyapi',
    model: 'qwen/qwen3.5-122b-a10b',
    label: 'Cliproxy Qwen 122B',
    description: '经 cliproxyapi 转发的 Qwen 3.5 122B',
    maxOutputTokens: 65536,
    aliases: ['cliproxy-qwen-fast'],
  },
  {
    id: 'cliproxyapi/qwen/qwen3.5-397b-a17b',
    provider: 'cliproxyapi',
    model: 'qwen/qwen3.5-397b-a17b',
    label: 'Cliproxy Qwen 397B',
    description: '经 cliproxyapi 转发的 Qwen 3.5 397B',
    maxOutputTokens: 81920,
    aliases: ['cliproxy-qwen-max'],
  },
  {
    id: 'cliproxyapi/nvidia/nemotron-3-super-120b-a12b',
    provider: 'cliproxyapi',
    model: 'nvidia/nemotron-3-super-120b-a12b',
    label: 'Cliproxy Nemotron 120B',
    description: '经 cliproxyapi 转发的 Nemotron',
    maxOutputTokens: 32768,
    aliases: ['cliproxy-nemotron'],
  },
  {
    id: 'cliproxyapi/openai/gpt-oss-120b',
    provider: 'cliproxyapi',
    model: 'openai/gpt-oss-120b',
    label: 'Cliproxy GPT-OSS 120B',
    description: '经 cliproxyapi 转发的 GPT-OSS',
    maxOutputTokens: 32768,
    aliases: ['cliproxy-gpt-oss'],
  },
  {
    id: 'cliproxyapi/opencode/qwen3.6-plus-free',
    provider: 'cliproxyapi',
    model: 'qwen3.6-plus-free',
    label: 'OpenCode Qwen 3.6 Plus Free',
    description: '经 cliproxyapi 转发的 OpenCode Zen Qwen 3.6 Plus Free',
    maxOutputTokens: 64000,
    aliases: ['opencode-qwen-free', 'qwen3.6-free'],
  },
  {
    id: 'cliproxyapi/opencode/mimo-v2-pro-free',
    provider: 'cliproxyapi',
    model: 'mimo-v2-pro-free',
    label: 'OpenCode MiMo V2 Pro Free',
    description: '经 cliproxyapi 转发的 OpenCode Zen MiMo V2 Pro Free',
    maxOutputTokens: 64000,
    aliases: ['opencode-mimo-pro-free', 'mimo-pro-free'],
  },
  {
    id: 'cliproxyapi/opencode/mimo-v2-omni-free',
    provider: 'cliproxyapi',
    model: 'mimo-v2-omni-free',
    label: 'OpenCode MiMo V2 Omni Free',
    description: '经 cliproxyapi 转发的 OpenCode Zen MiMo V2 Omni Free',
    maxOutputTokens: 64000,
    aliases: ['opencode-mimo-omni-free', 'mimo-omni-free'],
  },
] as const

const MODEL_INDEX = new Map<string, SaicodeModelEntry>()

for (const entry of SAICODE_MODELS) {
  MODEL_INDEX.set(entry.id.toLowerCase(), entry)
  for (const alias of entry.aliases) {
    MODEL_INDEX.set(alias.toLowerCase(), entry)
  }
}

export function isSaicodeModeEnabled(): boolean {
  return Boolean(
    process.env.SAICODE_PROVIDER ||
      process.env.SAICODE_MODEL ||
      process.env.SAICODE_DEFAULT_MODEL ||
      process.env.SAICODE_CONFIG_DIR,
  )
}

export function getSaicodeCatalog(): readonly SaicodeModelEntry[] {
  return SAICODE_MODELS
}

export function getSaicodeDefaultModelId(): string {
  return process.env.SAICODE_DEFAULT_MODEL || DEFAULT_MODEL_ID
}

export function getSaicodeBestModelId(): string {
  return process.env.SAICODE_DEFAULT_BEST_MODEL || DEFAULT_BEST_MODEL_ID
}

export function getSaicodeSmallFastModelId(): string {
  return process.env.SAICODE_SMALL_FAST_MODEL || DEFAULT_SMALL_FAST_MODEL_ID
}

export function getSaicodeModelEntry(
  modelInput: string | null | undefined,
): SaicodeModelEntry | undefined {
  if (!modelInput) return undefined
  return MODEL_INDEX.get(modelInput.trim().toLowerCase())
}

export function resolveSaicodeModelId(
  modelInput: string | null | undefined,
): string {
  if (!modelInput) return getSaicodeDefaultModelId()
  const direct = getSaicodeModelEntry(modelInput)
  if (direct) return direct.id
  return modelInput.trim()
}

export function resolveSaicodeModel(modelInput: string | null | undefined): {
  alias: string
  provider: string
  model: string
  maxOutputTokens: number
} {
  const resolvedId = resolveSaicodeModelId(modelInput)
  const entry = getSaicodeModelEntry(resolvedId)
  if (entry) {
    return {
      alias: entry.id,
      provider: entry.provider,
      model: entry.model,
      maxOutputTokens: entry.maxOutputTokens,
    }
  }

  const inferredProvider =
    process.env.SAICODE_PROVIDER ||
    process.env.SAICODE_DEFAULT_PROVIDER ||
    (resolvedId.startsWith('cliproxyapi/')
      ? 'cliproxyapi'
      : resolvedId.startsWith('nvidia/')
        ? 'nvidia'
        : 'nvidia')

  if (resolvedId.includes('/')) {
    const [, ...rest] = resolvedId.split('/')
    return {
      alias: resolvedId,
      provider: inferredProvider,
      model: rest.join('/') || resolvedId,
      maxOutputTokens: 32768,
    }
  }

  return {
    alias: resolvedId,
    provider: inferredProvider,
    model: resolvedId,
    maxOutputTokens: 32768,
  }
}

export function getSaicodeModelDisplayName(
  modelInput: string | null | undefined,
): string {
  if (modelInput === null) {
    const entry = getSaicodeModelEntry(getSaicodeDefaultModelId())
    return entry
      ? `Default (${entry.label})`
      : `Default (${getSaicodeDefaultModelId()})`
  }

  const entry = getSaicodeModelEntry(modelInput)
  if (entry) {
    return entry.label
  }

  return resolveSaicodeModelId(modelInput)
}

export function getSaicodeModelDescription(
  modelInput: string | null | undefined,
): string | undefined {
  const entry = getSaicodeModelEntry(modelInput)
  return entry?.description
}

export function getSaicodeModelOptions(): Array<{
  value: string | null
  label: string
  description: string
  descriptionForModel?: string
}> {
  const defaultEntry = getSaicodeModelEntry(getSaicodeDefaultModelId())
  return [
    {
      value: null,
      label: 'Default (recommended)',
      description: defaultEntry
        ? `Use the default model (currently ${defaultEntry.label})`
        : `Use the default model (currently ${getSaicodeDefaultModelId()})`,
      descriptionForModel: defaultEntry?.description,
    },
    ...SAICODE_MODELS.map(entry => ({
      value: entry.id,
      label: entry.label,
      description: entry.description,
      descriptionForModel: `${entry.label} · ${entry.id}`,
    })),
  ]
}

export function getSaicodeAgentModelOptions(): Array<{
  value: string
  label: string
  description: string
}> {
  return [
    {
      value: 'inherit',
      label: 'Inherit from parent',
      description: 'Use the same model as the main conversation',
    },
    ...SAICODE_MODELS.map(entry => ({
      value: entry.id,
      label: entry.label,
      description: entry.description,
    })),
  ]
}

export const SAICODE_MODEL_ALIASES = Array.from(
  new Set(
    SAICODE_MODELS.flatMap(entry => [entry.id, ...entry.aliases]).concat('best'),
  ),
)
