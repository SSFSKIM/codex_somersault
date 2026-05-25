// Stub for the missing-from-leak query/transitions module (spec 04 §2.5).
// Spec 04 §3.1 enumerates the observed terminal/continue reasons; this stub
// types them loosely as `any` for now. Phase 2 will author the proper
// discriminated-union types per spec 04 §3.1.

/* eslint-disable @typescript-eslint/no-explicit-any */

export type Terminal = any
export type Continue = any
