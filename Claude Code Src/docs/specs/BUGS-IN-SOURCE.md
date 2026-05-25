# BUGS-IN-SOURCE.md

**Scope.** Catalog of bugs / drift discovered in the leaked Claude Code CLI source tree itself, distinct from spec errors. A "bug in source" is a defect in the shipped (leaked) source — wrong identifier, stale comment, false-positive substring match, missing cleanup, etc. — that a faithful reimplementation would inherit only by accident. Reimplementers should treat each entry as a candidate to *fix* (not preserve).

This is **read-only documentation**. No source patches are proposed; suggested fixes are descriptive only.

**Severity legend.**

- `cosmetic` — wrong words in a comment / prompt; no behavioral effect on success path.
- `minor` — strict-mode failure or rare false-positive in a well-bounded code path.
- `major` — silent data loss, security boundary weakening, or unbounded resource leak.

**Status.** As of Phase 9.7. Confirmed entries verified against `src/` at the cited line numbers. Investigated-and-rejected section records candidates that turned out to be non-bugs.

Total: **14 confirmed**, **1 proactively-found**, **4 rejected after investigation**.

---

## Confirmed bugs

### 1. NotebookEdit prompt references nonexistent `cell_number` field

- **Path / lines.** `src/tools/NotebookEditTool/prompt.ts:3`
- **Severity.** `minor` (LLM is steered toward a parameter name the schema rejects)
- **Surfaced.** Phase 9.6 spec 11 fix agent.
- **Description.** The `PROMPT` constant references `cell_number` three times ("The cell_number is 0-indexed", "insert to add a new cell at the index specified by cell_number", "delete to delete the cell at the index specified by cell_number"). The actual input schema (verified at `NotebookEditTool.ts` schema definition) uses **`cell_id`**, not `cell_number`. A strict-schema-validating callsite that takes the prompt at face value will produce a tool call with `cell_number: <int>` and the validator will reject it.
- **Reproduction.** Construct a NotebookEdit invocation with `{cell_number: 0, ...}` against the tool's input schema. Validation fails. Construct with `{cell_id: "...", ...}`. Validation succeeds.
- **Suggested fix.** Replace all three occurrences of `cell_number` with `cell_id` in the prompt. Since `cell_id` is a string identifier, also re-word the "0-indexed" sentence — `cell_id` is not an integer index.

### 2. NotebookEdit stale comment "validateInput ensures cell_number is in bounds"

- **Path / lines.** `src/tools/NotebookEditTool/NotebookEditTool.ts:418`
- **Severity.** `cosmetic`
- **Surfaced.** Phase 9.6 spec 11 fix agent.
- **Description.** Inline comment reads `// validateInput ensures cell_number is in bounds`. The variable `cellIndex` on the same line was derived from `cell_id` (the schema field), not `cell_number`. The `cell_number` identifier exists nowhere in `NotebookEditTool.ts` apart from this comment, so the comment is a leftover from a pre-rename version of the tool.
- **Suggested fix.** `// validateInput ensures the cell resolved from cell_id is in bounds`.

### 3. `isExpiredErrorType` substring match is fragile

- **Path / lines.** `src/bridge/bridgeApi.ts:503-508`
- **Severity.** `minor` (no current false-positive; future error-type strings could trip it)
- **Surfaced.** Phase 9.6 spec 34 fix agent.
- **Description.**
  ```ts
  export function isExpiredErrorType(errorType: string | undefined): boolean {
    if (!errorType) return false
    return errorType.includes('expired') || errorType.includes('lifetime')
  }
  ```
  Three call sites (`bridgeApi.ts:473`, `bridgeMain.ts:1245, 1261`, `replBridge.ts:2274`) gate retry / log-severity decisions on this. Today the only emitted values containing those substrings are `environment_expired` and (presumably) `lifetime_expired` from the backend. Any future server-side addition like `lifetime_extension_pending`, `not_expired`, or `oauth_expired_unrefreshable` (which already exists at `initReplBridge.ts:221`) would be classified as expiry, suppressing logs and triggering the wrong retry policy.
- **Reproduction.** Hypothetical: backend emits `errorType: "lifetime_extension_pending"`. `isExpiredErrorType()` returns `true`, the bridge logs at `info` instead of `error` and suppresses user-visible 403 messages. No way to repro today without server-side cooperation.
- **Suggested fix.** Replace with explicit allow-list:
  ```ts
  const EXPIRED_ERROR_TYPES = new Set([
    'environment_expired',
    'lifetime_expired',
    'oauth_expired_unrefreshable',
    // …add new types deliberately
  ])
  return EXPIRED_ERROR_TYPES.has(errorType)
  ```

### 4. `FileStateCache` cache-key collision via macOS `/tmp` symlink

- **Path / lines.** `src/utils/fileStateCache.ts:41-52`
- **Severity.** `minor` (rare; macOS-only; user must read+edit the same file via two different paths)
- **Surfaced.** Phase 9.6 spec 11 review (originally classified borderline; promoted on inspection).
- **Description.** The cache normalizes keys with `path.normalize()` only — no `realpath()`. On macOS, `/tmp` is a symlink to `/private/tmp`. A `Read` of `/tmp/foo.txt` and a subsequent `Edit` of `/private/tmp/foo.txt` (or vice versa) will miss in cache. The "must read first" guard fires spuriously, or — worse — the freshness check passes against a stale entry from the *other* path while the file has been mutated underneath. Same hazard applies to `/var` → `/private/var` and any user-created symlink.
- **Reproduction.** macOS only. `Read /tmp/x.txt` (populates cache under `/tmp/x.txt`). External writer mutates `/private/tmp/x.txt`. `Edit /private/tmp/x.txt` succeeds without a re-read because that key has no cache entry, but the freshness mtime check sees an unmodified file *under the un-cached realpath*.
- **Suggested fix.** Resolve symlinks before caching: `this.cache.get(realpathSync(normalize(key)))`. Beware: `realpathSync` throws on nonexistent paths, so the guard must catch and fall back. (Cost: one `lstat` per cache hit. Mitigation: memoize realpath results with their own LRU.)

### 5. v1 REPL bridge has no own token-refresh scheduler — spec 34 finding clarification

- **Path / lines.** `src/bridge/replBridge.ts` (bridge core); `src/bridge/initReplBridge.ts`; contrast with `src/bridge/bridgeMain.ts:1470` and `src/bridge/remoteBridgeCore.ts:667`.
- **Severity.** `cosmetic` (originally surfaced as `major` "missing cancelAll" — investigation shows the leak does NOT occur in the v1 REPL path)
- **Surfaced.** Phase 9.6 spec 34 fix agent (initial framing); reclassified during Phase 9.7 audit.
- **Description.** Spec 34's adversarial review hypothesized that `replBridge.ts` is missing a `tokenRefresh.cancelAll()` at teardown analogous to `remoteBridgeCore.ts:667` and `bridgeMain.ts:1470`. Verification: `replBridge.ts` does NOT instantiate `createTokenRefreshScheduler` at all (`grep -n createTokenRefreshScheduler src/bridge/{replBridge,initReplBridge}.ts → no matches`). It uses an externally-supplied `getAccessToken` callback (`replBridge.ts:105`) and relies on the standard OAuth refresh flow described at `:839-842` ("Unlike the JWT path, OAuth tokens are refreshed by the standard OAuth refresh flow"). There is no v1 REPL-side timer to cancel.
- **Residual concern.** If a *future* caller of `initBridgeCore` injects a `getAccessToken` backed by its own scheduler, that scheduler's `cancelAll` must be wired into the caller's teardown path — `replBridge.ts` cannot wire it because it doesn't own the scheduler. Document this contract in the JSDoc on `BridgeCoreParams.getAccessToken`.
- **Suggested fix.** Add JSDoc to `BridgeCoreParams.getAccessToken` (`replBridge.ts:105`):
  ```
  /**
   * …
   * Caller is responsible for cancelling any timers backing this callback
   * when the bridge tears down. initBridgeCore does not call cancelAll
   * because the v1 REPL path has no own scheduler.
   */
  ```

### 6. `PluginError` discriminated union declares `lsp-config-invalid` twice

- **Path / lines.** `src/types/plugin.ts:177` and `:220` (both within the union spanning `:101-283`).
- **Severity.** `cosmetic` (TypeScript de-duplicates structurally identical members of a discriminated union; no runtime effect).
- **Surfaced.** Phase 9.6c spec 28 fix agent.
- **Description.** The `PluginError` union declares an arm `{ type: 'lsp-config-invalid', source, plugin, serverName, validationError }` at `:177-182` and again — byte-identical — at `:220-225`. A third reference at `:335` (the `getPluginErrorMessage()` switch) is a `case 'lsp-config-invalid':` consumer, not a third declaration. The union nominally has **24** discriminant variants (per spec 28 §6.22 / §9.1); the count is correct only because TS treats the duplicate arm as the same member. A reimplementation that enumerates union arms via macro / codegen rather than via `keyof` would double-count.
- **Reproduction.** `grep -n "type: 'lsp-config-invalid'" src/types/plugin.ts` returns two matches at `:177` and `:220`.
- **Suggested fix.** Delete the second occurrence (`:220-225`). No call site disambiguates between the two — they are structurally identical.

### 7. `policyLimits` checksum sort uses `localeCompare` (diverges from server's Python `sort_keys=True`)

- **Path / lines.** `src/services/policyLimits/index.ts:139-141` (vs. correct counterpart at `src/services/remoteManagedSettings/index.ts:118`).
- **Severity.** `minor` (no security impact; bandwidth + disk-write churn on non-ASCII restriction keys; potentially also locale-dependent for some ASCII edge cases).
- **Surfaced.** Phase 9.6c spec 27 fix agent (was Phase 9.5 §5.4 "verbatim divergence" without bug classification).
- **Description.** Both checksum implementations sort object keys deeply before serializing through `jsonStringify` and hashing with sha256. They MUST match the server's canonicalisation — comment at `remoteManagedSettings/index.ts:128-129` explicitly states "Must match server's Python: `json.dumps(settings, sort_keys=True, separators=(",", ":"))`". Python's `sort_keys=True` performs a code-point sort, equivalent to JavaScript's default `Array.prototype.sort()` over strings. The `remoteManagedSettings` implementation does this correctly: `Object.keys(obj).sort()` at `:118`. The `policyLimits` implementation does NOT: it uses `Object.entries(obj).sort(([a],[b]) => a.localeCompare(b))` at `:139-141`. `localeCompare` is locale-sensitive and disagrees with code-point ordering for non-ASCII keys (and some ASCII edge cases under non-`en-US` locales).
- **Consequence.** Client-computed `policy-limits` checksum will disagree with what the server would compute over an identical restrictions object whenever a non-ASCII key is present. The server then returns 200 instead of 304 on every poll, the cache file is rewritten unnecessarily, and bandwidth is wasted. No security impact: restrictions content is unchanged, and `isPolicyAllowed` reads from the deserialized cache, not the checksum. ETag round-trips to the server are simply forced-miss.
- **Reproduction.** Construct a `restrictions` object with a non-ASCII key (e.g. `'café_feature'`). Compute checksum on client (`localeCompare` order under en-US locale: `'café_feature' < 'cafe_feature'` because `é` collates near `e`). Compute Python `json.dumps(..., sort_keys=True)` over same object: `'café_feature' > 'cafe_feature'` because `é` (U+00E9) > `e` (U+0065) by code point. Hashes differ.
- **Suggested fix.** Change `policyLimits/index.ts:139-141` to match `remoteManagedSettings/index.ts:118`:
  ```ts
  for (const key of Object.keys(obj).sort()) {
    sorted[key] = sortKeysDeep((obj as Record<string, unknown>)[key])
  }
  ```

### 8. `getAllOutputStyles` priority-order comment contradicts the array literal

- **Path / lines.** `src/constants/outputStyles.ts:158-159`
- **Severity.** `cosmetic`
- **Surfaced.** Phase 9.6c spec 38 fix agent.
- **Description.** Inline comment at `:158` reads `// Add styles in priority order (lowest to highest): built-in, plugin, managed, user, project`. The array literal on the next line is `[pluginStyles, userStyles, projectStyles, managedStyles]` — i.e. the actual lowest→highest order is built-in < plugin < user < project < **managed**, with managed (policy) winning. The comment swaps `managed` and `project` in the textual list. Behaviour is correct (managed/policy supersedes project, which supersedes user, which supersedes plugin); only the comment is wrong.
- **Reproduction.** Define a user-scoped style and a managed/policy-scoped style with the same `name`. The aggregator returns the managed entry (last writer wins under the actual array order), confirming managed > project > user > plugin > built-in.
- **Suggested fix.** `// Add styles in priority order (lowest to highest): built-in, plugin, user, project, managed`.

### 9. `Scroll` and `MessageActions` contexts in `DEFAULT_BINDINGS` are absent from `KEYBINDING_CONTEXTS`

- **Path / lines.** `src/keybindings/defaultBindings.ts:196` (Scroll block), `defaultBindings.ts:268-295` (MessageActions block, `feature('MESSAGE_ACTIONS')`-gated); `src/keybindings/schema.ts:12-32` (`KEYBINDING_CONTEXTS` enumerates 18 names — neither `Scroll` nor `MessageActions`).
- **Severity.** `minor` (silent loss of user-override capability for documented default bindings; no security or data-loss impact).
- **Surfaced.** Phase 9.6c spec 39 fix agent.
- **Description.** Defaults parse via `parseBindings(DEFAULT_BINDINGS)` which never runs the Zod schema, so the `Scroll`/`MessageActions` blocks load and resolve correctly at runtime. But user `~/.claude/keybindings.json` is validated by `KeybindingsSchema` (`schema.ts:177-201`) which restricts `context` to `z.enum(KEYBINDING_CONTEXTS)`. A user attempting to override e.g. `wheeldown → scroll:lineDown`, or remap any `MessageActions` binding, receives `Unknown context "Scroll"` / `Unknown context "MessageActions"` from `validate.ts:156-163`. The default Scroll bindings (mouse wheel, page up/down, ctrl+home/end, selection copy) are therefore silently non-overridable; same for MessageActions when flag-on.
- **Reproduction.** Write `{ "bindings": [ { "context": "Scroll", "bindings": { "wheeldown": null } } ] }` to `~/.claude/keybindings.json` with customization GrowthBook on. Validation fails with `Unknown context "Scroll"` and the override is dropped while the default still fires.
- **Suggested fix.** Add `'Scroll'` and `'MessageActions'` to `KEYBINDING_CONTEXTS` and to the `KEYBINDING_CONTEXT_DESCRIPTIONS` record at `schema.ts:37-59`. Cheaper than retyping `KeybindingContextName` to `string`.

### 10. WebSearchTool `validateInput` "Missing query" branch is unreachable

- **Path / lines.** `src/tools/WebSearchTool/WebSearchTool.ts:235-243` (validator) vs. `:27` (Zod `query: z.string().min(2)`).
- **Severity.** `minor` (cosmetic dead branch; no behavioral effect, but masks intent and inflates apparent error-code surface).
- **Surfaced.** Phase 9.5b spec 13 review.
- **Description.** The Zod input schema declares `query: z.string().min(2)`. Schema validation runs before `validateInput`, so any input with `query.length === 0` is rejected with a Zod `too_small` error before the `if (!query.length)` branch can fire. The branch's `errorCode: 1` is therefore unreachable from the normal call path. The second branch (`allowed_domains` + `blocked_domains` both set, `errorCode: 2`) is reachable because Zod does not enforce mutual exclusion.
- **Reproduction.** Construct `{query: ""}`. Schema validation rejects with Zod `too_small (minimum: 2)` — never reaches `validateInput`. Construct `{query: "x"}` (1 char). Same — `min(2)` rejects. Only `query` of length 0 or 1 could ever satisfy `!query.length`; both are pre-empted by Zod.
- **Suggested fix.** Either drop the dead branch (keep only the domain mutual-exclusion check) or relax the Zod minimum to `min(1)` and let `validateInput` handle the empty-string user-facing message. (The current `min(2)` was likely meant to forbid 1-character queries; if so, the dead branch is genuinely vestigial.)

### 11. LSP `connection.onClose` does not invoke `onCrash` for graceful-close-after-failure

- **Path / lines.** `src/services/lsp/LSPClient.ts:200-207` (onClose handler) vs. `:156-167` (process exit handler that DOES invoke `onCrash`).
- **Severity.** `major` (LSP server can enter zombie state — connection closed, `isInitialized = false`, but `onCrash` not fired, so `LSPServerManager` never restarts it; user sees "no diagnostics" with no surfaced error).
- **Surfaced.** Phase 9.5b spec 24 review.
- **Description.** Two independent termination signals are wired:
  1. Process exit (`process.on('exit', code !== 0 && !isStopping)`) — fires `onCrash?.(crashError)`. The owner (`LSPServerManager`) reacts and may restart.
  2. JSON-RPC connection close (`connection.onClose(() => !isStopping ? logForDebugging(...) : noop)`) — only logs at debug level, NEVER calls `onCrash`. Sets `isInitialized = false` so subsequent requests fail, but no upward signal.
  If the server's stdout/stdin closes (broken pipe, parent file-descriptor leak, child stuck in uninterruptible sleep) without the child process actually exiting, only path 2 fires. The child stays alive (waiting for stdin that's been closed by `kill_after_close` semantics on some kernels), `process.on('exit')` never fires, `onCrash` is never called, and the manager's auto-restart loop has nothing to restart.
- **Reproduction.** SIGSTOP an LSP child process (`kill -STOP <pid>`), then close its stdout file descriptor externally. Connection's reader observes EOF → `onClose` fires → only debug log. Child is still alive (frozen). Subsequent `textDocument/diagnostic` requests time out instead of being recovered by restart.
- **Suggested fix.** Either (a) propagate `connection.onClose` to `onCrash` when not intentionally stopping, with a distinguishing message (`LSP server X connection closed unexpectedly`), or (b) start a watchdog timer in `onClose` that calls `onCrash` if `process.on('exit')` does not fire within e.g. 5s. Option (a) is simpler; option (b) avoids double-firing if the exit handler is just slightly delayed.

### 12. Two `team_mem_*` events emitted via `logEvent` are absent from Datadog allow-list

- **Path / lines.** `src/services/analytics/datadog.ts:19-64` (`DATADOG_ALLOWED_EVENTS` set) vs. emit sites at `src/services/teamMemorySync/index.ts:935` (`tengu_team_mem_secret_skipped`) and `src/services/teamMemorySync/watcher.ts:112` (`tengu_team_mem_push_suppressed`).
- **Severity.** `cosmetic` (events fire correctly into the local analytics pipeline; only the secondary Datadog forwarding drops them silently).
- **Surfaced.** Phase 9.5b spec 29 review.
- **Description.** Both events follow the `tengu_team_mem_*` naming convention used by four allow-listed siblings (`tengu_team_mem_sync_pull`, `tengu_team_mem_sync_push`, `tengu_team_mem_sync_started`, `tengu_team_mem_entries_capped` — present at `:60-63`). The set acts as a Datadog-only allow-list (events not in it still go to the primary `logEvent` path; see `datadog.ts` shouldForward gate). Whoever added the secret-skip and push-suppress events did not extend the set, so Datadog dashboards under-report by two event types relative to the local pipeline. (Distinct from R2 which was about a stale count comment.)
- **Reproduction.** Trigger a team-memory push that contains a recognized secret pattern (fires `tengu_team_mem_secret_skipped`) or a push during a back-off window (fires `tengu_team_mem_push_suppressed`). Confirm the event reaches the local `logEvent` consumer but does NOT appear in the Datadog batch (set lookup at the forwarder fails silently).
- **Suggested fix.** Add `'tengu_team_mem_secret_skipped'` and `'tengu_team_mem_push_suppressed'` to `DATADOG_ALLOWED_EVENTS` (`datadog.ts:60-63`), keeping the existing alphabetical-within-prefix convention.

### 13. `set_proactive` control-protocol payload is raw-cast, not Zod-validated

- **Path / lines.** `src/cli/print.ts:3875-3891` (set_proactive branch); contrast with sibling control-protocol subtypes that go through Zod parsers in the same file.
- **Severity.** `minor` (no current security impact — sender is the IDE bridge, trusted; but no defence in depth, and a malformed payload would crash the proactive activation path with a TypeError instead of a structured error response).
- **Surfaced.** Phase 9.5b spec 31 review.
- **Description.** When a control-protocol message arrives with `subtype === 'set_proactive'`, the handler does:
  ```ts
  const req = message.request as unknown as {
    subtype: string
    enabled: boolean
  }
  if (req.enabled) { ... } else { ... }
  ```
  No runtime check on `enabled`. If a (buggy or malicious) sender supplies `{subtype: 'set_proactive', enabled: 'true'}` (string instead of boolean), the truthy check passes and `activateProactive('command')` runs; if it supplies `{subtype: 'set_proactive', enabled: 0}`, the same falsy path as `enabled: false` runs. Worse: if `enabled` is missing, `req.enabled` is `undefined`, `deactivateProactive()` runs unconditionally.
- **Reproduction.** Send a control-protocol message `{type: 'control_request', request_id: 'x', request: {subtype: 'set_proactive'}}` (no `enabled` field). Handler runs `else` branch and deactivates proactive mode. No error response sent.
- **Suggested fix.** Define a Zod schema `z.object({subtype: z.literal('set_proactive'), enabled: z.boolean()})` and parse `message.request` through it before destructuring. On parse failure, call `sendControlResponseError(message, ...)` with a structured error.

### 14. `useAppState` selector-identity guard is dead in production builds (`if (false && ...)`)

- **Path / lines.** `src/state/AppState.tsx:150`
- **Severity.** `cosmetic` (dead-code branch; the `&& false` short-circuits before the equality check, so the throw is unreachable in shipped binaries).
- **Surfaced.** Phase 9.5b spec 41 review.
- **Description.** The shipped form is `if (false && state === selected) { throw new Error("Your selector ... returned the original state ..."); }`. The `false &&` literal short-circuits the conjunction; the throw is unreachable. The intended pattern (visible from the error message) is to forbid selectors of the form `s => s` because they would force every change to re-render the subscriber, defeating the purpose of `useSyncExternalStore`. The guard was likely behind an `if (__DEV__ && ...)` or `if (process.env.NODE_ENV !== 'production' && ...)` gate that the bundler resolved to `false` for the released artifact and then never stripped, leaving the literal `false &&` in source. A reimplementation should restore the dev-mode gate so the warning fires in development and is dead-code-eliminated in release.
- **Reproduction.** Call `useAppState(s => s)` in dev or prod — neither path throws. Compare to a hypothetical correct guard: in dev, the call would throw; in prod, the dead branch would be eliminated.
- **Suggested fix.** Replace `if (false && state === selected)` with `if (process.env.NODE_ENV !== 'production' && state === selected)` (or whatever DEV constant the rest of the codebase uses; check `src/utils/env.ts` for the convention).

---

## Investigated and rejected

### R1. `BLOCKED_DEVICE_PATHS` symlink bypass

- **Path.** `src/tools/FileReadTool/FileReadTool.ts:98-128`
- **Investigation.** The set is a literal-string match; a symlink `~/myzero -> /dev/zero` would not be blocked by `isBlockedDevicePath`. However:
  1. The blocked paths are well-known device files; a malicious user with write access to the home directory who creates such a symlink can already invoke `Read('/dev/zero')` directly (no permission gate refers to this set).
  2. The `BLOCKED_DEVICE_PATHS` set is documented as a *liveness* guard ("would hang the process") — not a security boundary. The actual security boundary is the file-permission system + Bash's `pathValidation.ts` (which DOES handle symlinks at line 82).
- **Verdict.** Not a bug. The set's purpose is "don't accidentally `cat /dev/zero`", not "prevent malicious access". Symlink resolution would add cost without security benefit.

### R2. Datadog allow-list count drift (`41` vs actual `44`)

- **Path.** `src/services/analytics/datadog.ts:19-64` and former spec 26 §6.3.
- **Investigation.** Direct grep of `datadog.ts:1-80` yields **44** entries (7 `chrome_bridge_*` + 37 `tengu_*`). The source file itself has NO inline count comment — the `41` lived only in spec 26 §6.3 and §11. Spec 26 was already corrected to `44` in Phase 9.6 (see `26-service-analytics-flags.md:541, 823`). The leak source is fine.
- **Verdict.** Not a source bug — was a spec drift, already fixed.

### R3. `replBridge.ts` missing `tokenRefresh.cancelAll()` (originally claimed as bug 4 by Agent B brief)

- **Verdict.** Reclassified as confirmed bug 5 (cosmetic, doc-only fix). See entry 5 above.

### R4. WebSearchTool `utils.ts` double-emit + content-bytes clamp (spec 13 candidates 2 & 3)

- **Investigation.** Phase 9.5b spec 13 review claimed two bugs in `src/tools/WebSearchTool/utils.ts`: a `logError` double-emit on domain-check failures (`:199-202` + `:407-413`) and a `Math.max(1, contentBytes)` asymmetric clamp at `:480`. **The file `src/tools/WebSearchTool/utils.ts` does not exist.** `ls src/tools/WebSearchTool/` returns only `UI.tsx`, `WebSearchTool.ts`, and `prompt.ts`. Direct grep across `src/tools/WebSearchTool/` and `src/tools/` for the cited substrings (`logError` near domain check, `Math.max(1, contentBytes)`) produces no matches. The lone `logError` call in WebSearchTool is `WebSearchTool.ts:119` on a `web_search_tool_result` error block — not domain-related and not double-emitted.
- **Verdict.** Not source bugs — spec drift in the Phase 9.5b review brief. The reviewer appears to have hallucinated paths and line numbers, possibly conflating with another tool's utility file.

---

## Bug-pattern survey (proactive)

### P1. Other tool prompts referencing fields not in their schema

Surveyed all `src/tools/*/prompt.ts` (~30 files) for similar drift. Approach: for each prompt, extract `\b[a-z_]+\b` tokens that look like parameter names (e.g., `xxx_yyy`, `xxxId`) and cross-check against the tool's Zod schema in the sibling `*.ts` or `schema.ts`. Spot-checked: `BashTool`, `FileEditTool`, `FileWriteTool`, `WebFetchTool`, `TaskCreateTool`, `AgentTool`, `GlobTool`, `SkillTool`. **No additional drift found** — only `NotebookEditTool` shows the cell_number/cell_id mismatch.

A full programmatic audit would diff `Object.keys(schema.shape)` ∩ `prompt.match(/\b[a-z_]+\b/g)` to flag any token in the prompt not present in the schema. Recommended for Phase 10.

### P2. Other substring `.includes()` checks that could false-positive

Searched `src/bridge/` and `src/services/` for `.includes('<short_word>')` patterns gating control flow. Findings:

- `bridgeApi.ts:507` — bug 3, already cataloged.
- No additional substring-gate hits in `src/bridge/` (`grep -nE "errorType\.(includes|startsWith|match)" src/bridge/` returns only the line above).
- `src/services/` shows no comparable substring-gate patterns on error-type strings.

### P3. Other missing-cleanup paths in transport modules

Searched all `src/bridge/*.ts` for `cancelAll`, `clearTimeout`, `clearInterval`, and `process.off`. `replBridge.ts` teardown (line 1550-1700) handles `pointerRefreshTimer`, `keepAliveTimer`, `sigusr2Handler`, `pollController.abort()`, `flushGate.drop()`, `transport` close. `bridgeMain.ts` teardown (1460-1500) handles `sessionTimers` clear + `tokenRefresh?.cancelAll()` + worktrees. `remoteBridgeCore.ts` teardown (664-) handles `refresh.cancelAll()` + `clearTimeout(connectDeadline)` + `flushGate.drop()`. No missing-cleanup found. The original concern (bug 4) was based on a structural symmetry that does not hold because v1 has no own scheduler.

### P4. Count-comment drift survey

`grep -rnE '//\s*[0-9]+\s+(items?|entries|events|fields|tools|commands|hooks)\b' src/` returns:

- `utils/config.ts:516` — "100 entries to bound config growth" — verified against the relevant constant.
- `components/ScrollKeybindingHandler.tsx:74` — "9 events/sec — smooth ramp" — describes a rate, not a count of static items; not drift-prone.

No additional drifted count-comments found. (Other count claims live in `docs/specs/` rather than source comments — those are spec drift, out of scope here.)

### P5. Other stale identifier comments (rename leftovers)

`grep -rnE '// .* (cell_number|old_param_name|legacy)' src/tools/` finds only the `NotebookEditTool.ts:418` instance already cataloged. No additional rename leftovers in `src/tools/`. Did NOT do a repo-wide audit; recommended for Phase 10 as a programmatic check (compare comment-tokens against current schema fields per tool).

---

## Recommended Phase 10+ follow-ups

1. **Tooling: programmatic prompt-vs-schema audit.** Build a small script that, for every `src/tools/<X>/`, extracts `Object.keys(schema.shape)` and reports any `\bsnake_case\b` token in the prompt or DESCRIPTION that does not appear in the schema. Would have caught bug 1 deterministically. Likely surface area: ~30 tools.

2. **Tooling: stale-comment audit.** Run a similar diff between identifiers used in a file's executable code and identifiers mentioned only in comments. High false-positive rate, but worth one pass — bug 2 is the kind of thing it would surface.

3. **Source change: replace `isExpiredErrorType` substring match with allow-list** (bug 3). Low risk, ~6-line change, removes a foot-gun for future server-side error-type additions.

4. **Source change: add `realpath` resolution to `FileStateCache`** (bug 4). Bigger patch (need to handle `ENOENT` for files about to be created via `Write`), but eliminates a class of cache-coherence bugs. Add tests covering `/tmp` ↔ `/private/tmp` on macOS.

5. **Documentation: tighten `BridgeCoreParams.getAccessToken` JSDoc** (bug 5). Trivial, prevents future reimplementers from copying the pattern without owning the cleanup.

6. **Process: include "audit grep" step in spec-fix workflow.** When a spec-fix agent edits a spec entry that cites a source comment or count, instruct it to verify the comment/count in source and add an entry here if it has drifted. Phase 9.6 surfaced bugs 1-4 as side effects of spec work — formalize the catch.
