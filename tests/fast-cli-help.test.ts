import { describe, expect, test } from 'bun:test'
import {
  isStandaloneHelpFlag,
  isStandaloneVersionFlag,
} from '../src/entrypoints/fastCliHelp.js'

describe('fast CLI help routing', () => {
  test('recognizes top-level help only for standalone help flags', () => {
    expect(isStandaloneHelpFlag(['--help'])).toBe(true)
    expect(isStandaloneHelpFlag(['-h'])).toBe(true)
    expect(isStandaloneHelpFlag(['mcp', '--help'])).toBe(false)
  })

  test('recognizes top-level version only for standalone version flags', () => {
    expect(isStandaloneVersionFlag(['--version'])).toBe(true)
    expect(isStandaloneVersionFlag(['-v'])).toBe(true)
    expect(isStandaloneVersionFlag(['-V'])).toBe(true)
    expect(isStandaloneVersionFlag(['agents', '--version'])).toBe(false)
  })
})
