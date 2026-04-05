import { isEnvDefinedFalsy, isEnvTruthy } from './envUtils.js'
import {
  usesOnlyLightweightHeadlessTools,
  usesOnlySimpleTools,
} from './lightweightHeadlessTools.js'

function collectVariadicOptionValues(
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

export function shouldUseRecoveryEntrypoint(cliArgs: string[]): boolean {
  if (process.env.SAICODE_FORCE_RECOVERY_CLI === '1') {
    return true
  }

  const isPrintMode =
    cliArgs.includes('-p') || cliArgs.includes('--print')
  if (!isPrintMode) {
    return false
  }

  for (let i = 0; i < cliArgs.length; i++) {
    const arg = cliArgs[i]
    if (!arg) continue

    if (
      arg === '-p' ||
      arg === '--print' ||
      arg === '--bare' ||
      arg === '--dangerously-skip-permissions' ||
      arg === '-h' ||
      arg === '--help' ||
      arg === '-v' ||
      arg === '-V' ||
      arg === '--version'
    ) {
      continue
    }

    if (
      arg === '--model' ||
      arg === '--system-prompt' ||
      arg === '--system-prompt-file' ||
      arg === '--append-system-prompt'
    ) {
      i += 1
      continue
    }

    if (arg === '--output-format') {
      const value = cliArgs[i + 1]
      if (value === 'stream-json') {
        return false
      }
      i += 1
      continue
    }

    if (
      arg === '--input-format' ||
      arg === '--include-hook-events' ||
      arg === '--include-partial-messages' ||
      arg === '--replay-user-messages' ||
      arg === '--tools' ||
      arg === '--allowedTools' ||
      arg === '--allowed-tools' ||
      arg === '--disallowedTools' ||
      arg === '--disallowed-tools' ||
      arg === '--permission-prompt-tool' ||
      arg === '--mcp-config' ||
      arg === '--sdk-url' ||
      arg === '--session-id' ||
      arg === '--continue' ||
      arg === '--resume' ||
      arg === '--fork-session' ||
      arg === '--max-turns' ||
      arg === '--max-budget-usd' ||
      arg === '--agent' ||
      arg === '--agents'
    ) {
      return false
    }

    if (arg.startsWith('-')) {
      return false
    }
  }

  return true
}

type AutoBareOptions = {
  print?: boolean
  bare?: boolean
  continue?: boolean
  resume?: unknown
  sdkUrl?: string
  inputFormat?: string
  systemPrompt?: string
  systemPromptFile?: string
  appendSystemPrompt?: string
  appendSystemPromptFile?: string
  addDir?: string[]
  mcpConfig?: string[]
  pluginDir?: string[]
  allowedTools?: string[]
  tools?: string[]
  agents?: string
  agent?: string
  init?: boolean
  initOnly?: boolean
  maintenance?: boolean
}

export function shouldAutoEnableBarePrint(
  options: AutoBareOptions,
): boolean {
  if (!options.print || options.bare) {
    return false
  }

  if (isEnvTruthy(process.env.CLAUDE_CODE_SIMPLE)) {
    return false
  }

  if (isEnvDefinedFalsy(process.env.SAICODE_AUTO_BARE_PRINT)) {
    return false
  }

  if (
    options.continue ||
    options.resume ||
    options.sdkUrl ||
    options.inputFormat === 'stream-json'
  ) {
    return false
  }

  if (
    options.systemPrompt ||
    options.systemPromptFile ||
    options.appendSystemPrompt ||
    options.appendSystemPromptFile
  ) {
    return false
  }

  if (
    (options.addDir?.length ?? 0) > 0 ||
    (options.mcpConfig?.length ?? 0) > 0 ||
    (options.pluginDir?.length ?? 0) > 0
  ) {
    return false
  }

  if (
    ((options.allowedTools?.length ?? 0) > 0 &&
      !usesOnlySimpleTools(options.allowedTools ?? [])) ||
    ((options.tools?.length ?? 0) > 0 &&
      !usesOnlySimpleTools(options.tools ?? []))
  ) {
    return false
  }

  if (
    options.agents ||
    options.agent ||
    options.init ||
    options.initOnly ||
    options.maintenance
  ) {
    return false
  }

  return true
}

function isRestrictedToolPrintCandidate(
  cliArgs: string[],
  acceptsTools: (values: string[]) => boolean,
): boolean {
  if (isEnvDefinedFalsy(process.env.SAICODE_AUTO_BARE_PRINT)) {
    return false
  }

  const isPrintMode =
    cliArgs.includes('-p') || cliArgs.includes('--print')
  if (!isPrintMode) {
    return false
  }

  let sawSimpleToolRestriction = false

  for (let i = 0; i < cliArgs.length; i++) {
    const arg = cliArgs[i]
    if (!arg) continue

    if (
      arg === '-p' ||
      arg === '--print' ||
      arg === '--bare' ||
      arg === '--dangerously-skip-permissions' ||
      arg === '--allow-dangerously-skip-permissions'
    ) {
      continue
    }

    if (
      arg === '-h' ||
      arg === '--help' ||
      arg === '-v' ||
      arg === '-V' ||
      arg === '--version'
    ) {
      return false
    }

    if (
      arg === '--model' ||
      arg === '--system-prompt' ||
      arg === '--system-prompt-file' ||
      arg === '--append-system-prompt' ||
      arg === '--append-system-prompt-file' ||
      arg === '--permission-mode' ||
      arg === '--fallback-model' ||
      arg === '--json-schema' ||
      arg === '--max-turns' ||
      arg === '--max-budget-usd' ||
      arg === '--task-budget' ||
      arg === '--name' ||
      arg === '-n'
    ) {
      i += 1
      continue
    }

    if (arg === '--output-format') {
      const value = cliArgs[i + 1]
      if (value === 'stream-json') {
        return false
      }
      i += 1
      continue
    }

    if (
      arg === '--tools' ||
      arg === '--allowedTools' ||
      arg === '--allowed-tools'
    ) {
      const { values, nextIndex } = collectVariadicOptionValues(cliArgs, i)
      if (!acceptsTools(values)) {
        return false
      }
      sawSimpleToolRestriction = true
      i = nextIndex
      continue
    }

    if (
      arg === '--disallowedTools' ||
      arg === '--disallowed-tools' ||
      arg === '--input-format' ||
      arg === '--include-hook-events' ||
      arg === '--include-partial-messages' ||
      arg === '--replay-user-messages' ||
      arg === '--permission-prompt-tool' ||
      arg === '--mcp-config' ||
      arg === '--sdk-url' ||
      arg === '--session-id' ||
      arg === '--continue' ||
      arg === '--resume' ||
      arg === '-c' ||
      arg === '-r' ||
      arg === '--fork-session' ||
      arg === '--agent' ||
      arg === '--agents' ||
      arg === '--plugin-dir' ||
      arg === '--add-dir' ||
      arg === '--settings' ||
      arg === '--strict-mcp-config' ||
      arg === '--ide' ||
      arg === '--init' ||
      arg === '--init-only' ||
      arg === '--maintenance'
    ) {
      return false
    }

    if (arg.startsWith('-')) {
      return false
    }
  }

  return sawSimpleToolRestriction
}

export function shouldPreEnableSimpleMode(cliArgs: string[]): boolean {
  if (isEnvTruthy(process.env.CLAUDE_CODE_SIMPLE)) {
    return false
  }

  return isRestrictedToolPrintCandidate(cliArgs, usesOnlySimpleTools)
}

export function shouldUseLightweightHeadlessPrintEntrypoint(
  cliArgs: string[],
): boolean {
  return isRestrictedToolPrintCandidate(
    cliArgs,
    usesOnlyLightweightHeadlessTools,
  )
}
