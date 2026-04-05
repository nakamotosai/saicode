import { readFileSync } from 'fs'

export type OutputFormat = 'text' | 'json'

export type LightweightHeadlessPrintArgs = {
  print: boolean
  bare: boolean
  dangerouslySkipPermissions: boolean
  allowDangerouslySkipPermissions: boolean
  model: string | undefined
  systemPrompt: string | undefined
  systemPromptFile: string | undefined
  appendSystemPrompt: string | undefined
  appendSystemPromptFile: string | undefined
  permissionMode: string | undefined
  fallbackModel: string | undefined
  jsonSchema: string | undefined
  maxTurns: number | undefined
  maxBudgetUsd: number | undefined
  taskBudget: number | undefined
  name: string | undefined
  outputFormat: OutputFormat
  allowedTools: string[]
  tools: string[]
  prompt: string
}

const SINGLE_VALUE_FLAGS = new Set([
  '--model',
  '--system-prompt',
  '--system-prompt-file',
  '--append-system-prompt',
  '--append-system-prompt-file',
  '--permission-mode',
  '--fallback-model',
  '--json-schema',
  '--max-turns',
  '--max-budget-usd',
  '--task-budget',
  '--output-format',
  '--name',
  '-n',
])

const VARIADIC_FLAGS = new Set([
  '--allowedTools',
  '--allowed-tools',
  '--tools',
])

function collectVariadicValues(
  cliArgs: string[],
  startIndex: number,
): { values: string[]; nextIndex: number } {
  const values: string[] = []
  let index = startIndex + 1

  while (index < cliArgs.length) {
    const value = cliArgs[index]
    if (!value) {
      index += 1
      continue
    }
    if (value.startsWith('-')) {
      break
    }
    values.push(value)
    index += 1
  }

  return {
    values,
    nextIndex: index - 1,
  }
}

function readFlagValue(
  cliArgs: string[],
  index: number,
): { value: string; nextIndex: number } {
  const arg = cliArgs[index]
  if (!arg) {
    throw new Error('Missing CLI flag')
  }

  const equalIndex = arg.indexOf('=')
  if (equalIndex !== -1) {
    return {
      value: arg.slice(equalIndex + 1),
      nextIndex: index,
    }
  }

  const value = cliArgs[index + 1]
  if (!value || value.startsWith('-')) {
    throw new Error(`Missing value for ${arg}`)
  }

  return {
    value,
    nextIndex: index + 1,
  }
}

function parsePositiveInteger(
  flag: string,
  value: string,
): number {
  const parsed = Number(value)
  if (!Number.isInteger(parsed) || parsed <= 0) {
    throw new Error(`${flag} must be a positive integer`)
  }
  return parsed
}

function parsePositiveNumber(
  flag: string,
  value: string,
): number {
  const parsed = Number(value)
  if (!Number.isFinite(parsed) || parsed <= 0) {
    throw new Error(`${flag} must be a positive number`)
  }
  return parsed
}

export function parseLightweightHeadlessPrintArgs(
  cliArgs: string[],
): LightweightHeadlessPrintArgs {
  const parsed: LightweightHeadlessPrintArgs = {
    print: false,
    bare: false,
    dangerouslySkipPermissions: false,
    allowDangerouslySkipPermissions: false,
    model: undefined,
    systemPrompt: undefined,
    systemPromptFile: undefined,
    appendSystemPrompt: undefined,
    appendSystemPromptFile: undefined,
    permissionMode: undefined,
    fallbackModel: undefined,
    jsonSchema: undefined,
    maxTurns: undefined,
    maxBudgetUsd: undefined,
    taskBudget: undefined,
    name: undefined,
    outputFormat: 'text',
    allowedTools: [],
    tools: [],
    prompt: '',
  }

  const positional: string[] = []

  for (let i = 0; i < cliArgs.length; i++) {
    const arg = cliArgs[i]
    if (!arg) continue

    if (arg === '-p' || arg === '--print') {
      parsed.print = true
      continue
    }
    if (arg === '--bare') {
      parsed.bare = true
      continue
    }
    if (arg === '--dangerously-skip-permissions') {
      parsed.dangerouslySkipPermissions = true
      continue
    }
    if (arg === '--allow-dangerously-skip-permissions') {
      parsed.allowDangerouslySkipPermissions = true
      continue
    }

    const normalizedSingleFlag = (() => {
      if (arg.startsWith('--model=')) return '--model'
      if (arg.startsWith('--system-prompt=')) return '--system-prompt'
      if (arg.startsWith('--system-prompt-file=')) return '--system-prompt-file'
      if (arg.startsWith('--append-system-prompt=')) {
        return '--append-system-prompt'
      }
      if (arg.startsWith('--append-system-prompt-file=')) {
        return '--append-system-prompt-file'
      }
      if (arg.startsWith('--permission-mode=')) return '--permission-mode'
      if (arg.startsWith('--fallback-model=')) return '--fallback-model'
      if (arg.startsWith('--json-schema=')) return '--json-schema'
      if (arg.startsWith('--max-turns=')) return '--max-turns'
      if (arg.startsWith('--max-budget-usd=')) return '--max-budget-usd'
      if (arg.startsWith('--task-budget=')) return '--task-budget'
      if (arg.startsWith('--output-format=')) return '--output-format'
      if (arg.startsWith('--name=')) return '--name'
      return arg
    })()

    if (SINGLE_VALUE_FLAGS.has(normalizedSingleFlag)) {
      const { value, nextIndex } = readFlagValue(cliArgs, i)

      switch (normalizedSingleFlag) {
        case '--model':
          parsed.model = value
          break
        case '--system-prompt':
          parsed.systemPrompt = value
          break
        case '--system-prompt-file':
          parsed.systemPromptFile = value
          break
        case '--append-system-prompt':
          parsed.appendSystemPrompt = value
          break
        case '--append-system-prompt-file':
          parsed.appendSystemPromptFile = value
          break
        case '--permission-mode':
          parsed.permissionMode = value
          break
        case '--fallback-model':
          parsed.fallbackModel = value
          break
        case '--json-schema':
          parsed.jsonSchema = value
          break
        case '--max-turns':
          parsed.maxTurns = parsePositiveInteger('--max-turns', value)
          break
        case '--max-budget-usd':
          parsed.maxBudgetUsd = parsePositiveNumber('--max-budget-usd', value)
          break
        case '--task-budget':
          parsed.taskBudget = parsePositiveInteger('--task-budget', value)
          break
        case '--output-format':
          if (value !== 'text' && value !== 'json') {
            throw new Error(
              '--output-format only supports "text" or "json" in lightweight headless mode',
            )
          }
          parsed.outputFormat = value
          break
        case '--name':
        case '-n':
          parsed.name = value
          break
      }

      i = nextIndex
      continue
    }

    if (VARIADIC_FLAGS.has(arg)) {
      const { values, nextIndex } = collectVariadicValues(cliArgs, i)
      if (arg === '--tools') {
        parsed.tools.push(...values)
      } else {
        parsed.allowedTools.push(...values)
      }
      i = nextIndex
      continue
    }

    if (arg.startsWith('-')) {
      throw new Error(
        `Unsupported flag for lightweight headless mode: ${arg}`,
      )
    }

    positional.push(arg)
  }

  parsed.prompt = positional.join(' ').trim()
  return parsed
}

export function buildAutoBareOptions(
  parsed: LightweightHeadlessPrintArgs,
): {
  print: boolean
  bare: boolean
  systemPrompt?: string
  systemPromptFile?: string
  appendSystemPrompt?: string
  appendSystemPromptFile?: string
  allowedTools?: string[]
  tools?: string[]
} {
  return {
    print: parsed.print,
    bare: parsed.bare,
    systemPrompt: parsed.systemPrompt,
    systemPromptFile: parsed.systemPromptFile,
    appendSystemPrompt: parsed.appendSystemPrompt,
    appendSystemPromptFile: parsed.appendSystemPromptFile,
    allowedTools: parsed.allowedTools,
    tools: parsed.tools,
  }
}

export function resolvePromptText(
  inlinePrompt: string | undefined,
  filePath: string | undefined,
  flagName: string,
  fileFlagName: string,
): string | undefined {
  if (inlinePrompt && filePath) {
    throw new Error(`Cannot use both ${flagName} and ${fileFlagName}`)
  }

  if (!filePath) {
    return inlinePrompt
  }

  return readFileSync(filePath, 'utf8')
}
