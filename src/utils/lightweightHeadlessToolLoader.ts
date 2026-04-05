import type { Tool, ToolPermissionContext, Tools } from '../Tool.js'
import { hasEmbeddedSearchTools } from './embeddedTools.js'
import {
  LIGHTWEIGHT_HEADLESS_TOOL_NAMES,
  normalizeToolRestrictionValues,
} from './lightweightHeadlessTools.js'
import { getDenyRuleForTool } from './permissions/permissions.js'

type LightweightHeadlessToolName =
  | 'Bash'
  | 'Glob'
  | 'Grep'
  | 'Read'
  | 'Edit'
  | 'Write'
  | 'WebFetch'
  | 'WebSearch'

type ToolLoader = () => Promise<Tool | null>

const LIGHTWEIGHT_HEADLESS_TOOL_LOADERS: Record<
  LightweightHeadlessToolName,
  ToolLoader
> = {
  Bash: async () => (await import('../tools/BashTool/BashTool.js')).BashTool,
  Glob: async () =>
    hasEmbeddedSearchTools()
      ? null
      : (await import('../tools/GlobTool/GlobTool.js')).GlobTool,
  Grep: async () =>
    hasEmbeddedSearchTools()
      ? null
      : (await import('../tools/GrepTool/GrepTool.js')).GrepTool,
  Read: async () =>
    (await import('../tools/FileReadTool/FileReadTool.js')).FileReadTool,
  Edit: async () =>
    (await import('../tools/FileEditTool/FileEditTool.js')).FileEditTool,
  Write: async () =>
    (await import('../tools/FileWriteTool/FileWriteTool.js')).FileWriteTool,
  WebFetch: async () =>
    (await import('../tools/WebFetchTool/WebFetchTool.js')).WebFetchTool,
  WebSearch: async () =>
    (await import('../tools/WebSearchTool/WebSearchTool.js')).WebSearchTool,
}

export async function loadRequestedLightweightHeadlessTools({
  allowedTools,
  tools,
  permissionContext,
}: {
  allowedTools: string[]
  tools: string[]
  permissionContext: ToolPermissionContext
}): Promise<Tools> {
  const requestedNames = Array.from(
    new Set(normalizeToolRestrictionValues([...tools, ...allowedTools])),
  ).filter(
    (
      value,
    ): value is LightweightHeadlessToolName =>
      LIGHTWEIGHT_HEADLESS_TOOL_NAMES.has(value),
  )

  const resolvedTools: Tool[] = []
  for (const name of requestedNames) {
    const loader = LIGHTWEIGHT_HEADLESS_TOOL_LOADERS[name]
    const tool = await loader()
    if (!tool) {
      continue
    }
    if (getDenyRuleForTool(permissionContext, tool)) {
      continue
    }
    if (!tool.isEnabled()) {
      continue
    }
    resolvedTools.push(tool)
  }

  return resolvedTools
}
