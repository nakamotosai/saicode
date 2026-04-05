import { describe, expect, test } from 'bun:test'
import { getEmptyToolPermissionContext, type ToolUseContext } from '../src/Tool.js'
import { FileReadTool } from '../src/tools/FileReadTool/FileReadTool.js'

function createToolUseContext(): ToolUseContext {
  return {
    options: {} as ToolUseContext['options'],
    abortController: new AbortController(),
    readFileState: {} as ToolUseContext['readFileState'],
    getAppState: () =>
      ({
        toolPermissionContext: getEmptyToolPermissionContext(),
      }) as ReturnType<ToolUseContext['getAppState']>,
    setAppState: () => {},
  } as unknown as ToolUseContext
}

describe('FileReadTool.validateInput', () => {
  test('treats blank pages as omitted', async () => {
    const result = await FileReadTool.validateInput(
      {
        file_path: '/tmp/saicode-probes/read-target.txt',
        pages: '',
      },
      createToolUseContext(),
    )

    expect(result).toEqual({ result: true })
  })

  test('still rejects invalid non-empty page ranges', async () => {
    const result = await FileReadTool.validateInput(
      {
        file_path: '/tmp/saicode-probes/read-target.txt',
        pages: '0',
      },
      createToolUseContext(),
    )

    expect(result).toEqual({
      result: false,
      message:
        'Invalid pages parameter: "0". Use formats like "1-5", "3", or "10-20". Pages are 1-indexed.',
      errorCode: 7,
    })
  })
})
