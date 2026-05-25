// Stub for the missing-from-leak snipCompact module (spec 04 §2.5).
// Gated by feature('HISTORY_SNIP') = false. Runtime require is unreached.

/* eslint-disable @typescript-eslint/no-explicit-any */

export const snipCompactIfNeeded: any = () => ({
  messages: [],
  snipTokensFreed: 0,
  boundaryMessage: null,
})
export const isSnipMarkerMessage: any = () => false
export const isSnipRuntimeEnabled: any = () => false
export const shouldNudgeForSnips: any = () => false
export const SNIP_NUDGE_TEXT =
  'Conversation history snipping is disabled in this build.'
