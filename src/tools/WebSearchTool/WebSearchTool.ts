import axios from 'axios'
import type { PermissionResult } from 'src/utils/permissions/PermissionResult.js'
import { z } from 'zod/v4'
import { buildTool, type ToolDef } from '../../Tool.js'
import { execFileNoThrow } from '../../utils/execFileNoThrow.js'
import { lazySchema } from '../../utils/lazySchema.js'
import { jsonStringify } from '../../utils/slowOperations.js'
import { getWebSearchPrompt, WEB_SEARCH_TOOL_NAME } from './prompt.js'
import {
  getToolUseSummary,
  renderToolResultMessage,
  renderToolUseMessage,
  renderToolUseProgressMessage,
} from './UI.js'

const SEARCH_TIMEOUT_MS = 20_000
const SAI_SEARCH_TIMEOUT_MS = 45_000
const PAGE_FETCH_TIMEOUT_MS = 12_000
const MAX_SEARCH_RESULTS = 8
const MAX_FETCHED_PAGES = 2
const MAX_PAGE_TEXT_LENGTH = 3_000
const MAX_EXCERPT_LENGTH = 500
const DEFAULT_SEARCH_BASE_URL = 'https://html.duckduckgo.com/html/'
const SEARCH_TOOL_USE_ID = 'web_search_1'
const SEARCH_USER_AGENT =
  'Mozilla/5.0 (compatible; saicode-web-search/1.0; +https://saicode.local)'
const DEFAULT_SAI_SEARCH_SSH_TARGET = 'ubuntu@vps-jp.tail4b5213.ts.net'
const DEFAULT_SAI_SEARCH_SSH_REMOTE_URL = 'http://127.0.0.1:18961/search'

const inputSchema = lazySchema(() =>
  z.strictObject({
    query: z.string().min(2).describe('The search query to use'),
    allowed_domains: z
      .array(z.string())
      .optional()
      .describe('Only include search results from these domains'),
    blocked_domains: z
      .array(z.string())
      .optional()
      .describe('Never include search results from these domains'),
  }),
)
type InputSchema = ReturnType<typeof inputSchema>

type Input = z.infer<InputSchema>

const searchHitSchema = z.object({
  title: z.string().describe('The title of the search result'),
  url: z.string().describe('The URL of the search result'),
})

const searchResultSchema = lazySchema(() =>
  z.object({
    tool_use_id: z.string().describe('ID of the tool use'),
    content: z.array(searchHitSchema).describe('Array of search hits'),
  }),
)

export type SearchResult = z.infer<ReturnType<typeof searchResultSchema>>

const fetchedPageSchema = z.object({
  title: z.string().describe('The title of the fetched page'),
  url: z.string().describe('The fetched page URL'),
  excerpt: z.string().describe('A short excerpt extracted from the page'),
})

export type FetchedPage = z.infer<typeof fetchedPageSchema>

const outputSchema = lazySchema(() =>
  z.object({
    query: z.string().describe('The search query that was executed'),
    results: z
      .array(z.union([searchResultSchema(), z.string()]))
      .describe('Search results and/or text commentary from the search backend'),
    fetchedPages: z
      .array(fetchedPageSchema)
      .describe('Top fetched pages automatically read after the search'),
    durationSeconds: z
      .number()
      .describe('Time taken to complete the search operation'),
  }),
)
type OutputSchema = ReturnType<typeof outputSchema>

export type Output = z.infer<OutputSchema>

export type { WebSearchProgress } from '../../types/tools.js'
import type { WebSearchProgress } from '../../types/tools.js'

type SearchHit = {
  title: string
  url: string
}

type SaiSearchResult = {
  title?: string
  url?: string
  site?: string
  snippet?: string
}

function buildSearchUrl(query: string): URL {
  const baseUrl =
    process.env.SAICODE_WEB_SEARCH_BASE_URL ||
    process.env.CLAW_WEB_SEARCH_BASE_URL ||
    DEFAULT_SEARCH_BASE_URL
  const url = new URL(baseUrl)
  url.searchParams.set('q', query)
  return url
}

function extractQuotedValue(input: string): { value: string; rest: string } | null {
  const quote = input[0]
  if (quote !== '"' && quote !== "'") {
    return null
  }
  const end = input.indexOf(quote, 1)
  if (end === -1) {
    return null
  }
  return {
    value: input.slice(1, end),
    rest: input.slice(end + 1),
  }
}

function decodeHtmlEntities(text: string): string {
  return text.replace(/&(#x?[0-9a-f]+|[a-z]+);/gi, (entity, body: string) => {
    const normalized = body.toLowerCase()
    switch (normalized) {
      case 'amp':
        return '&'
      case 'lt':
        return '<'
      case 'gt':
        return '>'
      case 'quot':
        return '"'
      case 'apos':
      case '#39':
        return "'"
      case 'nbsp':
        return ' '
      default:
        break
    }

    if (normalized.startsWith('#x')) {
      const codePoint = Number.parseInt(normalized.slice(2), 16)
      return Number.isNaN(codePoint) ? entity : String.fromCodePoint(codePoint)
    }

    if (normalized.startsWith('#')) {
      const codePoint = Number.parseInt(normalized.slice(1), 10)
      return Number.isNaN(codePoint) ? entity : String.fromCodePoint(codePoint)
    }

    return entity
  })
}

function htmlToText(html: string): string {
  return decodeHtmlEntities(
    html
      .replace(/<script[\s\S]*?<\/script>/gi, ' ')
      .replace(/<style[\s\S]*?<\/style>/gi, ' ')
      .replace(/<[^>]+>/g, ' ')
      .replace(/\s+/g, ' ')
      .trim(),
  )
}

function extractPrimaryHtmlRegion(html: string): string {
  const regions = [
    /<main\b[^>]*>([\s\S]*?)<\/main>/i,
    /<article\b[^>]*>([\s\S]*?)<\/article>/i,
    /<body\b[^>]*>([\s\S]*?)<\/body>/i,
  ]

  for (const pattern of regions) {
    const match = html.match(pattern)
    if (match?.[1]) {
      return match[1]
    }
  }

  return html
}

function decodeDuckDuckGoRedirect(url: string): string | null {
  const normalizedUrl =
    url.startsWith('//')
      ? `https:${url}`
      : url.startsWith('/')
        ? `https://duckduckgo.com${url}`
        : url

  let parsed: URL
  try {
    parsed = new URL(normalizedUrl)
  } catch {
    return null
  }

  if (!['http:', 'https:'].includes(parsed.protocol)) {
    return null
  }

  if (
    parsed.hostname.endsWith('duckduckgo.com') &&
    (parsed.pathname === '/l/' || parsed.pathname === '/l')
  ) {
    const redirected = parsed.searchParams.get('uddg')
    if (!redirected) {
      return null
    }
    return decodeHtmlEntities(redirected)
  }

  return decodeHtmlEntities(parsed.toString())
}

function shouldKeepGenericHit(url: string, title: string): boolean {
  if (!title.trim()) {
    return false
  }

  if (!url.startsWith('http://') && !url.startsWith('https://')) {
    return false
  }

  try {
    const parsed = new URL(url)
    const blockedHosts = new Set([
      'duckduckgo.com',
      'html.duckduckgo.com',
      'duckduckgo.onion',
    ])
    if (blockedHosts.has(parsed.hostname)) {
      return false
    }
    return true
  } catch {
    return false
  }
}

function extractSearchHits(html: string): SearchHit[] {
  const hits: SearchHit[] = []
  let remaining = html

  while (true) {
    const anchorStart = remaining.indexOf('result__a')
    if (anchorStart === -1) {
      break
    }

    const afterClass = remaining.slice(anchorStart)
    const hrefIndex = afterClass.indexOf('href=')
    if (hrefIndex === -1) {
      remaining = afterClass.slice(1)
      continue
    }

    const hrefSlice = afterClass.slice(hrefIndex + 5)
    const href = extractQuotedValue(hrefSlice)
    if (!href) {
      remaining = afterClass.slice(1)
      continue
    }

    const closeTagIndex = href.rest.indexOf('>')
    if (closeTagIndex === -1) {
      remaining = afterClass.slice(1)
      continue
    }

    const afterTag = href.rest.slice(closeTagIndex + 1)
    const endAnchorIndex = afterTag.indexOf('</a>')
    if (endAnchorIndex === -1) {
      remaining = afterTag.slice(1)
      continue
    }

    const decodedUrl = decodeDuckDuckGoRedirect(href.value)
    if (decodedUrl) {
      hits.push({
        title: htmlToText(afterTag.slice(0, endAnchorIndex)),
        url: decodedUrl,
      })
    }

    remaining = afterTag.slice(endAnchorIndex + 4)
  }

  return hits
}

function extractSearchHitsFromGenericLinks(html: string): SearchHit[] {
  const hits: SearchHit[] = []
  let remaining = html

  while (true) {
    const anchorStart = remaining.indexOf('<a')
    if (anchorStart === -1) {
      break
    }

    const afterAnchor = remaining.slice(anchorStart)
    const hrefIndex = afterAnchor.indexOf('href=')
    if (hrefIndex === -1) {
      remaining = afterAnchor.slice(2)
      continue
    }

    const hrefSlice = afterAnchor.slice(hrefIndex + 5)
    const href = extractQuotedValue(hrefSlice)
    if (!href) {
      remaining = afterAnchor.slice(2)
      continue
    }

    const closeTagIndex = href.rest.indexOf('>')
    if (closeTagIndex === -1) {
      remaining = afterAnchor.slice(2)
      continue
    }

    const afterTag = href.rest.slice(closeTagIndex + 1)
    const endAnchorIndex = afterTag.indexOf('</a>')
    if (endAnchorIndex === -1) {
      remaining = afterAnchor.slice(2)
      continue
    }

    const title = htmlToText(afterTag.slice(0, endAnchorIndex))
    const decodedUrl = decodeDuckDuckGoRedirect(href.value)
    if (decodedUrl && shouldKeepGenericHit(decodedUrl, title)) {
      hits.push({ title, url: decodedUrl })
    }

    remaining = afterTag.slice(endAnchorIndex + 4)
  }

  return hits
}

function hostMatchesList(url: string, domains: string[]): boolean {
  try {
    const hostname = new URL(url).hostname.toLowerCase()
    return domains.some(domain => {
      const normalized = domain.trim().toLowerCase()
      return hostname === normalized || hostname.endsWith(`.${normalized}`)
    })
  } catch {
    return false
  }
}

function dedupeHits(hits: SearchHit[]): SearchHit[] {
  const seen = new Set<string>()
  return hits.filter(hit => {
    const key = hit.url.trim().toLowerCase()
    if (!key || seen.has(key)) {
      return false
    }
    seen.add(key)
    return true
  })
}

function buildSummary(query: string, hits: SearchHit[]): string {
  if (hits.length === 0) {
    return `No web search results matched the query "${query}".`
  }

  const renderedHits = hits
    .map(hit => `- [${hit.title}](${hit.url})`)
    .join('\n')

  return `Search results for "${query}". Include a Sources section in the final answer.\n${renderedHits}`
}

function stripMarkdownArtifacts(text: string): string {
  return text
    .replace(/skip to main content/gi, ' ')
    .replace(/table of contents/gi, ' ')
    .replace(/```[\s\S]*?```/g, ' ')
    .replace(/`([^`]+)`/g, '$1')
    .replace(/!\[[^\]]*\]\([^)]+\)/g, ' ')
    .replace(/\[([^\]]+)\]\([^)]+\)/g, '$1')
    .replace(/^#{1,6}\s+/gm, '')
    .replace(/^\s*[-*+]\s+/gm, '')
    .replace(/\s+/g, ' ')
    .trim()
}

function getSearchFetchTopK(): number {
  const configured = Number.parseInt(
    process.env.SAICODE_WEB_SEARCH_FETCH_TOP_K || '',
    10,
  )
  if (Number.isNaN(configured)) {
    return MAX_FETCHED_PAGES
  }
  return Math.max(0, Math.min(configured, MAX_FETCHED_PAGES))
}

function extractQueryTerms(query: string): string[] {
  const rawTerms = query
    .toLowerCase()
    .split(/[\s,.;:!?()[\]{}"'`|/\\<>]+/)
    .map(term => term.trim())
    .filter(term => term.length >= 3)

  const stopWords = new Set([
    'the',
    'and',
    'for',
    'with',
    'from',
    'that',
    'this',
    'what',
    'when',
    'where',
    'which',
    'into',
    'about',
    'latest',
    'official',
  ])

  return [...new Set(rawTerms.filter(term => !stopWords.has(term)))]
}

function scoreExcerptSegment(segment: string, query: string, terms: string[]): number {
  const normalized = segment.toLowerCase()
  let score = 0
  if (query && normalized.includes(query.toLowerCase())) {
    score += 8
  }
  for (const term of terms) {
    if (normalized.includes(term)) {
      score += 2
    }
  }
  return score
}

function buildExcerptFromText(query: string, text: string): string {
  const normalized = stripMarkdownArtifacts(text).slice(0, MAX_PAGE_TEXT_LENGTH)
  if (!normalized) {
    return ''
  }

  const terms = extractQueryTerms(query)
  const segments = normalized
    .split(/(?<=[.!?。！？])\s+|\n+/)
    .map(segment => segment.trim())
    .filter(segment => segment.length >= 40)

  let bestSegment = ''
  let bestScore = -1
  for (const segment of segments) {
    const score = scoreExcerptSegment(segment, query, terms)
    if (score > bestScore) {
      bestScore = score
      bestSegment = segment
    }
  }

  const source = bestSegment || normalized
  const excerpt = source.slice(0, MAX_EXCERPT_LENGTH).trim()
  if (excerpt.length < source.length) {
    return `${excerpt}...`
  }
  return excerpt
}

async function fetchPageExcerpt(
  query: string,
  hit: SearchHit,
  signal: AbortSignal,
): Promise<FetchedPage | null> {
  try {
    const response = await axios.get<string>(hit.url, {
      headers: {
        Accept: 'text/html, text/plain, text/markdown;q=0.9, */*;q=0.8',
        'User-Agent': SEARCH_USER_AGENT,
      },
      maxRedirects: 5,
      responseType: 'text',
      signal,
      timeout: PAGE_FETCH_TIMEOUT_MS,
      transformResponse: value => value,
    })

    const rawBody = typeof response.data === 'string' ? response.data : ''
    const contentType = String(response.headers['content-type'] || '')
    const text = contentType.includes('text/html')
      ? htmlToText(extractPrimaryHtmlRegion(rawBody))
      : stripMarkdownArtifacts(rawBody)
    const excerpt = buildExcerptFromText(query, text)
    if (!excerpt) {
      return null
    }

    return {
      title: hit.title,
      url: hit.url,
      excerpt,
    }
  } catch {
    return null
  }
}

async function fetchTopPageExcerpts(
  query: string,
  hits: SearchHit[],
  signal: AbortSignal,
): Promise<FetchedPage[]> {
  const topK = getSearchFetchTopK()
  if (topK === 0 || hits.length === 0) {
    return []
  }

  const settled = await Promise.allSettled(
    hits.slice(0, topK).map(hit => fetchPageExcerpt(query, hit, signal)),
  )

  return settled
    .flatMap(result => (result.status === 'fulfilled' ? [result.value] : []))
    .filter((page): page is FetchedPage => page != null)
}

function buildFetchedPagesSummary(
  query: string,
  fetchedPages: FetchedPage[],
): string | null {
  if (fetchedPages.length === 0) {
    return null
  }

  const renderedPages = fetchedPages
    .map(
      (page, index) =>
        `${index + 1}. [${page.title}](${page.url})\n   ${page.excerpt}`,
    )
    .join('\n')

  return `Top page excerpts automatically fetched for "${query}":\n${renderedPages}`
}

function normalizeHits(input: Input, hits: SearchHit[]): SearchHit[] {
  let normalized = dedupeHits(hits).slice(0, MAX_SEARCH_RESULTS)
  if (input.allowed_domains?.length) {
    normalized = normalized.filter(hit =>
      hostMatchesList(hit.url, input.allowed_domains!),
    )
  }
  if (input.blocked_domains?.length) {
    normalized = normalized.filter(
      hit => !hostMatchesList(hit.url, input.blocked_domains!),
    )
  }
  return normalized.slice(0, MAX_SEARCH_RESULTS)
}

function makeOutput(
  query: string,
  hits: SearchHit[],
  fetchedPages: FetchedPage[],
  startedAt: number,
): Output {
  const fetchedSummary = buildFetchedPagesSummary(query, fetchedPages)
  return {
    query,
    results: [
      buildSummary(query, hits),
      ...(fetchedSummary ? [fetchedSummary] : []),
      {
        tool_use_id: SEARCH_TOOL_USE_ID,
        content: hits,
      },
    ],
    fetchedPages,
    durationSeconds: (performance.now() - startedAt) / 1000,
  }
}

async function executeDirectWebSearch(
  input: Input,
  signal: AbortSignal,
): Promise<Output> {
  const startedAt = performance.now()
  const searchUrl = buildSearchUrl(input.query)
  const response = await axios.get<string>(searchUrl.toString(), {
    headers: {
      'User-Agent': SEARCH_USER_AGENT,
    },
    maxRedirects: 10,
    responseType: 'text',
    signal,
    timeout: SEARCH_TIMEOUT_MS,
    transformResponse: value => value,
  })

  const html = typeof response.data === 'string' ? response.data : ''
  let hits = extractSearchHits(html)

  if (hits.length === 0) {
    hits = extractSearchHitsFromGenericLinks(html)
  }

  const normalizedHits = normalizeHits(input, hits)
  const fetchedPages = await fetchTopPageExcerpts(
    input.query,
    normalizedHits,
    signal,
  )
  return makeOutput(input.query, normalizedHits, fetchedPages, startedAt)
}

function getSaiSearchBaseUrl(): string | null {
  return process.env.SAICODE_SAI_SEARCH_BASE_URL?.trim() || null
}

function getSaiSearchSshTarget(): string | null {
  const configured = process.env.SAICODE_SAI_SEARCH_SSH_TARGET?.trim()
  if (configured) {
    return configured
  }
  if (process.env.SAICODE_DISABLE_SAI_SEARCH_SSH === '1') {
    return null
  }
  return DEFAULT_SAI_SEARCH_SSH_TARGET
}

function mapSaiSearchResults(input: Input, rawResults: SaiSearchResult[]): SearchHit[] {
  const hits = rawResults
    .map(result => ({
      title: (result.title || '').trim(),
      url: (result.url || '').trim(),
    }))
    .filter(hit => hit.title && hit.url)

  return normalizeHits(input, hits)
}

async function executeSaiSearchHttp(input: Input, signal: AbortSignal): Promise<Output> {
  const startedAt = performance.now()
  const baseUrl = getSaiSearchBaseUrl()
  if (!baseUrl) {
    throw new Error('SAICODE_SAI_SEARCH_BASE_URL is not configured')
  }

  const response = await axios.post(
    new URL('/search', baseUrl).toString(),
    {
      query: input.query,
      limit: MAX_SEARCH_RESULTS,
      include_content: false,
      progress: false,
      mode: 'single',
    },
    {
      headers: {
        'Content-Type': 'application/json',
        'User-Agent': SEARCH_USER_AGENT,
      },
      signal,
      timeout: SAI_SEARCH_TIMEOUT_MS,
    },
  )

  const hits = mapSaiSearchResults(
    input,
    Array.isArray(response.data?.results) ? response.data.results : [],
  )
  const fetchedPages = await fetchTopPageExcerpts(input.query, hits, signal)
  return makeOutput(input.query, hits, fetchedPages, startedAt)
}

async function executeSaiSearchOverSsh(input: Input, signal: AbortSignal): Promise<Output> {
  const startedAt = performance.now()
  const target = getSaiSearchSshTarget()
  if (!target) {
    throw new Error('SAICODE_SAI_SEARCH_SSH_TARGET is not configured')
  }

  const remoteUrl =
    process.env.SAICODE_SAI_SEARCH_SSH_REMOTE_URL?.trim() ||
    DEFAULT_SAI_SEARCH_SSH_REMOTE_URL

  const payload = JSON.stringify({
    query: input.query,
    limit: MAX_SEARCH_RESULTS,
    include_content: false,
    progress: false,
    mode: 'single',
  })

  const { stdout, stderr, code, error } = await execFileNoThrow(
    'ssh',
    [
      target,
      `curl -sS -X POST ${remoteUrl} -H "content-type: application/json" --data-binary @-`,
    ],
    {
      abortSignal: signal,
      input: payload,
      stdin: 'pipe',
      timeout: SAI_SEARCH_TIMEOUT_MS,
      useCwd: false,
    },
  )

  if (code !== 0) {
    throw new Error(stderr || error || 'sai-search SSH fallback failed')
  }

  const parsed = JSON.parse(stdout) as { results?: SaiSearchResult[] }
  const hits = mapSaiSearchResults(
    input,
    Array.isArray(parsed.results) ? parsed.results : [],
  )
  const fetchedPages = await fetchTopPageExcerpts(input.query, hits, signal)
  return makeOutput(input.query, hits, fetchedPages, startedAt)
}

async function executeSaiSearchFallback(
  input: Input,
  signal: AbortSignal,
): Promise<Output> {
  if (getSaiSearchBaseUrl()) {
    return executeSaiSearchHttp(input, signal)
  }
  return executeSaiSearchOverSsh(input, signal)
}

async function executeWebSearch(input: Input, signal: AbortSignal): Promise<Output> {
  try {
    const direct = await executeDirectWebSearch(input, signal)
    const directHits = direct.results.find(
      result => typeof result !== 'string',
    ) as SearchResult | undefined
    if ((directHits?.content.length ?? 0) > 0) {
      return direct
    }
  } catch {
    // Fall through to sai-search fallback below.
  }

  return executeSaiSearchFallback(input, signal)
}

export const WebSearchTool = buildTool({
  name: WEB_SEARCH_TOOL_NAME,
  searchHint: 'search the web for current information',
  maxResultSizeChars: 100_000,
  shouldDefer: true,
  async description(input) {
    return `saicode wants to search the web for: ${input.query}`
  },
  userFacingName() {
    return 'Web Search'
  },
  getToolUseSummary,
  getActivityDescription(input) {
    const summary = getToolUseSummary(input)
    return summary ? `Searching for ${summary}` : 'Searching the web'
  },
  isEnabled() {
    return true
  },
  get inputSchema(): InputSchema {
    return inputSchema()
  },
  get outputSchema(): OutputSchema {
    return outputSchema()
  },
  isConcurrencySafe() {
    return true
  },
  isReadOnly() {
    return true
  },
  toAutoClassifierInput(input) {
    return input.query
  },
  async checkPermissions(): Promise<PermissionResult> {
    return {
      behavior: 'passthrough',
      message: 'WebSearchTool requires permission.',
      suggestions: [
        {
          type: 'addRules',
          rules: [{ toolName: WEB_SEARCH_TOOL_NAME }],
          behavior: 'allow',
          destination: 'localSettings',
        },
      ],
    }
  },
  async prompt() {
    return getWebSearchPrompt()
  },
  renderToolUseMessage,
  renderToolUseProgressMessage,
  renderToolResultMessage,
  extractSearchText() {
    return ''
  },
  async validateInput(input) {
    const { query, allowed_domains, blocked_domains } = input
    if (!query.length) {
      return {
        result: false,
        message: 'Error: Missing query',
        errorCode: 1,
      }
    }
    if (allowed_domains?.length && blocked_domains?.length) {
      return {
        result: false,
        message:
          'Error: Cannot specify both allowed_domains and blocked_domains in the same request',
        errorCode: 2,
      }
    }
    return { result: true }
  },
  async call(input, context, _canUseTool, _parentMessage, onProgress) {
    onProgress?.({
      toolUseID: `search-progress-${Date.now()}`,
      data: {
        type: 'query_update',
        query: input.query,
      },
    })

    const data = await executeWebSearch(input, context.abortController.signal)
    const searchResult = data.results.find(
      result => typeof result !== 'string',
    ) as SearchResult | undefined

    onProgress?.({
      toolUseID: searchResult?.tool_use_id ?? SEARCH_TOOL_USE_ID,
      data: {
        type: 'search_results_received',
        query: input.query,
        resultCount: searchResult?.content.length ?? 0,
      },
    })

    return { data }
  },
  mapToolResultToToolResultBlockParam(output, toolUseID) {
    const { query, results } = output

    let formattedOutput = `Web search results for query: "${query}"\n\n`

    ;(results ?? []).forEach(result => {
      if (result == null) {
        return
      }
      if (typeof result === 'string') {
        formattedOutput += result + '\n\n'
      } else if (result.content?.length > 0) {
        formattedOutput += `Links: ${jsonStringify(result.content)}\n\n`
      } else {
        formattedOutput += 'No links found.\n\n'
      }
    })

    formattedOutput +=
      '\nREMINDER: You MUST include the sources above in your response to the user using markdown hyperlinks.'

    return {
      tool_use_id: toolUseID,
      type: 'tool_result',
      content: formattedOutput.trim(),
    }
  },
} satisfies ToolDef<InputSchema, Output, WebSearchProgress>)
