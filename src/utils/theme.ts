import chalk, { Chalk } from 'chalk'
import { env } from './env.js'

export type Theme = {
  autoAccept: string
  bashBorder: string
  claude: string
  claudeShimmer: string // Lighter version of claude color for shimmer effect
  claudeBlue_FOR_SYSTEM_SPINNER: string
  claudeBlueShimmer_FOR_SYSTEM_SPINNER: string
  permission: string
  permissionShimmer: string // Lighter version of permission color for shimmer effect
  planMode: string
  ide: string
  promptBorder: string
  promptBorderShimmer: string // Lighter version of promptBorder color for shimmer effect
  text: string
  inverseText: string
  inactive: string
  inactiveShimmer: string // Lighter version of inactive color for shimmer effect
  subtle: string
  suggestion: string
  remember: string
  background: string
  // Semantic colors
  success: string
  error: string
  warning: string
  merged: string
  warningShimmer: string // Lighter version of warning color for shimmer effect
  // Diff colors
  diffAdded: string
  diffRemoved: string
  diffAddedDimmed: string
  diffRemovedDimmed: string
  // Word-level diff highlighting
  diffAddedWord: string
  diffRemovedWord: string
  // Agent colors
  red_FOR_SUBAGENTS_ONLY: string
  blue_FOR_SUBAGENTS_ONLY: string
  green_FOR_SUBAGENTS_ONLY: string
  yellow_FOR_SUBAGENTS_ONLY: string
  purple_FOR_SUBAGENTS_ONLY: string
  orange_FOR_SUBAGENTS_ONLY: string
  pink_FOR_SUBAGENTS_ONLY: string
  cyan_FOR_SUBAGENTS_ONLY: string
  // Grove colors
  professionalBlue: string
  // Chrome colors
  chromeYellow: string
  // TUI V2 colors
  clawd_body: string
  clawd_background: string
  userMessageBackground: string
  userMessageBackgroundHover: string
  /** Message-actions selection. Cool shift toward `suggestion` blue; distinct from default AND userMessageBackground. */
  messageActionsBackground: string
  /** Text-selection highlight background (alt-screen mouse selection). Solid
   *  bg that REPLACES the cell's bg while preserving its fg — matches native
   *  terminal selection. Previously SGR-7 inverse (swapped fg/bg per cell),
   *  which fragmented badly over syntax highlighting. */
  selectionBg: string
  bashMessageBackgroundColor: string

  memoryBackgroundColor: string
  rate_limit_fill: string
  rate_limit_empty: string
  fastMode: string
  fastModeShimmer: string
  // Brief/assistant mode label colors
  briefLabelYou: string
  briefLabelClaude: string
  // Rainbow colors for ultrathink keyword highlighting
  rainbow_red: string
  rainbow_orange: string
  rainbow_yellow: string
  rainbow_green: string
  rainbow_blue: string
  rainbow_indigo: string
  rainbow_violet: string
  rainbow_red_shimmer: string
  rainbow_orange_shimmer: string
  rainbow_yellow_shimmer: string
  rainbow_green_shimmer: string
  rainbow_blue_shimmer: string
  rainbow_indigo_shimmer: string
  rainbow_violet_shimmer: string
}

export const THEME_NAMES = [
  'dark',
  'light',
  'light-daltonized',
  'dark-daltonized',
  'light-ansi',
  'dark-ansi',
] as const

/** A renderable theme. Always resolvable to a concrete color palette. */
export type ThemeName = (typeof THEME_NAMES)[number]

export const THEME_SETTINGS = ['auto', ...THEME_NAMES] as const

/**
 * A theme preference as stored in user config. `'auto'` follows the system
 * dark/light mode and is resolved to a ThemeName at runtime.
 */
export type ThemeSetting = (typeof THEME_SETTINGS)[number]

/**
 * Light theme using explicit RGB values to avoid inconsistencies
 * from users' custom terminal ANSI color definitions
 */
const lightTheme: Theme = {
  autoAccept: 'rgb(176,156,188)', // Morandi purple
  bashBorder: 'rgb(199,143,136)', // Morandi red
  claude: 'rgb(143,168,191)', // Morandi blue
  claudeShimmer: 'rgb(178,195,212)', // lighter Morandi blue
  claudeBlue_FOR_SYSTEM_SPINNER: 'rgb(143,168,191)',
  claudeBlueShimmer_FOR_SYSTEM_SPINNER: 'rgb(178,195,212)',
  permission: 'rgb(199,143,136)', // Morandi red
  permissionShimmer: 'rgb(221,180,174)', // lighter Morandi red
  planMode: 'rgb(216,195,143)', // Morandi yellow
  ide: 'rgb(155,179,155)', // Morandi green
  promptBorder: 'rgb(154,163,199)', // Morandi indigo
  promptBorderShimmer: 'rgb(194,199,223)', // lighter Morandi indigo
  text: 'rgb(0,0,0)', // Black
  inverseText: 'rgb(255,255,255)', // White
  inactive: 'rgb(102,102,102)', // Dark gray
  inactiveShimmer: 'rgb(142,142,142)', // Lighter gray for shimmer effect
  subtle: 'rgb(175,175,175)', // Light gray
  suggestion: 'rgb(143,168,191)', // Morandi blue
  remember: 'rgb(176,156,188)', // Morandi purple
  background: 'rgb(216,195,143)', // Morandi yellow
  success: 'rgb(155,179,155)', // Morandi green
  error: 'rgb(199,143,136)', // Morandi red
  warning: 'rgb(216,195,143)', // Morandi yellow
  merged: 'rgb(176,156,188)', // Morandi purple
  warningShimmer: 'rgb(231,216,178)', // lighter Morandi yellow
  diffAdded: 'rgb(105,219,124)', // Light green
  diffRemoved: 'rgb(255,168,180)', // Light red
  diffAddedDimmed: 'rgb(199,225,203)', // Very light green
  diffRemovedDimmed: 'rgb(253,210,216)', // Very light red
  diffAddedWord: 'rgb(47,157,68)', // Medium green
  diffRemovedWord: 'rgb(209,69,75)', // Medium red
  // Agent colors
  red_FOR_SUBAGENTS_ONLY: 'rgb(220,38,38)', // Red 600
  blue_FOR_SUBAGENTS_ONLY: 'rgb(37,99,235)', // Blue 600
  green_FOR_SUBAGENTS_ONLY: 'rgb(22,163,74)', // Green 600
  yellow_FOR_SUBAGENTS_ONLY: 'rgb(202,138,4)', // Yellow 600
  purple_FOR_SUBAGENTS_ONLY: 'rgb(147,51,234)', // Purple 600
  orange_FOR_SUBAGENTS_ONLY: 'rgb(251,188,5)', // Google yellow
  pink_FOR_SUBAGENTS_ONLY: 'rgb(219,39,119)', // Pink 600
  cyan_FOR_SUBAGENTS_ONLY: 'rgb(8,145,178)', // Cyan 600
  // Grove colors
  professionalBlue: 'rgb(216,195,143)',
  // Chrome colors
  chromeYellow: 'rgb(251,188,4)', // Chrome yellow
  // TUI V2 colors
  clawd_body: 'rgb(143,168,191)',
  clawd_background: 'rgb(0,0,0)',
  userMessageBackground: 'rgb(255, 248, 214)', // warm yellow card
  userMessageBackgroundHover: 'rgb(255, 252, 232)', // brighter yellow hover
  messageActionsBackground: 'rgb(240, 232, 255)', // pale purple action strip
  selectionBg: 'rgb(180, 213, 255)', // classic light-mode selection blue (macOS/VS Code-ish); dark fgs stay readable
  bashMessageBackgroundColor: 'rgb(229, 244, 234)',

  memoryBackgroundColor: 'rgb(232, 244, 253)',
  rate_limit_fill: 'rgb(155,179,155)', // Morandi green
  rate_limit_empty: 'rgb(122,39,30)', // dark warm contrast
  fastMode: 'rgb(155,179,155)', // Morandi green
  fastModeShimmer: 'rgb(191,208,191)', // lighter Morandi green
  // Brief/assistant mode
  briefLabelYou: 'rgb(155,179,155)', // Morandi green
  briefLabelClaude: 'rgb(143,168,191)', // Morandi blue
  rainbow_red: 'rgb(199,143,136)',
  rainbow_orange: 'rgb(207,167,132)',
  rainbow_yellow: 'rgb(216,195,143)',
  rainbow_green: 'rgb(155,179,155)',
  rainbow_blue: 'rgb(143,168,191)',
  rainbow_indigo: 'rgb(154,163,199)',
  rainbow_violet: 'rgb(176,156,188)',
  rainbow_red_shimmer: 'rgb(221,180,174)',
  rainbow_orange_shimmer: 'rgb(225,194,168)',
  rainbow_yellow_shimmer: 'rgb(231,216,178)',
  rainbow_green_shimmer: 'rgb(191,208,191)',
  rainbow_blue_shimmer: 'rgb(178,195,212)',
  rainbow_indigo_shimmer: 'rgb(194,199,223)',
  rainbow_violet_shimmer: 'rgb(204,190,214)',
}

/**
 * Light ANSI theme using only the 16 standard ANSI colors
 * for terminals without true color support
 */
const lightAnsiTheme: Theme = {
  autoAccept: 'ansi:magentaBright',
  bashBorder: 'ansi:red',
  claude: 'ansi:blue',
  claudeShimmer: 'ansi:cyan',
  claudeBlue_FOR_SYSTEM_SPINNER: 'ansi:blue',
  claudeBlueShimmer_FOR_SYSTEM_SPINNER: 'ansi:cyan',
  permission: 'ansi:red',
  permissionShimmer: 'ansi:redBright',
  planMode: 'ansi:yellow',
  ide: 'ansi:red',
  promptBorder: 'ansi:blueBright',
  promptBorderShimmer: 'ansi:cyanBright',
  text: 'ansi:black',
  inverseText: 'ansi:white',
  inactive: 'ansi:blackBright',
  inactiveShimmer: 'ansi:white',
  subtle: 'ansi:blackBright',
  suggestion: 'ansi:blue',
  remember: 'ansi:magenta',
  background: 'ansi:yellow',
  success: 'ansi:green',
  error: 'ansi:red',
  warning: 'ansi:yellow',
  merged: 'ansi:magenta',
  warningShimmer: 'ansi:yellowBright',
  diffAdded: 'ansi:green',
  diffRemoved: 'ansi:red',
  diffAddedDimmed: 'ansi:green',
  diffRemovedDimmed: 'ansi:red',
  diffAddedWord: 'ansi:greenBright',
  diffRemovedWord: 'ansi:redBright',
  // Agent colors
  red_FOR_SUBAGENTS_ONLY: 'ansi:red',
  blue_FOR_SUBAGENTS_ONLY: 'ansi:blue',
  green_FOR_SUBAGENTS_ONLY: 'ansi:green',
  yellow_FOR_SUBAGENTS_ONLY: 'ansi:yellow',
  purple_FOR_SUBAGENTS_ONLY: 'ansi:magenta',
  orange_FOR_SUBAGENTS_ONLY: 'ansi:yellowBright',
  pink_FOR_SUBAGENTS_ONLY: 'ansi:magentaBright',
  cyan_FOR_SUBAGENTS_ONLY: 'ansi:cyan',
  // Grove colors
  professionalBlue: 'ansi:yellow',
  // Chrome colors
  chromeYellow: 'ansi:yellow', // Chrome yellow
  // TUI V2 colors
  clawd_body: 'ansi:blue',
  clawd_background: 'ansi:black',
  userMessageBackground: 'ansi:yellow',
  userMessageBackgroundHover: 'ansi:yellowBright',
  messageActionsBackground: 'ansi:magenta',
  selectionBg: 'ansi:cyan', // lighter named bg for light-ansi; dark fgs stay readable
  bashMessageBackgroundColor: 'ansi:green',

  memoryBackgroundColor: 'ansi:cyan',
  rate_limit_fill: 'ansi:green',
  rate_limit_empty: 'ansi:black',
  fastMode: 'ansi:green',
  fastModeShimmer: 'ansi:green',
  briefLabelYou: 'ansi:green',
  briefLabelClaude: 'ansi:blue',
  rainbow_red: 'ansi:red',
  rainbow_orange: 'ansi:yellow',
  rainbow_yellow: 'ansi:yellow',
  rainbow_green: 'ansi:green',
  rainbow_blue: 'ansi:blue',
  rainbow_indigo: 'ansi:blueBright',
  rainbow_violet: 'ansi:magenta',
  rainbow_red_shimmer: 'ansi:redBright',
  rainbow_orange_shimmer: 'ansi:cyanBright',
  rainbow_yellow_shimmer: 'ansi:yellowBright',
  rainbow_green_shimmer: 'ansi:greenBright',
  rainbow_blue_shimmer: 'ansi:cyanBright',
  rainbow_indigo_shimmer: 'ansi:blueBright',
  rainbow_violet_shimmer: 'ansi:magentaBright',
}

/**
 * Dark ANSI theme using only the 16 standard ANSI colors
 * for terminals without true color support
 */
const darkAnsiTheme: Theme = {
  autoAccept: 'ansi:magentaBright',
  bashBorder: 'ansi:red',
  claude: 'ansi:blue',
  claudeShimmer: 'ansi:cyan',
  claudeBlue_FOR_SYSTEM_SPINNER: 'ansi:blue',
  claudeBlueShimmer_FOR_SYSTEM_SPINNER: 'ansi:cyan',
  permission: 'ansi:red',
  permissionShimmer: 'ansi:redBright',
  planMode: 'ansi:yellow',
  ide: 'ansi:red',
  promptBorder: 'ansi:blueBright',
  promptBorderShimmer: 'ansi:cyanBright',
  text: 'ansi:whiteBright',
  inverseText: 'ansi:black',
  inactive: 'ansi:white',
  inactiveShimmer: 'ansi:whiteBright',
  subtle: 'ansi:white',
  suggestion: 'ansi:blue',
  remember: 'ansi:magenta',
  background: 'ansi:yellow',
  success: 'ansi:green',
  error: 'ansi:redBright',
  warning: 'ansi:yellowBright',
  merged: 'ansi:magenta',
  warningShimmer: 'ansi:yellowBright',
  diffAdded: 'ansi:green',
  diffRemoved: 'ansi:red',
  diffAddedDimmed: 'ansi:green',
  diffRemovedDimmed: 'ansi:red',
  diffAddedWord: 'ansi:greenBright',
  diffRemovedWord: 'ansi:redBright',
  // Agent colors
  red_FOR_SUBAGENTS_ONLY: 'ansi:redBright',
  blue_FOR_SUBAGENTS_ONLY: 'ansi:blueBright',
  green_FOR_SUBAGENTS_ONLY: 'ansi:greenBright',
  yellow_FOR_SUBAGENTS_ONLY: 'ansi:yellowBright',
  purple_FOR_SUBAGENTS_ONLY: 'ansi:magentaBright',
  orange_FOR_SUBAGENTS_ONLY: 'ansi:yellowBright',
  pink_FOR_SUBAGENTS_ONLY: 'ansi:magentaBright',
  cyan_FOR_SUBAGENTS_ONLY: 'ansi:cyanBright',
  // Grove colors
  professionalBlue: 'ansi:yellow',
  // Chrome colors
  chromeYellow: 'ansi:yellowBright', // Chrome yellow
  // TUI V2 colors
  clawd_body: 'ansi:blue',
  clawd_background: 'ansi:black',
  userMessageBackground: 'ansi:yellow',
  userMessageBackgroundHover: 'ansi:yellowBright',
  messageActionsBackground: 'ansi:magenta',
  selectionBg: 'ansi:blue', // darker named bg for dark-ansi; bright fgs stay readable
  bashMessageBackgroundColor: 'ansi:green',

  memoryBackgroundColor: 'ansi:cyan',
  rate_limit_fill: 'ansi:green',
  rate_limit_empty: 'ansi:white',
  fastMode: 'ansi:green',
  fastModeShimmer: 'ansi:green',
  briefLabelYou: 'ansi:green',
  briefLabelClaude: 'ansi:blue',
  rainbow_red: 'ansi:red',
  rainbow_orange: 'ansi:yellow',
  rainbow_yellow: 'ansi:yellow',
  rainbow_green: 'ansi:green',
  rainbow_blue: 'ansi:blue',
  rainbow_indigo: 'ansi:blueBright',
  rainbow_violet: 'ansi:magenta',
  rainbow_red_shimmer: 'ansi:redBright',
  rainbow_orange_shimmer: 'ansi:cyanBright',
  rainbow_yellow_shimmer: 'ansi:yellowBright',
  rainbow_green_shimmer: 'ansi:greenBright',
  rainbow_blue_shimmer: 'ansi:cyanBright',
  rainbow_indigo_shimmer: 'ansi:blueBright',
  rainbow_violet_shimmer: 'ansi:magentaBright',
}

/**
 * Light daltonized theme (color-blind friendly) using explicit RGB values
 * to avoid inconsistencies from users' custom terminal ANSI color definitions
 */
const lightDaltonizedTheme: Theme = {
  autoAccept: 'rgb(135,0,255)', // Electric violet
  bashBorder: 'rgb(0,102,204)', // Blue instead of pink
  claude: 'rgb(36,134,185)', // saicode blue
  claudeShimmer: 'rgb(96,184,225)', // Lighter saicode blue for shimmer effect
  claudeBlue_FOR_SYSTEM_SPINNER: 'rgb(51,102,255)', // Bright blue for system spinner
  claudeBlueShimmer_FOR_SYSTEM_SPINNER: 'rgb(101,152,255)', // Lighter bright blue for system spinner shimmer
  permission: 'rgb(199,143,136)', // Morandi red
  permissionShimmer: 'rgb(221,180,174)', // lighter Morandi red
  planMode: 'rgb(216,195,143)', // Morandi yellow
  ide: 'rgb(199,143,136)', // match vertical separator red in this branch too
  promptBorder: 'rgb(154,163,199)', // Morandi indigo
  promptBorderShimmer: 'rgb(194,199,223)', // lighter Morandi indigo
  text: 'rgb(0,0,0)', // Black
  inverseText: 'rgb(255,255,255)', // White
  inactive: 'rgb(102,102,102)', // Dark gray
  inactiveShimmer: 'rgb(142,142,142)', // Lighter gray for shimmer effect
  subtle: 'rgb(175,175,175)', // Light gray
  suggestion: 'rgb(143,168,191)', // Morandi blue
  remember: 'rgb(176,156,188)', // Morandi purple
  background: 'rgb(216,195,143)', // Morandi yellow
  success: 'rgb(155,179,155)', // Morandi green
  error: 'rgb(199,143,136)', // Morandi red
  warning: 'rgb(216,195,143)', // Morandi yellow
  merged: 'rgb(135,0,255)', // Electric violet (matches autoAccept)
  warningShimmer: 'rgb(255,183,50)', // Lighter orange for shimmer
  diffAdded: 'rgb(153,204,255)', // Light blue instead of green
  diffRemoved: 'rgb(255,204,204)', // Light red
  diffAddedDimmed: 'rgb(209,231,253)', // Very light blue
  diffRemovedDimmed: 'rgb(255,233,233)', // Very light red
  diffAddedWord: 'rgb(51,102,204)', // Medium blue (less intense than deep blue)
  diffRemovedWord: 'rgb(153,51,51)', // Softer red (less intense than deep red)
  // Agent colors (daltonism-friendly)
  red_FOR_SUBAGENTS_ONLY: 'rgb(204,0,0)', // Pure red
  blue_FOR_SUBAGENTS_ONLY: 'rgb(0,102,204)', // Pure blue
  green_FOR_SUBAGENTS_ONLY: 'rgb(0,204,0)', // Pure green
  yellow_FOR_SUBAGENTS_ONLY: 'rgb(255,204,0)', // Golden yellow
  purple_FOR_SUBAGENTS_ONLY: 'rgb(128,0,128)', // True purple
  orange_FOR_SUBAGENTS_ONLY: 'rgb(36,134,185)', // Reused saicode blue
  pink_FOR_SUBAGENTS_ONLY: 'rgb(255,102,178)', // Adjusted pink
  cyan_FOR_SUBAGENTS_ONLY: 'rgb(0,178,178)', // Adjusted cyan
  // Grove colors
  professionalBlue: 'rgb(216,195,143)',
  // Chrome colors
  chromeYellow: 'rgb(251,188,4)', // Chrome yellow
  // TUI V2 colors
  clawd_body: 'rgb(36,134,185)',
  clawd_background: 'rgb(0,0,0)',
  userMessageBackground: 'rgb(255, 243, 212)', // warm morandi yellow card
  userMessageBackgroundHover: 'rgb(255, 249, 232)', // brighter warm hover
  messageActionsBackground: 'rgb(236, 225, 247)', // soft morandi violet strip
  selectionBg: 'rgb(180, 213, 255)', // light selection blue; daltonized fgs are yellows/blues, both readable on light blue
  bashMessageBackgroundColor: 'rgb(250, 245, 250)',

  memoryBackgroundColor: 'rgb(230, 245, 250)',
  rate_limit_fill: 'rgb(155,179,155)', // Morandi green
  rate_limit_empty: 'rgb(23,46,114)', // Dark blue
  fastMode: 'rgb(155,179,155)',
  fastModeShimmer: 'rgb(191,208,191)',
  briefLabelYou: 'rgb(155,179,155)',
  briefLabelClaude: 'rgb(143,168,191)',
  rainbow_red: 'rgb(199,143,136)',
  rainbow_orange: 'rgb(207,167,132)',
  rainbow_yellow: 'rgb(216,195,143)',
  rainbow_green: 'rgb(155,179,155)',
  rainbow_blue: 'rgb(143,168,191)',
  rainbow_indigo: 'rgb(154,163,199)',
  rainbow_violet: 'rgb(176,156,188)',
  rainbow_red_shimmer: 'rgb(250,155,147)',
  rainbow_orange_shimmer: 'rgb(166,214,245)',
  rainbow_yellow_shimmer: 'rgb(255,225,155)',
  rainbow_green_shimmer: 'rgb(185,230,180)',
  rainbow_blue_shimmer: 'rgb(180,205,240)',
  rainbow_indigo_shimmer: 'rgb(195,180,230)',
  rainbow_violet_shimmer: 'rgb(230,180,210)',
}

/**
 * Dark theme using explicit RGB values to avoid inconsistencies
 * from users' custom terminal ANSI color definitions
 */
const darkTheme: Theme = {
  autoAccept: 'rgb(185,163,197)', // Morandi violet
  bashBorder: 'rgb(199,143,136)', // Morandi red
  claude: 'rgb(143,168,191)', // Morandi blue
  claudeShimmer: 'rgb(178,195,212)', // lighter Morandi blue
  claudeBlue_FOR_SYSTEM_SPINNER: 'rgb(143,168,191)',
  claudeBlueShimmer_FOR_SYSTEM_SPINNER: 'rgb(178,195,212)',
  permission: 'rgb(199,143,136)', // Morandi red
  permissionShimmer: 'rgb(221,180,174)', // lighter red shimmer
  planMode: 'rgb(216,195,143)', // Morandi yellow
  ide: 'rgb(155,179,155)', // Morandi green
  promptBorder: 'rgb(154,163,199)', // Morandi indigo
  promptBorderShimmer: 'rgb(194,199,223)', // lighter Morandi indigo
  text: 'rgb(255,255,255)', // White
  inverseText: 'rgb(0,0,0)', // Black
  inactive: 'rgb(153,153,153)', // Light gray
  inactiveShimmer: 'rgb(193,193,193)', // Lighter gray for shimmer effect
  subtle: 'rgb(80,80,80)', // Dark gray
  suggestion: 'rgb(178,195,212)', // lighter Morandi blue
  remember: 'rgb(185,163,197)', // Morandi purple
  background: 'rgb(216,195,143)', // Morandi yellow
  success: 'rgb(155,179,155)', // Morandi green
  error: 'rgb(199,143,136)', // Morandi red
  warning: 'rgb(216,195,143)', // Morandi yellow
  merged: 'rgb(185,163,197)', // Morandi purple
  warningShimmer: 'rgb(231,216,178)', // lighter yellow
  diffAdded: 'rgb(34,92,43)', // Dark green
  diffRemoved: 'rgb(122,41,54)', // Dark red
  diffAddedDimmed: 'rgb(71,88,74)', // Very dark green
  diffRemovedDimmed: 'rgb(105,72,77)', // Very dark red
  diffAddedWord: 'rgb(56,166,96)', // Medium green
  diffRemovedWord: 'rgb(179,89,107)', // Softer red (less intense than bright red)
  // Agent colors
  red_FOR_SUBAGENTS_ONLY: 'rgb(220,38,38)', // Red 600
  blue_FOR_SUBAGENTS_ONLY: 'rgb(37,99,235)', // Blue 600
  green_FOR_SUBAGENTS_ONLY: 'rgb(22,163,74)', // Green 600
  yellow_FOR_SUBAGENTS_ONLY: 'rgb(202,138,4)', // Yellow 600
  purple_FOR_SUBAGENTS_ONLY: 'rgb(147,51,234)', // Purple 600
  orange_FOR_SUBAGENTS_ONLY: 'rgb(251,188,5)', // Google yellow
  pink_FOR_SUBAGENTS_ONLY: 'rgb(219,39,119)', // Pink 600
  cyan_FOR_SUBAGENTS_ONLY: 'rgb(8,145,178)', // Cyan 600
  // Grove colors
  professionalBlue: 'rgb(216,195,143)',
  // Chrome colors
  chromeYellow: 'rgb(251,188,4)', // Chrome yellow
  // TUI V2 colors
  clawd_body: 'rgb(143,168,191)',
  clawd_background: 'rgb(0,0,0)',
  userMessageBackground: 'rgb(76, 57, 18)', // deep amber card
  userMessageBackgroundHover: 'rgb(96, 71, 20)',
  messageActionsBackground: 'rgb(74, 43, 102)', // deep violet strip
  selectionBg: 'rgb(38, 79, 120)', // classic dark-mode selection blue (VS Code dark default); light fgs stay readable
  bashMessageBackgroundColor: 'rgb(25, 72, 46)',

  memoryBackgroundColor: 'rgb(23, 48, 92)',
  rate_limit_fill: 'rgb(155,179,155)', // Morandi green
  rate_limit_empty: 'rgb(112,49,39)', // deep warm contrast
  fastMode: 'rgb(155,179,155)', // Morandi green
  fastModeShimmer: 'rgb(191,208,191)', // lighter Morandi green
  briefLabelYou: 'rgb(155,179,155)', // Morandi green
  briefLabelClaude: 'rgb(143,168,191)', // Morandi blue
  rainbow_red: 'rgb(199,143,136)',
  rainbow_orange: 'rgb(207,167,132)',
  rainbow_yellow: 'rgb(216,195,143)',
  rainbow_green: 'rgb(155,179,155)',
  rainbow_blue: 'rgb(143,168,191)',
  rainbow_indigo: 'rgb(154,163,199)',
  rainbow_violet: 'rgb(185,163,197)',
  rainbow_red_shimmer: 'rgb(221,180,174)',
  rainbow_orange_shimmer: 'rgb(225,194,168)',
  rainbow_yellow_shimmer: 'rgb(231,216,178)',
  rainbow_green_shimmer: 'rgb(191,208,191)',
  rainbow_blue_shimmer: 'rgb(178,195,212)',
  rainbow_indigo_shimmer: 'rgb(194,199,223)',
  rainbow_violet_shimmer: 'rgb(214,197,224)',
}

/**
 * Dark daltonized theme (color-blind friendly) using explicit RGB values
 * to avoid inconsistencies from users' custom terminal ANSI color definitions
 */
const darkDaltonizedTheme: Theme = {
  autoAccept: 'rgb(175,135,255)', // Electric violet
  bashBorder: 'rgb(51,153,255)', // Bright blue
  claude: 'rgb(36,134,185)', // saicode blue
  claudeShimmer: 'rgb(96,184,225)', // Lighter saicode blue for shimmer effect
  claudeBlue_FOR_SYSTEM_SPINNER: 'rgb(153,204,255)', // Light blue for system spinner
  claudeBlueShimmer_FOR_SYSTEM_SPINNER: 'rgb(183,224,255)', // Lighter blue for system spinner shimmer
  permission: 'rgb(199,143,136)', // Morandi red
  permissionShimmer: 'rgb(221,180,174)', // lighter red for shimmer
  planMode: 'rgb(216,195,143)', // Morandi yellow
  ide: 'rgb(199,143,136)', // vertical separator red
  promptBorder: 'rgb(154,163,199)', // Morandi indigo
  promptBorderShimmer: 'rgb(194,199,223)', // lighter Morandi indigo
  text: 'rgb(255,255,255)', // White
  inverseText: 'rgb(0,0,0)', // Black
  inactive: 'rgb(153,153,153)', // Light gray
  inactiveShimmer: 'rgb(193,193,193)', // Lighter gray for shimmer effect
  subtle: 'rgb(80,80,80)', // Dark gray
  suggestion: 'rgb(178,195,212)', // lighter Morandi blue
  remember: 'rgb(185,163,197)', // Morandi purple
  background: 'rgb(216,195,143)', // Morandi yellow
  success: 'rgb(155,179,155)', // Morandi green
  error: 'rgb(199,143,136)', // Morandi red
  warning: 'rgb(216,195,143)', // Morandi yellow
  merged: 'rgb(175,135,255)', // Electric violet (matches autoAccept)
  warningShimmer: 'rgb(255,234,50)', // Lighter yellow-orange for shimmer
  diffAdded: 'rgb(0,68,102)', // Dark blue
  diffRemoved: 'rgb(102,0,0)', // Dark red
  diffAddedDimmed: 'rgb(62,81,91)', // Dimmed blue
  diffRemovedDimmed: 'rgb(62,44,44)', // Dimmed red
  diffAddedWord: 'rgb(0,119,179)', // Medium blue
  diffRemovedWord: 'rgb(179,0,0)', // Medium red
  // Agent colors (daltonism-friendly, dark mode)
  red_FOR_SUBAGENTS_ONLY: 'rgb(255,102,102)', // Bright red
  blue_FOR_SUBAGENTS_ONLY: 'rgb(102,178,255)', // Bright blue
  green_FOR_SUBAGENTS_ONLY: 'rgb(102,255,102)', // Bright green
  yellow_FOR_SUBAGENTS_ONLY: 'rgb(255,255,102)', // Bright yellow
  purple_FOR_SUBAGENTS_ONLY: 'rgb(178,102,255)', // Bright purple
  orange_FOR_SUBAGENTS_ONLY: 'rgb(36,134,185)', // Reused saicode blue
  pink_FOR_SUBAGENTS_ONLY: 'rgb(255,153,204)', // Bright pink
  cyan_FOR_SUBAGENTS_ONLY: 'rgb(102,204,204)', // Bright cyan
  // Grove colors
  professionalBlue: 'rgb(216,195,143)',
  // Chrome colors
  chromeYellow: 'rgb(251,188,4)', // Chrome yellow
  // TUI V2 colors
  clawd_body: 'rgb(36,134,185)',
  clawd_background: 'rgb(0,0,0)',
  userMessageBackground: 'rgb(92, 72, 32)', // dark amber card
  userMessageBackgroundHover: 'rgb(112, 84, 34)',
  messageActionsBackground: 'rgb(86, 56, 116)', // dark violet strip
  selectionBg: 'rgb(38, 79, 120)', // classic dark-mode selection blue (VS Code dark default); light fgs stay readable
  bashMessageBackgroundColor: 'rgb(65, 60, 65)',

  memoryBackgroundColor: 'rgb(55, 65, 70)',
  rate_limit_fill: 'rgb(155,179,155)', // Morandi green
  rate_limit_empty: 'rgb(69,92,115)', // Dark blue
  fastMode: 'rgb(155,179,155)',
  fastModeShimmer: 'rgb(191,208,191)',
  briefLabelYou: 'rgb(155,179,155)',
  briefLabelClaude: 'rgb(143,168,191)',
  rainbow_red: 'rgb(199,143,136)',
  rainbow_orange: 'rgb(207,167,132)',
  rainbow_yellow: 'rgb(216,195,143)',
  rainbow_green: 'rgb(155,179,155)',
  rainbow_blue: 'rgb(143,168,191)',
  rainbow_indigo: 'rgb(154,163,199)',
  rainbow_violet: 'rgb(185,163,197)',
  rainbow_red_shimmer: 'rgb(250,155,147)',
  rainbow_orange_shimmer: 'rgb(166,214,245)',
  rainbow_yellow_shimmer: 'rgb(255,225,155)',
  rainbow_green_shimmer: 'rgb(185,230,180)',
  rainbow_blue_shimmer: 'rgb(180,205,240)',
  rainbow_indigo_shimmer: 'rgb(195,180,230)',
  rainbow_violet_shimmer: 'rgb(230,180,210)',
}

export function getTheme(themeName: ThemeName): Theme {
  switch (themeName) {
    case 'light':
      return lightTheme
    case 'light-ansi':
      return lightAnsiTheme
    case 'dark-ansi':
      return darkAnsiTheme
    case 'light-daltonized':
      return lightDaltonizedTheme
    case 'dark-daltonized':
      return darkDaltonizedTheme
    default:
      return darkTheme
  }
}

// Create a chalk instance with 256-color level for Apple Terminal
// Apple Terminal doesn't handle 24-bit color escape sequences well
const chalkForChart =
  env.terminal === 'Apple_Terminal'
    ? new Chalk({ level: 2 }) // 256 colors
    : chalk

/**
 * Converts a theme color to an ANSI escape sequence for use with asciichart.
 * Uses chalk to generate the escape codes, with 256-color mode for Apple Terminal.
 */
export function themeColorToAnsi(themeColor: string): string {
  const rgbMatch = themeColor.match(/rgb\(\s?(\d+),\s?(\d+),\s?(\d+)\s?\)/)
  if (rgbMatch) {
    const r = parseInt(rgbMatch[1]!, 10)
    const g = parseInt(rgbMatch[2]!, 10)
    const b = parseInt(rgbMatch[3]!, 10)
    // Use chalk.rgb which auto-converts to 256 colors when level is 2
    // Extract just the opening escape sequence by using a marker
    const colored = chalkForChart.rgb(r, g, b)('X')
    return colored.slice(0, colored.indexOf('X'))
  }
  // Fallback to magenta if parsing fails
  return '\x1b[35m'
}
