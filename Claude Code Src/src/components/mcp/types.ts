export type ServerInfo = {
  name: string
  status?: string
  config?: any
  client?: any
  tools?: any[]
  resources?: any[]
  [key: string]: any
}

export type StdioServerInfo = ServerInfo & {
  type?: 'stdio'
}

export type SSEServerInfo = ServerInfo & {
  type?: 'sse'
  url?: string
}

export type HTTPServerInfo = ServerInfo & {
  type?: 'http'
  url?: string
}

export type ClaudeAIServerInfo = ServerInfo & {
  type?: 'claudeai'
}

export type AgentMcpServerInfo = ServerInfo & {
  agentName?: string
}

export type MCPViewState =
  | string
  | {
      type: string
      server?: ServerInfo
      client?: any
      tool?: any
      [key: string]: any
    }
