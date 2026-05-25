# 21b — Command Catalog: ANT-only / Internal

> Per-command spec for `INTERNAL_ONLY_COMMANDS` (`commands.ts:225-254`) plus the ANT top-level require `agentsPlatform`. Read 20-command-system.md and 21-command-catalog.md first.
>
> **Critical leak observation**: most ANT-only command source files in this leak ship as `index.js` containing only a stub:
> ```js
> export default { isEnabled: () => false, isHidden: true, name: 'stub' };
> ```
> These commands are documented at **registry-citation level only** — name, ANT-gating, and any conditional flag flag are recoverable from `commands.ts`; their behavior, prompts, and side effects are **not recoverable from this leak** and must be marked as such.

---

## 1. Purpose & Scope

This sub-file enumerates every command added to `COMMANDS()` only when `process.env.USER_TYPE === 'ant' && !process.env.IS_DEMO` (`commands.ts:343-345`). The list is `INTERNAL_ONLY_COMMANDS` (`commands.ts:225-254`):

```typescript
export const INTERNAL_ONLY_COMMANDS = [
  backfillSessions,
  breakCache,
  bughunter,
  commit,             // ← documented in 21a alongside other prompt commands
  commitPushPr,       // ← documented in 21a alongside other prompt commands
  ctx_viz,
  goodClaude,
  issue,
  initVerifiers,      // ← documented in 21a alongside /init
  ...(forceSnip ? [forceSnip] : []),         // feature('HISTORY_SNIP')
  mockLimits,
  bridgeKick,
  version,
  ...(ultraplan ? [ultraplan] : []),         // feature('ULTRAPLAN') — see 21c
  ...(subscribePr ? [subscribePr] : []),     // feature('KAIROS_GITHUB_WEBHOOKS') — see 21c
  resetLimits,
  resetLimitsNonInteractive,
  onboarding,
  share,
  summary,
  teleport,
  antTrace,
  perfIssue,
  env,
  oauthRefresh,
  debugToolCall,
  agentsPlatform,                            // ANT-only top-level require
  autofixPr,
].filter(Boolean)
```

Plus the additional ANT-only `process.env.USER_TYPE === 'ant'` runtime check on universally-registered commands `/files`, `/tag`, `/cost` (its visibility/output) — those entries live in 21a where they're registered.

### IN scope
- All `INTERNAL_ONLY_COMMANDS` not already covered in 21a (i.e., excluding `/commit`, `/commit-push-pr`, `/init-verifiers`).
- `agentsPlatform` (top-level require, `commands.ts:48-51`).

### OUT of scope
- Public-shaped commands with ANT runtime checks → 21a.
- Feature-flag–gated entries (forceSnip / ultraplan / subscribePr) → 21c (their gating goes there even though they're spread inside `INTERNAL_ONLY_COMMANDS`).

---

## 2. Source Map

| Path | Status | Role |
|---|---|---|
| `src/commands/version.ts` | ✅ read fully | `/version` — fully sourced |
| `src/commands/bridge-kick.ts` | ✅ read fully | `/bridge-kick` — fully sourced |
| `src/commands/autofix-pr/index.js` | stub | name: `'stub'`, isEnabled false |
| `src/commands/backfill-sessions/index.js` | stub | |
| `src/commands/break-cache/index.js` | stub | |
| `src/commands/bughunter/index.js` | stub | |
| `src/commands/ctx_viz/index.js` | stub | |
| `src/commands/good-claude/index.js` | stub | |
| `src/commands/issue/index.js` | stub | |
| `src/commands/mock-limits/index.js` | stub | |
| `src/commands/onboarding/index.js` | stub | |
| `src/commands/share/index.js` | stub | |
| `src/commands/summary/index.js` | stub (registered name: `'stub'`) — but appears as `summary` in `BRIDGE_SAFE_COMMANDS` per `commands.ts:651-660`. The `.filter((c): c is Command => c !== null)` guard tolerates stubs. |
| `src/commands/teleport/index.js` | stub | |
| `src/commands/ant-trace/index.js` | stub | |
| `src/commands/perf-issue/index.js` | stub | |
| `src/commands/env/index.js` | stub | |
| `src/commands/oauth-refresh/index.js` | stub | |
| `src/commands/debug-tool-call/index.js` | stub | |
| `src/commands/reset-limits/index.js` | stub | exports two: `resetLimits`, `resetLimitsNonInteractive` |
| `src/commands/agents-platform/index.js` | **missing entirely** | ANT top-level require at `commands.ts:50` |

`forceSnip`, `ultraplan`, `subscribePr` are feature-flag wrapped — see 21c.

---

## 3. Public Interface (Contract)

### 3.1 `/version` — Print version (ANT-only, sourced)

- Path: `src/commands/version.ts:12-20`
- Kind: `local`
- `isEnabled: () => process.env.USER_TYPE === 'ant'`
- `supportsNonInteractive: true`
- Description: `Print the version this session is running (not what autoupdate downloaded)`
- `load: () => Promise.resolve({ call })` — non-lazy module pattern.
- `call` body (verbatim, version.ts:3-10):
  ```typescript
  const call: LocalCommandCall = async () => {
    return {
      type: 'text',
      value: MACRO.BUILD_TIME
        ? `${MACRO.VERSION} (built ${MACRO.BUILD_TIME})`
        : MACRO.VERSION,
    }
  }
  ```
- `MACRO.VERSION` and `MACRO.BUILD_TIME` are bundle-time constants injected by Bun (per build pipeline; spec 01).

### 3.2 `/bridge-kick` — Inject bridge failure states (ANT-only, sourced)

- Path: `src/commands/bridge-kick.ts:191-200`
- Kind: `local`
- `isEnabled: () => process.env.USER_TYPE === 'ant'`
- `supportsNonInteractive: false`
- Description: `Inject bridge failure states for manual recovery testing`
- `load: () => Promise.resolve({ call })` — non-lazy.
- USAGE constant (verbatim, bridge-kick.ts:40-49):
  ```
  /bridge-kick <subcommand>
    close <code>              fire ws_closed with the given code (e.g. 1002)
    poll <status> [type]      next poll throws BridgeFatalError(status, type)
    poll transient            next poll throws axios-style rejection (5xx/net)
    register fail [N]         next N registers transient-fail (default 1)
    register fatal            next register 403s (terminal)
    reconnect-session fail    next POST /bridge/reconnect fails
    heartbeat <status>        next heartbeat throws BridgeFatalError(status)
    reconnect                 call reconnectEnvironmentWithSession directly
    status                    print bridge state
  ```
- Subcommands (verbatim behavior, bridge-kick.ts:51-188):
  - **No bridge handle**: `'No bridge debug handle registered. Remote Control must be connected (USER_TYPE=ant).'`
  - **`close <code>`**: validates numeric, calls `h.fireClose(code)`. Returns `Fired transport close(${code}). Watch debug.log for [bridge:repl] recovery.`
  - **`poll transient`**: `h.injectFault({method: 'pollForWork', kind: 'transient', status: 503, count: 1})`, `h.wakePollLoop()`. Returns `'Next poll will throw a transient (axios rejection). Poll loop woken.'`.
  - **`poll <status> [type]`**: default `errorType = b ?? (status === 404 ? 'not_found_error' : 'authentication_error')`. Calls `injectFault({method: 'pollForWork', kind: 'fatal', status, errorType, count: 1})` then `wakePollLoop()`. Returns `Next poll will throw BridgeFatalError(${status}, ${errorType}). Poll loop woken.`
  - **`register fatal`**: `injectFault({method: 'registerBridgeEnvironment', kind: 'fatal', status: 403, errorType: 'permission_error', count: 1})`. Returns `'Next registerBridgeEnvironment will 403. Trigger with close/reconnect.'`
  - **`register fail [N]`**: `injectFault({method: 'registerBridgeEnvironment', kind: 'transient', status: 503, count: n})`. Returns `Next ${n} registerBridgeEnvironment call(s) will transient-fail. Trigger with close/reconnect.`
  - **`reconnect-session fail`**: `injectFault({method: 'reconnectSession', kind: 'fatal', status: 404, errorType: 'not_found_error', count: 2})`. Returns `'Next 2 POST /bridge/reconnect calls will 404. doReconnect Strategy 1 falls through to Strategy 2.'`
  - **`heartbeat <status>`**: defaults `status = 401`; errorType `'authentication_error'` if 401 else `'not_found_error'`; `injectFault({method: 'heartbeatWork', kind: 'fatal', status, errorType, count: 1})`. Returns `Next heartbeat will ${status}. Watch for onHeartbeatFatal → work-state teardown.`
  - **`reconnect`**: `h.forceReconnect()`. Returns `'Called reconnectEnvironmentWithSession(). Watch debug.log.'`
  - **`status`**: returns `h.describe()`.
  - **default**: returns `USAGE`.
- Validation: `'close: need a numeric code\n${USAGE}'`, `'poll: need \'transient\' or a status code\n${USAGE}'`.
- Side effects: mutates `getBridgeDebugHandle()` runtime state (in-process, no FS/network).

### 3.3 Stubbed ANT commands (registry-citation only)

Each appears in `INTERNAL_ONLY_COMMANDS` and is added to `COMMANDS()` only when `USER_TYPE === 'ant' && !IS_DEMO`. **Source absent in this leak — replaced by `{ isEnabled: () => false, isHidden: true, name: 'stub' }`.** Reimplementer must reconstruct from external knowledge or runtime observation.

| Command import | Spread token | Probable purpose (from name) |
|---|---|---|
| `backfillSessions` | `backfillSessions` | Backfill missing session metadata into `~/.claude/projects/...`. |
| `breakCache` | `breakCache` | Break / poison the prompt cache (debug). |
| `bughunter` | `bughunter` | Trigger ad-hoc bug-hunter workflow (related to `/ultrareview` remote path, but local invocation). |
| `ctx_viz` | `ctx_viz` | Visualize context contents (likely a richer variant of `/context`). |
| `goodClaude` | `goodClaude` | Reinforcement / praise feedback channel. |
| `issue` | `issue` | Create a GitHub issue from the current session. |
| `mockLimits` | `mockLimits` | Mock API rate limits for testing. |
| `onboarding` | `onboarding` | Force-run onboarding flow. |
| `share` | `share` | Share a session (export to a public URL). |
| `summary` | `summary` | Conversation summary. **Also referenced in `BRIDGE_SAFE_COMMANDS`** (`commands.ts:656`) — the `.filter((c): c is Command => c !== null)` guard handles the stub case. |
| `teleport` | `teleport` | Teleport session to another remote env. |
| `antTrace` | `antTrace` | ANT-internal tracing toggle. |
| `perfIssue` | `perfIssue` | File a performance issue with current state. |
| `env` | `env` | Print/inspect environment. |
| `oauthRefresh` | `oauthRefresh` | Force OAuth refresh. |
| `debugToolCall` | `debugToolCall` | Debug a specific tool-call cycle. |
| `resetLimits`, `resetLimitsNonInteractive` | `resetLimits`, `resetLimitsNonInteractive` | Reset usage/limit counters; dual interactive/non-interactive entries. |
| `autofixPr` | `autofixPr` | Auto-apply fixes to a PR. |
| `agentsPlatform` | top-level require | Agents platform integration (commands.ts:48-51). Source missing entirely from leak. |

`commit`, `commitPushPr`, `initVerifiers` are documented in 21a §3.27, §3.73, §3.74.

`forceSnip`, `ultraplan`, `subscribePr` are documented in 21c.

---

## 4. Data Model & State

For sourced commands:
- `/version` — reads bundle-time MACRO constants, no state mutation.
- `/bridge-kick` — mutates the in-process bridge debug fault-injection queue via `getBridgeDebugHandle()`. No persistent state.

For stubbed commands: unknown.

---

## 5. Algorithm / Control Flow

For each stubbed command, the runtime path is:
1. `isEnabled()` returns `false`.
2. `isCommandEnabled(cmd)` (`types/command.ts:209-216`) returns `false`.
3. `getCommands()` filters it out (20 §5.5).
4. The command is invisible at typeahead and unreachable via `findCommand` lookups (it is, however, present in the in-memory array — just filtered out per call).

Reaching a stubbed command would require external reach (e.g., a programmatic test harness that bypasses `isCommandEnabled`). The stub `name: 'stub'` means EVERY stubbed entry shares the same name — `findCommand('stub', commands)` would match the first one (insertion order: `backfillSessions`).

---

## 6. Verbatim Assets

### 6.1 `INTERNAL_ONLY_COMMANDS` literal (verbatim, `commands.ts:225-254`)

(Reproduced in §1 above.)

### 6.2 ANT-gate at registry (verbatim, `commands.ts:343-345`)

```typescript
...(process.env.USER_TYPE === 'ant' && !process.env.IS_DEMO
  ? INTERNAL_ONLY_COMMANDS
  : []),
```

### 6.3 `agentsPlatform` top-level require (verbatim, `commands.ts:47-52`)

```typescript
/* eslint-disable @typescript-eslint/no-require-imports */
const agentsPlatform =
  process.env.USER_TYPE === 'ant'
    ? require('./commands/agents-platform/index.js').default
    : null
/* eslint-enable @typescript-eslint/no-require-imports */
```

### 6.4 Stub shape (verbatim, exemplar from `src/commands/onboarding/index.js:1`)

```javascript
export default { isEnabled: () => false, isHidden: true, name: 'stub' };
```

---

## 7. Side Effects & I/O

For stubbed commands, none (they never run). For sourced:
- `/version` — none.
- `/bridge-kick` — bridge-debug fault injection (in-process only).

---

## 8. Feature Flags & Variants

| Site | Flag | Effect |
|---|---|---|
| `commands.ts:343` | `USER_TYPE === 'ant' && !IS_DEMO` | Whole `INTERNAL_ONLY_COMMANDS` list spread |
| `commands.ts:50` | `USER_TYPE === 'ant'` | `agentsPlatform` top-level require resolves |
| `INTERNAL_ONLY_COMMANDS` spread | `HISTORY_SNIP` | adds `forceSnip` (see 21c) |
| `INTERNAL_ONLY_COMMANDS` spread | `ULTRAPLAN` | adds `ultraplan` (see 21c) |
| `INTERNAL_ONLY_COMMANDS` spread | `KAIROS_GITHUB_WEBHOOKS` | adds `subscribePr` (see 21c) |

`IS_DEMO` is read at `commands.ts:343`. When set, the entire ANT spread is suppressed even for ANT users — used for demo/screenshot recordings.

---

## 9. Error Handling & Edge Cases

- `agentsPlatform` require fails when not ANT → `null`; `null` filtered out by `.filter(Boolean)` at `commands.ts:254`.
- Stubbed commands: every one named `'stub'` → `findCommand('stub')` is ambiguous but only one ever matches because `isCommandEnabled` filters them all out before `findCommand` runs (per `getCommands()` flow, 20 §5.5).
- `/bridge-kick`: subcommand validation falls through to `USAGE` print on unknown subcommand or missing args.
- `/version`: no failure modes (reads constants).

---

## 10. Telemetry & Observability

For stubbed: none.
For sourced:
- `/bridge-kick` does NOT emit analytics (debug command).
- `/version` does NOT emit analytics.

---

## 11. Reimplementation Checklist

- [ ] `INTERNAL_ONLY_COMMANDS` is a `.filter(Boolean)` array — handles `null` from feature-gated `forceSnip`/`ultraplan`/`subscribePr`.
- [ ] ANT spread guard is `USER_TYPE === 'ant' && !IS_DEMO`. The `IS_DEMO` exclusion is intentional.
- [ ] `agentsPlatform` is a top-level conditional `require()`, NOT a `feature(...)` gate. Bun's DCE may NOT strip it because the gate is `process.env`. Document for build expectations.
- [ ] `/version` uses bundle-time `MACRO.VERSION` and `MACRO.BUILD_TIME` constants — these must be replaced at build time.
- [ ] `/bridge-kick` is non-lazy (`Promise.resolve({ call })`); preserve the inline pattern.
- [ ] `/bridge-kick` switch ordering matters for fault-injection semantics (e.g., `register fatal` before generic `register fail [N]`).
- [ ] All stubbed commands MUST be implemented from external knowledge (not present in this leak).

---

## 12. Open Questions / Unknowns

1. **All ~17 stubbed ANT commands** — source absent. Reconstructing requires either an unredacted internal build or runtime probing (set `USER_TYPE=ant`, observe behavior).
2. **`agentsPlatform` source missing** — even the index file is absent at `src/commands/agents-platform/index.js`. The require at `commands.ts:50` would throw at runtime in non-ANT environments where the file is required (it isn't — gate stops require evaluation).
3. **`summary` in `BRIDGE_SAFE_COMMANDS`** — the actual reference resolves to a stub in this leak. The bridge-safe set's `.filter((c): c is Command => c !== null)` guard tolerates it, but a stub-named-`stub` may still match `findCommand('stub')`. Investigate whether bridge inbound paths handle this.
4. **`forceSnip` / `ultraplan` / `subscribePr`** — feature-gated AND ANT-gated. Source presence:
   - `force-snip.ts` — source missing.
   - `ultraplan.tsx` — source present at `src/commands/ultraplan.tsx` (compiled). Spec 21c documents.
   - `subscribe-pr.ts` — source missing.
5. **`IS_DEMO` semantics** — used here and in screenshot/demo flows. Spec 02 (settings) or 26 (analytics) to enumerate.
6. **`MACRO.VERSION` / `MACRO.BUILD_TIME` injection mechanism** — Bun macro plugin or define replacement. Spec 01 (entrypoint) to confirm.
