import { afterEach, describe, expect, test } from 'bun:test'
import { getEmptyToolPermissionContext } from '../src/Tool.js'
import { getTools } from '../src/tools.js'
import {
  shouldAutoEnableBarePrint,
  shouldPreEnableSimpleMode,
  shouldUseLightweightHeadlessPrintEntrypoint,
  shouldUseRecoveryEntrypoint,
} from '../src/utils/nonInteractiveMode.js'

const ORIGINAL_SIMPLE = process.env.CLAUDE_CODE_SIMPLE

afterEach(() => {
  if (ORIGINAL_SIMPLE === undefined) {
    delete process.env.CLAUDE_CODE_SIMPLE
  } else {
    process.env.CLAUDE_CODE_SIMPLE = ORIGINAL_SIMPLE
  }
})

describe('shouldUseRecoveryEntrypoint', () => {
  test('keeps the recovery path for simple text print requests', () => {
    expect(shouldUseRecoveryEntrypoint(['-p', 'hello'])).toBe(true)
    expect(
      shouldUseRecoveryEntrypoint([
        '-p',
        '--output-format',
        'json',
        'hello',
      ]),
    ).toBe(true)
  })

  test('forces the full CLI for stream-json and tool-aware probes', () => {
    expect(
      shouldUseRecoveryEntrypoint([
        '-p',
        '--output-format',
        'stream-json',
        'hello',
      ]),
    ).toBe(false)
    expect(
      shouldUseRecoveryEntrypoint([
        '-p',
        '--tools',
        'Read',
        'hello',
      ]),
    ).toBe(false)
  })
})

describe('shouldAutoEnableBarePrint', () => {
  test('enables lean mode for plain one-shot print tasks', () => {
    expect(shouldAutoEnableBarePrint({ print: true })).toBe(true)
  })

  test('keeps full mode when explicit context or session features are in play', () => {
    expect(
      shouldAutoEnableBarePrint({
        print: true,
        systemPrompt: 'custom',
      }),
    ).toBe(false)
    expect(
      shouldAutoEnableBarePrint({
        print: true,
        resume: 'session-id',
      }),
    ).toBe(false)
    expect(
      shouldAutoEnableBarePrint({
        print: true,
        inputFormat: 'stream-json',
      }),
    ).toBe(false)
    expect(
      shouldAutoEnableBarePrint({
        print: true,
        tools: ['WebSearch'],
      }),
    ).toBe(false)
    expect(
      shouldAutoEnableBarePrint({
        print: true,
        allowedTools: ['Read,Grep'],
      }),
    ).toBe(true)
  })
})

describe('shouldPreEnableSimpleMode', () => {
  test('pre-enables simple mode for local simple-tool print tasks', () => {
    expect(
      shouldPreEnableSimpleMode([
        '-p',
        '--allowedTools',
        'Read,Grep,Edit',
      ]),
    ).toBe(true)
    expect(
      shouldPreEnableSimpleMode([
        '-p',
        '--tools',
        'Bash',
        'Read',
      ]),
    ).toBe(true)
  })

  test('keeps simple mode off for non-simple or ambiguous print features', () => {
    expect(
      shouldPreEnableSimpleMode([
        '-p',
        '--tools',
        'WebSearch',
      ]),
    ).toBe(false)
    expect(
      shouldPreEnableSimpleMode([
        '-p',
        '--allowedTools',
        'Read',
        '--output-format',
        'stream-json',
      ]),
    ).toBe(false)
    expect(
      shouldPreEnableSimpleMode([
        '-p',
        '--allowedTools',
        'Agent',
      ]),
    ).toBe(false)
    expect(
      shouldPreEnableSimpleMode([
        '-p',
        '--allowedTools',
        'Read',
        '--mcp-config',
        'mcp.json',
      ]),
    ).toBe(false)
  })
})

describe('shouldUseLightweightHeadlessPrintEntrypoint', () => {
  test('routes simple local-tool print tasks to the lightweight headless entrypoint', () => {
    expect(
      shouldUseLightweightHeadlessPrintEntrypoint([
        '-p',
        'hello',
        '--allowedTools',
        'Read,Grep',
      ]),
    ).toBe(true)
    expect(
      shouldUseLightweightHeadlessPrintEntrypoint([
        '-p',
        'hello',
        '--tools',
        'WebSearch',
      ]),
    ).toBe(true)
    expect(
      shouldUseLightweightHeadlessPrintEntrypoint([
        '-p',
        'hello',
        '--allowedTools',
        'WebFetch',
      ]),
    ).toBe(true)
  })

  test('keeps session and streaming features on the full CLI path', () => {
    expect(
      shouldUseLightweightHeadlessPrintEntrypoint([
        '-p',
        '--allowedTools',
        'Read',
        '--resume',
        'session-id',
      ]),
    ).toBe(false)
    expect(
      shouldUseLightweightHeadlessPrintEntrypoint([
        '-p',
        '--tools',
        'Agent',
      ]),
    ).toBe(false)
    expect(
      shouldUseLightweightHeadlessPrintEntrypoint([
        '-p',
        '--allowedTools',
        'Read',
        '--output-format',
        'stream-json',
      ]),
    ).toBe(false)
  })
})

describe('simple mode tools', () => {
  test('keeps write support for lean one-shot tasks', () => {
    process.env.CLAUDE_CODE_SIMPLE = '1'
    const toolNames = getTools(getEmptyToolPermissionContext()).map(
      tool => tool.name,
    )
    expect(toolNames).toContain('Read')
    expect(toolNames).toContain('Edit')
    expect(toolNames).toContain('Write')
  })
})
