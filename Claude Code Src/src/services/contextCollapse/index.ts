// Stub for the missing-from-leak contextCollapse module (spec 04 §2.5).
// Gated by feature('CONTEXT_COLLAPSE') = false. Runtime require unreached.

/* eslint-disable @typescript-eslint/no-explicit-any */

export const applyCollapsesIfNeeded: any = async (
  messages: unknown,
): Promise<unknown> => messages
export const recoverFromOverflow: any = async () => ({ committed: 0 })
export const resetContextCollapse: any = () => {}
export const initContextCollapse: any = () => {}
export const isWithheldPromptTooLong: any = () => false
export const isContextCollapseEnabled: any = () => false
export const subscribe: any = () => () => {}
export const getStats: any = () => ({
  collapsedSpans: 0,
  collapsedMessages: 0,
  stagedSpans: 0,
  health: {
    totalSpawns: 0,
    totalErrors: 0,
    totalEmptySpawns: 0,
    emptySpawnWarningEmitted: false,
  },
})
