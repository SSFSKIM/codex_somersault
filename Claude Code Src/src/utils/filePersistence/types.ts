export const FILE_COUNT_LIMIT = 100
export const DEFAULT_UPLOAD_CONCURRENCY = 5
export const OUTPUTS_SUBDIR = 'outputs'

export type TurnStartTime = number

export type PersistedFile = {
  filename: string
  file_id?: string
  path?: string
  [key: string]: any
}

export type FailedPersistence = {
  filename: string
  error: string
  [key: string]: any
}

export type FilesPersistedEventData = {
  files: PersistedFile[]
  failed: FailedPersistence[]
  processed_at?: string
  [key: string]: any
}
