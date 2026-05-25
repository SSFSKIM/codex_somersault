export type LspServerConfig = {
  command?: string
  args?: string[]
  env?: Record<string, string>
  [key: string]: any
}

export type ScopedLspServerConfig = LspServerConfig & {
  name?: string
  scope?: string
}

export type LspServerState =
  | 'stopped'
  | 'starting'
  | 'running'
  | 'error'
  | 'connected'
  | 'disconnected'
  | (string & {})
