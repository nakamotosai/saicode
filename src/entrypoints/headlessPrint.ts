import {
  shouldAutoEnableBarePrint,
  shouldPreEnableSimpleMode,
} from '../utils/nonInteractiveMode.js'
import { LIGHTWEIGHT_HEADLESS_TOOL_NAMES } from '../utils/lightweightHeadlessTools.js'
import type { LightweightHeadlessPrintArgs } from './headlessPrintArgs.js'
import {
  buildAutoBareOptions,
  parseLightweightHeadlessPrintArgs,
  resolvePromptText,
} from './headlessPrintArgs.js'

export type LightweightHeadlessExecutionRequest = {
  cliArgs: string[]
  parsed: LightweightHeadlessPrintArgs
  prompt: string
  shouldEnableSimpleMode: boolean
}

async function readInputPrompt(
  prompt: string,
  allowStdin: boolean,
): Promise<string> {
  if (!allowStdin || process.stdin.isTTY) {
    return prompt
  }

  process.stdin.setEncoding('utf8')
  let data = ''
  for await (const chunk of process.stdin) {
    data += chunk
  }

  return [prompt, data].filter(Boolean).join('\n')
}

export async function resolveLightweightHeadlessExecutionRequest(
  cliArgs: string[],
  options?: {
    allowStdin?: boolean
  },
): Promise<LightweightHeadlessExecutionRequest> {
  const parsed = parseLightweightHeadlessPrintArgs(cliArgs)
  const shouldEnableSimpleMode =
    parsed.bare ||
    shouldPreEnableSimpleMode(cliArgs) ||
    shouldAutoEnableBarePrint(buildAutoBareOptions(parsed))

  const prompt = await readInputPrompt(parsed.prompt, options?.allowStdin ?? true)

  return {
    cliArgs: [...cliArgs],
    parsed,
    prompt,
    shouldEnableSimpleMode,
  }
}

export function shouldFallbackToFullCli(
  request: LightweightHeadlessExecutionRequest,
): boolean {
  return request.prompt.trimStart().startsWith('/')
}

export function applyLightweightHeadlessProcessPrelude(
  shouldEnableSimpleMode: boolean,
): void {
  process.env.COREPACK_ENABLE_AUTO_PIN = '0'
  process.env.NoDefaultCurrentDirectoryInExePath = '1'

  if (shouldEnableSimpleMode) {
    process.env.CLAUDE_CODE_SIMPLE = '1'
  }

  process.env.SAICODE_LIGHTWEIGHT_HEADLESS = '1'
}

export async function initializeLightweightHeadlessRuntime(): Promise<void> {
  const {
    setClientType,
    setIsInteractive,
    setQuestionPreviewFormat,
  } = await import('../bootstrap/state.js')
  setIsInteractive(false)
  setClientType('sdk-cli')
  setQuestionPreviewFormat('markdown')
  process.env.CLAUDE_CODE_ENTRYPOINT ??= 'sdk-cli'

  if (process.env.SAICODE_ENABLE_LIGHTWEIGHT_HEADLESS_INIT === '1') {
    const { initLightweightHeadless } = await import(
      './initLightweightHeadless.js'
    )
    await initLightweightHeadless()
    return
  }

  const { init } = await import('./init.js')
  await init()
}

export async function runLightweightHeadlessExecutionRequest(
  request: LightweightHeadlessExecutionRequest,
  options?: {
    cwd?: string
  },
): Promise<void> {
  const { parsed, prompt } = request
  const cwd = options?.cwd ?? process.cwd()

  const systemPrompt = resolvePromptText(
    parsed.systemPrompt,
    parsed.systemPromptFile,
    '--system-prompt',
    '--system-prompt-file',
  )
  const appendSystemPrompt = resolvePromptText(
    parsed.appendSystemPrompt,
    parsed.appendSystemPromptFile,
    '--append-system-prompt',
    '--append-system-prompt-file',
  )

  const {
    initialPermissionModeFromCLI,
    initializeToolPermissionContext,
  } = await import('../utils/permissions/permissionSetup.js')
  const { setSessionBypassPermissionsMode } = await import(
    '../bootstrap/state.js'
  )
  const { mode: permissionMode } = initialPermissionModeFromCLI({
    permissionModeCli: parsed.permissionMode,
    dangerouslySkipPermissions: parsed.dangerouslySkipPermissions,
  })
  setSessionBypassPermissionsMode(permissionMode === 'bypassPermissions')

  const initResult = await initializeToolPermissionContext({
    allowedToolsCli: parsed.allowedTools,
    disallowedToolsCli: [],
    baseToolsCli: parsed.tools,
    toolUniverseCli: Array.from(LIGHTWEIGHT_HEADLESS_TOOL_NAMES),
    permissionMode,
    allowDangerouslySkipPermissions: parsed.allowDangerouslySkipPermissions,
    addDirs: [],
  })
  initResult.warnings.forEach(warning => {
    process.stderr.write(`${warning}\n`)
  })

  const { setupLightweightHeadless } = await import(
    '../setupLightweightHeadless.js'
  )
  await setupLightweightHeadless(
    cwd,
    permissionMode,
    parsed.allowDangerouslySkipPermissions,
  )

  const { applyConfigEnvironmentVariables } = await import(
    '../utils/managedEnv.js'
  )
  applyConfigEnvironmentVariables()

  const { validateForceLoginOrg } = await import('../utils/auth.js')
  const orgValidation = await validateForceLoginOrg()
  if (!orgValidation.valid) {
    const message =
      'message' in orgValidation
        ? orgValidation.message
        : 'Organization validation failed.'
    process.stderr.write(`${message}\n`)
    process.exitCode = 1
    return
  }

  if (parsed.outputFormat === 'json') {
    const { setHasFormattedOutput } = await import('../utils/debug.js')
    setHasFormattedOutput(true)
  }

  if (parsed.name?.trim()) {
    const { cacheSessionTitle } = await import('../utils/sessionStorage.js')
    cacheSessionTitle(parsed.name.trim())
  }

  let tools =
    parsed.allowedTools.length > 0 || parsed.tools.length > 0
      ? await (
          await import('../utils/lightweightHeadlessToolLoader.js')
        ).loadRequestedLightweightHeadlessTools({
          allowedTools: parsed.allowedTools,
          tools: parsed.tools,
          permissionContext: initResult.toolPermissionContext,
        })
      : (
          await import('../tools.js')
        ).getTools(initResult.toolPermissionContext)

  let jsonSchema: Record<string, unknown> | undefined
  if (parsed.jsonSchema) {
    jsonSchema = JSON.parse(parsed.jsonSchema) as Record<string, unknown>
    const { createSyntheticOutputTool } = await import(
      '../tools/SyntheticOutputTool/SyntheticOutputTool.js'
    )
    const syntheticTool = createSyntheticOutputTool(jsonSchema)
    if (!('tool' in syntheticTool)) {
      throw new Error(
        syntheticTool.error || 'Invalid JSON schema for structured output',
      )
    }
    tools = [...tools, syntheticTool.tool]
  }

  const { shouldEnableThinkingByDefault } = await import(
    '../utils/thinking.js'
  )
  const thinkingConfig =
    shouldEnableThinkingByDefault() !== false
      ? ({ type: 'adaptive' } as const)
      : ({ type: 'disabled' } as const)

  const { getDefaultAppState } = await import('../state/AppStateStore.js')
  const { onChangeAppState } = await import('../state/onChangeAppState.js')
  const { createStore } = await import('../state/store.js')
  const defaultState = getDefaultAppState()
  const headlessStore = createStore(
    {
      ...defaultState,
      toolPermissionContext: initResult.toolPermissionContext,
    },
    onChangeAppState,
  )

  if (
    parsed.allowDangerouslySkipPermissions ||
    initResult.toolPermissionContext.mode === 'bypassPermissions'
  ) {
    const { checkAndDisableBypassPermissions } = await import(
      '../utils/permissions/permissionSetup.js'
    )
    void checkAndDisableBypassPermissions(initResult.toolPermissionContext)
  }

  const { runHeadless } = await import('../cli/print.js')
  await runHeadless(
    prompt,
    () => headlessStore.getState(),
    headlessStore.setState,
    [],
    tools,
    {},
    [],
    {
      continue: undefined,
      resume: undefined,
      resumeSessionAt: undefined,
      verbose: undefined,
      outputFormat: parsed.outputFormat,
      jsonSchema,
      permissionPromptToolName: undefined,
      allowedTools: parsed.allowedTools,
      thinkingConfig,
      maxTurns: parsed.maxTurns,
      maxBudgetUsd: parsed.maxBudgetUsd,
      taskBudget: parsed.taskBudget
        ? { total: parsed.taskBudget }
        : undefined,
      systemPrompt,
      appendSystemPrompt,
      userSpecifiedModel: parsed.model,
      fallbackModel: parsed.fallbackModel,
      teleport: undefined,
      sdkUrl: undefined,
      replayUserMessages: undefined,
      includePartialMessages: undefined,
      forkSession: false,
      rewindFiles: undefined,
      enableAuthStatus: undefined,
      agent: undefined,
      workload: undefined,
    },
  )
}

export async function main(): Promise<void> {
  const request = await resolveLightweightHeadlessExecutionRequest(
    process.argv.slice(2),
  )

  if (shouldFallbackToFullCli(request)) {
    await import('./cli.js')
    return
  }

  applyLightweightHeadlessProcessPrelude(request.shouldEnableSimpleMode)
  await initializeLightweightHeadlessRuntime()
  await runLightweightHeadlessExecutionRequest(request)
}

if (import.meta.main) {
  void main().catch(error => {
    const message =
      error instanceof Error ? error.stack || error.message : String(error)
    process.stderr.write(`${message}\n`)
    process.exitCode = 1
  })
}
