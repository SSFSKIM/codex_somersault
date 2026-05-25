export type SecureStorageData = Record<string, any>

export type SecureStorage = {
  name: string
  get?: () => Promise<SecureStorageData | null>
  set?: (data: SecureStorageData) => Promise<void>
  read: () => SecureStorageData | null
  readAsync: () => Promise<SecureStorageData | null>
  update: (data: SecureStorageData) => { success: boolean; warning?: string }
  delete: () => boolean
  [key: string]: any
}
