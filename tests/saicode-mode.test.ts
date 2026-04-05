import { afterEach, describe, expect, test } from 'bun:test'
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import {
  hasSaicodeRuntimeConfig,
  isSaicodeModeEnabled,
} from '../src/utils/model/saicodeCatalog.js'
import { getAPIProvider } from '../src/utils/model/providers.js'

const ORIGINAL_ENV = { ...process.env }

afterEach(() => {
  for (const key of Object.keys(process.env)) {
    if (!(key in ORIGINAL_ENV)) {
      delete process.env[key]
    }
  }
  Object.assign(process.env, ORIGINAL_ENV)
})

describe('saicode mode detection', () => {
  test('detects saicode mode from config.json providers without env vars', () => {
    const configDir = mkdtempSync(join(tmpdir(), 'saicode-mode-'))
    mkdirSync(configDir, { recursive: true })
    writeFileSync(
      join(configDir, 'config.json'),
      JSON.stringify({
        providers: {
          cpa: {
            api: 'openai-responses',
            baseUrl: 'http://127.0.0.1:8317/v1',
            apiKey: 'test-key',
          },
        },
      }),
    )

    delete process.env.SAICODE_PROVIDER
    delete process.env.SAICODE_MODEL
    delete process.env.SAICODE_DEFAULT_MODEL
    process.env.SAICODE_CONFIG_DIR = configDir

    expect(hasSaicodeRuntimeConfig()).toBe(true)
    expect(isSaicodeModeEnabled()).toBe(true)
    expect(getAPIProvider()).toBe('foundry')

    rmSync(configDir, { recursive: true, force: true })
  })
})
