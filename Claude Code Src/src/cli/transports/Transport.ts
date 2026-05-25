import type { StdoutMessage } from 'src/entrypoints/sdk/controlTypes.js'

export type Transport = {
  connect(): Promise<void> | void
  write(message: StdoutMessage): Promise<void> | void
  close(): void
  isConnectedStatus(): boolean
  isClosedStatus(): boolean
  setOnData(callback: (data: string) => void): void
  setOnClose(callback: (closeCode?: number) => void): void
  setOnConnect?(callback: () => void): void
  getStateLabel?(): string
  getLastSequenceNum?(): number
  [key: string]: any
}
