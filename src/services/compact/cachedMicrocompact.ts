export type CacheEditsBlock = {
  type: 'cache_edits'
  deletes: string[]
}

export type PinnedCacheEdits = {
  userMessageIndex: number
  block: CacheEditsBlock
}

export type CachedMCState = {
  registeredTools: Set<string>
  toolOrder: string[]
  deletedRefs: Set<string>
  pinnedEdits: PinnedCacheEdits[]
}

export function createCachedMCState(): CachedMCState {
  return {
    registeredTools: new Set<string>(),
    toolOrder: [],
    deletedRefs: new Set<string>(),
    pinnedEdits: [],
  }
}

export function resetCachedMCState(state: CachedMCState): void {
  state.registeredTools.clear()
  state.toolOrder.length = 0
  state.deletedRefs.clear()
  state.pinnedEdits.length = 0
}

export function markToolsSentToAPI(_state: CachedMCState): void {}

export function isCachedMicrocompactEnabled(): boolean {
  return false
}

export function isModelSupportedForCacheEditing(_model: string): boolean {
  return false
}

export function getCachedMCConfig(): {
  triggerThreshold: number
  keepRecent: number
} {
  return {
    triggerThreshold: 0,
    keepRecent: 0,
  }
}

export function registerToolResult(
  state: CachedMCState,
  toolUseId: string,
): void {
  state.registeredTools.add(toolUseId)
  state.toolOrder.push(toolUseId)
}

export function registerToolMessage(
  _state: CachedMCState,
  _groupIds: string[],
): void {}

export function getToolResultsToDelete(_state: CachedMCState): string[] {
  return []
}

export function createCacheEditsBlock(
  _state: CachedMCState,
  toolsToDelete: string[],
): CacheEditsBlock | null {
  if (toolsToDelete.length === 0) {
    return null
  }
  return {
    type: 'cache_edits',
    deletes: toolsToDelete,
  }
}
