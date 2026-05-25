# Phase 9.6c — Spec 34 Inversion #7 Wording Correction

**Trigger**: Phase 9.5b spec 33 reviewer flagged Phase 9.6 spec 34 fix wording as OVERSTATED.

## The original Phase 9.6 (B-full) wording (incorrect)

§5.8 + §9.4 said: "Daemon callers without refresh handler get IMMEDIATE BridgeFatalError, not 'retry once'."

## Code reality (verified)

`src/bridge/bridgeApi.ts:117-120`:

```ts
if (!deps.onAuth401) {
  debug(`[bridge:api] ${context}: 401 received, no refresh handler`)
  return response
}
```

`withOAuthRetry` *returns* the 401 response. The fatal throw happens DOWNSTREAM in `handleErrorStatus` (per the doc-comment on `:104`: "the 401 response is returned for handleErrorStatus to throw BridgeFatalError"). It is NOT thrown at the `withOAuthRetry` control point.

## Behavioral equivalence vs throw-site difference

- **Behavioral outcome**: identical (fatal + no retry attempt) — the 9.6 fix's high-level claim still holds for daemon callers.
- **Throw site**: differs — matters for stack traces, exception filtering, and any `try/catch` wrapped narrowly around `withOAuthRetry` (which would NOT catch the fatal under the corrected description).

## Correction applied

§5.8 "Daemon-mode caveat" + §9.4 third bullet: replaced "immediate `BridgeFatalError(401)` on the first 401" with the precise return-then-downstream-throw description, citing `bridgeApi.ts:117-120` (return) and `:104` doc-comment (downstream throw site).

§5.8 footer cross-refs `PHASE9-FIXES-34.md` (Phase 9.6 B-full original fix log) and `PHASE9-FIXES-34b.md` (this file).

## Files touched

- `docs/specs/34-mode-bridge.md` — §5.8, §9.4
- `docs/specs/PHASE9-FIXES-34b.md` — this fix log (new)

## Verification

Re-read `bridgeApi.ts:95-134` confirms `:117-120` is `return response` (not `throw`) when `deps.onAuth401` is undefined, and the doc-comment on `:99-105` explicitly identifies `handleErrorStatus` as the throw site.
