export function isContextCollapseEnabled(): boolean {
  return false
}

export function getStats(): {
  savedTokens: number
  savedPercent: number
  totalSpawns: number
  totalErrors: number
  totalEmptySpawns: number
  emptySpawnWarningEmitted: boolean
  health: {
    totalSpawns: number
    totalErrors: number
    totalEmptySpawns: number
    emptySpawnWarningEmitted: boolean
    lastError?: string
  }
  collapsedSpans: number
  collapsedMessages: number
  stagedSpans: number
  lastError?: string
} {
  return {
    savedTokens: 0,
    savedPercent: 0,
    totalSpawns: 0,
    totalErrors: 0,
    totalEmptySpawns: 0,
    emptySpawnWarningEmitted: false,
    health: {
      totalSpawns: 0,
      totalErrors: 0,
      totalEmptySpawns: 0,
      emptySpawnWarningEmitted: false,
    },
    collapsedSpans: 0,
    collapsedMessages: 0,
    stagedSpans: 0,
  }
}

export function maybeCollapseContext<T>(value: T): T {
  return value
}

export function collapseContext<T>(value: T): T {
  return value
}
