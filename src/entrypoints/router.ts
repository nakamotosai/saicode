import {
  isStandaloneHelpFlag,
  isStandaloneVersionFlag,
  printFastCliHelp,
  printFastCliVersion,
} from './fastCliHelp.js'
import {
  shouldUseLightweightHeadlessPrintEntrypoint,
  shouldUseRecoveryEntrypoint,
} from '../utils/nonInteractiveMode.js'

export async function main(): Promise<void> {
  const args = process.argv.slice(2)
  const recoveryEntrypoint = '../localRecoveryCli.js'
  const headlessPrintEntrypoint = './headlessPrint.js'
  const fullCliEntrypoint = './cli.js'

  if (isStandaloneVersionFlag(args)) {
    printFastCliVersion()
    return
  }

  if (isStandaloneHelpFlag(args)) {
    printFastCliHelp()
    return
  }

  if (shouldUseRecoveryEntrypoint(args)) {
    await import(recoveryEntrypoint)
    return
  }

  if (shouldUseLightweightHeadlessPrintEntrypoint(args)) {
    const { main: runHeadlessPrint } = await import(headlessPrintEntrypoint)
    await runHeadlessPrint()
    return
  }

  await import(fullCliEntrypoint)
}
