import { getIsNonInteractiveSession } from '../bootstrap/state.js'
import { applyExtraCACertsFromConfig } from '../utils/caCertsConfig.js'
import { enableConfigs } from '../utils/config.js'
import { ConfigParseError } from '../utils/errors.js'
import {
  gracefulShutdownSync,
  setupGracefulShutdown,
} from '../utils/gracefulShutdown.js'
import { applySafeConfigEnvironmentVariables } from '../utils/managedEnv.js'
import { configureGlobalMTLS } from '../utils/mtls.js'
import { configureGlobalAgents } from '../utils/proxy.js'
import { profileCheckpoint } from '../utils/startupProfiler.js'
import { setShellIfWindows } from '../utils/windowsPaths.js'

let initPromise: Promise<void> | null = null

async function runInit(): Promise<void> {
  try {
    profileCheckpoint('lightweight_init_function_start')

    enableConfigs()
    profileCheckpoint('lightweight_init_configs_enabled')

    applySafeConfigEnvironmentVariables()
    applyExtraCACertsFromConfig()
    profileCheckpoint('lightweight_init_safe_env_vars_applied')

    setupGracefulShutdown()
    profileCheckpoint('lightweight_init_after_graceful_shutdown')

    configureGlobalMTLS()
    configureGlobalAgents()
    profileCheckpoint('lightweight_init_network_configured')

    setShellIfWindows()
    profileCheckpoint('lightweight_init_function_end')
  } catch (error) {
    if (error instanceof ConfigParseError && getIsNonInteractiveSession()) {
      process.stderr.write(
        `Configuration error in ${error.filePath}: ${error.message}\n`,
      )
      gracefulShutdownSync(1)
      return
    }

    throw error
  }
}

export function initLightweightHeadless(): Promise<void> {
  if (!initPromise) {
    initPromise = runInit().catch(error => {
      initPromise = null
      throw error
    })
  }

  return initPromise
}
