// Stub for the missing-from-leak reactiveCompact module (spec 04 §2.5).
// Gated by feature('REACTIVE_COMPACT') = false. Runtime require is unreached.

/* eslint-disable @typescript-eslint/no-explicit-any */

export const tryReactiveCompact: any = async () => null
export const isReactiveCompactEnabled: any = () => false
export const isWithheldPromptTooLong: any = () => false
export const isWithheldMediaSizeError: any = () => false
export const isReactiveOnlyMode: any = () => false
export const reactiveCompactOnPromptTooLong: any = async () => null
