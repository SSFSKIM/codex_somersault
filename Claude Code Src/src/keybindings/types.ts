export type KeybindingContextName = string
export type KeybindingAction = string

export type ParsedKeystroke = {
  key: string
  ctrl: boolean
  alt: boolean
  shift: boolean
  meta: boolean
  super: boolean
}

export type Chord = ParsedKeystroke[]

export type ParsedBinding = {
  context: KeybindingContextName
  action: KeybindingAction
  chord: Chord
  source?: string
  raw?: string
  [key: string]: any
}

export type KeybindingBlock = {
  context: KeybindingContextName
  bindings: Record<string, KeybindingAction | null | undefined>
  [key: string]: any
}
