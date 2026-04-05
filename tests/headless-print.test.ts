import { describe, expect, test } from 'bun:test'
import { parseLightweightHeadlessPrintArgs } from '../src/entrypoints/headlessPrintArgs.js'

describe('parseLightweightHeadlessPrintArgs', () => {
  test('parses the supported lightweight headless flags', () => {
    const parsed = parseLightweightHeadlessPrintArgs([
      '-p',
      '--allowedTools',
      'Read,Grep',
      '--model',
      'cpa/gpt-5.4',
      '--system-prompt',
      'system',
      '--append-system-prompt',
      'append',
      '--permission-mode',
      'default',
      '--fallback-model',
      'cpa/gpt-5.4-mini',
      '--max-turns',
      '3',
      '--max-budget-usd',
      '1.5',
      '--task-budget',
      '8000',
      '--output-format',
      'json',
      '-n',
      'quick-run',
      'hello',
    ])

    expect(parsed.print).toBe(true)
    expect(parsed.allowedTools).toEqual(['Read,Grep'])
    expect(parsed.model).toBe('cpa/gpt-5.4')
    expect(parsed.systemPrompt).toBe('system')
    expect(parsed.appendSystemPrompt).toBe('append')
    expect(parsed.permissionMode).toBe('default')
    expect(parsed.fallbackModel).toBe('cpa/gpt-5.4-mini')
    expect(parsed.maxTurns).toBe(3)
    expect(parsed.maxBudgetUsd).toBe(1.5)
    expect(parsed.taskBudget).toBe(8000)
    expect(parsed.outputFormat).toBe('json')
    expect(parsed.name).toBe('quick-run')
    expect(parsed.prompt).toBe('hello')
  })

  test('rejects unsupported output formats for the lightweight path', () => {
    expect(() =>
      parseLightweightHeadlessPrintArgs([
        '-p',
        '--allowedTools',
        'Read',
        '--output-format',
        'stream-json',
      ]),
    ).toThrow('lightweight headless mode')
  })
})
