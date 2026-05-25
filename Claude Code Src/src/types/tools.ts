// Structural replacement for the missing generated tool-progress module.
// The shapes below mirror the progress payloads emitted by the concrete tools
// and documented in docs/specs/08-tool-base-registry.md section 4.4.

import type {
  AssistantMessage,
  NormalizedUserMessage,
} from './message.js'

type AnyRecord = Record<string, any>

export type ShellProgress = {
  type: 'progress' | 'bash_progress' | 'powershell_progress'
  output: string
  fullOutput: string
  elapsedTimeSeconds: number
  totalLines: number
  totalBytes: number
  taskId?: string
  timeoutMs?: number
} & AnyRecord

export type BashProgress = ShellProgress & { type: 'bash_progress' }
export type PowerShellProgress = ShellProgress & { type: 'powershell_progress' }

export type AgentToolProgress = {
  type: 'agent_progress'
  message: AssistantMessage | NormalizedUserMessage
  prompt?: string
  agentId?: string
} & AnyRecord

export type SkillToolProgress = {
  type: 'skill_progress'
  message: AssistantMessage | NormalizedUserMessage
  prompt?: string
  agentId?: string
} & AnyRecord

export type MCPProgress = {
  type?: 'mcp_progress'
  status?: 'started' | 'completed' | 'failed' | string
  serverName?: string
  toolName?: string
  elapsedTimeMs?: number
  progress?: number
  total?: number
  progressMessage?: string
} & AnyRecord

export type WebSearchProgress =
  | ({
      type: 'query_update'
      query: string
    } & AnyRecord)
  | ({
      type: 'search_results_received'
      query: string
      resultCount: number
    } & AnyRecord)

export type TaskOutputProgress = {
  type: 'waiting_for_task'
  taskDescription?: string
  taskType?: string
} & AnyRecord

export type REPLToolProgress = {
  type: string
} & AnyRecord

export type SdkWorkflowProgress = {
  type: string
  index?: number
  phaseIndex?: number
  status?: string
  title?: string
  detail?: string
} & AnyRecord

export type ToolProgressData =
  | AgentToolProgress
  | BashProgress
  | MCPProgress
  | PowerShellProgress
  | REPLToolProgress
  | SdkWorkflowProgress
  | ShellProgress
  | SkillToolProgress
  | TaskOutputProgress
  | WebSearchProgress
