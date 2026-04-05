import { afterEach, describe, expect, test } from 'bun:test'
import { mkdtempSync, rmSync, symlinkSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { spawnSync } from 'node:child_process'

const repoRoot = resolve(import.meta.dir, '..')
const wrapperPath = join(repoRoot, 'bin', 'saicode')
const bunBinDir = dirname(process.execPath)
const originalPath = process.env.PATH ?? ''

let tempDir: string | null = null

afterEach(() => {
  if (tempDir) {
    rmSync(tempDir, { recursive: true, force: true })
    tempDir = null
  }
})

describe('saicode wrapper', () => {
  test('resolves the repo root correctly when invoked through a symlink', () => {
    tempDir = mkdtempSync(join(tmpdir(), 'saicode-wrapper-'))
    const linkPath = join(tempDir, 'saicode')
    symlinkSync(wrapperPath, linkPath)

    const result = spawnSync(linkPath, ['--help'], {
      cwd: repoRoot,
      encoding: 'utf8',
      env: {
        ...process.env,
        PATH: `${bunBinDir}:${originalPath}`,
        SAICODE_DISABLE_NATIVE_LAUNCHER: '1',
      },
    })

    expect(result.status).toBe(0)
    expect(result.stderr).toBe('')
    expect(result.stdout).toContain('Usage: saicode')
  })

  test('keeps the full CLI fallback working outside the repo cwd', () => {
    tempDir = mkdtempSync(join(tmpdir(), 'saicode-wrapper-full-'))
    const linkPath = join(tempDir, 'saicode')
    symlinkSync(wrapperPath, linkPath)

    const result = spawnSync(linkPath, ['mcp', '--help'], {
      cwd: tempDir,
      encoding: 'utf8',
      env: {
        ...process.env,
        PATH: `${bunBinDir}:${originalPath}`,
        SAICODE_DISABLE_NATIVE_LAUNCHER: '1',
      },
    })

    expect(result.status).toBe(0)
    expect(result.stderr).not.toContain('ReferenceError')
    expect(result.stdout).toContain('Usage: saicode mcp')
  })
})
