export type SaicodeModelEntry = {
  id: string
  provider: string
  model: string
  label: string
  description: string
  maxOutputTokens: number
  aliases: readonly string[]
}

const DEFAULT_MODEL_ID = 'cpa/qwen/qwen3.5-397b-a17b'
const DEFAULT_BEST_MODEL_ID = 'cpa/qwen/qwen3.5-397b-a17b'
const DEFAULT_SMALL_FAST_MODEL_ID = 'cpa/gpt-5.4-mini'

const SAICODE_MODELS: readonly SaicodeModelEntry[] = [
  {
    id: 'cpa/qwen/qwen3.5-122b-a10b',
    provider: 'cpa',
    model: 'qwen/qwen3.5-122b-a10b',
    label: 'Qwen 3.5 122B',
    description: '经 cliproxyapi 转发的 Qwen 3.5 122B',
    maxOutputTokens: 65536,
    aliases: [
      'qwen-fast',
      'qwen-122b',
      'cliproxyapi/qwen/qwen3.5-122b-a10b',
      'nvidia/qwen/qwen3.5-122b-a10b',
      'cliproxy-qwen-fast',
    ],
  },
  {
    id: 'cpa/qwen/qwen3.5-397b-a17b',
    provider: 'cpa',
    model: 'qwen/qwen3.5-397b-a17b',
    label: 'Qwen 3.5 397B',
    description: '经 cliproxyapi 转发的 Qwen 3.5 397B',
    maxOutputTokens: 81920,
    aliases: [
      'qwen-max',
      'qwen-397b',
      'qwen397',
      'best',
      'default',
      'cliproxyapi/qwen/qwen3.5-397b-a17b',
      'nvidia/qwen/qwen3.5-397b-a17b',
      'cliproxy-qwen-max',
    ],
  },
  {
    id: 'cpa/qwen3-coder-plus',
    provider: 'cpa',
    model: 'qwen3-coder-plus',
    label: 'Qwen3 Coder Plus',
    description: '适合编码、重构和代码理解的 Qwen 编码模型',
    maxOutputTokens: 32768,
    aliases: [
      'qwen-coder-plus',
      'qwen-coder',
      'cliproxyapi/qwen3-coder-plus',
    ],
  },
  {
    id: 'cpa/vision-model',
    provider: 'cpa',
    model: 'vision-model',
    label: 'Qwen3 Vision Model',
    description: '适合识图、截图理解和多模态分析的 Qwen 视觉模型',
    maxOutputTokens: 32768,
    aliases: [
      'qwen-vision',
      'qwen3-vision',
      'vision',
      'cliproxyapi/vision-model',
    ],
  },
  {
    id: 'cpa/nvidia/nemotron-3-super-120b-a12b',
    provider: 'cpa',
    model: 'nvidia/nemotron-3-super-120b-a12b',
    label: 'Nemotron 120B',
    description: '经 cliproxyapi 转发的 Nemotron',
    maxOutputTokens: 32768,
    aliases: [
      'nemotron',
      'cliproxy-nemotron',
      'cliproxyapi/nvidia/nemotron-3-super-120b-a12b',
      'nvidia/nvidia/nemotron-3-super-120b-a12b',
    ],
  },
  {
    id: 'cpa/openai/gpt-oss-120b',
    provider: 'cpa',
    model: 'openai/gpt-oss-120b',
    label: 'GPT-OSS 120B',
    description: '经 cliproxyapi 转发的 GPT-OSS',
    maxOutputTokens: 32768,
    aliases: [
      'gpt-oss',
      'cliproxy-gpt-oss',
      'cliproxyapi/openai/gpt-oss-120b',
      'nvidia/openai/gpt-oss-120b',
    ],
  },
  {
    id: 'cpa/gpt-5.4',
    provider: 'cpa',
    model: 'gpt-5.4',
    label: 'GPT-5.4',
    description: 'cliproxyapi 上的 Codex / GPT-5.4 路线',
    maxOutputTokens: 32768,
    aliases: ['codex', 'gpt-5.4', 'cliproxyapi/gpt-5.4'],
  },
  {
    id: 'cpa/gpt-5.4-mini',
    provider: 'cpa',
    model: 'gpt-5.4-mini',
    label: 'GPT-5.4 Mini',
    description: '更快更轻的 Codex 线路',
    maxOutputTokens: 32768,
    aliases: ['codex-mini', 'gpt-5.4-mini', 'cliproxyapi/gpt-5.4-mini'],
  },
  {
    id: 'cpa/opencode/qwen3.6-plus-free',
    provider: 'cpa',
    model: 'qwen3.6-plus-free',
    label: 'OpenCode Qwen 3.6 Plus Free',
    description: '经 cliproxyapi 转发的 OpenCode Zen Qwen 3.6 Plus Free',
    maxOutputTokens: 64000,
    aliases: [
      'opencode-qwen-free',
      'qwen3.6-free',
      'cliproxyapi/opencode/qwen3.6-plus-free',
    ],
  },
  {
    id: 'cpa/opencode/mimo-v2-pro-free',
    provider: 'cpa',
    model: 'mimo-v2-pro-free',
    label: 'OpenCode MiMo V2 Pro Free',
    description: '经 cliproxyapi 转发的 OpenCode Zen MiMo V2 Pro Free',
    maxOutputTokens: 64000,
    aliases: [
      'opencode-mimo-pro-free',
      'mimo-pro-free',
      'cliproxyapi/opencode/mimo-v2-pro-free',
    ],
  },
  {
    id: 'cpa/opencode/mimo-v2-omni-free',
    provider: 'cpa',
    model: 'mimo-v2-omni-free',
    label: 'OpenCode MiMo V2 Omni Free',
    description: '经 cliproxyapi 转发的 OpenCode Zen MiMo V2 Omni Free',
    maxOutputTokens: 64000,
    aliases: [
      'opencode-mimo-omni-free',
      'mimo-omni-free',
      'cliproxyapi/opencode/mimo-v2-omni-free',
    ],
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
    (resolvedId.startsWith('cpa/')
      ? 'cpa'
      : resolvedId.startsWith('cliproxyapi/')
      ? 'cliproxyapi'
      : resolvedId.startsWith('nvidia/')
        ? 'nvidia'
        : 'cpa')

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
