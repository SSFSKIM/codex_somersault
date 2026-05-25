export type QueueOperation = 'enqueue' | 'dequeue' | 'remove' | 'popAll' | string

export type QueueOperationMessage = {
  type: 'queue-operation'
  operation: QueueOperation
  timestamp: string
  sessionId?: string
  content?: string
  [key: string]: any
}
