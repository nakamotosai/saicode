import memoize from 'lodash-es/memoize.js'
import {
  ListResourcesResultSchema,
  ReadResourceResultSchema,
  type ReadResourceResult,
  type Resource,
} from '@modelcontextprotocol/sdk/types.js'
import type { Command } from '../types/command.js'
import type { ConnectedMCPServer } from '../services/mcp/types.js'
import { buildMcpToolName } from '../services/mcp/mcpStringUtils.js'
import { normalizeNameForMCP } from '../services/mcp/normalization.js'
import { errorMessage } from '../utils/errors.js'
import { parseFrontmatter } from '../utils/frontmatterParser.js'
import { logForDebugging } from '../utils/debug.js'
import { getMCPSkillBuilders } from './mcpSkillBuilders.js'

function isSkillResource(resource: Resource): boolean {
  return resource.uri.startsWith('skill://')
}

function getResourceLeaf(resource: Resource): string {
  const raw =
    resource.name ||
    resource.uri.replace(/^skill:\/\//, '').split(/[?#]/, 1)[0] ||
    'skill'
  const leaf = raw.split('/').filter(Boolean).at(-1) || raw
  return leaf.replace(/\.md$/i, '').replace(/^SKILL$/i, 'skill')
}

function getCommandName(resource: Resource, serverName: string): string {
  return buildMcpToolName(serverName, getResourceLeaf(resource))
}

async function readSkillMarkdown(
  client: ConnectedMCPServer,
  resource: Resource,
): Promise<string | null> {
  const result = (await client.client.request(
    {
      method: 'resources/read',
      params: { uri: resource.uri },
    },
    ReadResourceResultSchema,
  )) as ReadResourceResult

  const texts = result.contents
    .filter(
      (
        entry,
      ): entry is Extract<ReadResourceResult['contents'][number], { text: string }> =>
        'text' in entry && typeof entry.text === 'string',
    )
    .map(entry => entry.text.trim())
    .filter(Boolean)

  return texts.length > 0 ? texts.join('\n\n') : null
}

function buildMcpSkillCommand(
  serverName: string,
  resource: Resource,
  markdown: string,
): Command {
  const commandName = getCommandName(resource, serverName)
  const { frontmatter, content } = parseFrontmatter(markdown, resource.uri)
  const { createSkillCommand, parseSkillFrontmatterFields } =
    getMCPSkillBuilders()
  const parsed = parseSkillFrontmatterFields(frontmatter, content, commandName)
  const shortName = normalizeNameForMCP(getResourceLeaf(resource))
  const resourceDescription = resource.description?.trim()

  return createSkillCommand({
    ...parsed,
    skillName: commandName,
    displayName:
      parsed.displayName || `${serverName}:${shortName} (MCP skill)`,
    description: parsed.description || resourceDescription || commandName,
    hasUserSpecifiedDescription:
      parsed.hasUserSpecifiedDescription || !!resourceDescription,
    markdownContent: content,
    source: 'mcp',
    baseDir: undefined,
    loadedFrom: 'mcp',
    paths: undefined,
  })
}

export const fetchMcpSkillsForClient = memoize(
  async (client: ConnectedMCPServer): Promise<Command[]> => {
    if (!client.capabilities?.resources) {
      return []
    }

    try {
      const result = await client.client.request(
        { method: 'resources/list' },
        ListResourcesResultSchema,
      )
      const resources = (result.resources ?? []).filter(isSkillResource)
      if (resources.length === 0) {
        return []
      }

      const commands = await Promise.all(
        resources.map(async resource => {
          try {
            const markdown = await readSkillMarkdown(client, resource)
            if (!markdown) {
              return null
            }
            return buildMcpSkillCommand(client.name, resource, markdown)
          } catch (error) {
            logForDebugging(
              `[mcp-skills] failed to load ${resource.uri} from ${client.name}: ${errorMessage(error)}`,
            )
            return null
          }
        }),
      )

      return commands.filter((command): command is Command => command !== null)
    } catch (error) {
      logForDebugging(
        `[mcp-skills] failed to list skills for ${client.name}: ${errorMessage(error)}`,
      )
      return []
    }
  },
  client => client.name,
)
