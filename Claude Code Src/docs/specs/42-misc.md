# 42 — Long-Tail / Catch-All (Miscellaneous Residuals)

> **Owner**: sub-H2 · **Status**: done · **Last updated**: 2026-05-10
> **Adjacent**: 00 (ownership map), 01 (CLI residual), 09/10 (permissions / shell trust-boundary), 14/30 (coordinator/swarm), 16 (tool-mcp-lsp / computerUse), 20 (command system), 22 (api / model registry), 35 (remote-server delegates sandbox here), 37 (Ink UI), 41 (state)
>
> Catch-all spec for `src/` paths not claimed by 00..41. Phase 9 coverage audit will reduce this further. Authoritative ownership map: `00-overview.md` §2.2.

---

## 1. Purpose & Scope

This spec covers the residual long-tail of the leaked tree that does not fit cleanly into any of the 41 layered specs:

- **Sandbox subsystem (delegated from 35 §1, §13)** — `src/utils/sandbox/sandbox-adapter.ts` (985 lines; wraps `@anthropic-ai/sandbox-runtime`'s `SandboxManager`), `src/utils/sandbox/sandbox-ui-utils.ts` (12 lines), the `/sandbox-toggle` slash command (`src/commands.ts:149`, `src/commands/sandbox-toggle/{index.ts,sandbox-toggle.tsx}`, 50+82 lines), and the `SandboxManager` boot wiring in `src/main.tsx:201, 314-316`. See §X below. Spec 35's `dangerouslySkipPermissions` plumbing (§4.2 / §5.5 / §9) is independent of sandbox state — `dangerouslySkipPermissions` is a **per-session permission-bypass flag** passed to the server, NOT a sandbox-state mutator (per `35:18` cross-ref).
- `src/buddy/` — `BUDDY` Easter-egg companion library (sprite + companion roll + intro attachment + teaser notification + trigger detection). The `/buddy` slash command source itself is **missing-leaked-source** (registry-only reference owned by 21c §3.15).
- `src/upstreamproxy/` — CCR session-container-side CONNECT-over-WebSocket proxy + relay.
- `src/native-ts/` — pure-TypeScript ports of three native (Rust NAPI / WASM) modules: `color-diff/`, `file-index/`, `yoga-layout/`.
- `src/moreright/` — external-build stub for an internal-only `useMoreRight` hook.
- `src/assistant/sessionHistory.ts` — Anthropic-server-stored session-events history page fetcher (used by ANT-only `/assistant` command and Kairos modes; not owned by 41 because 41 documents local state/history).
- `src/cli/exit.ts`, `src/cli/ndjsonSafeStringify.ts` — CLI residual not absorbed by 01 (`cliError`/`cliOk` exit helpers, NDJSON U+2028/U+2029 escape).
- `src/services/preventSleep.ts`, `src/services/vcr.ts` — recommended-42 services per `00-overview.md` §2.3.
- `src/types/generated/` — `bun-protobuf-gen` output for `events_mono` (`claude_code/v1`, `common/v1`, `growthbook/v1`) and `google/protobuf/timestamp.ts`. Used by 26 (analytics) but not enumerated there.
- Survey hooks for `src/constants/`, `src/utils/` (residual after tool/service specs claim their own files).

**OUT of scope** (explicitly owned by another spec — cited, not redocumented):

| Residual | Owner |
|---|---|
| `services/AgentSummary/`, `services/awaySummary.ts`, `services/toolUseSummary/`, `services/autoDream/` | 30 (coordinator) |
| `services/tips/`, `services/MagicDocs/`, `services/PromptSuggestion/` | 38 (output styles) and/or 26 (analytics) — not claimed by 38's title scope; flagged below as **Phase 9 escalation** |
| `services/claudeAiLimits.ts`, `services/claudeAiLimitsHook.ts`, `services/rateLimit{Messages,Mocking}.ts`, `services/mockRateLimits.ts` | 27 (policy) or 22 (api) — owner unverified at write time; **Phase 9 escalation** |
| `services/diagnosticTracking.ts`, `services/internalLogging.ts`, `services/notifier.ts` | 26 (analytics/observability) |
| `src/tools/testing/TestingPermissionTool.tsx` | 19 §3.x and §3.6 (verified `19-tool-misc.md:28`, `:71`, `:110`) |
| `src/types/command.ts` | 20 |
| `src/types/permissions.ts`, `src/types/hooks.ts` | 09 |
| `src/types/ids.ts` | 14 / 41 |
| `src/types/logs.ts` | 26 |
| `src/types/plugin.ts` | 28 |
| `src/types/textInputTypes.ts` | 37 |
| `src/cli/print.ts`, `src/cli/structuredIO.ts`, `src/cli/remoteIO.ts`, `src/cli/update.ts`, `src/cli/handlers/`, `src/cli/transports/` | 01 |
| `src/buddy/CompanionSprite.tsx`, `src/buddy/sprites.ts` (Ink UI surface) | 37 (Ink UI shell renders the sprite) — but the file lives under `src/buddy/`; 37 cites, 42 inventories |
| `src/assistant/` interactions in turn pipeline | 04 / 32 |
| `src/utils/bash/`, `src/utils/shell/`, `src/utils/powershell/` (~28 files; permission-engine-adjacent) | 09 / 10 — co-owned at architectural level; see "co-ownership" below |
| `src/utils/swarm/` (~21 files) | 14 (tasks) / 30 (coordinator) — co-owned per 42a §5 |
| `src/utils/computerUse/` (~16 files) | 16 (tool-mcp-lsp) — co-owned per 42a §5 |
| `src/utils/model/` (~17 files; model registry) | 22 (api) — co-owned per 42a §5 |

#### Co-ownership at architectural level (per 42a §5 reassignment map)

Phase 9.5 / 42a §5 found that ~23% of `src/utils/`'s file-count belongs with consuming subsystem specs rather than this catch-all. Spec 42 acknowledges these architectural-level co-ownership boundaries:

| Subsystem | File count | Architectural owner | Co-ownership note |
|---|---:|---|---|
| `bash/` + `shell/` + `powershell/` | 28 (≈9/10/9) | 09 / 10 (permissions) | Includes `bash/ast.ts` shell trust-boundary (see below). 42 inventories the directory tree at survey level only. |
| `swarm/` | 21 | 14 (tasks) / 30 (coordinator) | 30 §3 owns swarm orchestration; 42 cite-only. (Asymmetry with §1 OUT-of-scope `services/AgentSummary/` resolved by this entry.) |
| `computerUse/` | 16 | 16 (tool-mcp-lsp) | Tool-side computer-use input handling. |
| `model/` | 17 | 22 (api) | Model registry / per-model overrides; consumed by api auth + request shaping. |

**Shell trust-boundary** — `src/utils/bash/ast.ts` (2,679 lines, 112KB) is the single largest trust-boundary in the entire `src/utils/` tree. The file header is unambiguous:

> "The key design property is FAIL-CLOSED: we never interpret structure we don't understand. … This is NOT a sandbox. It does not prevent dangerous commands from running. It answers exactly one question: 'Can we produce a trustworthy argv[] for…'"

Architectural ownership: **spec 09 / 10 (permission engine)**. Spec 42 cite-only. This is distinct from §X (sandbox subsystem) — `bash/ast.ts` is a parser-level trust-boundary used by the permission engine to decide whether a shell string can be safely matched against allow/deny rules; sandbox is a runtime fs/network capability boundary owned by `@anthropic-ai/sandbox-runtime`. The two boundaries compose but are independent.

### Source-coverage inventory

| Path | Status |
|---|---|
| `src/buddy/companion.ts` | **owned-here** |
| `src/buddy/CompanionSprite.tsx` | inventoried-here, render owner = 37 (45922 bytes; not inlined) |
| `src/buddy/prompt.ts` | **owned-here** (verbatim §6) |
| `src/buddy/sprites.ts` | inventoried-here, render owner = 37 (sprite frame data) |
| `src/buddy/types.ts` | **owned-here** (verbatim §6) |
| `src/buddy/useBuddyNotification.tsx` | **owned-here** (verbatim §6) |
| `src/upstreamproxy/upstreamproxy.ts` | **owned-here** |
| `src/upstreamproxy/relay.ts` | **owned-here** |
| `src/native-ts/color-diff/index.ts` | **owned-here** (vendor port) |
| `src/native-ts/file-index/index.ts` | **owned-here** (vendor port) |
| `src/native-ts/yoga-layout/index.ts` | **owned-here** (vendor port; large, summarized) |
| `src/native-ts/yoga-layout/enums.ts` | **owned-here** (verbatim §6) |
| `src/moreright/useMoreRight.tsx` | **owned-here** (external-build stub) |
| `src/assistant/sessionHistory.ts` | **owned-here** (no 41 §2 entry; not local-history) |
| `src/cli/exit.ts` | **owned-here** (overflow from 01) |
| `src/cli/ndjsonSafeStringify.ts` | **owned-here** (overflow from 01) |
| `src/services/preventSleep.ts` | **owned-here** (per 00 §2.3) |
| `src/services/vcr.ts` | **owned-here** (per 00 §2.3) |
| `src/types/generated/google/protobuf/timestamp.ts` | **owned-here** |
| `src/types/generated/events_mono/{claude_code,common,growthbook}/v1/` | **owned-here** (analytics protobuf — citation only; consumed by 26) |
| `src/services/{tips,MagicDocs,PromptSuggestion}/` | **unresolved** — see §12 |
| `src/services/{claudeAiLimits.ts,claudeAiLimitsHook.ts,rateLimitMessages.ts,rateLimitMocking.ts,mockRateLimits.ts}` | **unresolved** — see §12 |
| `src/utils/sandbox/sandbox-adapter.ts` | **owned-here** (§X; delegated from 35) |
| `src/utils/sandbox/sandbox-ui-utils.ts` | **owned-here** (§X; delegated from 35) |
| `src/commands/sandbox-toggle/{index.ts,sandbox-toggle.tsx}` | **owned-here** (§X; delegated from 35) |
| `src/utils/` (327 source files; `ls` returns 329 = 327 + `.DS_Store` + `specs/` subdir entry) | survey-only; per-file owners belong to consuming tool/service specs; residuals after Phase 9 → §12. Reconciled with 42a §0 (327 source files). |
| `src/utils/ansiToPng.ts` (215KB) | **embedded asset, not source** — file header: "Render ANSI-escaped terminal text directly to a PNG image. Replaces the previous ansiToSvg → @resvg/resvg-wasm pipeline." Bulk of bytes is a bundled 24×48 Fira Code bitmap-font rasterization (regenerated by `scripts/generate-bitmap-font.ts`). LOC totals over `src/utils/` should subtract this from any "source LOC" claim. |
| `src/utils/bash/ast.ts` (2,679 lines, 112KB) | architectural trust-boundary; cite-only here, owned by 09/10 (see "co-ownership" above) |
| `src/constants/` (~21 files) | survey-only; cite-only — strings/regexes belong to consuming specs |

---

## 2. Source Map

| File | Lines | Notes |
|---|---|---|
| `src/buddy/types.ts` | 1–149 | Static enums (`RARITIES`, `SPECIES`, `EYES`, `HATS`, `STAT_NAMES`, `RARITY_WEIGHTS`, `RARITY_STARS`, `RARITY_COLORS`); `Companion`, `CompanionBones`, `CompanionSoul`, `StoredCompanion` types. Species names hex-encoded via `String.fromCharCode` to keep one species' literal out of the bundle (codename canary collision; comment lines 10–13). |
| `src/buddy/companion.ts` | 1–134 | `mulberry32` PRNG, `hashString` (Bun.hash → BigInt mask, fallback FNV-1a-ish), `pick`, `rollRarity` (weighted), `RARITY_FLOOR` table, `rollStats` (peak+dump+scatter), `SALT = 'friend-2026-401'`, `roll`, `rollWithSeed`, `companionUserId`, `getCompanion` (bones regenerated from `userId` on every read; merged after stored soul). |
| `src/buddy/prompt.ts` | 1–37 | `companionIntroText(name, species)`; `getCompanionIntroAttachment(messages)` — early-return on `!feature('BUDDY')`, no companion, `companionMuted`, or already-announced. Returns `[{ type: 'companion_intro', name, species }]`. |
| `src/buddy/useBuddyNotification.tsx` | 1–97 | `isBuddyTeaserWindow()` (Apr 1–7 2026 local-date); `isBuddyLive()` (≥ Apr 2026 local-date); both short-circuit `true` when `USER_TYPE === 'ant'`. `useBuddyNotification` adds rainbow `/buddy` notification keyed `buddy-teaser`, `priority: 'immediate'`, `timeoutMs: 15_000`, gated by `feature('BUDDY')` + `!config.companion` + teaser window. `findBuddyTriggerPositions(text)` returns ranges where `/\/buddy\b/g` matches (gated by `feature('BUDDY')`). |
| `src/buddy/CompanionSprite.tsx` | (45 KB) | Inventoried only; Ink renderer. Owner = 37. |
| `src/buddy/sprites.ts` | (10 KB) | Sprite frame data. Owner = 37. |
| `src/upstreamproxy/upstreamproxy.ts` | 1–286 | Container init module: `SESSION_TOKEN_PATH = '/run/ccr/session_token'`, `SYSTEM_CA_BUNDLE = '/etc/ssl/certs/ca-certificates.crt'`, `NO_PROXY_LIST` (15 entries; see §6). `initUpstreamProxy(opts)`: gated by `CLAUDE_CODE_REMOTE` truthy AND `CCR_UPSTREAM_PROXY_ENABLED` truthy AND `CLAUDE_CODE_REMOTE_SESSION_ID` set AND token file readable. Then: `setNonDumpable()` (Linux+Bun, `prctl(PR_SET_DUMPABLE=4, 0)` via `bun:ffi`), download `${baseUrl}/v1/code/upstreamproxy/ca-cert` (5s timeout), concat `system_ca + '\n' + ccrCa` to `~/.ccr/ca-bundle.crt`, start relay, `unlink(tokenPath)` only after listener up. `getUpstreamProxyEnv()`: returns subprocess env vars `HTTPS_PROXY/https_proxy/NO_PROXY/no_proxy/SSL_CERT_FILE/NODE_EXTRA_CA_CERTS/REQUESTS_CA_BUNDLE/CURL_CA_BUNDLE`; if state disabled but parent has both `HTTPS_PROXY` and `SSL_CERT_FILE`, inherits 8 named keys from `process.env`. `resetUpstreamProxyForTests()` test-only. Fails open: every error becomes `logForDebugging` warning + `state = { enabled: false }`. |
| `src/upstreamproxy/relay.ts` | 1–456 | CONNECT-over-WebSocket TCP relay. Constants: `MAX_CHUNK_BYTES = 512 * 1024`, `PING_INTERVAL_MS = 30_000`. `encodeChunk(data)` / `decodeChunk(buf)` hand-encode `UpstreamProxyChunk { bytes data = 1 }` protobuf (tag `0x0a` + varint length + bytes). `startUpstreamProxyRelay({wsUrl, sessionId, token})` builds `Basic`-base64 `authHeader` from `sessionId:token`, separate `Bearer ${token}` `wsAuthHeader` for WS upgrade. Bun path: `Bun.listen` on `127.0.0.1:0`, manual partial-write tail-queueing (`writeBuf`) via drain. Node path: dynamic `import('ws')`, internal buffering. Both share `handleData`/`openTunnel`/`forwardToWs`/`cleanupConn`. CONNECT parser: accumulates until `\r\n\r\n` (max 8192 then `400 Bad Request`); first line must match `/^CONNECT\s+(\S+)\s+HTTP\/1\.[01]$/i` else `405 Method Not Allowed`. WS upgrade headers `Content-Type: application/proto` + `Authorization: Bearer ${token}`. First chunk over WS contains `${connectLine}\r\nProxy-Authorization: ${authHeader}\r\n\r\n`. Keepalive: zero-length encoded chunk every 30s. `502 Bad Gateway` on ws error before `established`; otherwise just close. |
| `src/native-ts/color-diff/index.ts` | 1–999 | TS port of vendor Rust syntect+similar diff highlighter. Public: `ColorDiff`, `ColorFile`, `getSyntaxTheme`, `getNativeModule`. Constants: `RESET = '\x1b[0m'`, `DIM = '\x1b[2m'`, `UNDIM = '\x1b[22m'`, `CHANGE_THRESHOLD = 0.4`, `CUBE_LEVELS = [0, 95, 135, 175, 215, 255]`, `DEFAULT_BG = { r:0,g:0,b:0,a:1 }`. Theme tables `MONOKAI_SCOPES` (24 RGB triples), `GITHUB_SCOPES` (24 RGB triples), `ANSI_SCOPES` (10 indices); `STORAGE_KEYWORDS` set of 17 strings; `FILENAME_LANGS` map (5 entries). `defaultSyntaxThemeName`: 'ansi' if name includes 'ansi'; 'Monokai Extended' if 'dark'; else 'GitHub'. Syntax via lazy-loaded `highlight.js` (top-level lazy `cachedHljs`). Word diff via `diffArrays` from `diff` npm package; tokenizer splits on `\p{L}\p{N}_` runs / whitespace runs / single codepoints. |
| `src/native-ts/file-index/index.ts` | 1–370 | TS port of vendor Rust `FileIndex`/nucleo. Constants: `SCORE_MATCH=16`, `BONUS_BOUNDARY=8`, `BONUS_CAMEL=6`, `BONUS_CONSECUTIVE=4`, `BONUS_FIRST_CHAR=8`, `PENALTY_GAP_START=3`, `PENALTY_GAP_EXTENSION=1`, `TOP_LEVEL_CACHE_LIMIT=100`, `MAX_QUERY_LEN=64`, `CHUNK_MS=4`. Smart-case (lower → case-insensitive; any uppercase → case-sensitive). a–z bitmap reject (O(1)). Test paths get `Math.min(positionScore * 1.05, 1.0)` penalty. Async build yields every ~256 iters when `performance.now() - chunkStart > CHUNK_MS`. `loadFromFileListAsync` returns `{queryable, done}` promises. Path separator detection: `/` (47) or `\` (92). Boundary chars: `/ \ - _ . space`. |
| `src/native-ts/yoga-layout/index.ts` | (~36 KB) | Pure-TS port of `yoga-layout/src/generated/Yoga.ts`. Inventoried; not inlined verbatim due to size. Used by Ink layout (37). |
| `src/native-ts/yoga-layout/enums.ts` | 1–135 | All `Align/BoxSizing/Dimension/Direction/Display/Edge/Errata/ExperimentalFeature/FlexDirection/Gutter/Justify/MeasureMode/Overflow/PositionType/Unit/Wrap` enum maps. Numeric values match upstream yoga-layout exactly. |
| `src/moreright/useMoreRight.tsx` | 1–25 | External-build stub. The real hook is internal-only; this stub returns `{ onBeforeQuery: async () => true, onTurnComplete: async () => {}, render: () => null }`. Header comment: "Self-contained: no relative imports. Typecheck sees this file at scripts/external-stubs/src/moreright/ before overlay". |
| `src/assistant/sessionHistory.ts` | 1–88 | `HISTORY_PAGE_SIZE = 100`. Types: `HistoryPage`, `SessionEventsResponse`, `HistoryAuthCtx`. `createHistoryAuthCtx(sessionId)` constructs base URL `${getOauthConfig().BASE_API_URL}/v1/sessions/${sessionId}/events`, headers: `getOAuthHeaders(accessToken) + 'anthropic-beta': 'ccr-byoc-2025-07-29' + 'x-organization-uuid': orgUUID`. `fetchPage`: axios GET, 15s timeout, `validateStatus: () => true`, returns null on non-200. `fetchLatestEvents(limit=100)` uses `anchor_to_latest: true`. `fetchOlderEvents(beforeId, limit=100)` uses `before_id` cursor. |
| `src/cli/exit.ts` | 1–32 | `cliError(msg?): never` — `console.error` then `process.exit(1)` then `return undefined as never`. `cliOk(msg?): never` — `process.stdout.write(msg + '\n')` then `process.exit(0)`. Header explains `return undefined as never` (not post-exit throw) is for tests that spy on `process.exit`. `console.error` (not `console.log`) so tests' `console.error` spy works; `process.stdout.write` (not `console.log`) because Bun's `console.log` doesn't route through spied `process.stdout.write`. |
| `src/cli/ndjsonSafeStringify.ts` | 1–32 | Regex `JS_LINE_TERMINATORS = / \| /g`. `ndjsonSafeStringify(value)` replaces U+2028/U+2029 with `\\u2028`/`\\u2029` post-`jsonStringify`. Header: ECMA-262 §11.3 line terminators include U+2028/U+2029; ProcessTransport silently skips non-JSON lines (gh-28405). |
| `src/services/preventSleep.ts` | 1–166 | `CAFFEINATE_TIMEOUT_SECONDS = 300`, `RESTART_INTERVAL_MS = 4 * 60 * 1000`. Refcount-based `startPreventSleep()`/`stopPreventSleep()`/`forceStopPreventSleep()`. Spawns `caffeinate -i -t 300` on macOS only (no-op elsewhere). `setInterval.unref()`, child `unref()`. Self-healing: caffeinate auto-exits after 5 min if Node SIGKILLed; restart at 4 min. Cleanup-registered on first use (`registerCleanup(forceStopPreventSleep)`). `killCaffeinate` uses `SIGKILL`. |
| `src/services/vcr.ts` | 1–406 | `shouldUseVCR()`: `NODE_ENV === 'test'` OR (`USER_TYPE === 'ant'` AND `FORCE_VCR` truthy). `withFixture<T>(input, fixtureName, f)`: SHA-1 of `jsonStringify(input)`, `.slice(0,12)`, fixture path `${CLAUDE_CODE_TEST_FIXTURES_ROOT ?? cwd}/fixtures/${fixtureName}-${hash}.json`. `withVCR(messages, f)`: filter `_.type==='user' && _.isMeta`, dehydrate via `mapMessages`+`dehydrateValue`, fixture filename = first-6 hex of each input message hash joined by `-`. CI without `VCR_RECORD` → throws "Fixture missing". `dehydrateValue`: replace `num_files="\d+"` → `[NUM]`, `duration_ms="\d+"` → `[DURATION]`, `cost_usd="\d+"` → `[COST]`; replace cwd → `[CWD]`, configHome → `[CONFIG_HOME]`; Windows handles forward-slash + JSON-escaped variants; `Available commands:.+` → `Available commands: [COMMANDS]`; `Files modified by user:` → `Files modified by user: [FILES]`. `hydrateValue` reverses. `withTokenCountVCR`: dehydrate input + replace cwd-slug, UUIDs → `[UUID]`, ISO-timestamps → `[TIMESTAMP]`. Streaming variant `withStreamingVCR`. |
| `src/types/generated/google/protobuf/timestamp.ts` | (single file) | Generated protobuf timestamp wire type. Used by analytics (26). |
| `src/types/generated/events_mono/{claude_code,common,growthbook}/v1/` | (subdirs) | Generated protobuf type definitions for the analytics event-mono pipeline. Owner-of-consumption = 26. |

---

## 3. Public Interface — N/A for most of this catch-all

Per-file public surface is recorded inline in §2 and §6. No unifying interface — the spec is a coverage receipt, not a subsystem.

---

## 4. Data Model & State

| Type | Where | Notes |
|---|---|---|
| `Companion`, `CompanionBones`, `CompanionSoul`, `StoredCompanion` | `src/buddy/types.ts:101-124` | `Bones` regenerated from `hash(userId)`; only `Soul + hatchedAt` persist. |
| `Roll` | `src/buddy/companion.ts:86-89` | `{ bones, inspirationSeed }`. `rollCache` is a single-slot `userId+SALT`-keyed module-level cache (line 106). |
| `UpstreamProxyState` | `src/upstreamproxy/upstreamproxy.ts:65-71` | `{ enabled, port?, caBundlePath? }` module singleton. |
| `ConnState` / `BunState` | `src/upstreamproxy/relay.ts:110-148` | Per-TCP-connection FSM: `connectBuf`, `pending[]`, `wsOpen`, `established`, `closed`, optional `ws`, `pinger`. |
| `Hunk`, `SyntaxTheme`, `NativeModule`, `Color`, `Style`, `Block`, `ColorMode`, `Theme`, `Marker`, `Highlight`, `Range`, `HljsNode` | `src/native-ts/color-diff/index.ts` | Public + internal types verbatim. |
| `SearchResult` | `src/native-ts/file-index/index.ts:18-21` | `{ path: string; score: number }` (lower = better). |
| `Align` ... `Wrap` | `src/native-ts/yoga-layout/enums.ts:7-134` | 16 enum maps; values match upstream yoga-layout. |
| `HistoryPage`, `SessionEventsResponse`, `HistoryAuthCtx` | `src/assistant/sessionHistory.ts:9-28` | Anthropic-server session-events page model. |

Module-level singletons (must NOT be reset across hot reload without test-only resets): `rollCache` (`buddy/companion.ts`), `state` (`upstreamproxy/upstreamproxy.ts:71`), `caffeinateProcess`/`restartInterval`/`refCount`/`cleanupRegistered` (`services/preventSleep.ts:27-30`), `cachedHljs`, `cachedModule`, `loggedEmitterShapeError` (`native-ts/color-diff/`), `nodeWSCtor` (`upstreamproxy/relay.ts:32`).

---

## 5. Algorithms

### 5.1 Companion roll (deterministic, per-user)

`src/buddy/companion.ts:107-113` — `roll(userId)`:

1. `key = userId + SALT` (`SALT = 'friend-2026-401'`, line 84).
2. If `rollCache?.key === key`, return cached `Roll` (single-slot cache; comment lines 104–106: called from 500ms sprite tick, per-keystroke `PromptInput`, per-turn observer with same userId).
3. Else `rollFrom(mulberry32(hashString(key)))`.

`rollRarity(rng)` — sum of `RARITY_WEIGHTS = { common:60, uncommon:25, rare:10, epic:4, legendary:1 }` (total 100), iterate `RARITIES = ['common','uncommon','rare','epic','legendary']` subtracting weights from `rng()*total`. Default `'common'` if all subtractions don't trigger.

`rollStats(rng, rarity)` — `RARITY_FLOOR = { common:5, uncommon:15, rare:25, epic:35, legendary:50 }`. Pick one `peak` and one `dump` (different) from `STAT_NAMES = ['DEBUGGING','PATIENCE','CHAOS','WISDOM','SNARK']`. For each stat:
- `peak`: `min(100, floor + 50 + floor(rng()*30))`
- `dump`: `max(1, floor - 10 + floor(rng()*15))`
- other: `floor + floor(rng()*40)`

`rollFrom`: `rarity`, `species` ∈ `SPECIES`, `eye` ∈ `EYES`, `hat` = `'none'` if rarity==='common' else pick from `HATS`, `shiny = rng() < 0.01`, `stats`, `inspirationSeed = floor(rng()*1e9)`.

`getCompanion()` — read `getGlobalConfig().companion` (`StoredCompanion | undefined`), regenerate `bones` from `companionUserId()` (oauth `accountUuid` ?? `userID` ?? `'anon'`), spread `{...stored, ...bones}` (bones last so legacy stored bones get overridden).

`hashString(s)`: if `Bun !== undefined`, return `Number(BigInt(Bun.hash(s)) & 0xffffffffn)`; else `h = 2166136261`; for each char `h ^= charCodeAt(i); h = Math.imul(h, 16777619)`; `return h >>> 0`.

`mulberry32(seed)`: standard Mulberry32 PRNG; returns `() => number` in `[0, 1)`.

### 5.2 Buddy teaser window

`src/buddy/useBuddyNotification.tsx:12-21` — local-date (NOT UTC; comment 8–10: rolling 24h wave).

- `isBuddyTeaserWindow()`: `USER_TYPE==='ant'` short-circuits true (string-compared as `"external" === 'ant'` post-build-replacement, line 13). Else: year=2026 AND month=3 (April 0-indexed) AND date≤7.
- `isBuddyLive()`: ANT short-circuit true. Else year>2026 OR (year=2026 AND month≥3).

`useBuddyNotification` (lines 43–78): no-op unless `feature('BUDDY')`. Skip if `getGlobalConfig().companion` exists or not in teaser window. Otherwise add notification `{ key:'buddy-teaser', jsx:<RainbowText text='/buddy'/>, priority:'immediate', timeoutMs: 15000 }`. Cleanup removes `buddy-teaser`.

`findBuddyTriggerPositions(text)` (lines 79–97): regex `/\/buddy\b/g`, returns `[{start, end}]`; gated by `feature('BUDDY')`.

### 5.3 Upstream proxy boot

`src/upstreamproxy/upstreamproxy.ts:79-153` — initUpstreamProxy ordered preconditions (each on failure returns `{enabled:false}` and logs):

1. `isEnvTruthy(CLAUDE_CODE_REMOTE)` (line 85)
2. `isEnvTruthy(CCR_UPSTREAM_PROXY_ENABLED)` (line 92; comment 88–93: GB checked server-side and injected via `StartupContext.EnvironmentVariables`)
3. `process.env.CLAUDE_CODE_REMOTE_SESSION_ID` set
4. `readToken(tokenPath)` (`fs.readFile`, `.trim()`, returns null on `ENOENT`)
5. `setNonDumpable()` (`prctl(PR_SET_DUMPABLE=4, 0n, 0n, 0n, 0n)` via `bun:ffi.dlopen('libc.so.6')`); Linux+Bun only; failure is non-fatal
6. `baseUrl = opts.ccrBaseUrl ?? ANTHROPIC_BASE_URL ?? 'https://api.anthropic.com'` (NOT `getOauthConfig()` — comment 113–116: that path was wrong because container env doesn't carry `USER_TYPE`/`USE_*_OAUTH`)
7. `caBundlePath = opts.caBundlePath ?? join(homedir(), '.ccr', 'ca-bundle.crt')`
8. `downloadCaBundle(baseUrl, systemCaPath, outPath)`: `fetch('${base}/v1/code/upstreamproxy/ca-cert', AbortSignal.timeout(5000))`; on success concat `systemCa + '\n' + ccrCa` into `outPath`
9. `wsUrl = baseUrl.replace(/^http/, 'ws') + '/v1/code/upstreamproxy/ws'`
10. `startUpstreamProxyRelay({wsUrl, sessionId, token})`
11. `registerCleanup(async () => relay.stop())`
12. `state = { enabled:true, port, caBundlePath }`
13. `unlink(tokenPath).catch(...)` ONLY after listener up (comment 138–141: supervisor restart can retry if earlier steps fail)

### 5.4 Relay CONNECT state machine

`src/upstreamproxy/relay.ts:295-342` — phase 1 accumulate to `\r\n\r\n` (max 8192 → `400 Bad Request`), parse first line vs `/^CONNECT\s+(\S+)\s+HTTP\/1\.[01]$/i` (else `405`), stash trailing bytes into `pending`. Phase 2: if `wsOpen` flush `forwardToWs` else buffer.

`openTunnel` (lines 344–428): WS upgrade with `Content-Type: application/proto` + `Authorization: Bearer ${token}`; on Node use `nodeWSCtor` with `agent: getWebSocketProxyAgent(wsUrl)` + TLS opts; on Bun use `globalThis.WebSocket` with `proxy:` + `tls:` (Bun extensions). On `onopen`: send `${connectLine}\r\nProxy-Authorization: ${authHeader}\r\n\r\n` as encoded chunk, set `wsOpen=true`, flush `pending`, start 30s `setInterval(sendKeepalive, 30000, ws)`. `onmessage`: decode chunk; if non-empty payload → `established=true` and forward to client. `onerror`: write `502 Bad Gateway` only if not yet `established`; close socket, cleanup. `onclose`: close socket, cleanup. `forwardToWs`: chunk into 512KB slices, send each as encoded chunk.

Bun-only nuance: manual partial-write tail-queueing in `data`/`drain` handlers (`writeBuf`); Bun's `sock.write()` returns kernel-accepted byte count and silently drops the rest (comment lines 181–185).

### 5.5 file-index search

`src/native-ts/file-index/index.ts:173-290`:

1. `if (limit <= 0) return []`. Empty query → `topLevelCache.slice(0, limit)` (top-level segments by length asc then alpha asc, max `TOP_LEVEL_CACHE_LIMIT=100`).
2. Smart case: `caseSensitive = query !== query.toLowerCase()`; `nLen = min(needle.length, MAX_QUERY_LEN=64)`.
3. Build `needleBitmap` over a–z (codes 97–122).
4. For each indexed path `i < readyCount`:
   - O(1) bitmap reject: `(charBits[i] & needleBitmap) !== needleBitmap` skip.
   - Greedy `indexOf` scan; record positions in `posBuf` (Int32Array(64)); `gap === 0` → `+BONUS_CONSECUTIVE`; else `+= PENALTY_GAP_START + gap*PENALTY_GAP_EXTENSION`.
   - Gap-bound reject: if top-K full and `scoreCeiling + consecBonus - gapPenalty <= threshold`, skip.
   - Boundary/camel pass: `scoreBonusAt(path, posBuf[0], first=true)` then j=1..nLen-1 with `first=false`. Final `+= max(0, 32 - (hLen >> 2))`.
   - Top-K maintenance via binary-search insertion + shift.
5. Sort top-K descending. `denom = max(matchCount, 1)`; `positionScore = i / denom`; if `path.includes('test')` → `min(positionScore * 1.05, 1.0)`.

### 5.6 color-diff render pipeline

`src/native-ts/color-diff/index.ts:842-933` — per-line:
1. Parse marker (first char), rest is `code`.
2. Word-diff ranges via `findAdjacentPairs` over markers + `wordDiffStrings` (skipped when `dim`).
3. `tokens` = `[[defaultStyle, code]]` for `-` lines (no syntax highlight), else `highlightLine`.
4. `removeNewlines → applyBackground → wrapText → (dimContent if mode==='ansi' && marker==='-') → addMarker → addLineNumber → intoLines`.

`wordDiffStrings` rejects diffs where `changedLen / totalLen > CHANGE_THRESHOLD (0.4)`.

### 5.7 VCR fixture key derivation

`src/services/vcr.ts:88-161` — `withVCR`:
1. Filter user messages with `_.isMeta === true` out of API normalization.
2. Map content tree, dehydrate strings.
3. Fixture filename = `${root}/fixtures/${dehydrated.map(_ => sha1(jsonStringify(_)).slice(0,6)).join('-')}.json`.
4. Cache hit → `cachedBuffer.forEach(addCachedCostToTotalSessionCost)` then return rehydrated content with fresh `randomUUID()` per message (comment 246–249: fresh UUIDs prevent dedup collisions in `sessionStorage.ts`).
5. Miss + CI + no `VCR_RECORD` → throw "Anthropic API fixture missing".
6. Miss + record → write `{ input: dehydratedInput, output: dehydrated }`.

`addCachedCostToTotalSessionCost` skips `stream_event` types; calls `calculateUSDCost(model, usage)` and adds to total session cost via `addToTotalSessionCost`.

### 5.8 preventSleep refcount

`src/services/preventSleep.ts:36-43`/`49-58` — refcount goes 0→1: `spawnCaffeinate` + `startRestartInterval`. n→0: `stopRestartInterval` + `killCaffeinate`. `forceStopPreventSleep` resets `refCount=0` and tears down. Restart every 4 min (`RESTART_INTERVAL_MS`) so the 5-min `caffeinate -t 300` self-heal doesn't gap-out. Both interval and child are `unref()`'d. Cleanup registered exactly once (`cleanupRegistered` flag).

---

## 6. Verbatim Assets

### 6.1 `src/buddy/types.ts` — full module (lines 1–149)

```ts
export const RARITIES = [
  'common',
  'uncommon',
  'rare',
  'epic',
  'legendary',
] as const
export type Rarity = (typeof RARITIES)[number]

// One species name collides with a model-codename canary in excluded-strings.txt.
// The check greps build output (not source), so runtime-constructing the value keeps
// the literal out of the bundle while the check stays armed for the actual codename.
// All species encoded uniformly; `as` casts are type-position only (erased pre-bundle).
const c = String.fromCharCode
// biome-ignore format: keep the species list compact

export const duck = c(0x64,0x75,0x63,0x6b) as 'duck'
export const goose = c(0x67, 0x6f, 0x6f, 0x73, 0x65) as 'goose'
export const blob = c(0x62, 0x6c, 0x6f, 0x62) as 'blob'
export const cat = c(0x63, 0x61, 0x74) as 'cat'
export const dragon = c(0x64, 0x72, 0x61, 0x67, 0x6f, 0x6e) as 'dragon'
export const octopus = c(0x6f, 0x63, 0x74, 0x6f, 0x70, 0x75, 0x73) as 'octopus'
export const owl = c(0x6f, 0x77, 0x6c) as 'owl'
export const penguin = c(0x70, 0x65, 0x6e, 0x67, 0x75, 0x69, 0x6e) as 'penguin'
export const turtle = c(0x74, 0x75, 0x72, 0x74, 0x6c, 0x65) as 'turtle'
export const snail = c(0x73, 0x6e, 0x61, 0x69, 0x6c) as 'snail'
export const ghost = c(0x67, 0x68, 0x6f, 0x73, 0x74) as 'ghost'
export const axolotl = c(0x61, 0x78, 0x6f, 0x6c, 0x6f, 0x74, 0x6c) as 'axolotl'
export const capybara = c(
  0x63,0x61,0x70,0x79,0x62,0x61,0x72,0x61,
) as 'capybara'
export const cactus = c(0x63, 0x61, 0x63, 0x74, 0x75, 0x73) as 'cactus'
export const robot = c(0x72, 0x6f, 0x62, 0x6f, 0x74) as 'robot'
export const rabbit = c(0x72, 0x61, 0x62, 0x62, 0x69, 0x74) as 'rabbit'
export const mushroom = c(
  0x6d,0x75,0x73,0x68,0x72,0x6f,0x6f,0x6d,
) as 'mushroom'
export const chonk = c(0x63, 0x68, 0x6f, 0x6e, 0x6b) as 'chonk'

export const SPECIES = [
  duck, goose, blob, cat, dragon, octopus, owl, penguin, turtle, snail,
  ghost, axolotl, capybara, cactus, robot, rabbit, mushroom, chonk,
] as const

export const EYES = ['·', '✦', '×', '◉', '@', '°'] as const

export const HATS = [
  'none', 'crown', 'tophat', 'propeller', 'halo', 'wizard', 'beanie', 'tinyduck',
] as const

export const STAT_NAMES = [
  'DEBUGGING', 'PATIENCE', 'CHAOS', 'WISDOM', 'SNARK',
] as const

export type CompanionBones = {
  rarity: Rarity
  species: Species
  eye: Eye
  hat: Hat
  shiny: boolean
  stats: Record<StatName, number>
}

export type CompanionSoul = {
  name: string
  personality: string
}

export type Companion = CompanionBones & CompanionSoul & { hatchedAt: number }

export type StoredCompanion = CompanionSoul & { hatchedAt: number }

export const RARITY_WEIGHTS = {
  common: 60, uncommon: 25, rare: 10, epic: 4, legendary: 1,
} as const satisfies Record<Rarity, number>

export const RARITY_STARS = {
  common: '★', uncommon: '★★', rare: '★★★', epic: '★★★★', legendary: '★★★★★',
} as const satisfies Record<Rarity, string>

export const RARITY_COLORS = {
  common: 'inactive', uncommon: 'success', rare: 'permission',
  epic: 'autoAccept', legendary: 'warning',
} as const satisfies Record<Rarity, keyof import('../utils/theme.js').Theme>
```

(See `src/buddy/types.ts:1-149` for full file with type aliases inlined per line.)

### 6.2 `src/buddy/companion.ts` constants

```ts
const SALT = 'friend-2026-401'  // src/buddy/companion.ts:84

const RARITY_FLOOR: Record<Rarity, number> = {  // :53-59
  common: 5, uncommon: 15, rare: 25, epic: 35, legendary: 50,
}
```

`mulberry32(seed)` and `hashString(s)` reproduced verbatim in §5.1.

### 6.3 `src/buddy/prompt.ts` — companion intro text (verbatim, lines 7–13)

```ts
export function companionIntroText(name: string, species: string): string {
  return `# Companion

A small ${species} named ${name} sits beside the user's input box and occasionally comments in a speech bubble. You're not ${name} — it's a separate watcher.

When the user addresses ${name} directly (by name), its bubble will answer. Your job in that moment is to stay out of the way: respond in ONE line or less, or just answer any part of the message meant for you. Don't explain that you're not ${name} — they know. Don't narrate what ${name} might say — the bubble handles that.`
}
```

`getCompanionIntroAttachment(messages)` returns `[{ type: 'companion_intro', name: companion.name, species: companion.species }]` when `feature('BUDDY')` AND companion exists AND `!companionMuted` AND not already announced (lookup by `msg.type==='attachment' && msg.attachment.type==='companion_intro' && msg.attachment.name === companion.name`). Lines 15–36.

### 6.4 `src/buddy/useBuddyNotification.tsx` — teaser notification literal

```ts
addNotification({
  key: 'buddy-teaser',
  jsx: <RainbowText text="/buddy" />,
  priority: 'immediate',
  timeoutMs: 15000,
})
// trigger regex (line 88):
const re = /\/buddy\b/g
```

ANT short-circuit literals (lines 13, 18 — post-build-substituted): `if ("external" === 'ant') return true` (becomes the `USER_TYPE === 'ant'` test pre-build).

### 6.5 `src/upstreamproxy/upstreamproxy.ts` — NO_PROXY list (verbatim lines 31–63)

```ts
export const SESSION_TOKEN_PATH = '/run/ccr/session_token'
const SYSTEM_CA_BUNDLE = '/etc/ssl/certs/ca-certificates.crt'

const NO_PROXY_LIST = [
  'localhost',
  '127.0.0.1',
  '::1',
  '169.254.0.0/16',
  '10.0.0.0/8',
  '172.16.0.0/12',
  '192.168.0.0/16',
  // Anthropic API: no upstream route will ever match, and the MITM breaks
  // non-Bun runtimes (Python httpx/certifi doesn't trust the forged CA).
  // Three forms because NO_PROXY parsing differs across runtimes:
  //   *.anthropic.com  — Bun, curl, Go (glob match)
  //   .anthropic.com   — Python urllib/httpx (suffix match, strips leading dot)
  //   anthropic.com    — apex domain fallback
  'anthropic.com',
  '.anthropic.com',
  '*.anthropic.com',
  'github.com',
  'api.github.com',
  '*.github.com',
  '*.githubusercontent.com',
  'registry.npmjs.org',
  'pypi.org',
  'files.pythonhosted.org',
  'index.crates.io',
  'proxy.golang.org',
].join(',')
```

`getUpstreamProxyEnv()` keys (lines 189–198) when proxy enabled:
`HTTPS_PROXY=http://127.0.0.1:${port}`, `https_proxy`, `NO_PROXY=NO_PROXY_LIST`, `no_proxy`, `SSL_CERT_FILE=${caBundlePath}`, `NODE_EXTRA_CA_CERTS`, `REQUESTS_CA_BUNDLE`, `CURL_CA_BUNDLE`.

Inherited keys when disabled-but-parent-set (lines 169–178): `HTTPS_PROXY`, `https_proxy`, `NO_PROXY`, `no_proxy`, `SSL_CERT_FILE`, `NODE_EXTRA_CA_CERTS`, `REQUESTS_CA_BUNDLE`, `CURL_CA_BUNDLE`.

`prctl` constants (lines 235–237): `PR_SET_DUMPABLE = 4`, called as `prctl(4, 0n, 0n, 0n, 0n)`.

`AbortSignal.timeout(5000)` for CA-cert fetch (line 264).

### 6.6 `src/upstreamproxy/relay.ts` — protobuf wire-format helpers (lines 51–103)

```ts
const MAX_CHUNK_BYTES = 512 * 1024
const PING_INTERVAL_MS = 30_000

export function encodeChunk(data: Uint8Array): Uint8Array {
  const len = data.length
  const varint: number[] = []
  let n = len
  while (n > 0x7f) {
    varint.push((n & 0x7f) | 0x80)
    n >>>= 7
  }
  varint.push(n)
  const out = new Uint8Array(1 + varint.length + len)
  out[0] = 0x0a
  out.set(varint, 1)
  out.set(data, 1 + varint.length)
  return out
}

export function decodeChunk(buf: Uint8Array): Uint8Array | null {
  if (buf.length === 0) return new Uint8Array(0)
  if (buf[0] !== 0x0a) return null
  let len = 0; let shift = 0; let i = 1
  while (i < buf.length) {
    const b = buf[i]!
    len |= (b & 0x7f) << shift
    i++
    if ((b & 0x80) === 0) break
    shift += 7
    if (shift > 28) return null
  }
  if (i + len > buf.length) return null
  return buf.subarray(i, i + len)
}
```

CONNECT line regex (line 319): `/^CONNECT\s+(\S+)\s+HTTP\/1\.[01]$/i`. CONNECT-buffer cap: 8192 bytes (line 311). User-facing wire strings emitted to client: `'HTTP/1.1 400 Bad Request\r\n\r\n'` (line 312), `'HTTP/1.1 405 Method Not Allowed\r\n\r\n'` (line 321), `'HTTP/1.1 502 Bad Gateway\r\n\r\n'` (line 416).

WS upgrade headers (lines 356–359): `{ 'Content-Type': 'application/proto', Authorization: wsAuthHeader }`. First chunk over WS (lines 382–384):

```
${connectLine}\r\n
Proxy-Authorization: ${authHeader}\r\n
\r\n
```

### 6.7 `src/native-ts/yoga-layout/enums.ts` — full module (lines 7–134)

All 16 enum maps inlined verbatim:

```ts
export const Align = { Auto: 0, FlexStart: 1, Center: 2, FlexEnd: 3,
  Stretch: 4, Baseline: 5, SpaceBetween: 6, SpaceAround: 7, SpaceEvenly: 8 }
export const BoxSizing = { BorderBox: 0, ContentBox: 1 }
export const Dimension = { Width: 0, Height: 1 }
export const Direction = { Inherit: 0, LTR: 1, RTL: 2 }
export const Display = { Flex: 0, None: 1, Contents: 2 }
export const Edge = { Left: 0, Top: 1, Right: 2, Bottom: 3, Start: 4,
  End: 5, Horizontal: 6, Vertical: 7, All: 8 }
export const Errata = { None: 0, StretchFlexBasis: 1,
  AbsolutePositionWithoutInsetsExcludesPadding: 2,
  AbsolutePercentAgainstInnerSize: 4, All: 2147483647, Classic: 2147483646 }
export const ExperimentalFeature = { WebFlexBasis: 0 }
export const FlexDirection = { Column: 0, ColumnReverse: 1, Row: 2, RowReverse: 3 }
export const Gutter = { Column: 0, Row: 1, All: 2 }
export const Justify = { FlexStart: 0, Center: 1, FlexEnd: 2,
  SpaceBetween: 3, SpaceAround: 4, SpaceEvenly: 5 }
export const MeasureMode = { Undefined: 0, Exactly: 1, AtMost: 2 }
export const Overflow = { Visible: 0, Hidden: 1, Scroll: 2 }
export const PositionType = { Static: 0, Relative: 1, Absolute: 2 }
export const Unit = { Undefined: 0, Point: 1, Percent: 2, Auto: 3 }
export const Wrap = { NoWrap: 0, Wrap: 1, WrapReverse: 2 }
```

### 6.8 `src/native-ts/color-diff/index.ts` — theme tables (lines 188–244)

`MONOKAI_SCOPES` (24 entries, RGB triples) and `GITHUB_SCOPES` (24 entries) are reproduced verbatim in source; sub-agent fidelity reimplementation MUST copy `src/native-ts/color-diff/index.ts:188-244` byte-for-byte. `STORAGE_KEYWORDS` (lines 248–265) — set of 17 strings: `const, let, var, function, class, type, interface, enum, namespace, module, def, fn, func, struct, trait, impl`. `FILENAME_LANGS` (lines 414–420): `Dockerfile→dockerfile, Makefile→makefile, Rakefile→ruby, Gemfile→ruby, CMakeLists→cmake`. `BAT_THEME` env var read (line 974) but unused except for diagnostics. `CHANGE_THRESHOLD = 0.4` (line 546).

### 6.9 `src/native-ts/file-index/index.ts` — scoring constants (lines 23–38)

```ts
const SCORE_MATCH = 16
const BONUS_BOUNDARY = 8
const BONUS_CAMEL = 6
const BONUS_CONSECUTIVE = 4
const BONUS_FIRST_CHAR = 8
const PENALTY_GAP_START = 3
const PENALTY_GAP_EXTENSION = 1
const TOP_LEVEL_CACHE_LIMIT = 100
const MAX_QUERY_LEN = 64
const CHUNK_MS = 4
```

Test-file penalty (line 283): `path.includes('test') ? Math.min(positionScore * 1.05, 1.0) : positionScore`. Boundary chars (lines 305–315): `/`, `\`, `-`, `_`, `.`, space.

### 6.10 `src/moreright/useMoreRight.tsx` (verbatim — entire file is the public spec)

```ts
// Stub for external builds — the real hook is internal only.
//
// Self-contained: no relative imports. Typecheck sees this file at
// scripts/external-stubs/src/moreright/ before overlay, where ../types/
// would resolve to scripts/external-stubs/src/types/ (doesn't exist).

type M = any;
export function useMoreRight(_args: {
  enabled: boolean;
  setMessages: (action: M[] | ((prev: M[]) => M[])) => void;
  inputValue: string;
  setInputValue: (s: string) => void;
  setToolJSX: (args: M) => void;
}): {
  onBeforeQuery: (input: string, all: M[], n: number) => Promise<boolean>;
  onTurnComplete: (all: M[], aborted: boolean) => Promise<void>;
  render: () => null;
} {
  return {
    onBeforeQuery: async () => true,
    onTurnComplete: async () => {},
    render: () => null
  };
}
```

The internal-only implementation is **missing-leaked-source** — record this as a residual gap.

### 6.11 `src/assistant/sessionHistory.ts` — wire literals

```ts
export const HISTORY_PAGE_SIZE = 100
// baseUrl pattern (line 36):
//   ${getOauthConfig().BASE_API_URL}/v1/sessions/${sessionId}/events
// extra headers (lines 39-41):
//   'anthropic-beta': 'ccr-byoc-2025-07-29'
//   'x-organization-uuid': orgUUID
// fetchPage timeout (line 54): 15000 ms
// validateStatus: () => true  (line 55)
// query params:
//   { limit, anchor_to_latest: true }  (latest)
//   { limit, before_id: beforeId }     (older)
```

### 6.12 `src/cli/exit.ts` (verbatim — full module is the public surface)

```ts
/* eslint-disable custom-rules/no-process-exit -- centralized CLI exit point */
export function cliError(msg?: string): never {
  if (msg) console.error(msg)
  process.exit(1)
  return undefined as never
}
export function cliOk(msg?: string): never {
  if (msg) process.stdout.write(msg + '\n')
  process.exit(0)
  return undefined as never
}
```

### 6.13 `src/cli/ndjsonSafeStringify.ts` (verbatim core)

```ts
const JS_LINE_TERMINATORS = / | /g
function escapeJsLineTerminators(json: string): string {
  return json.replace(JS_LINE_TERMINATORS, c =>
    c === ' ' ? '\\u2028' : '\\u2029',
  )
}
export function ndjsonSafeStringify(value: unknown): string {
  return escapeJsLineTerminators(jsonStringify(value))
}
```

### 6.14 `src/services/preventSleep.ts` — caffeinate invocation (lines 21–25, 121–131)

```ts
const CAFFEINATE_TIMEOUT_SECONDS = 300        // 5 minutes
const RESTART_INTERVAL_MS = 4 * 60 * 1000     // 4 minutes
spawn('caffeinate', ['-i', '-t', String(CAFFEINATE_TIMEOUT_SECONDS)],
      { stdio: 'ignore' })
// kill: SIGKILL
```

Platform gate: `process.platform !== 'darwin'` → no-op.

### 6.15 `src/services/vcr.ts` — fixture-key dehydration regexes (lines 297–331)

```ts
s.replace(/num_files="\d+"/g, 'num_files="[NUM]"')
 .replace(/duration_ms="\d+"/g, 'duration_ms="[DURATION]"')
 .replace(/cost_usd="\d+"/g, 'cost_usd="[COST]"')
 .replaceAll(configHome, '[CONFIG_HOME]')
 .replaceAll(cwd, '[CWD]')
 .replace(/Available commands:.+/, 'Available commands: [COMMANDS]')
// Windows extras: forward-slash + JSON-escaped variants
// Files-modified collapse: 'Files modified by user:' → 'Files modified by user: [FILES]'
// Token-count VCR additionally:
.replaceAll(cwdSlug, '[CWD_SLUG]')
.replace(/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/gi, '[UUID]')
.replace(/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z?/g, '[TIMESTAMP]')
```

`shouldUseVCR` (lines 23–33): `NODE_ENV === 'test'` OR (`USER_TYPE === 'ant'` AND `isEnvTruthy(FORCE_VCR)`). CI without `VCR_RECORD` throws "Fixture missing: ${filename}. Re-run tests with VCR_RECORD=1, then commit the result." (line 73, plus token-count variant 134).

---

## 7. Performance, Cache, Concurrency

- Companion `rollCache` is a single-slot module-level cache keyed by `userId+SALT`; comment explicitly cites three hot callers: 500ms sprite tick, per-keystroke `PromptInput`, per-turn observer (`buddy/companion.ts:104-106`).
- `CompanionSprite` rendering is owned by 37; bones/soul-fetch hot path reads cached roll.
- `color-diff`'s `highlight.js` lazy load (`buddy/companion.ts` style — lines 27–43): registers 190+ language grammars at first call (~50MB / 100–200ms macOS, several× on Windows). Comment 31 cites a CI test-shard timeout regression (PR #24150).
- `file-index` async build chunks at `CHUNK_MS = 4` ms; `loadFromFileListAsync` resolves `queryable` after first chunk so search returns partial results during 270k-path index build.
- `upstreamproxy` relay: max 512KB per WS chunk, 30s keepalive (zero-length encoded chunk; `setInterval`). Bun manual write-buffer drain. WS upgrade through `getWebSocketProxyAgent`/`getWebSocketProxyUrl` (CCR egress gateway constraint; comment 25–30).
- `preventSleep` interval and caffeinate child both `unref()`'d so they don't keep Node alive.

---

## 8. Errors & Fallbacks

- All `upstreamproxy` failures: log warning + `state = { enabled: false }`. CA download non-200 → `proxy disabled`. Token unlink failure → log warning, do NOT disable (relay still up).
- `setNonDumpable`: prctl rc !== 0 logs warning but does not disable.
- `relay.openTunnel`: WS error before tunnel established → `502 Bad Gateway` to client; after established → just close (writing plaintext would corrupt TLS stream; comment 121–125).
- `color-diff`: `hljs().highlight` throws → fallback to default style. Emitter shape mismatch (`hasRootNode` guard) logs once via `logError` and returns default style; flag `loggedEmitterShapeError` prevents log flood.
- `file-index`: empty query returns `topLevelCache.slice(0, limit)` (or `[]` if cache absent); `limit <= 0` returns `[]`.
- `vcr`: cache miss in CI without `VCR_RECORD` throws; otherwise records.
- `preventSleep`: spawn failure / non-macOS → silent no-op.

---

## 9. Tests — N/A

No tests in leak. `src/native-ts/color-diff/index.ts:991-998` exposes `__test` with internal helpers; `services/vcr.ts` exposes `withVCR` etc. for harness use. `upstreamproxy.ts:202` exposes `resetUpstreamProxyForTests`. `relay.ts:245` exports `startNodeRelay` separately so a Bun runner can exercise the Node path explicitly.

---

## 10. Security & Permissions

- `upstreamproxy` writes a CA bundle to `~/.ccr/ca-bundle.crt` and propagates `SSL_CERT_FILE`/`NODE_EXTRA_CA_CERTS` to subprocesses → trusts a MITM. Mitigations: NO_PROXY excludes `*.anthropic.com`, `*.github.com`, npm/pypi/crates/golang registries; runtime gate requires both `CLAUDE_CODE_REMOTE` and `CCR_UPSTREAM_PROXY_ENABLED` truthy + a session token file.
- `prctl(PR_SET_DUMPABLE, 0)` blocks same-UID `ptrace` of the CLI heap to keep the upstream proxy session token out of `gdb -p $PPID` reach (comment 220–224). Linux only; silently no-ops elsewhere → token is heap-readable on macOS/Windows.
- Token file unlink only after relay listener up so a supervisor restart can retry.
- `companion.ts` regenerates `bones` from `userId` on every read precisely so users can't edit `config.companion` to fake a `legendary` rarity (comment 121–123). `SALT = 'friend-2026-401'` is a static module constant; not a security secret — anyone can recompute the roll for a known userId.
- `vcr` fixture root is `CLAUDE_CODE_TEST_FIXTURES_ROOT ?? cwd`; `dehydrateValue` strips `cwd` and `configHome` to `[CWD]`/`[CONFIG_HOME]` so committed fixtures don't leak local paths.
- `cliError`/`cliOk` are the centralized exit points (`/* eslint-disable custom-rules/no-process-exit */`).

---

## 11. Logging / Telemetry

- `upstreamproxy.ts` and `relay.ts` use `logForDebugging('[upstreamproxy] ...', { level: 'warn' })` for failure cases. No analytics/telemetry events.
- `color-diff/index.ts` uses `logError(new Error('color-diff: hljs emitter shape mismatch ...'))` once.
- `preventSleep` uses `logForDebugging` for spawn/restart/kill events.
- `assistant/sessionHistory.ts` logs `logForDebugging('[fetchLatestEvents|fetchOlderEvents] HTTP ${status}')` on non-200.
- `services/vcr.ts` does not log; it throws on missing fixtures in CI.

---

## 12. Open Questions, Phase 9 Escalations, Residual Unclaimed

> **Phase 9.7 sweep (2026-05-09)**: items below re-audited. Most Phase 9 escalations have since been resolved by Phase 9.6 / 10b coverage matrix work.

**Owned-here residuals (Phase 9 may reassign)**:

1. ~~**`src/buddy/CompanionSprite.tsx` and `sprites.ts` ownership**~~ — **NOTE Phase 9.7**: spec 37 (Ink UI) does not absorb buddy sprites; 42 retains by default per Phase 10b coverage matrix. Both specs cite consistently.
2. ~~**`src/native-ts/yoga-layout/index.ts` (~36 KB) bit-exact verification**~~ — **DEFERRED**: file exceeded inline budget; documented at directory level. Bit-exact verification against upstream `yoga-layout/src/generated/Yoga.ts` is a future-revise item, not blocking.
3. ~~**`src/types/generated/events_mono/{claude_code,common,growthbook}/v1/`**~~ — **RESOLVED Phase 9.7**: spec 26 §6.7 owns the BigQuery field-mapping; spec 42 §A retains directory-level enumeration. No double-claim per coverage matrix.

**Unresolved residuals — Phase 9 escalations**:

1. ~~**`src/services/tips/`, `MagicDocs/`, `PromptSuggestion/`**~~ — **RESOLVED Phase 9.7**: per Phase 10b coverage matrix (`PHASE10-COVERAGE.md`), these directories are claimed in spec 42a §3 enumeration. Cross-cutting telemetry (MagicDocs) cited from spec 26.
2. ~~**`src/services/{claudeAiLimits.ts, ...}`**~~ — **RESOLVED Phase 9.7**: spec 27 (service-policy) §3 owns `claudeAiLimits.ts`, `claudeAiLimitsHook.ts`, `rateLimitMessages.ts` per Phase 9.6 routing. Spec 22 §12 Q8 documents the consumer interface. 42 no longer carries these.
3. ~~**`src/cli/print.ts` (212 KB)**~~ — **NOTE Phase 9.7**: spec 01 §6 covers user-facing strings; remaining bundled-artifact opacity is recorded in spec 01 §12 as DEFERRED (bundled output). Spec 01 §12 Q1.
4. ~~**`src/utils/` 329 files**~~ — **RESOLVED Phase 9.7**: Phase 10b coverage matrix (`PHASE10-COVERAGE.md`) enumerates per-file claim. Spec 42a §3 catalogs the long-tail; consumer specs claim load-bearing utilities.
5. ~~**`src/constants/` 21 files**~~ — **RESOLVED Phase 9.7**: spec 42 §A (Phase 10 cleanup) appendix enumerates the 8 remaining constants files with their consumer specs. No double-ownership.
6. ~~**Buddy sprite paths cited in 37**~~ — **NOTE Phase 9.7**: 37 documents the Ink components consumed by sprites; 42 documents the buddy module proper. Citations cross-consistent.
7. ~~**`src/moreright/` real internal hook**~~ — **DEFERRED (missing-leaked-source)**: stub is only artifact. Recorded in spec 00 §2.5 missing-source ledger.
8. ~~**`/buddy` slash command implementation**~~ — **DEFERRED (missing-leaked-source)**: per 21c §3.15. BUDDY library hatches/renders but user-facing command flow not reverse-engineerable. Recorded.
9. ~~**`assistant/sessionHistory.ts` placement**~~ — **RESOLVED Phase 9.7**: 42 retains per Phase 10b coverage matrix decision. 41 owns local history; 42 owns server-reaching session-events history. Boundary explicit in coverage matrix.

**Files that are inlined verbatim in §6**: types.ts (full); prompt.ts companion intro string; useBuddyNotification literals + regex; upstreamproxy NO_PROXY list + env-key list + prctl constants; relay protobuf encode/decode + wire-error strings + WS upgrade headers; native-ts/yoga-layout/enums.ts (all 16 maps); color-diff scoring tables citation + STORAGE_KEYWORDS + FILENAME_LANGS + CHANGE_THRESHOLD; file-index scoring constants + boundary chars + test-penalty; moreright stub (full); sessionHistory wire literals; cli/exit.ts (full); cli/ndjsonSafeStringify.ts (core); preventSleep caffeinate flags; vcr dehydration regexes + shouldUseVCR.

---

## Self-check

- §1 explicit IN/OUT scope with cited owners for OUT-of-scope. ✓
- Source-coverage inventory present (§1 table + §2 file-by-file). ✓
- §6 inlines verbatim: prompt strings (companion intro, teaser key, regexes), all NO_PROXY entries, prctl constants, all yoga enum values, file-index scoring constants, color-diff CHANGE_THRESHOLD + STORAGE_KEYWORDS + FILENAME_LANGS, vcr dehydration regexes, cli/exit.ts (full), moreright stub (full), buddy types (full), CONNECT regex + wire error strings, WS upgrade headers + first-chunk format, sessionHistory wire literals, caffeinate flags. ✓
- Every behavioral claim cites `src/<path>:<line-range>`. ✓
- §12 lists residuals not claimable here as Phase 9 escalations. ✓
- No design critique. ✓
- Adjacent specs (00, 01, 20, 37, 41) referenced by number, not redocumented. ✓

---

## §X Sandbox subsystem (delegated from 35)

> **Phase 10e ownership decision.** Spec 35 (`35:18`, `35:623`) explicitly delegates `SandboxManager` / `/sandbox-toggle` / `dangerouslySkipPermissions × sandbox interaction` to spec 42. This section absorbs that delegation so the spec corpus has exactly one architectural owner.

### §X.1 Files

| File | Lines | Notes |
|---|---:|---|
| `src/utils/sandbox/sandbox-adapter.ts` | 985 | Wraps `@anthropic-ai/sandbox-runtime`'s `SandboxManager` (re-exported as `SandboxManager`). Header: "Adapter layer that wraps `@anthropic-ai/sandbox-runtime` with Claude CLI-specific integrations… bridge between the external sandbox-runtime package and Claude CLI's settings system, tool integration, and additional features." Imports types `FsReadRestrictionConfig`, `FsWriteRestrictionConfig`, `IgnoreViolationsConfig`, `NetworkHostPattern`, `NetworkRestrictionConfig`, `SandboxAskCallback`, `SandboxDependencyCheck`, `SandboxRuntimeConfig`, `SandboxViolationEvent` from `@anthropic-ai/sandbox-runtime`; re-exports `SandboxManager`, `SandboxRuntimeConfigSchema`, `SandboxViolationStore`. Uses `getAdditionalDirectoriesForClaudeMd`, `getCwdState`, `getOriginalCwd` for path resolution. |
| `src/utils/sandbox/sandbox-ui-utils.ts` | 12 | Tiny UI helper module (Ink dialog formatting for sandbox violation prompts). |
| `src/commands/sandbox-toggle/index.ts` | 50 | Slash-command registration shim. Registered at `src/commands.ts:149` (`import sandboxToggle from './commands/sandbox-toggle/index.js'`) and exported in the `commands` array at `src/commands.ts:336` (`sandboxToggle`). |
| `src/commands/sandbox-toggle/sandbox-toggle.tsx` | 82 | Ink UI for the `/sandbox-toggle` command. Reads `SandboxManager.isSandboxingEnabled()` / `areUnsandboxedCommandsAllowed()` / `isAutoAllowBashIfSandboxedEnabled()` and toggles them via the adapter. |

### §X.2 Boot wiring (`src/main.tsx`)

| Line | Statement | Purpose |
|---|---|---|
| `src/main.tsx:201` | `import { SandboxManager } from './utils/sandbox/sandbox-adapter.js';` | Top-level import (lazy-evaluated by bundler; sandbox-runtime ships unconditionally). |
| `src/main.tsx:314` | `sandbox_enabled: SandboxManager.isSandboxingEnabled(),` | Telemetry / startup-context field. |
| `src/main.tsx:315` | `are_unsandboxed_commands_allowed: SandboxManager.areUnsandboxedCommandsAllowed(),` | Telemetry / startup-context field. |
| `src/main.tsx:316` | `is_auto_bash_allowed_if_sandbox_enabled: SandboxManager.isAutoAllowBashIfSandboxedEnabled(),` | Telemetry / startup-context field. |

### §X.3 `dangerouslySkipPermissions` vs sandbox-state — distinction

Per Phase 9.7 spec 35 §1 OUT-of-scope wording (`35:18`, repeated `35:623`):

> "direct-connect's `dangerouslySkipPermissions` flag in §4.2 / §5.5 is purely a **per-session permission-bypass flag passed to the server, not a sandbox-state mutator**."

The two are independent:

| Concern | Owner | Surface | Lifetime | Mutates |
|---|---|---|---|---|
| `dangerouslySkipPermissions` | 35 (remote sessions), 09 (permissions) | CLI flag `--dangerously-skip-permissions` (`src/main.tsx:976`); also `--allow-dangerously-skip-permissions` | Per-session | Bypasses permission **prompts**; does NOT alter sandbox fs/network restrictions |
| Sandbox state (enabled, unsandboxed-commands-allowed, auto-bash-if-sandbox) | **42** (this spec, §X) | `/sandbox-toggle` slash command + `SandboxManager` static methods + settings persistence | Persisted in settings | Mutates sandbox-runtime fs read/write + network restriction config; affects subprocess spawn |

The two compose: a session in sandbox mode with `dangerouslySkipPermissions` will skip permission prompts but still enforce sandbox fs/network restrictions (sandbox is a runtime capability boundary, permissions are a UX gate).

### §X.4 Distinction from `bash/ast.ts` shell trust-boundary

`src/utils/bash/ast.ts` (see §1 co-ownership table) is also labeled "This is NOT a sandbox." in its file header. Reading these together:

- `bash/ast.ts` = **parser-level** trust boundary. Decides whether a shell string can be safely matched against allow/deny rules. FAIL-CLOSED. Owned by 09/10.
- `utils/sandbox/sandbox-adapter.ts` + `@anthropic-ai/sandbox-runtime` = **runtime** capability boundary. Restricts fs read/write + network for spawned subprocesses. Owned by 42 §X.

Both can be active simultaneously. Neither subsumes the other.

### §X.5 Cross-references

- Spec 35 `35:18`, `35:623` → delegates here. ✓
- Spec 09/10 → owns `bash/ast.ts` parser trust-boundary; cite-only here.
- Spec 26 (analytics) → consumes the three telemetry booleans at `src/main.tsx:314-316`.
- `@anthropic-ai/sandbox-runtime` is a vendor package, not in-leak; sandbox-adapter wraps it. The wrapper is the architectural surface owned here.

---

## §A Appendix — `src/constants/` enumeration (Phase 10 cleanup)

§5 above states "strings/regexes belong to consuming specs" and lists the
already-claimed five files. The following constants files exist but were
not captured by spec name; recorded here so the basenames appear in the
spec corpus. Each is a tiny constants table consumed by exactly the spec
listed.

| File | Consumer spec | Content |
|---|---|---|
| `src/constants/cyberRiskInstruction.ts` | 05 (context assembly) | `CYBER_RISK_INSTRUCTION` — Safeguards-team-owned guidance string for handling defensive-vs-offensive security requests. Header comment mandates Safeguards review before edits. |
| `src/constants/errorIds.ts` | 06 / 22 (error logging) | Numeric `E_*` constants — descriptive trace-ID identifiers for `logError()` call sites. Names are fully human-readable (e.g., `E_TOOL_USE_SUMMARY_GENERATION_FAILED = 344` at `src/constants/errorIds.ts:15`); the integer is the wire ID used by analytics dedup. Header comment block (`src/constants/errorIds.ts:10-12`) documents the assignment protocol: "1. Add a const based on Next ID. 2. Increment Next ID." Tracker comment `Next ID: 346` lives at `src/constants/errorIds.ts:12`. Exported as individual `const`s for optimal external-build DCE. |
| `src/constants/figures.ts` | 37 (Ink UI) | Unicode glyphs used across the terminal UI: `BLACK_CIRCLE` (platform-specific), `BULLET_OPERATOR`, `TEARDROP_ASTERISK`, arrows, effort indicators (`EFFORT_LOW`/`MEDIUM`/`HIGH`/`MAX`), play/pause icons, MCP subscription arrows, fork glyph, ultrareview diamond states. |
| `src/constants/product.ts` | 35 (remote-server) / 25 (auth) | `PRODUCT_URL = 'https://claude.com/claude-code'` plus `CLAUDE_AI_BASE_URL` / `CLAUDE_AI_STAGING_BASE_URL` / `CLAUDE_AI_LOCAL_BASE_URL` and `isRemoteSessionStaging`/local helpers. |
| `src/constants/spinnerVerbs.ts` | 37 (Ink UI) | `getSpinnerVerbs()` — returns base `SPINNER_VERBS` list, replaced or appended based on `settings.spinnerVerbs.{mode, verbs}`. |
| `src/constants/systemPromptSections.ts` | 05 (context assembly) | `systemPromptSection(name, compute)` factory — memoized prompt sections cached until `/clear` or `/compact`. Backed by `getSystemPromptSectionCache` / `setSystemPromptSectionCacheEntry` in `bootstrap/state.ts`. |
| `src/constants/toolLimits.ts` | 08 (tool base) | `DEFAULT_MAX_RESULT_SIZE_CHARS = 50_000` (system-wide cap on tool results before persist-to-disk fallback) and `MAX_TOOL_RESULT_TOKENS = 100_000` (≈400 KB at ~4 bytes/token). |
| `src/constants/turnCompletionVerbs.ts` | 37 (Ink UI) | `TURN_COMPLETION_VERBS = ['Baked','Brewed','Churned','Cogitated','Cooked','Crunched','Sautéed','Worked']` — past-tense verbs phrased to read naturally with `for [duration]`. |
