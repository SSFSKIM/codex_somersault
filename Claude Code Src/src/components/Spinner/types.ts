export type SpinnerMode =
  | 'requesting'
  | 'responding'
  | 'thinking'
  | 'tool_use'
  | 'compact'
  | 'stalled'
  | 'idle'
  | string

export type RGBColor = {
  r: number
  g: number
  b: number
}
