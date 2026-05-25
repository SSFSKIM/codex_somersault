# Phase 9.5b Adversarial Review — Spec 23 (`docs/specs/23-service-mcp.md`)

**Reviewer role:** Skeptic. Read-only verification against `src/services/mcp/`.
**Spec size:** 1115 lines. **Source touched:** ~25 reads/greps across `client.ts`, `useManageMCPConnections.ts`, `types.ts`, `InProcessTransport.ts`, `SdkControlTransport.ts`, `auth.ts`, `elicitationHandler.ts`, `vscodeSdkMcp.ts`, `MCPConnectionManager.tsx`, `mcpServerApproval.tsx`, `src/components/mcp/`.

## Severity counts

| Severity | Count |
|---|---|
| **Critical (factually wrong / breaks reimpl)** | 0 |
| **High (misleading / cross-spec gap)** | 2 |
| **Medium (under-specified / drift risk)** | 3 |
| **Low (cosmetic / nit)** | 4 |

## Top 5 findings

### H1 — `MCPServerConnection` discriminator order is mis-stated in the prose chart

`§4.1` state diagram says reconnect emits `pending(reconnectAttempt++)` only for **remote** transports. Source confirms (`useManageMCPConnections.ts:354+`: `if configType in {stdio, sdk}: updateServer({...client, type:'failed'}); return`). However, §4.1 caption "isMcpServerDisabled? yes/no" branches *under* the connected node imply the disabled check runs on every onclose, but the actual code (`useManageMCPConnections.ts:520`-ish) re-reads disk state — spec correctly notes "AppState may be stale" in §5.5 but the diagram conflates these. Reimplementer reading only §4.1 will conflate AppState-disabled and disk-disabled. **Recommend:** add explicit "(disk re-read)" annotation on the diagram's disabled? branch.

### H2 — ElicitationDialog UI surface (180KB → 1168 lines actual) is referenced but its location is **never named** in §2

The instructions hint expected a reference to `ElicitationDialog.tsx`. Spec §6.5 inlines channel-blocked toast strings and approval-dialog choices but contains **zero references** to `src/components/mcp/ElicitationDialog.tsx` (1168 lines, confirmed). §5.6 (`callMCPToolWithUrlElicitationRetry`) describes the queue payload `{params: elicitation, signal, waitingState, respond, onWaitingDismiss}` exactly as the dialog consumes — but the consumer file is unmentioned. §12 also doesn't disclaim it. **Cross-spec gap:** spec 23's "elicitation queue protocol" is the contract that ElicitationDialog renders; either §3 or §12 should add a one-line cross-cite to spec 37 (UI shell) — or to wherever ElicitationDialog ends up owned. Without it, modifying the queue shape silently breaks UI.

### M1 — `_NOT_CODE_OR_FILEPATHS` redaction policy: spec 23 inherits but never declares the boundary

Phase 9.5 spec 26 finding: `tengu_internal_record_permission_context` casts paths via `_NOT_CODE_OR_FILEPATHS` analytics-metadata type. **Verified:** the marker class `TelemetrySafeError_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS` and `AnalyticsMetadata_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS` appear ~25× across `client.ts` and `auth.ts` (e.g. `auth.ts:828, 838, 880, 1245, 1323`; `client.ts:177, 1061, 1698, 2702, 2746`). Spec 23 §6.6 names `TelemetrySafeError_…` as the wrap class but **does not state the redaction contract**: "all `mcpServerBaseUrl`, `transportType`, `serverVersion`, `failureStage` strings emitted into `tengu_mcp_*` events are cast as `_NOT_CODE_OR_FILEPATHS`". Spec 09 owns redaction policy; spec 23 should add a one-liner in §10 deferring to it ("All analytics fields cast `as AnalyticsMetadata_…NOT_CODE_OR_FILEPATHS` per spec 09"). Phase 9.7 §12 sweep apparently did not touch this.

### M2 — `vscodeSdkMcp.ts` is gated by `USER_TYPE === 'ant'`, not `feature(...)`; spec lists it under §2.1 as "OAuth-flow internals owned by spec 25"

Source: `vscodeSdkMcp.ts:44`: `if (process.env.USER_TYPE !== 'ant' || !vscodeMcpClient) return`. This is **ANT-only IDE-bridge code** (it filters `claude-vscode` SDK clients, fires `tengu_vscode_*` events, and reads `tengu_vscode_review_upsell` / `tengu_vscode_onboarding` Statsig gates). Spec 23 lumps it with `oauthPort.ts`, `xaaIdpLogin.ts` etc. under §2.1 as "OAuth-flow internals owned by spec 25." It is **not** OAuth — it is VS Code IDE-bridge plumbing tied to spec 34. §8 ("ANT-only behavior") only mentions `useManageMCPConnections.ts:988-1007`. **Recommend:** §8 should explicitly add `vscodeSdkMcp.ts` as a second ANT-only gate; §2.1 row needs re-classification (cross-cite spec 34, not spec 25).

### M3 — `MCP_SKILLS` ripple: §10 lacks the analytics row, but spec coverage of §8 is correct

Confirmed: `MCP_SKILLS` callsites at `client.ts:117, 1392, 1670, 2174, 2348` and `useManageMCPConnections.ts:22, 684, 718, 723, 729` (all match spec). Spec 17 → 28 ripple is consistent: §12 Q3 correctly defers `mcpSkills.ts` impl to spec 17 and `getMcpSkillCommands` (commands.ts:550) to spec 20. **However**, §10's analytics inventory is missing: when `MCP_SKILLS` is on and a `resources/list_changed` arrives, are skills re-fetched silently or does it emit `tengu_mcp_list_changed{type:'resources'}`? Spec says yes (line 1053), but doesn't disambiguate whether skills get a separate `type:'skills'` event. Source: only `'tools'|'prompts'|'resources'` (`useManageMCPConnections.ts:638, 651, 675, 713`). Spec is correct by omission but a reader might expect a `'skills'` row. Add explicit "(no separate skills event)" note.

## Verdict

**Spec is technically accurate; recommend approval after H2 + M1 + M2 patches.**

- All 17 verbatim constants (DEFAULT_MCP_TOOL_TIMEOUT_MS, MAX_RECONNECT_ATTEMPTS, MAX_FETCH_CACHE_SIZE, MAX_ERRORS_BEFORE_RECONNECT, MAX_SESSION_RETRIES, MAX_URL_ELICITATION_RETRIES, INITIAL_BACKOFF_MS, MAX_BACKOFF_MS, MAX_MCP_DESCRIPTION_LENGTH, MCP_REQUEST_TIMEOUT_MS, the SIGINT/SIGTERM/SIGKILL 100/400/500/600 ms budgets, etc.) **verified bit-exact**.
- Transport list (stdio, sse, sse-ide, http, ws, ws-ide, sdk, claudeai-proxy + in-process pair) **matches `types.ts:23-25` enum + claudeai-proxy + ws-ide unions exactly**.
- Error class names (`McpAuthError`, `McpSessionExpiredError`, `McpToolCallError_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS`, `TelemetrySafeError_…`) **match source verbatim** including the `_I_VERIFIED…` suffix.
- JSON-RPC code mapping `-32000`/`-32001`/`-32042` **verified**.
- `MAX_RECONNECT_ATTEMPTS = 5` with backoff `Math.min(INITIAL_BACKOFF_MS * Math.pow(2, attempt-1), MAX_BACKOFF_MS)` **matches `useManageMCPConnections.ts:88-90, 446-453`**.
- §5.7 McpAuthTool race-and-swap algorithm matches the cross-spec to spec 16 §3.4 — no contradiction.
- §12 open-questions enumeration is honest and consistent with Phase 9.7 §12 sweep markings (5 of 8 entries explicitly cross-cite owning specs: 17, 25, 32, 34, 37).

## Cross-spec impact

| Spec | Impact |
|---|---|
| **09** (redaction) | M1: spec 23 must defer `_NOT_CODE_OR_FILEPATHS` casting policy explicitly, not implicitly |
| **16** (MCP tool surface) | OK — `McpAuthTool` per-server factory + `getToolNameForPermissionCheck` cross-cite is consistent |
| **17** (skills) | OK — `MCP_SKILLS` deltas align; spec 17 must own `fetchMcpSkillsForClient` impl |
| **25** (OAuth) | OK — `performMCPOAuthFlow`, `ClaudeAuthProvider`, XAA, keychain all deferred correctly |
| **28** (plugins) | OK — `pluginSource`, `excludeStalePluginClients`, plugin-MCP key namespace all cross-cited |
| **32** (KAIROS channels) | §12 Q1 honestly notes channel state machine deferred — no contradiction |
| **34** (IDE bridge) | M2: `vscodeSdkMcp.ts` is mis-classified; should be cross-cite to 34 not 25 |
| **37** (UI shell) | H2: ElicitationDialog (1168 lines) referenced by behavior but never named — gap |

## Hardest-to-verify claim

**§5.6 URL-elicitation retry pseudocode** — specifically the assertion that `elicitations = filter rawElicitations to objects with mode='url' && string url + elicitationId + message; if elicitations.length === 0: throw`. Verifying this required tracing `client.ts:2813-3027` while simultaneously cross-referencing `elicitationHandler.ts:53-72` (find queued elicitation by `elicitationId`) and `client.ts:2850 MAX_URL_ELICITATION_RETRIES = 3` and the runtime contract with the React queue at `setAppState(prev => ({...prev, elicitation:{queue: [...]})`. The validation predicate filtering and the **fall-through-to-throw on empty filtered list** is the most subtle invariant — a reimplementer who treats `elicitations.length === 0` as a no-op (just retry) would silently break the contract, and no test in this leak-archive can be run to catch it. The spec captures it correctly, but verification required pivoting between three files and the React state shape, which is the kind of distributed-truth claim that's genuinely hard to defend without running it.
