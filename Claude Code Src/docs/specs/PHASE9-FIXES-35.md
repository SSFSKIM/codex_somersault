# PHASE 9.6 B-full — Fix log: spec 35 (mode-remote-server)

Reviewer findings source: `docs/specs/PHASE9-ADVERSARIAL-35.md`
Phase 9.6 B-mini already addressed H1 (added `/ultrareview` as 6th sub-mode in §1 + 21d cross-ref). This log covers the remaining 1 HIGH + 4 MEDIUM findings applied in B-full.

## Findings status

| ID | Severity | Status | Notes |
|---|---|---|---|
| H1 | HIGH | DONE in B-mini | `/ultrareview` added as 6th sub-mode + 21d cross-ref. Not redone. |
| H2 | HIGH | **APPLIED (B-full)** | §6.4 split into A/B/C; §6.6 keepalive line clarified. |
| M1 | MEDIUM | **APPLIED (B-full)** | §3.3 retracted `server-lock.json` to "TBD per §12.5". |
| M2 | MEDIUM | **APPLIED (B-full)** | §13 SSETransport row gained POST retry contract. |
| M3 | MEDIUM | **APPLIED (B-full)** | §1 added explicit OUT-of-scope sandbox line + §11 added 42 cross-ref. |
| M4 | MEDIUM | SKIPPED | `cc+unix://` claim re-checked: §4.3 already names the source as `./server/parseConnectUrl.js` and §12.5 already enumerates `parseConnectUrl.ts` as deferred. The grammar surface (`cc://` and `cc+unix://`) appears verbatim at `main.tsx:614` (call-site), so the *recognition* of these two schemes is grounded; the *parser implementation* is correctly deferred. The reviewer's request to "mark with same TBD treatment as serverBanner" is already satisfied by the existing §12.5 entry. No edit. |
| LOW × 4 | LOW | NOT IN SCOPE | B-full charter is HIGH + MEDIUM; LOW deferred. |
| NIT × 3 | NIT | NOT IN SCOPE | Same. |

## Edits applied (top 3 by impact)

### 1. H2 — Disambiguate parallel WS stacks (largest edit)

§6.4 reshaped from a single flat constants table into three sub-tables with a leading disambiguation paragraph:

- **§6.4.A** — Stack A: `src/remote/SessionsWebSocket.ts` (CCR control-plane). Attempt-count budget `MAX_RECONNECT_ATTEMPTS=5`, `PERMANENT_CLOSE_CODES = {4003}`, `PING_INTERVAL_MS=30000`, no token-refresh-on-close, no buffered replay.
- **§6.4.B** — Stack B: `src/cli/transports/WebSocketTransport.ts` (SDK / CCR-v2 worker session-ingress data-plane). Time-budget `DEFAULT_RECONNECT_GIVE_UP_MS=600_000`, `PERMANENT_CLOSE_CODES = {1002, 4001, 4003}`, `DEFAULT_KEEPALIVE_INTERVAL=300_000` (suppressed under `CLAUDE_CODE_REMOTE`), 4003-with-`refreshHeaders()` one-shot recovery, `lastSentId` → `X-Last-Request-Id` replay.
- **§6.4.C** — Setup / direct-connect / `CLAUDE_CODE_REMOTE` env constants (the rest of the original table).

Disambiguation paragraph: "These stacks never share a connection. Stack A carries the CCR control-plane (permission prompts, interrupt, session events); stack B carries the SDK session-ingress data-plane."

§6.6's previous one-liner — *"WebSocketTransport gating: `cli/transports/WebSocketTransport.ts:771`"* — was misleading because line 771 is *not* a stack-selection gate; it is the keepalive-suppression branch. Replaced with a clarifying paragraph that:
1. Identifies `:771-791` as the keepalive-suppression branch only.
2. Explains *why* it suppresses (Stack A's 30s ping covers liveness inside CCR).
3. Redirects readers to `cli/transports/transportUtils.ts` (§13) for the actual transport-selection gate.

### 2. M2 — SSETransport POST retry contract added

§13's SSETransport row now documents:
- `POST_MAX_RETRIES=10` (`:30`), `POST_BASE_DELAY_MS=500` (`:31`), `POST_MAX_DELAY_MS=8000` (`:32`)
- Delay = `min(base * 2^(attempt-1), max)` with ±25% jitter (`:639-649`)
- 4xx non-429 = permanent drop
- Loop continues after exhaustion with warning log (`:639-641`)
- **Critical contrast**: SSETransport's POSTs are *inline* `sleep+retry` per-call; HybridTransport's POSTs go through `SerialBatchEventUploader`. Reviewer flagged this contrast as missing.

### 3. M3 — Sandbox boundary explicit

§1's OUT-of-scope sentence now explicitly excludes "sandbox toggle / `SandboxManager` / `dangerouslySkipPermissions` × sandbox interaction → 42" with verified source citations (`src/commands.ts:149`, `src/main.tsx:201, 314-316`). It also disambiguates that direct-connect's `dangerouslySkipPermissions` (§4.2 / §5.5 / §9) is *not* a sandbox-state mutator — it is a per-session permission-bypass flag forwarded to the server. §11 cross-references 42.

## Parallel WS stacks disambiguation summary

| Aspect | Stack A (`src/remote/SessionsWebSocket.ts`) | Stack B (`src/cli/transports/WebSocketTransport.ts`) |
|---|---|---|
| Powers | `RemoteSessionManager` → CCR control-plane (permission prompts, interrupt, session events) | `getTransportForUrl`-selected transport for SDK session-ingress data-plane (CCR-v2 worker) |
| Subscribes to | `wss://…/v1/sessions/ws/{id}/subscribe?organization_uuid={org}` | Per-session URL passed by caller; 5xx-replay via `X-Last-Request-Id` |
| Reconnect budget | **Attempt-count**: `MAX_RECONNECT_ATTEMPTS=5` | **Time-budget**: `DEFAULT_RECONNECT_GIVE_UP_MS=600_000` (10 min wall clock) |
| Permanent close codes | `{4003}` only | `{1002, 4001, 4003}` |
| 4003 recovery | None (refresh handled out-of-band by `getAccessToken()` closure on next reconnect attempt) | One-shot `refreshHeaders()` per disconnect (`:424-438`); survives the otherwise-permanent 4003 |
| Buffered replay | None | `lastSentId` re-sent as `X-Last-Request-Id` header on reconnect |
| Liveness | 30s `PING_INTERVAL_MS` | 5min `DEFAULT_KEEPALIVE_INTERVAL` data frames; **suppressed when `CLAUDE_CODE_REMOTE` truthy** (Stack A's ping covers liveness) |
| Selected when | Always — for the CCR session subscription bridge | Default selection in `transportUtils.ts` when neither `CLAUDE_CODE_USE_CCR_V2` (→ SSETransport) nor `CLAUDE_CODE_POST_FOR_SESSION_INGRESS_V2` (→ HybridTransport) are set |

The two stacks share no code path. A CCR session typically uses **both simultaneously**: Stack A for the control-plane subscription, Stack B (or its sibling SSE/Hybrid transports) for SDK session ingress.

## Sandbox boundary decision

**Decision: explicit OUT-of-scope line referencing spec 42.**

Rationale:
- `/sandbox-toggle` is a real registered command (`src/commands.ts:149`) and `SandboxManager` is a real wired-in subsystem (`src/main.tsx:201, 314-316`). Verified live.
- However, no source in spec 35's coverage inventory references `SandboxManager` or sandbox-state mutation. The `dangerouslySkipPermissions` flag that *does* appear in §4.2 / §5.5 / §9 is a **session-init permission-bypass flag** sent to the direct-connect server (`server/types.ts` body field `dangerously_skip_permissions`), not a sandbox-state mutator.
- Adding sandbox semantics to spec 35 would duplicate spec 42's coverage. The right pattern is the explicit OUT-of-scope line + cross-ref, matching how 33/34/41 are handled.

This also closes the reviewer's "one-way link" concern: spec 35 now back-references 42, so 42 → 35 ripple isn't required for symmetry (spec 42 can independently choose whether to mention CCR's `dangerouslySkipPermissions`).

## Verification

All four findings verified before edit:
- **H2**: `find` confirmed two distinct files exist (`src/remote/SessionsWebSocket.ts`, `src/cli/transports/WebSocketTransport.ts`); `grep` confirmed Stack B has `DEFAULT_RECONNECT_GIVE_UP_MS=600_000`, `PERMANENT_CLOSE_CODES = {1002, 4001, 4003}`, `lastSentId`, `X-Last-Request-Id`, `refreshHeaders` 4003-recovery, `DEFAULT_KEEPALIVE_INTERVAL=300_000` suppressed under `CLAUDE_CODE_REMOTE` at `:771`.
- **M1**: `find ~/.claude -name "server-*.json"` returned empty (no live lock file at the asserted path); `grep` of `src/server/`, `src/remote/` for `server-lock\|server-sessions` matched only `server-sessions.json` at `src/server/types.ts:43`.
- **M2**: `grep` of `src/cli/transports/SSETransport.ts` confirmed `POST_MAX_RETRIES=10` (`:30`), `POST_BASE_DELAY_MS=500` (`:31`), `POST_MAX_DELAY_MS=8000` (`:32`), retry loop at `:591-649`.
- **M3**: `grep` confirmed `/sandbox-toggle` at `src/commands.ts:149` and `SandboxManager` at `src/main.tsx:201, 314-316`.

## Phase 10 ripple

- 21d already cross-references 35 for `/ultrareview` (Phase 9.6 B-mini delivered the reciprocal link).
- 42 should be expected to optionally back-link to 35 §4.2/§5.5/§9 if 42's sandbox semantics turn out to interact with `dangerouslySkipPermissions`; spec 35 imposes no requirement.
- 22 SSETransport POST retries are now documented in 35 §13 with explicit "not via `SerialBatchEventUploader`" disambiguation, so 22's `withRetry` coverage is not duplicated.
