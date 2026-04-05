import { env } from './utils/env.js'
import { envDynamic } from './utils/envDynamic.js'
import { isEnvTruthy } from './utils/envUtils.js'
import type { PermissionMode } from './utils/permissions/PermissionMode.js'
import { setCwd } from './utils/Shell.js'
import {
  setOriginalCwd,
  setProjectRoot,
} from './bootstrap/state.js'

export async function setupLightweightHeadless(
  cwd: string,
  permissionMode: PermissionMode,
  allowDangerouslySkipPermissions: boolean,
): Promise<void> {
  setCwd(cwd)
  setOriginalCwd(cwd)
  setProjectRoot(cwd)

  if (
    permissionMode !== 'bypassPermissions' &&
    !allowDangerouslySkipPermissions
  ) {
    return
  }

  if (
    process.platform !== 'win32' &&
    typeof process.getuid === 'function' &&
    process.getuid() === 0 &&
    process.env.IS_SANDBOX !== '1' &&
    !isEnvTruthy(process.env.CLAUDE_CODE_BUBBLEWRAP)
  ) {
    console.error(
      '--dangerously-skip-permissions cannot be used with root/sudo privileges for security reasons',
    )
    process.exit(1)
  }

  if (
    process.env.USER_TYPE === 'ant' &&
    process.env.CLAUDE_CODE_ENTRYPOINT !== 'local-agent' &&
    process.env.CLAUDE_CODE_ENTRYPOINT !== 'claude-desktop'
  ) {
    const [isDocker, hasInternet] = await Promise.all([
      envDynamic.getIsDocker(),
      env.hasInternetAccess(),
    ])
    const isBubblewrap = envDynamic.getIsBubblewrapSandbox()
    const isSandbox = process.env.IS_SANDBOX === '1'
    const isSandboxed = isDocker || isBubblewrap || isSandbox
    if (!isSandboxed || hasInternet) {
      console.error(
        `--dangerously-skip-permissions can only be used in Docker/sandbox containers with no internet access but got Docker: ${isDocker}, Bubblewrap: ${isBubblewrap}, IS_SANDBOX: ${isSandbox}, hasInternet: ${hasInternet}`,
      )
      process.exit(1)
    }
  }
}
