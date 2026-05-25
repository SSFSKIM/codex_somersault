// Structural replacement for the missing generated message module.
// It follows the turn-pipeline stream/message union in docs/specs/04 and the
// Anthropic API stream envelope in docs/specs/22, while leaving nested SDK
// payloads permissive because those are owned by provider adapters.

import type { APIError } from '@anthropic-ai/sdk'
import type { BetaUsage } from '@anthropic-ai/sdk/resources/beta/messages/messages.mjs'

type AnyRecord = Record<string, any>
type UUID = string
type ContentBlock = any
type UserContent = any
type AssistantContent = any[]

export type MessageOrigin =
  {
    kind:
      | 'human'
      | 'channel'
      | 'task-notification'
      | 'coordinator'
      | 'local_command'
      | 'slash_command'
      | 'sdk'
      | 'hook'
      | 'system'
      | string
    [key: string]: any
  }

export type PartialCompactDirection = 'from' | 'up_to'

export type MessageBase = {
  uuid: UUID | string
  timestamp: string
  isMeta?: boolean
  isVirtual?: boolean
  [key: string]: any
}

export type UserMessage = MessageBase & {
  type: 'user'
  message: {
    role: 'user'
    content: UserContent
    [key: string]: any
  }
  isVisibleInTranscriptOnly?: true
  isCompactSummary?: true
  summarizeMetadata?: {
    messagesSummarized: number
    userContext?: string
    direction?: PartialCompactDirection
  }
  toolUseResult?: unknown
  mcpMeta?: {
    _meta?: Record<string, unknown>
    structuredContent?: Record<string, unknown>
  }
  imagePasteIds?: number[]
  sourceToolAssistantUUID?: UUID | string
  permissionMode?: string
  origin?: MessageOrigin
}

export type AssistantMessage = MessageBase & {
  type: 'assistant'
  message: {
    id: string
    type: 'message'
    role: 'assistant'
    model: string
    content: AssistantContent
    stop_reason?: string | null
    stop_sequence?: string | null
    usage: BetaUsage
    context_management?: unknown
    [key: string]: any
  }
  requestId?: string
  apiError?: string
  error?: string
  errorDetails?: string
  isApiErrorMessage?: boolean
  advisorModel?: string
}

export type AttachmentMessage<T = any> = MessageBase & {
  type: 'attachment'
  attachment: T
}

export type ProgressMessage<T = any> = MessageBase & {
  type: 'progress'
  data: T
  toolUseID: string
  parentToolUseID?: string
}

export type SystemMessageLevel = 'info' | 'warning' | 'error' | 'suggestion'

export type StopHookInfo = {
  command: string
  promptText?: string
  durationMs?: number
  [key: string]: any
}

type SystemBase<Subtype extends string> = MessageBase & {
  type: 'system'
  subtype: Subtype
  level?: SystemMessageLevel
  content?: string
  toolUseID?: string
}

export type SystemInformationalMessage = SystemBase<'informational'> & {
  content: string
  level: SystemMessageLevel
  preventContinuation?: boolean
}

export type SystemLocalCommandMessage = SystemBase<'local_command'> & {
  content: string
  level: 'info'
}

export type SystemPermissionRetryMessage = SystemBase<'permission_retry'> & {
  content: string
  commands: string[]
  level: 'info'
}

export type SystemBridgeStatusMessage = SystemBase<'bridge_status'> & {
  content: string
  url: string
  upgradeNudge?: string
}

export type SystemScheduledTaskFireMessage =
  SystemBase<'scheduled_task_fire'> & {
    content: string
  }

export type SystemStopHookSummaryMessage =
  SystemBase<'stop_hook_summary'> & {
    hookCount: number
    hookInfos: StopHookInfo[]
    hookErrors: string[]
    preventedContinuation: boolean
    stopReason?: string
    hasOutput: boolean
    hookLabel?: string
    totalDurationMs?: number
  }

export type SystemTurnDurationMessage = SystemBase<'turn_duration'> & {
  durationMs: number
  budgetTokens?: number
  budgetLimit?: number
  budgetNudges?: number
  messageCount?: number
}

export type SystemAwaySummaryMessage = SystemBase<'away_summary'> & {
  content: string
}

export type SystemMemorySavedMessage = SystemBase<'memory_saved'> & {
  writtenPaths: string[]
}

export type SystemAgentsKilledMessage = SystemBase<'agents_killed'>

export type SystemApiMetricsMessage = SystemBase<'api_metrics'> & {
  ttftMs: number
  otps: number
  isP50?: boolean
  hookDurationMs?: number
  turnDurationMs?: number
  toolDurationMs?: number
  classifierDurationMs?: number
  toolCount?: number
  hookCount?: number
  classifierCount?: number
  configWriteCount?: number
}

export type SystemCompactBoundaryMessage = SystemBase<'compact_boundary'> & {
  content: string
  level: 'info'
	  compactMetadata: {
	    trigger: 'manual' | 'auto'
	    preTokens: number
	    userContext?: string
	    messagesSummarized?: number
	    preservedSegment?: AnyRecord
	    preCompactDiscoveredTools?: string[]
	  }
  logicalParentUuid?: UUID | string
}

export type SystemMicrocompactBoundaryMessage =
  SystemBase<'microcompact_boundary'> & {
    content: string
    level: 'info'
    microcompactMetadata: {
      trigger: 'auto'
      preTokens: number
      tokensSaved: number
      compactedToolIds: string[]
      clearedAttachmentUUIDs: string[]
    }
  }

export type SystemAPIErrorMessage = SystemBase<'api_error'> & {
  level: 'error'
  error: APIError
  cause?: Error
  retryInMs: number
  retryAttempt: number
  maxRetries: number
}

export type SystemThinkingMessage = SystemBase<'thinking'> & {
  content?: string
}

export type SystemFileSnapshotMessage = SystemBase<'file_snapshot'> & AnyRecord

export type SystemMessage =
  | SystemInformationalMessage
  | SystemLocalCommandMessage
  | SystemPermissionRetryMessage
  | SystemBridgeStatusMessage
  | SystemScheduledTaskFireMessage
  | SystemStopHookSummaryMessage
  | SystemTurnDurationMessage
  | SystemAwaySummaryMessage
  | SystemMemorySavedMessage
  | SystemAgentsKilledMessage
  | SystemApiMetricsMessage
  | SystemCompactBoundaryMessage
  | SystemMicrocompactBoundaryMessage
  | SystemAPIErrorMessage
  | SystemThinkingMessage
  | SystemFileSnapshotMessage
  | (SystemBase<string> & AnyRecord)

export type Message =
  | UserMessage
  | AssistantMessage
  | AttachmentMessage
  | ProgressMessage
  | SystemMessage

export type NormalizedUserMessage = MessageBase & {
  type: 'user'
  message: {
    role: 'user'
    content: ContentBlock[]
    [key: string]: any
  }
  isVisibleInTranscriptOnly?: true
  isCompactSummary?: true
  summarizeMetadata?: UserMessage['summarizeMetadata']
  toolUseResult?: unknown
  mcpMeta?: UserMessage['mcpMeta']
  imagePasteIds?: number[]
  sourceToolAssistantUUID?: UUID | string
  permissionMode?: string
  origin?: MessageOrigin
}

export type NormalizedAssistantMessage<T = any> = AssistantMessage & {
  message: AssistantMessage['message'] & {
    content: [ContentBlock] | ContentBlock[]
  }
}

export type NormalizedMessage =
  | NormalizedUserMessage
  | NormalizedAssistantMessage
  | AttachmentMessage
  | ProgressMessage
  | SystemMessage

export type RenderableMessage =
  | NormalizedMessage
  | GroupedToolUseMessage
  | CollapsedReadSearchGroup

export type RequestStartEvent = {
  type: 'stream_request_start'
}

export type StreamEvent = {
  type: 'stream_event'
  event: AnyRecord & { type: string }
  ttftMs?: number
}

export type TombstoneMessage = {
  type: 'tombstone'
  message: Message
}

export type ToolUseSummaryMessage = MessageBase & {
  type: 'tool_use_summary'
  summary: string
  precedingToolUseIds: string[]
}

export type HookResultMessage = AttachmentMessage | ProgressMessage

export type GroupedToolUseMessage = {
  type: 'grouped_tool_use'
  toolUseIDs?: string[]
  messages: NormalizedAssistantMessage[]
  results: NormalizedUserMessage[]
  [key: string]: any
}

export type CollapsedReadSearchGroup = {
  type: 'collapsed_read_search'
  messages: CollapsibleMessage[]
  [key: string]: any
}

export type AnyMessage = Message | StreamEvent | RequestStartEvent
export type CollapsibleMessage = RenderableMessage
export type CompactMetadata = any
