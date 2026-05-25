import { beforeEach, expect, test } from 'bun:test'
import type { Options } from './claude.js'
import { queryOpenAIWithStreaming } from './openai.js'
import type { Message } from '../../types/message.js'
import { asSystemPrompt } from '../../utils/systemPromptType.js'
import { getEmptyToolPermissionContext } from '../../Tool.js'

type CapturedRequest = {
  url: string
  body: any
}

const userMessage: Message = {
  type: 'user',
  uuid: 'user-1',
  timestamp: '2026-05-11T00:00:00.000Z',
  message: {
    role: 'user',
    content: [{ type: 'text', text: 'say hi' }],
  },
}

beforeEach(() => {
  process.env.OPENAI_API_KEY = 'sk-test'
})

test('streams text through the Responses API and sends OpenAI reasoning effort', async () => {
  const captured: CapturedRequest[] = []
  const fetchOverride = makeFetch(captured, [
    responseEvent('response.created', {
      type: 'response.created',
      response: responseShell('resp_text', 'gpt-5.5'),
      sequence_number: 0,
    }),
    responseEvent('response.output_item.added', {
      type: 'response.output_item.added',
      output_index: 0,
      item: {
        id: 'msg_text',
        type: 'message',
        role: 'assistant',
        status: 'in_progress',
        content: [],
      },
      sequence_number: 1,
    }),
    responseEvent('response.content_part.added', {
      type: 'response.content_part.added',
      output_index: 0,
      content_index: 0,
      item_id: 'msg_text',
      part: { type: 'output_text', text: '', annotations: [] },
      sequence_number: 2,
    }),
    responseEvent('response.output_text.delta', {
      type: 'response.output_text.delta',
      output_index: 0,
      content_index: 0,
      item_id: 'msg_text',
      delta: 'Hello',
      sequence_number: 3,
    }),
    responseEvent('response.output_text.done', {
      type: 'response.output_text.done',
      output_index: 0,
      content_index: 0,
      item_id: 'msg_text',
      text: 'Hello',
      sequence_number: 4,
    }),
    responseEvent('response.output_item.done', {
      type: 'response.output_item.done',
      output_index: 0,
      item: {
        id: 'msg_text',
        type: 'message',
        role: 'assistant',
        status: 'completed',
        content: [{ type: 'output_text', text: 'Hello', annotations: [] }],
      },
      sequence_number: 5,
    }),
    responseEvent('response.completed', {
      type: 'response.completed',
      response: {
        ...responseShell('resp_text', 'gpt-5.5'),
        status: 'completed',
        output: [],
        usage: {
          input_tokens: 10,
          input_tokens_details: { cached_tokens: 3 },
          output_tokens: 2,
          output_tokens_details: { reasoning_tokens: 1 },
          total_tokens: 12,
        },
      },
      sequence_number: 6,
    }),
  ])

  const events = await collect(
    makeParams(fetchOverride, {
      model: 'openai/gpt-5.5',
      effortValue: 'max',
    }),
  )

  expect(captured).toHaveLength(1)
  expect(captured[0]!.url).toContain('/responses')
  expect(captured[0]!.body).toMatchObject({
    model: 'gpt-5.5',
    stream: true,
    reasoning: { effort: 'xhigh' },
  })
  expect(captured[0]!.body.input[0]).toMatchObject({
    role: 'system',
    content: [{ type: 'input_text', text: 'You are terse.' }],
  })
  expect(captured[0]!.body.input[1]).toMatchObject({
    role: 'user',
    content: [{ type: 'input_text', text: 'say hi' }],
  })

  const textDelta = events.find(
    event =>
      event.type === 'stream_event' &&
      event.event.type === 'content_block_delta',
  )
  expect(textDelta?.event.delta).toEqual({
    type: 'text_delta',
    text: 'Hello',
  })

  const assistant = events.find(event => event.type === 'assistant')
  expect(assistant?.message.content).toEqual([{ type: 'text', text: 'Hello' }])
  expect(assistant?.message.usage.input_tokens).toBe(10)
  expect(assistant?.message.usage.cache_read_input_tokens).toBe(3)
  expect(assistant?.message.usage.output_tokens).toBe(2)
})

test('streams Responses function calls as Anthropic tool_use blocks', async () => {
  const captured: CapturedRequest[] = []
  const fetchOverride = makeFetch(captured, [
    responseEvent('response.created', {
      type: 'response.created',
      response: responseShell('resp_tool', 'gpt-5.5'),
      sequence_number: 0,
    }),
    responseEvent('response.output_item.added', {
      type: 'response.output_item.added',
      output_index: 0,
      item: {
        id: 'fc_1',
        type: 'function_call',
        call_id: 'call_1',
        name: 'Read',
        arguments: '',
        status: 'in_progress',
      },
      sequence_number: 1,
    }),
    responseEvent('response.function_call_arguments.delta', {
      type: 'response.function_call_arguments.delta',
      output_index: 0,
      item_id: 'fc_1',
      delta: '{"file_path"',
      sequence_number: 2,
    }),
    responseEvent('response.function_call_arguments.delta', {
      type: 'response.function_call_arguments.delta',
      output_index: 0,
      item_id: 'fc_1',
      delta: ':"README.md"}',
      sequence_number: 3,
    }),
    responseEvent('response.output_item.done', {
      type: 'response.output_item.done',
      output_index: 0,
      item: {
        id: 'fc_1',
        type: 'function_call',
        call_id: 'call_1',
        name: 'Read',
        arguments: '{"file_path":"README.md"}',
        status: 'completed',
      },
      sequence_number: 4,
    }),
    responseEvent('response.completed', {
      type: 'response.completed',
      response: {
        ...responseShell('resp_tool', 'gpt-5.5'),
        status: 'completed',
        output: [],
        usage: {
          input_tokens: 8,
          output_tokens: 4,
          output_tokens_details: { reasoning_tokens: 0 },
          total_tokens: 12,
        },
      },
      sequence_number: 5,
    }),
  ])

  const events = await collect(makeParams(fetchOverride))

  const start = events.find(
    event =>
      event.type === 'stream_event' &&
      event.event.type === 'content_block_start',
  )
  expect(start?.event.content_block).toEqual({
    type: 'tool_use',
    id: 'call_1',
    name: 'Read',
    input: {},
  })

  const deltas = events
    .filter(
      event =>
        event.type === 'stream_event' &&
        event.event.type === 'content_block_delta',
    )
    .map(event => event.event.delta.partial_json)
  expect(deltas).toEqual(['{"file_path"', ':"README.md"}'])

  const assistant = events.find(event => event.type === 'assistant')
  expect(assistant?.message.content).toEqual([
    {
      type: 'tool_use',
      id: 'call_1',
      name: 'Read',
      input: { file_path: 'README.md' },
    },
  ])
})

async function collect(
  params: Parameters<typeof queryOpenAIWithStreaming>[0],
) {
  const events = []
  for await (const event of queryOpenAIWithStreaming(params)) {
    events.push(event)
  }
  return events
}

function makeParams(
  fetchOverride: Options['fetchOverride'],
  overrides: Partial<Options> = {},
): Parameters<typeof queryOpenAIWithStreaming>[0] {
  const controller = new AbortController()
  return {
    messages: [userMessage],
    systemPrompt: asSystemPrompt(['You are terse.']),
    thinkingConfig: { type: 'disabled' },
    tools: [],
    signal: controller.signal,
    options: {
      getToolPermissionContext: async () => getEmptyToolPermissionContext(),
      model: 'openai/gpt-5.5',
      isNonInteractiveSession: true,
      querySource: 'sdk' as any,
      agents: [],
      hasAppendSystemPrompt: false,
      mcpTools: [],
      fetchOverride,
      ...overrides,
    },
  }
}

function makeFetch(
  captured: CapturedRequest[],
  events: string[],
): Options['fetchOverride'] {
  return async (input, init) => {
    const url = input instanceof Request ? input.url : String(input)
    const bodyText =
      typeof init?.body === 'string'
        ? init.body
        : input instanceof Request
          ? await input.clone().text()
          : ''
    captured.push({ url, body: JSON.parse(bodyText) })
    return new Response(`${events.join('')}\ndata: [DONE]\n\n`, {
      headers: { 'content-type': 'text/event-stream' },
    })
  }
}

function responseEvent(type: string, data: any): string {
  return `event: ${type}\ndata: ${JSON.stringify(data)}\n\n`
}

function responseShell(id: string, model: string): any {
  return {
    id,
    object: 'response',
    created_at: 1,
    status: 'in_progress',
    error: null,
    incomplete_details: null,
    model,
    output: [],
    parallel_tool_calls: true,
    previous_response_id: null,
    reasoning: { effort: null, summary: null },
    store: false,
    text: { format: { type: 'text' } },
    tool_choice: 'auto',
    tools: [],
    truncation: 'disabled',
    usage: null,
    metadata: {},
  }
}
