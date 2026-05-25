import { randomUUID } from 'crypto'
import OpenAI from 'openai'
import type { ReasoningEffort } from 'openai/resources/shared.js'
import type {
  FunctionTool,
  ResponseCreateParamsStreaming,
  ResponseInputItem,
  ResponseStreamEvent,
} from 'openai/resources/responses/responses.js'
import type { BetaContentBlock } from '@anthropic-ai/sdk/resources/beta/messages/messages.mjs'
import type { Tools } from '../../Tool.js'
import type {
  AssistantMessage,
  Message,
  StreamEvent,
  SystemAPIErrorMessage,
  UserMessage,
} from '../../types/message.js'
import { toolToAPISchema } from '../../utils/api.js'
import { logForDebugging } from '../../utils/debug.js'
import { getEffortEnvOverride } from '../../utils/effort.js'
import { safeParseJSON } from '../../utils/json.js'
import {
  createAssistantAPIErrorMessage,
  ensureToolResultPairing,
  normalizeContentFromAPI,
  normalizeMessagesForAPI,
} from '../../utils/messages.js'
import type { ThinkingConfig } from '../../utils/thinking.js'
import type { queryModelWithStreaming } from './claude.js'
import { EMPTY_USAGE } from './emptyUsage.js'
import { getCodexCredential } from './openaiCodexAuth.js'

type QueryParams = Parameters<typeof queryModelWithStreaming>[0]
type QueryResult = ReturnType<typeof queryModelWithStreaming>
type AnyRecord = Record<string, any>
type OpenAIUsage = AnyRecord | null | undefined

type ActiveBlock =
  | {
      type: 'text'
      key: string
      index: number
      text: string
      stopped: boolean
    }
  | {
      type: 'thinking'
      key: string
      index: number
      thinking: string
      stopped: boolean
    }
  | {
      type: 'tool_use'
      key: string
      index: number
      id: string
      name: string
      input: string
      stopped: boolean
    }

const DEFAULT_OPENAI_BASE_URL = 'https://api.openai.com/v1'

// Anthropic message shape -> OpenAI Responses mapping:
// - systemPrompt[] -> leading system input message with input_text content
// - user text/image blocks -> message content parts: input_text/input_image
// - user tool_result blocks -> top-level function_call_output items
// - assistant text blocks -> assistant output message items with output_text
// - assistant tool_use blocks -> top-level function_call items
// - assistant thinking blocks -> top-level reasoning items with summary_text
// - cache_control and Anthropic-only metadata -> dropped
//
// OpenAI Responses stream -> Anthropic stream mapping:
// - response.created -> message_start
// - response.content_part.added/output_text.delta -> text block start/delta
// - response.output_item.added(function_call) -> tool_use block start
// - response.function_call_arguments.delta -> input_json_delta
// - response.output_item.added(reasoning)/reasoning_*_delta -> thinking block
// - response.output_item.done -> assistant block + content_block_stop
// - response.completed -> message_delta + message_stop
export async function* queryOpenAIWithStreaming(
  params: QueryParams,
): QueryResult {
  try {
    yield* streamWithRetries(params)
  } catch (error) {
    yield createAssistantAPIErrorMessage({
      content: `API Error: ${getErrorMessage(error)}`,
      apiError: 'openai_error',
      error: 'unknown',
      errorDetails: getErrorMessage(error),
    })
  }
}

async function* streamWithRetries(params: QueryParams): AsyncGenerator<
  StreamEvent | AssistantMessage | SystemAPIErrorMessage,
  void
> {
  const maxRetries = getOpenAIMaxRetries()
  let attempt = 0

  while (true) {
    try {
      yield* streamOpenAI(params)
      return
    } catch (error) {
      if (
        params.signal.aborted ||
        !isRetryableOpenAIError(error) ||
        attempt >= maxRetries
      ) {
        throw error
      }
      const delayMs = getRetryDelayMs(error, attempt)
      logForDebugging(
        `[OpenAI] retrying request after ${delayMs}ms: ${getErrorMessage(error)}`,
      )
      await sleep(delayMs, params.signal)
      attempt++
    }
  }
}

async function* streamOpenAI({
  messages,
  systemPrompt,
  thinkingConfig,
  tools,
  signal,
  options,
}: QueryParams): AsyncGenerator<
  StreamEvent | AssistantMessage | SystemAPIErrorMessage,
  void
> {
  const client = await getOpenAIClient(options.fetchOverride)
  const model = normalizeOpenAIModel(options.model)
  const instructions = extractSystemPrompt(systemPrompt)
  const input = buildResponsesInput(messages, tools)
  const apiTools = await buildResponsesTools(tools, options)
  const reasoning = buildReasoningConfig(
    model,
    options.effortValue,
    thinkingConfig,
  )
  const startedAt = Date.now()
  const newMessages: AssistantMessage[] = []
  const activeBlocks = new Map<string, ActiveBlock>()
  let messageId = `resp_${randomUUID().replaceAll('-', '')}`
  let usage = { ...EMPTY_USAGE }
  let stopReason: string | null = null
  let sawMessageStart = false
  let ttftMs: number | undefined
  let nextContentIndex = 0

  // Codex backend rejects max_output_tokens and requires the system prompt
  // at the top-level `instructions` field rather than in input[]. Detect by
  // base URL so the standard api.openai.com path keeps full behaviour.
  const isCodexBackend = client.baseURL.includes(
    'chatgpt.com/backend-api/codex',
  )

  const request: ResponseCreateParamsStreaming = {
    model,
    input,
    stream: true,
    store: false,
    // Codex backend rejects `stream_options` ("Unsupported parameter"
    // HTTP 400). Standard api.openai.com/v1/responses accepts it for
    // the obfuscation toggle.
    ...(!isCodexBackend && {
      stream_options: { include_obfuscation: false },
    }),
    ...(instructions && { instructions }),
    ...(apiTools.length > 0 && { tools: apiTools }),
    ...(apiTools.length > 0 && {
      tool_choice: mapToolChoice(options.toolChoice),
    }),
    ...(reasoning && { reasoning }),
    ...(!isCodexBackend && options.maxOutputTokensOverride && {
      max_output_tokens: options.maxOutputTokensOverride,
    }),
    ...(options.temperatureOverride !== undefined && {
      temperature: options.temperatureOverride,
    }),
  }

  const stream = await client.responses.create(request, { signal })

  for await (const event of stream) {
    switch (event.type) {
      case 'response.created': {
        messageId = event.response.id || messageId
        usage = toAnthropicUsage(event.response.usage)
        sawMessageStart = true
        ttftMs = Date.now() - startedAt
        yield streamEvent(
          {
            type: 'message_start',
            message: buildPartialMessage({
              id: messageId,
              model,
              usage,
              stopReason: null,
            }),
          },
          ttftMs,
        )
        break
      }

      case 'response.output_item.added': {
        if (!sawMessageStart) {
          sawMessageStart = true
          ttftMs = Date.now() - startedAt
          yield streamEvent(
            {
              type: 'message_start',
              message: buildPartialMessage({
                id: messageId,
                model,
                usage,
                stopReason: null,
              }),
            },
            ttftMs,
          )
        }
        const item = event.item as AnyRecord
        if (item.type === 'function_call') {
          const block = ensureToolBlock(
            activeBlocks,
            String(event.output_index),
            event.output_index,
            item,
          )
          nextContentIndex = Math.max(nextContentIndex, block.index + 1)
          yield streamEvent({
            type: 'content_block_start',
            index: block.index,
            content_block: {
              type: 'tool_use',
              id: block.id,
              name: block.name,
              input: {},
            },
          })
        } else if (item.type === 'reasoning') {
          const block = ensureThinkingBlock(
            activeBlocks,
            String(event.output_index),
            nextContentIndex++,
          )
          const summary = reasoningItemToText(item)
          if (summary) block.thinking = summary
          yield streamEvent({
            type: 'content_block_start',
            index: block.index,
            content_block: {
              type: 'thinking',
              thinking: '',
              signature: '',
            },
          })
        }
        break
      }

      case 'response.content_part.added': {
        const part = event.part as AnyRecord
        const key = contentPartKey(event.output_index, event.content_index)
        if (part.type === 'output_text' || part.type === 'refusal') {
          const block = ensureTextBlock(
            activeBlocks,
            key,
            nextContentIndex++,
          )
          block.text = part.text ?? block.text
          yield streamEvent({
            type: 'content_block_start',
            index: block.index,
            content_block: { type: 'text', text: '' },
          })
        } else if (part.type === 'reasoning_text') {
          const block = ensureThinkingBlock(
            activeBlocks,
            key,
            nextContentIndex++,
          )
          block.thinking = part.text ?? block.thinking
          yield streamEvent({
            type: 'content_block_start',
            index: block.index,
            content_block: {
              type: 'thinking',
              thinking: '',
              signature: '',
            },
          })
        }
        break
      }

      case 'response.output_text.delta': {
        const key = contentPartKey(event.output_index, event.content_index)
        const block = ensureTextBlock(activeBlocks, key, nextContentIndex++)
        block.text += event.delta
        yield streamEvent({
          type: 'content_block_delta',
          index: block.index,
          delta: { type: 'text_delta', text: event.delta },
        })
        break
      }

      case 'response.reasoning_summary_text.delta':
      case 'response.reasoning_text.delta': {
        const key = String(event.output_index)
        const block = ensureThinkingBlock(
          activeBlocks,
          key,
          nextContentIndex++,
        )
        block.thinking += event.delta
        yield streamEvent({
          type: 'content_block_delta',
          index: block.index,
          delta: { type: 'thinking_delta', thinking: event.delta },
        })
        break
      }

      case 'response.function_call_arguments.delta': {
        const block = ensureToolBlock(
          activeBlocks,
          String(event.output_index),
          event.output_index,
          {},
        )
        block.input += event.delta
        nextContentIndex = Math.max(nextContentIndex, block.index + 1)
        yield streamEvent({
          type: 'content_block_delta',
          index: block.index,
          delta: {
            type: 'input_json_delta',
            partial_json: event.delta,
          },
        })
        break
      }

      case 'response.output_text.done': {
        const key = contentPartKey(event.output_index, event.content_index)
        const block = ensureTextBlock(activeBlocks, key, nextContentIndex++)
        block.text = event.text
        break
      }

      case 'response.reasoning_summary_text.done':
      case 'response.reasoning_text.done': {
        const block = ensureThinkingBlock(
          activeBlocks,
          String(event.output_index),
          nextContentIndex++,
        )
        block.thinking = event.text
        break
      }

      case 'response.function_call_arguments.done': {
        const block = ensureToolBlock(
          activeBlocks,
          String(event.output_index),
          event.output_index,
          { name: event.name },
        )
        block.input = event.arguments
        break
      }

      case 'response.output_item.done': {
        const item = event.item as AnyRecord
        if (item.type === 'function_call') {
          const block = ensureToolBlock(
            activeBlocks,
            String(event.output_index),
            event.output_index,
            item,
          )
          block.input = item.arguments ?? block.input
          yield* finalizeBlock(block, {
            messageId,
            model,
            usage,
            stopReason,
            tools,
            newMessages,
          })
        } else if (item.type === 'reasoning') {
          const block = ensureThinkingBlock(
            activeBlocks,
            String(event.output_index),
            nextContentIndex++,
          )
          const summary = reasoningItemToText(item)
          if (summary) block.thinking = summary
          yield* finalizeBlock(block, {
            messageId,
            model,
            usage,
            stopReason,
            tools,
            newMessages,
          })
        } else if (item.type === 'message') {
          const blocks = blocksForOutputIndex(activeBlocks, event.output_index)
          if (blocks.length === 0) {
            for (const block of messageItemToBlocks(
              item,
              event.output_index,
              () => nextContentIndex++,
            )) {
              activeBlocks.set(block.key, block)
              blocks.push(block)
            }
          }
          for (const block of blocks.sort((a, b) => a.index - b.index)) {
            yield* finalizeBlock(block, {
              messageId,
              model,
              usage,
              stopReason,
              tools,
              newMessages,
            })
          }
        }
        break
      }

      case 'response.completed': {
        usage = toAnthropicUsage(event.response.usage)
        stopReason = mapResponseStatus(event.response)
        for (const block of [...activeBlocks.values()].sort(
          (a, b) => a.index - b.index,
        )) {
          yield* finalizeBlock(block, {
            messageId,
            model,
            usage,
            stopReason,
            tools,
            newMessages,
          })
        }
        for (const msg of newMessages) {
          msg.message.usage = usage as any
          msg.message.stop_reason = stopReason
        }
        yield streamEvent({
          type: 'message_delta',
          delta: { stop_reason: stopReason, stop_sequence: null },
          usage: usage as any,
        })
        yield streamEvent({ type: 'message_stop' })
        break
      }

      case 'response.incomplete':
      case 'response.failed': {
        usage = toAnthropicUsage(event.response.usage)
        stopReason = mapResponseStatus(event.response)
        break
      }

      case 'error': {
        yield createAssistantAPIErrorMessage({
          content: `API Error: ${event.message}`,
          apiError: 'openai_error',
          error: 'unknown',
          errorDetails: event.message,
        })
        break
      }
    }
  }
}

async function getOpenAIClient(
  fetchOverride: QueryParams['options']['fetchOverride'],
) {
  // Auth resolution order:
  //   1. OPENAI_API_KEY env var — explicit pay-per-token. Wins if set.
  //   2. ~/.codex/auth.json — ChatGPT Plus/Pro subscription via Codex backend.
  //      Bootstrapped on first use into ~/.somersault/codex-auth.json so our
  //      refresh cadence doesn't fight the official `codex` CLI.
  //   3. Fall through with no key — caller sees an auth error.

  if (process.env.OPENAI_API_KEY) {
    return new OpenAI({
      apiKey: process.env.OPENAI_API_KEY,
      baseURL: process.env.OPENAI_BASE_URL || DEFAULT_OPENAI_BASE_URL,
      ...(fetchOverride && { fetch: fetchOverride }),
    })
  }

  const codex = await getCodexCredential()
  if (codex) {
    return new OpenAI({
      apiKey: codex.accessToken, // SDK sets `Authorization: Bearer <key>`
      baseURL: codex.baseURL, // 'https://chatgpt.com/backend-api/codex'
      ...(fetchOverride && { fetch: fetchOverride }),
      defaultHeaders: {
        // Identify ourselves as the Codex CLI (matches official client
        // behaviour). Some Codex backend code paths gate on originator.
        originator: 'codex_cli_rs',
        // Tells the backend which ChatGPT account to bill against. Some
        // installs have multiple linked accounts; this disambiguates.
        ...(codex.accountId && { 'chatgpt-account-id': codex.accountId }),
      },
    })
  }

  // No credentials available. Let the SDK surface the auth error so the
  // failure mode matches stock OpenAI behaviour.
  return new OpenAI({
    apiKey: undefined,
    baseURL: process.env.OPENAI_BASE_URL || DEFAULT_OPENAI_BASE_URL,
    ...(fetchOverride && { fetch: fetchOverride }),
  })
}

async function buildResponsesTools(
  tools: Tools,
  options: QueryParams['options'],
): Promise<FunctionTool[]> {
  const schemas = await Promise.all(
    tools.map(tool =>
      toolToAPISchema(tool, {
        getToolPermissionContext: options.getToolPermissionContext,
        tools,
        agents: options.agents,
        allowedAgentTypes: options.allowedAgentTypes,
        model: options.model,
      }),
    ),
  )

  return schemas
    .filter(
      schema => schema && typeof schema === 'object' && 'input_schema' in schema,
    )
    .map(schema => {
      const s = schema as AnyRecord
      return {
        type: 'function',
        name: String(s.name),
        description: String(s.description ?? ''),
        parameters: (s.input_schema ?? { type: 'object' }) as AnyRecord,
        strict: s.strict === true,
      }
    })
}

function extractSystemPrompt(
  systemPrompt: QueryParams['systemPrompt'],
): string {
  const text = Array.isArray(systemPrompt)
    ? systemPrompt.filter(Boolean).join('\n\n')
    : String(systemPrompt ?? '')
  return text.trim()
}

function buildResponsesInput(
  messages: Message[],
  tools: Tools,
): ResponseInputItem[] {
  // System prompt is hoisted to the top-level `instructions` field on the
  // request; it is NOT included as an input[] item. The Codex backend
  // (chatgpt.com/backend-api/codex) rejects requests with system content
  // in input[], and api.openai.com/v1/responses treats `instructions` as
  // the canonical place for it anyway.
  const input: ResponseInputItem[] = []
  const normalized = ensureToolResultPairing(
    normalizeMessagesForAPI(messages, tools),
  )
  for (const message of normalized) {
    input.push(...messageToResponsesInput(message))
  }
  return input
}

function messageToResponsesInput(
  message: UserMessage | AssistantMessage,
): ResponseInputItem[] {
  if (message.type === 'assistant') {
    return assistantMessageToResponsesInput(message)
  }
  return userMessageToResponsesInput(message)
}

function assistantMessageToResponsesInput(
  message: AssistantMessage,
): ResponseInputItem[] {
  const input: ResponseInputItem[] = []
  for (const block of message.message.content) {
    switch (block.type) {
      case 'text':
        input.push({
          type: 'message',
          role: 'assistant',
          content: [
            {
              type: 'output_text',
              text: block.text ?? '',
              annotations: [],
            },
          ],
        } as unknown as ResponseInputItem)
        break
      case 'tool_use':
        input.push({
          type: 'function_call',
          call_id: block.id,
          name: block.name,
          arguments:
            typeof block.input === 'string'
              ? block.input
              : JSON.stringify(block.input ?? {}),
          status: 'completed',
        } as ResponseInputItem)
        break
      case 'thinking':
        input.push({
          type: 'reasoning',
          id: `rs_${randomUUID().replaceAll('-', '')}`,
          summary: [
            {
              type: 'summary_text',
              text: block.thinking ?? '',
            },
          ],
          status: 'completed',
        } as ResponseInputItem)
        break
      case 'redacted_thinking':
        break
      default:
        input.push({
          type: 'message',
          role: 'assistant',
          content: [
            {
              type: 'output_text',
              text: blockToText(block),
              annotations: [],
            },
          ],
        } as unknown as ResponseInputItem)
    }
  }
  return input
}

function userMessageToResponsesInput(message: UserMessage): ResponseInputItem[] {
  const content = message.message.content
  if (typeof content === 'string') {
    return [
      {
        type: 'message',
        role: 'user',
        content: [{ type: 'input_text', text: content }],
      } as ResponseInputItem,
    ]
  }

  const input: ResponseInputItem[] = []
  let contentParts: AnyRecord[] = []
  const flushContentParts = () => {
    if (contentParts.length === 0) return
    input.push({
      type: 'message',
      role: 'user',
      content: contentParts,
    } as ResponseInputItem)
    contentParts = []
  }

  for (const block of content) {
    if (block.type === 'tool_result') {
      flushContentParts()
      input.push({
        type: 'function_call_output',
        call_id: block.tool_use_id,
        output: toolResultToString(block.content),
      } as ResponseInputItem)
      continue
    }

    const part = userBlockToResponsesInput(block)
    if (part) contentParts.push(part)
  }
  flushContentParts()
  return input
}

function userBlockToResponsesInput(block: AnyRecord): AnyRecord | null {
  switch (block.type) {
    case 'text':
      return { type: 'input_text', text: block.text ?? '' }
    case 'image': {
      const url = imageBlockToURL(block)
      return url ? { type: 'input_image', image_url: url } : null
    }
    case 'document':
    case 'search_result':
    case 'tool_reference':
      return { type: 'input_text', text: blockToText(block) }
    default:
      return { type: 'input_text', text: blockToText(block) }
  }
}

function imageBlockToURL(block: AnyRecord): string | null {
  const source = block.source
  if (!source || typeof source !== 'object') return null
  if (source.type === 'url' && typeof source.url === 'string') {
    return source.url
  }
  if (
    source.type === 'base64' &&
    typeof source.media_type === 'string' &&
    typeof source.data === 'string'
  ) {
    return `data:${source.media_type};base64,${source.data}`
  }
  return null
}

function toolResultToString(content: unknown): string {
  if (typeof content === 'string') return content
  if (Array.isArray(content)) {
    return content
      .map(part => {
        if (part && typeof part === 'object' && 'text' in part) {
          return String((part as AnyRecord).text ?? '')
        }
        return blockToText(part)
      })
      .join('\n')
  }
  return blockToText(content)
}

function blockToText(block: unknown): string {
  if (typeof block === 'string') return block
  if (!block || typeof block !== 'object') return String(block ?? '')
  const b = block as AnyRecord
  if (typeof b.text === 'string') return b.text
  return JSON.stringify(b)
}

function buildReasoningConfig(
  model: string,
  effortValue: QueryParams['options']['effortValue'],
  thinkingConfig: ThinkingConfig,
): { effort?: ReasoningEffort; summary?: 'auto' } | undefined {
  if (!isOpenAIReasoningModel(model)) return undefined
  const envOverride = getEffortEnvOverride()
  const resolved = envOverride === null ? undefined : envOverride ?? effortValue
  const effort = mapReasoningEffort(
    model,
    resolved,
  )
  if (!effort && thinkingConfig.type === 'disabled') return undefined
  return {
    ...(effort && { effort }),
    ...(thinkingConfig.type !== 'disabled' && { summary: 'auto' as const }),
  }
}

function mapReasoningEffort(
  model: string,
  effortValue: unknown,
): ReasoningEffort | undefined {
  if (effortValue === undefined || effortValue === null) return undefined
  if (typeof effortValue === 'number') {
    if (effortValue <= 50) return 'low'
    if (effortValue <= 85) return 'medium'
    if (effortValue <= 100) return 'high'
    return supportsXHighReasoning(model) ? 'xhigh' : 'high'
  }
  const value = String(effortValue).toLowerCase()
  if (value === 'low' || value === 'medium' || value === 'high') return value
  if (value === 'max' || value === 'xhigh') {
    return supportsXHighReasoning(model) ? 'xhigh' : 'high'
  }
  return undefined
}

function isOpenAIReasoningModel(model: string): boolean {
  const m = model.toLowerCase()
  return /^o\d/.test(m) || m.startsWith('gpt-5')
}

function supportsXHighReasoning(model: string): boolean {
  const m = model.toLowerCase()
  if (m.includes('codex-max')) return true
  const dotted = /^gpt-5\.(\d+)/.exec(m)
  if (dotted) return Number.parseInt(dotted[1]!, 10) > 1
  return m.startsWith('gpt-5.5') || m.startsWith('gpt-5.4')
}

function mapToolChoice(
  toolChoice: QueryParams['options']['toolChoice'],
): ResponseCreateParamsStreaming['tool_choice'] {
  if (!toolChoice) return 'auto'
  const type = (toolChoice as AnyRecord).type
  if (type === 'auto') return 'auto'
  if (type === 'any') return 'required'
  if (type === 'tool' && 'name' in toolChoice) {
    return {
      type: 'function',
      name: toolChoice.name,
    } as ResponseCreateParamsStreaming['tool_choice']
  }
  return 'auto'
}

function ensureTextBlock(
  activeBlocks: Map<string, ActiveBlock>,
  key: string,
  index: number,
) {
  const existing = activeBlocks.get(key)
  if (existing?.type === 'text') return existing
  const block: ActiveBlock = {
    type: 'text',
    key,
    index,
    text: '',
    stopped: false,
  }
  activeBlocks.set(key, block)
  return block
}

function ensureThinkingBlock(
  activeBlocks: Map<string, ActiveBlock>,
  key: string,
  index: number,
) {
  const existing = activeBlocks.get(key)
  if (existing?.type === 'thinking') return existing
  const block: ActiveBlock = {
    type: 'thinking',
    key,
    index,
    thinking: '',
    stopped: false,
  }
  activeBlocks.set(key, block)
  return block
}

function ensureToolBlock(
  activeBlocks: Map<string, ActiveBlock>,
  key: string,
  index: number,
  item: AnyRecord,
) {
  const existing = activeBlocks.get(key)
  if (existing?.type === 'tool_use') {
    if (item.call_id) existing.id = item.call_id
    if (item.name) existing.name = item.name
    return existing
  }
  const block: ActiveBlock = {
    type: 'tool_use',
    key,
    index,
    id: item.call_id ?? item.id ?? `call_${randomUUID().replaceAll('-', '')}`,
    name: item.name ?? 'unknown_tool',
    input: item.arguments ?? '',
    stopped: false,
  }
  activeBlocks.set(key, block)
  return block
}

function blocksForOutputIndex(
  activeBlocks: Map<string, ActiveBlock>,
  outputIndex: number,
): ActiveBlock[] {
  const prefix = `${outputIndex}:`
  return [...activeBlocks.values()].filter(
    block => block.key === String(outputIndex) || block.key.startsWith(prefix),
  )
}

function messageItemToBlocks(
  item: AnyRecord,
  outputIndex: number,
  nextIndex: () => number,
): ActiveBlock[] {
  const blocks: ActiveBlock[] = []
  for (const [contentIndex, part] of (item.content ?? []).entries()) {
    const key = contentPartKey(outputIndex, contentIndex)
    if (part.type === 'output_text' || part.type === 'refusal') {
      blocks.push({
        type: 'text',
        key,
        index: nextIndex(),
        text: part.text ?? '',
        stopped: false,
      })
    } else if (part.type === 'reasoning_text') {
      blocks.push({
        type: 'thinking',
        key,
        index: nextIndex(),
        thinking: part.text ?? '',
        stopped: false,
      })
    }
  }
  return blocks
}

function* finalizeBlock(
  block: ActiveBlock,
  {
    messageId,
    model,
    usage,
    stopReason,
    tools,
    newMessages,
  }: {
    messageId: string
    model: string
    usage: AnyRecord
    stopReason: string | null
    tools: Tools
    newMessages: AssistantMessage[]
  },
): Generator<StreamEvent | AssistantMessage, void> {
  if (block.stopped) return
  block.stopped = true
  const assistant = buildAssistantMessage({
    id: messageId,
    model,
    contentBlock: toAnthropicContentBlock(block),
    usage,
    stopReason,
    tools,
  })
  newMessages.push(assistant)
  yield assistant
  yield streamEvent({ type: 'content_block_stop', index: block.index })
}

function toAnthropicContentBlock(block: ActiveBlock): BetaContentBlock {
  if (block.type === 'text') {
    return { type: 'text', text: block.text } as BetaContentBlock
  }
  if (block.type === 'thinking') {
    return {
      type: 'thinking',
      thinking: block.thinking,
      signature: '',
    } as unknown as BetaContentBlock
  }
  return {
    type: 'tool_use',
    id: block.id,
    name: block.name,
    input: block.input,
  } as unknown as BetaContentBlock
}

function buildPartialMessage({
  id,
  model,
  usage,
  stopReason,
}: {
  id: string
  model: string
  usage: AnyRecord
  stopReason: string | null
}) {
  return {
    id,
    type: 'message' as const,
    role: 'assistant' as const,
    model,
    content: [],
    stop_reason: stopReason,
    stop_sequence: null,
    usage: usage as any,
    context_management: null,
  }
}

function buildAssistantMessage({
  id,
  model,
  contentBlock,
  usage,
  stopReason,
  tools,
}: {
  id: string
  model: string
  contentBlock: BetaContentBlock
  usage: AnyRecord
  stopReason: string | null
  tools: Tools
}): AssistantMessage {
  return {
    type: 'assistant',
    uuid: randomUUID(),
    timestamp: new Date().toISOString(),
    message: {
      ...buildPartialMessage({ id, model, usage, stopReason }),
      content: normalizeContentFromAPI([contentBlock], tools, undefined),
    },
  }
}

function streamEvent(event: AnyRecord & { type: string }, ttftMs?: number): StreamEvent {
  return {
    type: 'stream_event',
    event,
    ...(ttftMs !== undefined && { ttftMs }),
  }
}

function toAnthropicUsage(usage: OpenAIUsage): AnyRecord {
  return {
    ...EMPTY_USAGE,
    input_tokens: usage?.input_tokens ?? 0,
    output_tokens: usage?.output_tokens ?? 0,
    cache_read_input_tokens: usage?.input_tokens_details?.cached_tokens ?? 0,
    reasoning_output_tokens:
      usage?.output_tokens_details?.reasoning_tokens ?? 0,
  }
}

function mapResponseStatus(response: AnyRecord): string | null {
  if (response.status === 'completed') return 'end_turn'
  if (response.status === 'incomplete') {
    const reason = response.incomplete_details?.reason
    if (reason === 'max_output_tokens') return 'max_tokens'
    if (reason === 'content_filter') return 'stop_sequence'
    return 'max_tokens'
  }
  if (response.status === 'failed') return 'stop_sequence'
  return null
}

function reasoningItemToText(item: AnyRecord): string {
  const summary = Array.isArray(item.summary)
    ? item.summary
        .map((part: AnyRecord) => part?.text)
        .filter((text: unknown): text is string => typeof text === 'string')
        .join('')
    : ''
  if (summary) return summary
  return Array.isArray(item.content)
    ? item.content
        .map((part: AnyRecord) => part?.text)
        .filter((text: unknown): text is string => typeof text === 'string')
        .join('')
    : ''
}

function contentPartKey(outputIndex: number, contentIndex: number): string {
  return `${outputIndex}:${contentIndex}`
}

function normalizeOpenAIModel(model: string): string {
  return model.startsWith('openai/') ? model.slice('openai/'.length) : model
}

function getOpenAIMaxRetries(): number {
  const raw = process.env.OPENAI_MAX_RETRIES
  if (!raw) return 2
  const parsed = Number.parseInt(raw, 10)
  return Number.isFinite(parsed) && parsed >= 0 ? parsed : 2
}

function isRetryableOpenAIError(error: unknown): boolean {
  const status = getErrorStatus(error)
  if (status === 408 || status === 409 || status === 429) return true
  if (status !== undefined && status >= 500) return true
  const type = (error as AnyRecord)?.error?.type ?? (error as AnyRecord)?.type
  return type === 'rate_limit_error' || type === 'server_error'
}

function getRetryDelayMs(error: unknown, attempt: number): number {
  const retryAfter = (error as AnyRecord)?.headers?.['retry-after']
  const parsed = retryAfter ? Number.parseFloat(String(retryAfter)) : NaN
  if (Number.isFinite(parsed) && parsed > 0) return parsed * 1000
  return Math.min(32000, 500 * 2 ** attempt)
}

function getErrorStatus(error: unknown): number | undefined {
  const status = (error as AnyRecord)?.status
  return typeof status === 'number' ? status : undefined
}

function getErrorMessage(error: unknown): string {
  if (error instanceof Error) return error.message
  return String(error)
}

async function sleep(ms: number, signal: AbortSignal): Promise<void> {
  if (ms <= 0) return
  await new Promise<void>((resolve, reject) => {
    const timeout = setTimeout(resolve, ms)
    const onAbort = () => {
      clearTimeout(timeout)
      reject(new Error('OpenAI request aborted'))
    }
    signal.addEventListener('abort', onAbort, { once: true })
  })
}
