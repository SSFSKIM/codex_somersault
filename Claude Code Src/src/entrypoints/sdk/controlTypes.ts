// Stub for the missing-from-leak entrypoints/sdk/controlTypes module
// (referenced by bridge/, sdk/, and remote/ subsystems — all gated by
// feature('BRIDGE_MODE') etc. and dead at runtime via the shim).

/* eslint-disable @typescript-eslint/no-explicit-any */

import type { HookEvent, HookInput } from './coreTypes.js'

export type SDKHookCallbackMatcher = {
  matcher?: string
  hookCallbackIds: string[]
  timeout?: number
}

export type SDKControlPermissionRequest = {
  type?: 'control_request'
  subtype: 'permission_request'
  [key: string]: any
}
export type SDKControlResponse = {
  type: 'control_response'
  uuid?: string
  subtype?: string
  response?: any
  error?: string
  [key: string]: any
}
export type StdoutMessage = any
export type SDKControlInitializeRequest = {
  type?: 'control_request'
  subtype: 'initialize'
  hooks?: Partial<Record<HookEvent, SDKHookCallbackMatcher[]>>
  sdkMcpServers?: string[]
  jsonSchema?: Record<string, unknown>
  systemPrompt?: string
  appendSystemPrompt?: string
  agents?: Record<string, any>
  promptSuggestions?: boolean
  agentProgressSummaries?: boolean
  [key: string]: any
}
export type SDKControlRequest =
  | {
      type: 'control_request'
      request_id: string
      request: any
      [key: string]: any
    }
  | SDKControlInitializeRequest
  | SDKControlPermissionRequest
  | SDKControlCancelRequest
  | SDKControlElicitationResponse
  | SDKControlRequestInner
export type SDKControlInitializeResponse = any
export type SDKControlMcpSetServersResponse = any
export type SDKControlReloadPluginsResponse = any
export type SDKControlMcpStatusResponse = any
export type SDKControlGetContextUsageResponse = any
export type SDKControlRewindFilesResponse = any
export type SDKControlCancelAsyncMessageResponse = any
export type SDKControlGetSettingsResponse = any
export type SDKControlElicitationRequest = any
export type SDKControlElicitationResponse = {
  type?: 'control_request'
  subtype: 'elicitation_response'
  [key: string]: any
}
export type SDKControlCancelRequest = {
  type?: 'control_request'
  subtype: 'cancel' | 'cancel_async_message'
  [key: string]: any
}
export type SDKKeepAliveMessage = { type: 'keep_alive' }
export type SDKUpdateEnvironmentVariablesMessage = {
  type: 'update_environment_variables'
  variables: Record<string, string>
}
export type StdinMessage =
  | SDKKeepAliveMessage
  | SDKUpdateEnvironmentVariablesMessage
  | SDKControlResponse
  | SDKControlRequest
export type SDKPartialAssistantMessage = any
export type SDKControlRequestInner = {
  type?: 'control_request'
  subtype: string
  input?: HookInput
  [key: string]: any
}
