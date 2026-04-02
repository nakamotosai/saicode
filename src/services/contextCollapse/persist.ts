import type {
  ContextCollapseCommitEntry,
  ContextCollapseSnapshotEntry,
} from '../../types/logs.js'
import { resetContextCollapse } from './index.js'

type RestoredContextCollapseState = {
  commits: ContextCollapseCommitEntry[]
  snapshot: ContextCollapseSnapshotEntry | undefined
}

let restoredState: RestoredContextCollapseState = {
  commits: [],
  snapshot: undefined,
}

export function restoreFromEntries(
  commits: ContextCollapseCommitEntry[],
  snapshot?: ContextCollapseSnapshotEntry,
): void {
  resetContextCollapse()
  restoredState = {
    commits: [...commits],
    snapshot,
  }
}

export function getRestoredContextCollapseState(): RestoredContextCollapseState {
  return {
    commits: [...restoredState.commits],
    snapshot: restoredState.snapshot,
  }
}
