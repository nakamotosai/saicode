import { readFileSync } from 'fs'
import { createInterface } from 'readline'
import { getEmptyToolPermissionContext } from './Tool.js'
import { saicodeQueryModelWithoutStreaming } from './services/api/saicodeRuntime.js'
import type { Message } from './types/message.js'
import { createUserMessage, extractTextContent } from './utils/messages.js'
import { asSystemPrompt } from './utils/systemPromptType.js'

type OutputFormat = 'text' | 'json'

const VERSION = '1.0.0'
const DEFAULT_MODEL = 'nvidia/qwen/qwen3.5-122b-a10b'

function printHelp(): void {
  process.stdout.write(
    [
      'Usage: saicode [options] [prompt]',
      '',
      'Local recovery mode for saicode.',
      '',
      'Options:',
      '  -h, --help                    Show help',
      '  -v, --version                 Show version',
      '  (no args)                     Start local interactive mode',
      '  -p, --print                   Send a single prompt and print the result',
      '  --model <model>               Override model',
      '  --system-prompt <text>        Override system prompt',
      '  --system-prompt-file <file>   Read system prompt from file',
      '  --append-system-prompt <text> Append to the system prompt',
      '  --output-format <format>      text (default) or json',
      '',
      'Environment:',
      '  SAICODE_MODEL',
      '  SAICODE_PROVIDER',
      '  SAICODE_CONFIG_DIR',
      '  NVIDIA_API_KEY / NVIDIA_BASE_URL',
      '  CLIPROXYAPI_API_KEY / CLIPROXYAPI_BASE_URL',
      '  API_TIMEOUT_MS',
      '',
    ].join('\n'),
  )
}

function printVersion(): void {
  process.stdout.write(`${VERSION} (saicode recovery)\n`)
}

function parseArgs(argv: string[]) {
  let print = false
  let model = process.env.SAICODE_MODEL || process.env.SAICODE_DEFAULT_MODEL
  let systemPrompt: string | undefined
  let appendSystemPrompt: string | undefined
  let outputFormat: OutputFormat = 'text'
  const positional: string[] = []

  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i]
    if (!arg) continue

    if (arg === '-h' || arg === '--help') {
      return { command: 'help' as const }
    }
    if (arg === '-v' || arg === '--version' || arg === '-V') {
      return { command: 'version' as const }
    }
    if (arg === '-p' || arg === '--print') {
      print = true
      continue
    }
    if (arg === '--bare') {
      continue
    }
    if (arg === '--dangerously-skip-permissions') {
      continue
    }
    if (arg === '--model') {
      model = argv[++i]
      continue
    }
    if (arg === '--system-prompt') {
      systemPrompt = argv[++i]
      continue
    }
    if (arg === '--system-prompt-file') {
      const file = argv[++i]
      systemPrompt = readFileSync(file!, 'utf8')
      continue
    }
    if (arg === '--append-system-prompt') {
      appendSystemPrompt = argv[++i]
      continue
    }
    if (arg === '--output-format') {
      const value = argv[++i]
      if (value === 'json' || value === 'text') {
        outputFormat = value
      }
      continue
    }
    if (arg.startsWith('-')) {
      continue
    }
    positional.push(arg)
  }

  return {
    command: 'run' as const,
    print,
    model,
    systemPrompt,
    appendSystemPrompt,
    outputFormat,
    prompt: positional.join(' ').trim(),
  }
}

async function readPromptFromStdin(): Promise<string> {
  if (process.stdin.isTTY) return ''
  const chunks: Buffer[] = []
  for await (const chunk of process.stdin) {
    chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(String(chunk)))
  }
  return Buffer.concat(chunks).toString('utf8').trim()
}

function getResolvedModel(model: string | undefined): string {
  return (
    model ||
    process.env.SAICODE_MODEL ||
    process.env.SAICODE_DEFAULT_MODEL ||
    process.env.SAICODE_DEFAULT_SONNET_MODEL ||
    DEFAULT_MODEL
  )
}

function getSystemPrompt(
  systemPrompt: string | undefined,
  appendSystemPrompt: string | undefined,
): string | undefined {
  if (systemPrompt && appendSystemPrompt) {
    return `${systemPrompt}\n\n${appendSystemPrompt}`
  }
  return systemPrompt ?? appendSystemPrompt
}

async function querySaicode(args: {
  messages: Message[]
  model: string
  systemPrompt?: string
}): Promise<Message> {
  return saicodeQueryModelWithoutStreaming({
    messages: args.messages,
    systemPrompt: asSystemPrompt(
      args.systemPrompt ? [args.systemPrompt] : [],
    ),
    tools: [],
    signal: AbortSignal.timeout(
      parseInt(process.env.API_TIMEOUT_MS || String(600_000), 10),
    ),
    options: {
      model: args.model,
      getToolPermissionContext: async () => getEmptyToolPermissionContext(),
    },
  })
}

async function run(): Promise<void> {
  const parsed = parseArgs(process.argv.slice(2))

  if (parsed.command === 'help') {
    printHelp()
    return
  }
  if (parsed.command === 'version') {
    printVersion()
    return
  }

  if (!parsed.print) {
    await runInteractive(parsed)
    return
  }

  const prompt = parsed.prompt || (await readPromptFromStdin())
  if (!prompt) {
    process.stderr.write('Error: prompt is required\n')
    process.exitCode = 1
    return
  }

  const model = getResolvedModel(parsed.model)
  const response = await querySaicode({
    messages: [createUserMessage({ content: prompt })],
    model,
    systemPrompt: getSystemPrompt(parsed.systemPrompt, parsed.appendSystemPrompt),
  })

  if (parsed.outputFormat === 'json') {
    process.stdout.write(`${JSON.stringify(response, null, 2)}\n`)
    return
  }

  const text = extractTextContent(response.message.content, '\n')
  process.stdout.write(`${text}\n`)
}

async function runInteractive(parsed: {
  model?: string
  systemPrompt?: string
  appendSystemPrompt?: string
}): Promise<void> {
  const model = getResolvedModel(parsed.model)
  const system = getSystemPrompt(parsed.systemPrompt, parsed.appendSystemPrompt)
  const messages: Message[] = []
  const rl = createInterface({
    input: process.stdin,
    output: process.stdout,
    prompt: 'you> ',
  })

  process.stdout.write(
    `saicode local interactive mode\nmodel: ${model}\ncommands: /exit, /clear\n\n`,
  )
  rl.prompt()

  for await (const line of rl) {
    const input = line.trim()
    if (!input) {
      rl.prompt()
      continue
    }
    if (input === '/exit' || input === '/quit') {
      rl.close()
      break
    }
    if (input === '/clear') {
      messages.length = 0
      process.stdout.write('history cleared\n')
      rl.prompt()
      continue
    }

    messages.push(createUserMessage({ content: input }))
    try {
      const response = await querySaicode({
        messages,
        model,
        systemPrompt: system,
      })
      const text = extractTextContent(response.message.content, '\n')
      process.stdout.write(`saicode> ${text}\n\n`)
      messages.push(response)
    } catch (error) {
      const message =
        error instanceof Error ? error.message : String(error)
      process.stderr.write(`error: ${message}\n`)
    }
    rl.prompt()
  }
}

void run().catch(error => {
  const message = error instanceof Error ? error.stack || error.message : String(error)
  process.stderr.write(`${message}\n`)
  process.exitCode = 1
})
