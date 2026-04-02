// Local recovery surface for file persistence.
// Keep these field names aligned with filePersistence.ts call sites.

export const DEFAULT_UPLOAD_CONCURRENCY = 5
export const FILE_COUNT_LIMIT = 100
export const OUTPUTS_SUBDIR = 'outputs'

export interface FailedPersistence {
  filename: string
  error: string
}

export interface PersistedFile {
  filename: string
  file_id: string
}

export interface FilesPersistedEventData {
  files: PersistedFile[]
  failed: FailedPersistence[]
}

export type TurnStartTime = number
