import { getSaicodeCliVersion } from '../utils/cliVersion.js'

const HELP_FLAGS = new Set(['-h', '--help'])
const VERSION_FLAGS = new Set(['-v', '-V', '--version'])

const FAST_HELP_TEXT = [
  'Usage: saicode [options] [command] [prompt]',
  '',
  'saicode - starts an interactive session by default, use -p/--print for',
  'non-interactive output',
  '',
  'Arguments:',
  '  prompt                                            Your prompt',
  '',
  'Options:',
  '  --add-dir <directories...>                        Additional directories to allow tool access to',
  '  --agent <agent>                                   Agent for the current session. Overrides the \'agent\' setting.',
  '  --agents <json>                                   JSON object defining custom agents (e.g. \'{"reviewer": {"description": "Reviews code", "prompt": "You are a code reviewer"}}\')',
  '  --allow-dangerously-skip-permissions              Allow Full Access to appear as a selectable option without enabling it by default. Recommended only for sandboxes with no internet access.',
  '  --allowedTools, --allowed-tools <tools...>        Comma or space-separated list of tool names to allow',
  '  --append-system-prompt <prompt>                   Append a system prompt to the default system prompt',
  '  --bare                                            Minimal mode: skip hooks, LSP, plugin sync, attribution, auto-memory, background prefetches, keychain reads, and SAICODE.md auto-discovery.',
  '  --betas <betas...>                                Beta headers to include in API requests (API key users only)',
  '  -c, --continue                                    Continue the most recent conversation in the current directory',
  '  --dangerously-skip-permissions                    Enable Full Access mode',
  '  -d, --debug [filter]                              Enable debug mode with optional category filtering',
  '  --disallowedTools, --disallowed-tools <tools...>  Comma or space-separated list of tool names to deny',
  '  --effort <level>                                  Effort level for the current session',
  '  --fallback-model <model>                          Enable fallback model for --print',
  '  --file <specs...>                                 File resources to download at startup',
  '  --fork-session                                    When resuming, create a new session ID instead of reusing the original',
  '  --from-pr [value]                                 Resume a session linked to a PR by PR number/URL',
  '  -h, --help                                        Display help for command',
  '  --ide                                             Automatically connect to IDE on startup if exactly one valid IDE is available',
  '  --include-hook-events                             Include all hook lifecycle events in the output stream',
  '  --include-partial-messages                        Include partial chunks for print streaming',
  '  --input-format <format>                           Input format for --print',
  '  --json-schema <schema>                            JSON Schema for structured output validation',
  '  --max-budget-usd <amount>                         Maximum API budget for --print',
  '  --mcp-config <configs...>                         Load MCP servers from JSON files or strings',
  '  --mcp-debug                                       [DEPRECATED. Use --debug instead] Enable MCP debug mode',
  '  --model <model>                                   Model for the current session',
  '  -n, --name <name>                                 Set a display name for this session',
  '  --no-session-persistence                          Disable session persistence for --print',
  '  --output-format <format>                          Output format for --print',
  '  --permission-mode <mode>                          Permission mode for the session',
  '  --plugin-dir <path>                               Load plugins from a directory for this session only',
  '  -p, --print                                       Print response and exit',
  '  --replay-user-messages                            Re-emit stdin user messages on stdout for acknowledgment',
  '  -r, --resume [value]                              Resume a conversation by session ID or picker',
  '  --session-id <uuid>                               Use a specific session ID',
  '  --setting-sources <sources>                       Comma-separated list of setting sources to load',
  '  --settings <file-or-json>                         Load additional settings',
  '  --strict-mcp-config                               Only use MCP servers from --mcp-config',
  '  --system-prompt <prompt>                          System prompt override',
  '  --tmux                                            Create a tmux session for the worktree',
  '  --tools <tools...>                                Specify built-in tools to expose',
  '  --verbose                                         Override verbose mode setting from config',
  '  -v, --version                                     Output the version number',
  '  -w, --worktree [name]                             Create a new git worktree for this session',
  '',
  'Commands:',
  '  agents [options]                                  List configured agents',
  '  mcp                                               Configure and manage MCP servers',
  '  plugin|plugins                                    Manage saicode plugins',
]

export function isStandaloneHelpFlag(args: string[]): boolean {
  return args.length === 1 && HELP_FLAGS.has(args[0] ?? '')
}

export function isStandaloneVersionFlag(args: string[]): boolean {
  return args.length === 1 && VERSION_FLAGS.has(args[0] ?? '')
}

export function printFastCliHelp(): void {
  process.stdout.write(`${FAST_HELP_TEXT.join('\n')}\n`)
}

export function printFastCliVersion(): void {
  process.stdout.write(`${getSaicodeCliVersion()} (saicode)\n`)
}
