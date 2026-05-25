export type Tip = {
  id: string
  text?: string
  content: (context: TipContext) => string | Promise<string>
  cooldownSessions: number
  isRelevant: (context: TipContext) => boolean | Promise<boolean>
  priority?: number
  [key: string]: any
}

export type TipContext = Record<string, any>
