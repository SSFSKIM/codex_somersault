export type ConnectorTextBlock = {
  type: 'connector_text' | string
  text?: string
  source?: string
  title?: string
  url?: string
  [key: string]: any
}

export type ConnectorTextDelta = {
  type?: 'connector_text_delta' | string
  text?: string
  [key: string]: any
}

export function isConnectorTextBlock(value: unknown): value is ConnectorTextBlock {
  return (
    typeof value === 'object' &&
    value !== null &&
    'type' in value &&
    (value as { type?: unknown }).type === 'connector_text'
  )
}
