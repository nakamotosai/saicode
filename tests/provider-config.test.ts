import { afterEach, describe, expect, test } from 'bun:test'
import {
  getSaicodeBestModelId,
  getSaicodeDefaultModelId,
  getSaicodeSmallFastModelId,
  resolveSaicodeModelId,
} from '../src/utils/model/saicodeCatalog.js'
import {
  saicodeGetProviderConfigForTesting,
} from '../src/services/api/saicodeRuntime.js'

const ORIGINAL_ENV = { ...process.env }

afterEach(() => {
  for (const key of Object.keys(process.env)) {
    if (!(key in ORIGINAL_ENV)) {
      delete process.env[key]
    }
  }

  for (const [key, value] of Object.entries(ORIGINAL_ENV)) {
    if (value === undefined) {
      delete process.env[key]
    } else {
      process.env[key] = value
    }
  }
})

describe('saicode model aliases', () => {
  test('keeps the current default catalog on the gpt-5.4 baseline', () => {
    delete process.env.SAICODE_DEFAULT_MODEL
    delete process.env.SAICODE_DEFAULT_BEST_MODEL
    delete process.env.SAICODE_SMALL_FAST_MODEL

    expect(getSaicodeDefaultModelId()).toBe('cpa/gpt-5.4')
    expect(getSaicodeBestModelId()).toBe('cpa/gpt-5.4')
    expect(getSaicodeSmallFastModelId()).toBe('cpa/gpt-5.4-mini')
    expect(resolveSaicodeModelId(undefined)).toBe('cpa/gpt-5.4')
  })

  test('keeps legacy cliproxyapi and nvidia aliases resolving to cpa ids', () => {
    expect(
      resolveSaicodeModelId('cliproxyapi/qwen/qwen3.5-397b-a17b'),
    ).toBe('cpa/qwen/qwen3.5-397b-a17b')
    expect(
      resolveSaicodeModelId('nvidia/qwen/qwen3.5-397b-a17b'),
    ).toBe('cpa/qwen/qwen3.5-397b-a17b')
  })
})

describe('cpa provider config', () => {
  test('accepts CPA_* env aliases before cliproxyapi envs', () => {
    process.env.CPA_API_KEY = 'cpa-key'
    process.env.CPA_BASE_URL = 'http://127.0.0.1:9999/v1'
    process.env.CPA_API = 'openai-chat-completions'
    process.env.CLIPROXYAPI_API_KEY = 'cliproxy-key'

    const provider = saicodeGetProviderConfigForTesting(
      'cpa/qwen/qwen3.5-397b-a17b',
    )

    expect(provider.id).toBe('cpa')
    expect(provider.apiKey).toBe('cpa-key')
    expect(provider.baseUrl).toBe('http://127.0.0.1:9999/v1')
    expect(provider.api).toBe('openai-chat-completions')
  })
})
