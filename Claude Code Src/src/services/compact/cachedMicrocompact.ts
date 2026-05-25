// Stub for the missing-from-leak cachedMicrocompact module (spec 04 §2.5).
// Gated by feature('CACHED_MICROCOMPACT') which is false in bundle-shim.ts,
// so the dynamic `await import('./cachedMicrocompact.js')` never executes
// at runtime. This stub exists only for typecheck resolution of
// `typeof import('./cachedMicrocompact.js')` namespace references.

/* eslint-disable @typescript-eslint/no-explicit-any */

export type CachedMCState = any
export type CacheEditsBlock = any
export type PinnedCacheEdits = any

export const consumePendingCacheEdits: any = () => null
export const getPinnedCacheEdits: any = () => []
export const createCachedMCState: any = () => ({})
export const resetCachedMCState: any = () => {}
export const getCachedMCConfig: any = () => null
export const registerToolResult: any = () => {}
export const registerToolMessage: any = () => {}
export const getToolResultsToDelete: any = () => []
export const createCacheEditsBlock: any = () => null
export const cachedMicrocompactPath: any = async () => null
export const isCachedMicrocompactEnabled: any = () => false
export const isModelSupportedForCacheEditing: any = () => false
export const markToolsSentToAPIState: any = () => {}
export const markToolsSentToAPI: any = () => {}
export const CACHE_EDITING_BETA_HEADER = ''
