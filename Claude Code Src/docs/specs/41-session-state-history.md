# 41 — Session State, History & Resume Specification

> Layer H (Persistence). Owns the in-process AppState store and the on-disk JSONL transcript layer that drives `--resume`/`--continue`. Adjacent: 04, 14, 15, 29, 34, 35, 37, 40.

---

## 1. Purpose & Scope

**Glossary alignment (cross-spec 00:396 3-way "session"):** this spec owns the **(a) transcript-file session** cleanly (every JSONL path, Entry type, and writer lives here) and the **persistence-relevant slice of (b) Ink-REPL session** (the `AppState` store + `onChangeAppState` diff effects that mirror to `globalConfig`). The pure-rendering slice of (b) is owned by 38; **(c) remote-server session** is owned by 35 and consumed read-only here (§3.5, §5.17). See §3.0 for a row-by-row breakdown.

This spec is the authority for three intertwined concerns:

1. **In-process app state** — the immutable Zustand-style `AppState` store under `src/state/` that every UI surface, hook, and tool reads/writes through.
2. **On-disk session transcripts** — append-only JSONL files under `~/.claude/projects/<sanitized-cwd>/<sessionId>.jsonl`, plus per-agent sidechain files, file-history backups, remote-agent sidecars, and prompt history (`history.jsonl`).
3. **Session resume** — `--resume`, `--continue`, mid-session `/resume`, `--fork-session`, `--rewind-files`, and the related transcript walk that rebuilds `AppState`/messages from a JSONL.

In scope: `src/state/` (full), `src/history.ts`, `src/assistant/sessionHistory.ts`, `src/projectOnboardingState.ts`, `src/utils/sessionStorage.ts` (writer body — `recordTranscript`, `flushSessionStorage`, `Project`, `tailUuid`/`applyPreservedSegmentRelinks`, sidechain transcripts, `writeAgentMetadata`, `getLastSessionLog`), `src/utils/sessionRestore.ts`, `src/utils/sessionState.ts`, `src/utils/fileHistory.ts` (state shape + lazy 100ms write queue + `MAX_SNAPSHOTS`), `src/types/logs.ts` (entry schemas), the cost/usage and identity slices in `src/bootstrap/state.ts` that govern `getSessionId()` / `switchSession()` / `getSessionProjectDir()`, `BG_SESSIONS` background-task summary attachments and `AWAY_SUMMARY` referenced from 04, and the `PROJECT_ONBOARDING_STATE` schema.

Out of scope (cross-reference by spec #):

- Turn pipeline message lifecycle, system-reminder injection, hook fan-out → 04.
- Tool surfaces and per-tool side effects → 10..19.
- Coordinator algorithm and agent spawn topology → 30; AgentTool surface → 14; Task surface → 15.
- Remote session WS transport, ingress URL plumbing, CCR v2 internal-event reader semantics → 35; bridge auth/permission callbacks → 34.
- Ink rendering, dialogs, REPL footer surface → 37.
- `MEMORY.md` storage and memory-extraction algorithm → 40 / 29.
- Compaction algorithm itself → 07 (this spec owns ONLY the durable persistence ordering at the boundary).

---

## 2. Source Map

### 2.1 Owned files (read-fully unless noted)

| Path | Purpose | Coverage |
|---|---|---|
| `src/state/AppState.tsx` (200 lines) | React provider + `useAppState` selector hook + ant-only voice provider gate | full |
| `src/state/AppStateStore.ts` (570 lines) | `AppState` shape (verbatim §6.3), `getDefaultAppState()`, `CompletionBoundary`, `SpeculationState` | full |
| `src/state/store.ts` (35 lines) | `createStore<T>` (Object.is-gated `setState`, listener fan-out, optional `onChange`) | full |
| `src/state/onChangeAppState.ts` (171 lines) | Diff-based effects: CCR mode push, `mainLoopModel` settings sync, `verbose`/`tungstenPanelVisible` config sync, auth-cache clear | full |
| `src/state/selectors.ts` (76 lines) | `getViewedTeammateTask`, `getActiveAgentForInput` | full |
| `src/state/teammateViewHelpers.ts` (141 lines) | `enterTeammateView`/`exitTeammateView`/`stopOrDismissAgent` (PANEL_GRACE_MS = 30_000) | full |
| `src/history.ts` (465 lines) | Prompt history (`addToHistory`, `makeHistoryReader`, `getTimestampedHistory`, paste-store refs, `removeLastFromHistory`) | full |
| `src/assistant/sessionHistory.ts` (88 lines) | Remote SDK session-events pagination (`HISTORY_PAGE_SIZE = 100`, `before_id`/`anchor_to_latest`) | full |
| `src/projectOnboardingState.ts` (84 lines) | `PROJECT_ONBOARDING_STATE` (verbatim §6.6) | full |
| `src/utils/sessionStorage.ts` (5105 lines) | Owner of the `Project` writer singleton, JSONL append, dedup-against-`messageSet`, sidechain routing, metadata re-append, tombstone | full for owned regions: `:139-261`, `:283-303`, `:337-399`, `:443-528`, `:530-1351`, `:1408-1622`, `:1839-1956`, `:3869-3932` |
| `src/utils/sessionRestore.ts` (551 lines) | `loadConversationForResume` post-load fan-out: cost restore, agent restore, attribution restore, worktree cd-back, `restoreSessionStateFromLog` | full |
| `src/utils/sessionState.ts` (150 lines) | `SessionState` (`idle`/`running`/`requires_action`), `SessionExternalMetadata` (CCR push), listener registry | full |
| `src/utils/fileHistory.ts` (1115 lines) | `FileHistoryState`, `MAX_SNAPSHOTS = 100`, three-phase `fileHistoryTrackEdit` / `fileHistoryMakeSnapshot`, restore-state-from-log, hardlink migration on resume | sampled (1-220, 880-1000); rest grep-inspected |
| `src/types/logs.ts` (330 lines) | All on-disk `Entry` subtype schemas (verbatim §6.4) | full |
| `src/bootstrap/state.ts` (lines 40-540, 1300-1340) | `STATE` singleton, `getSessionId`/`switchSession`/`regenerateSessionId`/`getSessionProjectDir`/`getOriginalCwd`/`getProjectRoot`/`isSessionPersistenceDisabled`; cost/usage slice | sampled |

### 2.2 Cited from other specs (NOT redocumented here)

- `recordTranscript` cited from 03 (QueryEngine call sites: `QueryEngine.ts:451,609,712,728,730,780,834`); writer body owned here.
- `flushSessionStorage` cited from 03 (`QueryEngine.ts:460,614,848,978,1021,1078`); writer body owned here.
- Compact-boundary persistence ordering cited from 03 §5.5; durable invariants owned here.
- `BG_SESSIONS` background-task summary attachments cited from 04 (`query.ts:118,1685`); on-disk format owned here.
- `AWAY_SUMMARY` cited from 04 (`hooks/useAwaySummary.ts:54`); persistence path owned here.
- Sidechain transcripts (`recordSidechainTranscript`, `getAgentTranscript`, `writeAgentMetadata`) cited from 30; writer bodies owned here.
- Plan/worktree state persistence cited from 18; transcript entry types (`worktree-state`, `mode`) owned here.
- Cost/usage slice cited from 06; bootstrap-state slot owned here.

### 2.3 Imports (writer side)

`sessionStorage.ts` imports: `bun:bundle.feature`, `crypto.UUID`, `fs/promises` (open/append/read/write/mkdir/unlink/stat/readdir), sync `fs` (closeSync/fstatSync/openSync/readSync) for `readFileTailSync`, `lodash-es/memoize`, `path` (basename/dirname/join), `services/analytics`, `bootstrap/state` (`getOriginalCwd`/`getPlanSlugCache`/`getPromptId`/`getSessionId`/`getSessionProjectDir`/`isSessionPersistenceDisabled`/`switchSession`), `commands.builtInCommandNames`, `services/api/sessionIngress`, `tools/REPLTool/constants`, `types/ids`, `types/logs`, `types/message`, `types/messageQueueTypes`, `utils/cleanupRegistry`, `utils/concurrentSessions.updateSessionName`, `utils/diagLogs`, `utils/envUtils`, `utils/fileHistory`, `utils/getWorktreePaths`, `utils/git.getBranch`, `utils/gracefulShutdown`, `utils/json.parseJSONL`, `utils/lockfile`, `utils/messages`, `utils/path.sanitizePath`, `utils/sessionStoragePortable` (`extractJsonStringField`/`extractLastJsonStringField`/`LITE_READ_BUF_SIZE`/`readHeadAndTail`/`readTranscriptForLoad`/`SKIP_PRECOMPACT_THRESHOLD`), `utils/settings/settings.getSettings_DEPRECATED`, `utils/slowOperations` (`jsonParse`/`jsonStringify`), `utils/toolResultStorage`, `utils/uuid.validateUuid` (`sessionStorage.ts:1-95`).

### 2.4 Imported by

`QueryEngine.ts`, `query.ts`, `utils/queryHelpers.ts`, `utils/plans.ts`, `utils/sessionRestore.ts`, `utils/conversationRecovery.ts`, `utils/forkedAgent.ts`, `tasks/LocalMainSessionTask.ts`, `tasks/LocalAgentTask/LocalAgentTask.tsx`, `tasks/RemoteAgentTask/RemoteAgentTask.tsx`, `tools/AgentTool/{AgentTool,runAgent,resumeAgent}.ts`, `commands/clear/conversation.ts`, `commands/exit/exit.tsx`, `screens/REPL.tsx`, `services/AgentSummary/agentSummary.ts`, `hooks/useLogMessages.ts`, `hooks/useAwaySummary.ts`, `cli/print.ts`, `services/contextCollapse/persist.ts`, `bridge/replBridge.ts`, `components/DesktopHandoff.tsx`.

### 2.5 Missing-leaked-source ledger

None — every writer reference resolves to an owned region. Two upstream registry references that flow through this spec but live elsewhere:

- `services/contextCollapse/persist.ts` (`restoreFromEntries`) — referenced by `sessionRestore.ts:128-135` and `:494-502` but not re-described here (owned by 07/CONTEXT_COLLAPSE).
- `services/api/sessionIngress.ts` — referenced by `sessionStorage.ts:1333,1597`; transport semantics owned by 35.

---

## 3. Public Interface (Contract)

### 3.0 Three-place ownership boundary (disambiguation)

This spec straddles three substrates that all share the word "history" or "session". The split is load-bearing — confusing them produces wrong fixes. Aligned with spec 00:396 3-way "session" glossary (a)/(b)/(c):

| File / surface | Owns | On-disk? | Spec 00 sense |
|---|---|---|---|
| `src/state/*` (§3.1, §5.18) | In-process **`AppState`** Zustand-style store + diff-effect listener + selectors. Ink-REPL UI slice. | No (effects mirror to `globalConfig` only) | (b) Ink REPL — *persistence-relevant slice owned here; UI-only slice owned by 38* |
| `src/history.ts` (§3.4, §5.15) | **Prompt** history — the up-arrow / Ctrl-R picker over CLI input lines. NOT the conversation transcript. `removeLastFromHistory` (`:453`) is one-shot **prompt-history** undo, distinct from `/rewind`. | Yes — `~/.claude/history.jsonl` (cross-project), `~/.claude/paste-store/<contentHash>` | n/a (orthogonal) |
| `src/utils/sessionStorage.ts` + `src/state/`-driven writers (§3.3, §5.2-5.7) | **Conversation transcript** session — JSONL writer, tombstones, compact relinks, sidechain forks. | Yes — `~/.claude/projects/<encodedCwd>/<sessionId>.jsonl` | (a) transcript-file *(this spec owns cleanly)* |
| `src/assistant/sessionHistory.ts` (§3.5) | **Remote SDK pagination** — read-only HTTP client over CCR `session_events`. `ccr-byoc-2025-07-29` beta header. | No (HTTP only) | (c) remote-server — *35-owned, this spec consumes* |

A reader investigating "where is conversation history written" must not land in `history.ts`; that file's `LogEntry` is shell prompts, not `TranscriptMessage`s.

### 3.1 AppState store (in-process)

| Symbol | Signature | Citation |
|---|---|---|
| `AppStateProvider({ children, initialState?, onChangeAppState? })` | React provider; throws on nesting; calls `createStore(initialState ?? getDefaultAppState(), onChangeAppState)` once via `useState(() => …)` so context is stable | `src/state/AppState.tsx:37-110` |
| `useAppState<T>(selector: (s: AppState) => T): T` | `useSyncExternalStore`-based slice subscription; `Object.is`-gated; ANT-only sentinel guard intends to reject identity selectors (`state === selected`) — see footnote ‡ | `src/state/AppState.tsx:142-163` |

‡ **Source-bug note:** `AppState.tsx:150` reads `if (false && state === selected)` — the `false &&` short-circuits the throw unconditionally. The guard is *aspirationally* ANT-only but is presently a **dead branch in all builds** (the `'external' === 'ant'` literal collapsed to `false` post-bundle). Listed in `BUGS-IN-SOURCE.md`. A re-implementer should restore the runtime check rather than mirror the dead-code shape.
| `useSetAppState(): (updater: (prev) => AppState) => void` | Stable-ref setter, never triggers re-render | `src/state/AppState.tsx:170-172` |
| `useAppStateStore(): AppStateStore` | Direct store handle for non-React callers | `src/state/AppState.tsx:177-179` |
| `useAppStateMaybeOutsideOfProvider<T>(selector): T \| undefined` | Safe variant; returns `undefined` if no provider; wires `NOOP_SUBSCRIBE` so `useSyncExternalStore` is happy | `src/state/AppState.tsx:186-199` |
| `createStore<T>(initialState, onChange?)` → `Store<T>` | `getState`/`setState`/`subscribe`; `setState` no-ops on `Object.is(prev,next)`; `onChange({newState,oldState})` runs BEFORE listener fan-out | `src/state/store.ts:10-34` |
| `onChangeAppState({newState,oldState})` | Diff effects: see §5.5 | `src/state/onChangeAppState.ts:43-171` |
| `externalMetadataToAppState(metadata)` | Inverse of CCR push; restores `permission_mode`/`is_ultraplan_mode` | `src/state/onChangeAppState.ts:24-41` |
| `enterTeammateView(taskId, setAppState)` / `exitTeammateView(setAppState)` / `stopOrDismissAgent(taskId, setAppState)` | Teammate transcript view transitions; `release(task)` resets retain/messages/diskLoaded; sets `evictAfter = Date.now() + PANEL_GRACE_MS` for terminal | `src/state/teammateViewHelpers.ts:46-141` |
| `getViewedTeammateTask(state)` / `getActiveAgentForInput(state)` | Pure selectors, no side effects | `src/state/selectors.ts` |

### 3.2 Bootstrap session-identity slice

| Symbol | Citation |
|---|---|
| `getSessionId(): SessionId` | `src/bootstrap/state.ts:431-433` |
| `regenerateSessionId({setCurrentAsParent?}): SessionId` (deletes plan-slug for outgoing id, resets `sessionProjectDir = null`, `randomUUID()`) | `:435-450` |
| `switchSession(sessionId, projectDir = null)` (atomic: both fields change together; deletes outgoing plan-slug; emits `sessionSwitched`) | `:468-479` |
| `onSessionSwitch(cb)` (subscribe to switch events) | `:481-489` |
| `getSessionProjectDir(): string \| null` | `:496-498` |
| `getOriginalCwd()`, `setOriginalCwd(cwd)` (NFC-normalized) | `:500-517` |
| `getProjectRoot()`, `setProjectRoot(cwd)` (only `--worktree` flag uses setter — mid-session EnterWorktreeTool MUST NOT) | `:511-525` |
| `isSessionPersistenceDisabled()`, `setSessionPersistenceDisabled(b)` | `:1325-1331` |
| `getParentSessionId()` | `:452-454` |

### 3.3 Transcript writer (`Project` singleton via `getProject()`)

All public entries below are `async` and return `Promise<...>`; bodies are awaitable through `flushSessionStorage()`.

| Symbol | Effect | Citation |
|---|---|---|
| `getTranscriptPath(): string` | `${projectDir}/${sessionId}.jsonl`; `projectDir = getSessionProjectDir() ?? getProjectDir(getOriginalCwd())` | `sessionStorage.ts:202-205` |
| `getTranscriptPathForSession(sid)` | Honors `sessionProjectDir` only when `sid === getSessionId()`; otherwise derives from `originalCwd` | `:207-225` |
| `MAX_TRANSCRIPT_READ_BYTES = 50 * 1024 * 1024` | Read-side bail | `:229` |
| `getProjectsDir()` | `${getClaudeConfigHomeDir()}/projects` | `:198-200` |
| `getProjectDir(cwd)` | Memoized `join(getProjectsDir(), sanitizePath(cwd))` | `:436-438` |
| `getAgentTranscriptPath(agentId)` | `${projectDir}/${sessionId}/subagents[/<subdir>]/agent-<id>.jsonl` | `:247-258` |
| `setAgentTranscriptSubdir(agentId, subdir)` / `clearAgentTranscriptSubdir(agentId)` | In-memory map, consulted by `getAgentTranscriptPath` | `:236-245` |
| `getAgentMetadataPath(agentId)` | `<agentTranscriptPath>.replace(/\.jsonl$/, '.meta.json')` | `:260-262` |
| `writeAgentMetadata(agentId, {agentType, worktreePath?, description?})` | `mkdir(dirname,{recursive}); writeFile(JSON.stringify(metadata))` | `:283-303` |
| `readAgentMetadata(agentId)` | Returns `null` on `isFsInaccessible` | `:292-303` |
| `writeRemoteAgentMetadata(taskId, RemoteAgentMetadata)` / `readRemoteAgentMetadata` / `deleteRemoteAgentMetadata` / `listRemoteAgentMetadata` | Sidecar dir `${projectDir}/${sessionId}/remote-agents/remote-agent-<taskId>.meta.json` | `:337-399` |
| `recordTranscript(messages, teamInfo?, startingParentUuidHint?, allMessages?): Promise<UUID \| null>` | Dedup-prefix walk → `insertMessageChain`; returns last recorded chain-participant UUID | `:1408-1449` |
| `recordSidechainTranscript(messages, agentId?, startingParentUuid?)` | `insertMessageChain(..., isSidechain=true, agentId, startingParentUuid)` | `:1451-1462` |
| `recordQueueOperation(queueOp)` | Appends `queue-operation` entry | `:1464-1466` |
| `removeTranscriptMessage(targetUuid)` | Tombstone via `removeMessageByUuid` (positional truncate of last entry; falls back to whole-file rewrite under `MAX_TOMBSTONE_REWRITE_BYTES = 50MB`) | `:1472-1474`, `:871-951` |
| `recordFileHistorySnapshot(messageId, snapshot, isSnapshotUpdate)` | Appends `file-history-snapshot` | `:1476-1486` |
| `recordAttributionSnapshot(snapshot)` | Appends `attribution-snapshot` (ANT-only via `feature('COMMIT_ATTRIBUTION')` upstream) | `:1488-1492` |
| `recordContentReplacement(replacements, agentId?)` | Routes to agent file when `agentId`, else session file | `:1494-1499`, `:1200-1207` |
| `resetSessionFilePointer()` | Calls `Project.resetSessionFile()` (sets `sessionFile = null` + clears `pendingEntries`) | `:1505-1507`, `:688-691` |
| `adoptResumedSessionFile()` | Sets `sessionFile = getTranscriptPath()` and calls `reAppendSessionMetadata(true)` | `:1530-1534` |
| `recordContextCollapseCommit(commit)` / `recordContextCollapseSnapshot(snapshot)` | Appends `marble-origami-commit` / `marble-origami-snapshot` (gated by `feature('CONTEXT_COLLAPSE')` upstream) | `:1541-1581` |
| `flushSessionStorage()` | Cancels timer, awaits `activeDrain`, drains queues, then awaits `pendingWriteCount` to hit zero | `:1583-1585`, `:841-861` |
| `hydrateRemoteSession(sessionId, ingressUrl)` | `switchSession`, fetch via `sessionIngress.getSessionLogs`, replace JSONL atomically, then `setRemoteIngressUrl` | `:1587-1622` |
| `hydrateFromCCRv2InternalEvents(sessionId)` | Uses registered foreground+subagent readers; writes per-agent files; mode 0o600/0o700 | `:1632-1723` |
| `getLastSessionLog(sid)` | Single-read `loadSessionFile`, primes `getSessionMessages` cache only if empty, finds latest non-sidechain leaf, builds chain | `:3869-3932` |
| `loadFullLog(log)` | Lite → full enrichment via `loadTranscriptFile` | `:2949-3056` |

### 3.4 Prompt history (`history.ts`)

| Symbol | Citation |
|---|---|
| `MAX_HISTORY_ITEMS = 100`; `MAX_PASTED_CONTENT_LENGTH = 1024` | `history.ts:19-20` |
| `addToHistory(command: HistoryEntry \| string)` | Skip when `CLAUDE_CODE_SKIP_PROMPT_HISTORY`; registers cleanup once; void-dispatches `addToPromptHistory` | `:411-434` |
| `getHistory()` async-iter | Current-project, current-session-first; same `MAX_HISTORY_ITEMS` window | `:190-217` |
| `getTimestampedHistory()` async-iter | Deduped by display, lazy `resolve()` | `:162-180` |
| `makeHistoryReader()` async-iter | Resolves paste refs to `HistoryEntry` | `:145-149` |
| `removeLastFromHistory()` | One-shot undo: pops from `pendingEntries` if present, else `skippedTimestamps.add(ts)` | `:453-464` |
| `clearPendingHistoryEntries()` | Resets in-memory buffer | `:436-440` |
| `formatPastedTextRef(id, numLines)` / `formatImageRef(id)` / `parseReferences(input)` / `expandPastedTextRefs(input, pastedContents)` | `:51-100` |
| `getPastedTextRefNumLines(text)` (returns count of `/\r\n\|\r\|\n/g` matches — preserves "+2 lines for 3-line paste" historical behavior) | `:47-49` |

### 3.5 Remote SDK pagination (`assistant/sessionHistory.ts`)

| Symbol | Citation |
|---|---|
| `HISTORY_PAGE_SIZE = 100` | `:7` |
| `createHistoryAuthCtx(sessionId)` | OAuth + `anthropic-beta: ccr-byoc-2025-07-29`; base `${BASE_API_URL}/v1/sessions/<sid>/events` | `:31-43` |
| `fetchLatestEvents(ctx, limit?)` | `params={limit, anchor_to_latest: true}` | `:73-78` |
| `fetchOlderEvents(ctx, beforeId, limit?)` | `params={limit, before_id: beforeId}` | `:81-87` |

### 3.6 Project onboarding (`projectOnboardingState.ts`)

| Symbol | Citation |
|---|---|
| `getSteps(): Step[]` (workspace step gated on `isDirEmpty(getCwd())`; CLAUDE.md step gated on `existsSync('CLAUDE.md')` and `!isWorkspaceDirEmpty`) | `:19-41` |
| `isProjectOnboardingComplete()` | `:43-47` |
| `maybeMarkProjectOnboardingComplete()` (short-circuits on `hasCompletedProjectOnboarding`) | `:49-61` |
| `shouldShowProjectOnboarding` (`memoize`d; returns `false` when `hasCompletedProjectOnboarding \|\| projectOnboardingSeenCount >= 4 \|\| process.env.IS_DEMO`) | `:63-76` |
| `incrementProjectOnboardingSeenCount()` | `:78-83` |

### 3.7 File history (`utils/fileHistory.ts`)

| Symbol | Citation |
|---|---|
| `MAX_SNAPSHOTS = 100` | `:54` |
| `fileHistoryEnabled()` (interactive: `globalConfig.fileCheckpointingEnabled !== false && !CLAUDE_CODE_DISABLE_FILE_CHECKPOINTING`; SDK: `CLAUDE_CODE_ENABLE_SDK_FILE_CHECKPOINTING && !CLAUDE_CODE_DISABLE_FILE_CHECKPOINTING`) | `:63-78` |
| `fileHistoryTrackEdit(updateFileHistoryState, filePath, messageId)` (3-phase: capture → async backup → commit) | `:86-193` |
| `fileHistoryMakeSnapshot(updateFileHistoryState, messageId)` | `:198-...` |
| `fileHistoryRestoreStateFromLog(snapshots, onUpdateState)` (rebuilds `trackedFiles` set; sets `snapshotSequence = snapshots.length`) | `:888-917` |
| `copyFileHistoryForResume(log)` (hard-link, falls back to copy on `EEXIST`/`ENOENT`) | `:922-...` |

### 3.8 Session-state listeners (`utils/sessionState.ts`)

`SessionState = 'idle' \| 'running' \| 'requires_action'`; `RequiresActionDetails`, `SessionExternalMetadata`. `setSessionStateChangedListener` / `setSessionMetadataChangedListener` / `setPermissionModeChangedListener`; `notifySessionStateChanged(state, details?)` mirrors `pending_action` to `external_metadata` and emits `system.session_state_changed` SDK event when `CLAUDE_CODE_EMIT_SESSION_STATE_EVENTS`. `notifySessionMetadataChanged`, `notifyPermissionModeChanged`, `getSessionState`. Cited verbatim in `sessionState.ts:1-150`.

---

## 4. Data Model & State

### 4.1 Singletons & module-state

- `STATE: State` (`bootstrap/state.ts:429`) holds `sessionId`, `sessionProjectDir`, `parentSessionId`, `originalCwd`, `projectRoot`, `cwd`, plan-slug cache, sessions-only crons, persistence-disabled flag, plus the cost/usage slice owned by spec 06 (`mainLoopModelOverride`, `modelUsage`, `totalCostUSD`, `lastMainRequestId`, `lastApiCompletionTimestamp`, `pendingPostCompaction`, etc.). The slot inventory is in `bootstrap/state.ts:45-257`.
- `project: Project | null` (`sessionStorage.ts:440`) — lazy-instantiated singleton; `cleanupRegistered` ensures the cleanup hook is registered exactly once and runs `flush()` then `reAppendSessionMetadata()` on shutdown (`:443-466`).
- `agentTranscriptSubdirs: Map<string, string>` (`sessionStorage.ts:234`).
- `pendingEntries / lastAddedEntry / skippedTimestamps / cleanupRegistered` for prompt history (`history.ts:281-289`).

### 4.2 In-memory state machines

#### Project writer (per-file queue)

```
sessionFile: null | string   // null until materializeSessionFile()
pendingEntries: Entry[]      // buffered while sessionFile is null
writeQueues: Map<filePath, Array<{entry, resolve}>>
flushTimer: Timer | null
activeDrain: Promise<void> | null
pendingWriteCount: number
flushResolvers: Array<() => void>
FLUSH_INTERVAL_MS = 100      // default; lowered to REMOTE_FLUSH_INTERVAL_MS (10) when remote ingress or CCR v2 internal writer is registered
MAX_CHUNK_BYTES = 100 * 1024 * 1024
```

Lifecycle (`sessionStorage.ts:606-686`):

```
enqueueWrite(filePath, entry):
    queue = writeQueues.get(filePath) ?? new []; writeQueues.set(...); queue.push({entry, resolve})
    scheduleDrain()

scheduleDrain():
    if (flushTimer) return                                       // coalesce
    flushTimer = setTimeout(async () => {
        flushTimer = null
        activeDrain = drainWriteQueue()
        await activeDrain
        activeDrain = null
        if (writeQueues.size > 0) scheduleDrain()
    }, FLUSH_INTERVAL_MS)

drainWriteQueue():
    for each [filePath, queue]:
        if empty continue
        batch = queue.splice(0)
        content = ''; resolvers = []
        for each {entry, resolve} in batch:
            line = jsonStringify(entry) + '\n'
            if content.length + line.length >= MAX_CHUNK_BYTES:
                appendToFile(filePath, content); resolve all; reset
            content += line; resolvers.push(resolve)
        if content.length > 0: appendToFile(filePath, content); resolve all
    cleanup empty queues

flush():
    clear flushTimer; await activeDrain; drainWriteQueue()
    if pendingWriteCount === 0: return
    return new Promise(r => flushResolvers.push(r))
```

`appendToFile` (`:634-643`) tries `fsAppendFile(..., {mode: 0o600})` then falls back to `mkdir({recursive: true, mode: 0o700})` + retry on any error (NFS error-code variance).

#### Materialization gate

```
shouldSkipPersistence() := (NODE_ENV === 'test' && !TEST_ENABLE_SESSION_PERSISTENCE)
                       || cleanupPeriodDays === 0
                       || isSessionPersistenceDisabled()
                       || CLAUDE_CODE_SKIP_PROMPT_HISTORY                   (sessionStorage.ts:960-970)

materializeSessionFile():
    if shouldSkipPersistence(): return
    ensureCurrentSessionFile()         // sessionFile = getTranscriptPath()
    reAppendSessionMetadata()           // writes cached title/tag/agent/mode/worktree/pr-link
    drain pendingEntries via appendEntry
```

Triggered by the first `user|assistant` message inside `insertMessageChain` (`sessionStorage.ts:1003-1010`).

#### AppState immutable update model

`createStore.setState(updater)`:

```
prev = state
next = updater(prev)
if (Object.is(next, prev)) return
state = next
onChange?.({newState: next, oldState: prev})
for listener in listeners: listener()
```

Verbatim `src/state/store.ts:20-27`. `AppStateProvider` calls `useState(() => createStore(initialState ?? getDefaultAppState(), onChangeAppState))` exactly once, so the context value is stable across re-renders (`AppState.tsx:48-57`).

Speculation lifecycle is `'idle' \| 'active'` with mutable `messagesRef` / `writtenPathsRef` / `contextRef` to avoid array spreading per message; `IDLE_SPECULATION_STATE = {status:'idle'}` is a frozen sentinel reused across resets (`AppStateStore.ts:52-79`).

`CompletionBoundary` discriminated-union: `complete | bash | edit | denied_tool` (`AppStateStore.ts:41-50`).

### 4.3 On-disk layout (durable)

```
~/.claude/
  history.jsonl                                     # global prompt history (cross-project; LogEntry)
  paste-store/<contentHash>                          # large pasted text > 1024 chars
  file-history/<sessionId>/<backupFileName>          # per-session file backups (hard-linked on resume)
  projects/
    <sanitizePath(originalCwd)>/                     # project dir (memoized)
      <sessionId>.jsonl                              # main session transcript
      <sessionId>.jsonl.lock                         # lockfile (lock())
      <sessionId>/
        subagents/
          [<subdir>/]agent-<agentId>.jsonl           # sidechain transcripts
          [<subdir>/]agent-<agentId>.meta.json       # AgentMetadata sidecar
        remote-agents/
          remote-agent-<taskId>.meta.json            # RemoteAgentMetadata sidecar
```

File mode: `0o600` for files, `0o700` for `mkdir({recursive:true})` (`sessionStorage.ts:636-641`, `:1601`, `:1655`, `:1689-1696`).

### 4.4 `AppState` shape — see §6.3 (verbatim).

### 4.5 `Entry` schemas — see §6.4 (verbatim).

---

## 5. Algorithm / Control Flow

### 5.1 Session-id minting

1. At process start, `getInitialState()` (bootstrap/state.ts) sets `sessionId = randomUUID()` (`:331`), `sessionProjectDir = null` (`:408`), `originalCwd = realpathSync(cwd()).normalize('NFC')` falling back to `cwd().normalize('NFC')` on EPERM (`:271-274`), and `projectRoot = resolvedCwd` (`:279`).
2. `--continue`/`--resume`/`/resume` later calls `switchSession(asSessionId(sid), dirname(transcriptPath) ?? null)` (`sessionRestore.ts:442-445`); this atomically rewrites both `sessionId` and `sessionProjectDir`, deletes the outgoing plan-slug cache entry, and emits `sessionSwitched`.
3. `regenerateSessionId({setCurrentAsParent})` is used by `clearConversation` and `--fork-session`: it stashes outgoing id into `parentSessionId`, deletes plan-slug, sets `sessionProjectDir = null`, mints a fresh `randomUUID()`.
4. `concurrentSessions.updateSessionName` subscribes via `onSessionSwitch` so the PID file's sessionId stays in sync (`bootstrap/state.ts:481-489`).

Persistence guard: `--no-session-persistence` calls `setSessionPersistenceDisabled(true)`, which makes `shouldSkipPersistence()` short-circuit every transcript write.

### 5.2 `recordTranscript` (writer)

Owned algorithm (`sessionStorage.ts:1408-1449`):

```
recordTranscript(messages, teamInfo?, startingParentUuidHint?, allMessages?):
    cleaned = cleanMessagesForLogging(messages, allMessages)
    sessionId = getSessionId()
    messageSet = await getSessionMessages(sessionId)             # cached UUID set
    newMessages = []; startingParentUuid = startingParentUuidHint; seenNewMessage = false
    for m in cleaned:
        if messageSet.has(m.uuid):
            if !seenNewMessage and isChainParticipant(m):
                startingParentUuid = m.uuid                       # prefix-only tracking
        else:
            newMessages.push(m); seenNewMessage = true
    if newMessages.length > 0:
        await getProject().insertMessageChain(newMessages, isSidechain=false, undefined, startingParentUuid, teamInfo)
    lastRecorded = newMessages.findLast(isChainParticipant)
    return lastRecorded?.uuid ?? startingParentUuid ?? null
```

Invariants:
- The "prefix only" rule (`!seenNewMessage`) is what makes compaction work correctly: post-compact, `messagesToKeep` re-appear AFTER the new compact-boundary, so they are NOT tracked → boundary's `parentUuid` becomes `null`, truncating the `--continue` chain at the boundary (`:1392-1407`).
- `isChainParticipant(m) := m.type !== 'progress'` (`:154-156`). Progress messages are persisted but no message chains TO them. Pre-PR#24099 progress entries on disk are bridged at load time by `loadTranscriptFile`'s `progressBridge` rewrite (`:151-156`).

### 5.3 `insertMessageChain` (Project method)

```
insertMessageChain(messages, isSidechain, agentId?, startingParentUuid?, teamInfo?):
    parentUuid = startingParentUuid ?? null
    if sessionFile === null && messages.some(m => m.type==='user'||m.type==='assistant'):
        await materializeSessionFile()
    gitBranch = await getBranch() catch undefined
    sessionId = getSessionId()
    slug = getPlanSlugCache().get(sessionId)
    for message in messages:
        isCompactBoundary = isCompactBoundaryMessage(message)
        effectiveParentUuid = parentUuid
        if message.type==='user' && message.sourceToolAssistantUUID:
            effectiveParentUuid = message.sourceToolAssistantUUID    # tool_result threads
        transcriptMessage = {
            parentUuid: isCompactBoundary ? null : effectiveParentUuid,
            logicalParentUuid: isCompactBoundary ? parentUuid : undefined,
            isSidechain, teamName, agentName,
            promptId: message.type==='user' ? getPromptId() ?? undefined : undefined,
            agentId,
            ...message,                                 # spread first
            userType: getUserType(),                    # then re-stamp identity fields
            entrypoint: getEntrypoint(),
            cwd: getCwd(),
            sessionId, version: VERSION, gitBranch, slug,
        }
        await this.appendEntry(transcriptMessage)
        if isChainParticipant(message): parentUuid = message.uuid
    if !isSidechain:
        text = getFirstMeaningfulUserMessageTextContent(messages)
        if text: this.currentSessionLastPrompt = truncateTo200(text)
```

Body cited at `sessionStorage.ts:993-1083`. The "spread first, re-stamp last" ordering is critical for `--fork-session`/`--resume`: `removeExtraFields()` only strips `parentUuid`/`isSidechain`, so messages still carry source-session `sessionId`/`cwd`/`userType`. Without re-stamping, `loadFullLog`'s `sessionId`-keyed `contentReplacements` lookup would miss and content blocks would be misclassified as FROZEN (`:1049-1056`).

### 5.4 `appendEntry` dispatch table (Project)

```
appendEntry(entry, sessionId = getSessionId()):
    if shouldSkipPersistence(): return
    isCurrentSession = sessionId === getSessionId()
    if isCurrentSession:
        if sessionFile === null: pendingEntries.push(entry); return     # pre-materialize buffer
        sessionFile = this.sessionFile
    else:
        sessionFile = await getExistingSessionFile(sessionId) or log + return

    switch entry.type:
        case 'summary' | 'custom-title' | 'ai-title' | 'last-prompt' | 'task-summary'
           | 'tag' | 'agent-name' | 'agent-color' | 'agent-setting' | 'pr-link'
           | 'file-history-snapshot' | 'attribution-snapshot' | 'speculation-accept'
           | 'mode' | 'worktree-state' | 'marble-origami-commit' | 'marble-origami-snapshot':
            enqueueWrite(sessionFile, entry)
        case 'content-replacement':
            target = entry.agentId ? getAgentTranscriptPath(entry.agentId) : sessionFile
            enqueueWrite(target, entry)
        case 'queue-operation':
            enqueueWrite(sessionFile, entry)
        default (TranscriptMessage):
            messageSet = await getSessionMessages(sessionId)
            isAgentSidechain = entry.isSidechain && entry.agentId !== undefined
            target = isAgentSidechain ? getAgentTranscriptPath(asAgentId(entry.agentId)) : sessionFile
            isNewUuid = !messageSet.has(entry.uuid)
            if (isAgentSidechain || isNewUuid):
                enqueueWrite(target, entry)
                if !isAgentSidechain:
                    messageSet.add(entry.uuid)
                    if isTranscriptMessage(entry): await persistToRemote(sessionId, entry)
```

Verbatim at `sessionStorage.ts:1128-1265`. Sidechain bypass of `messageSet` is documented at `:1247-1256` — fork-inherited parent messages share UUIDs with the main transcript, so deduping against the main set would drop them.

### 5.5 Compact-boundary persistence ordering

The compact path (cited from 03 §5.5) calls (in order):

1. `recordTranscript(allPriorMessages)` — flush every pre-compact message that is still un-persisted.
2. Construct `SystemCompactBoundaryMessage` with `compactMetadata.preservedSegment = {headUuid, tailUuid, anchorUuid}` if any messages are preserved.
3. Append boundary via `recordTranscript([boundary, ...preservedSegment, summary])` — boundary becomes the new chain root because `insertMessageChain` writes `parentUuid: null` for boundaries (`:1040`).
4. `flushSessionStorage()` — drain timer, await `activeDrain`, await `pendingWriteCount`.

QueryEngine call sites (cited from 03): `QueryEngine.ts:451,609,712,728,730,780,834` for `recordTranscript`; `:460,614,848,978,1021,1078` for `flushSessionStorage`.

The `preservedSegment.tailUuid` must reference a message ALREADY persisted. The defensive walk in `QueryEngine.ts:706-714` searches `mutableMessages.slice(0, tailIdx + 1)` for `m.uuid === tailUuid` and emits an extra `recordTranscript` for that prefix BEFORE appending the boundary. Without this, mid-turn-yielded attachments push to `mutableMessages` without being recordTranscript'd; on resume `applyPreservedSegmentRelinks` finds tailUuid missing from the map, the tail→head walk breaks, and the relinker logs `tengu_relink_walk_broken` and bails — restoring the full pre-compact history.

### 5.6 `applyPreservedSegmentRelinks` (load-side splice)

`sessionStorage.ts:1839-1956`. Mutates the loaded `Map<UUID, TranscriptMessage>` in place:

1. Single pass to find `absoluteLastBoundaryIdx` and the last boundary carrying `compactMetadata.preservedSegment` (`lastSegBoundaryIdx`). They can differ if a manual `/compact` followed a reactive compact.
2. If no `preservedSegment` anywhere → return (no-op).
3. `segIsLive := lastSegBoundaryIdx === absoluteLastBoundaryIdx`. If false, the seg is stale — skip relink, but still prune.
4. If live: walk `tailUuid → parentUuid → ... → headUuid` building `preservedUuids`. If walk doesn't reach `headUuid`, log `tengu_relink_walk_broken` and return (full pre-compact history loaded).
5. Splice: `head.parentUuid := anchorUuid`; for every other child of `anchorUuid` (besides head), rewrite `parentUuid := tailUuid`. Idempotent on rerun.
6. Zero stale usage on every preserved assistant: `input_tokens=0, output_tokens=0, cache_creation_input_tokens=0, cache_read_input_tokens=0` — otherwise on-disk tokens reflect pre-compact ~190K context and resume immediately autocompacts.
7. Prune everything physically before `absoluteLastBoundaryIdx` not in `preservedUuids`. When `!segIsLive`, `preservedUuids` is empty → full prune.

### 5.7 Tombstone (`removeMessageByUuid`)

`sessionStorage.ts:871-951`. Fast path: read trailing `LITE_READ_BUF_SIZE` (64 KiB) bytes; locate `"uuid":"<UUID>"` (the full key-value pattern, never bare UUID, to avoid matching `parentUuid` in a child entry); locate enclosing newlines; `fh.truncate(absLineStart)` and re-write any trailing lines with `fh.write`. Slow path (target older than tail window): full read + rewrite, but ONLY if `fileSize <= MAX_TOMBSTONE_REWRITE_BYTES = 50 MB` (`sessionStorage.ts:123`, gated at `:927`). Above that, log warn and skip — the truncation is silently abandoned.

### 5.7a `/rewind` slash command — user-facing path into the tombstone primitive

The `/rewind` slash command (alias `/checkpoint`, type `'local'`, `supportsNonInteractive: false` — `src/commands/rewind/index.ts:3-13`) is the **interactive UI path** that drives §5.7. Its handler is a four-line wrapper (`src/commands/rewind/rewind.ts:8-12`):

```ts
if (context.openMessageSelector) {
  context.openMessageSelector()
}
return { type: 'skip' }
```

Full call chain (cross-spec 21d owns the picker UI; this spec owns the writer-side gating):

```
/rewind            → context.openMessageSelector()                       [21d: picker UI]
                   → user picks target message (rendered from AppState)  [38: rendering]
                   → removeTranscriptMessage(targetUuid)                 [sessionStorage.ts:1472]
                   → getProject().removeMessageByUuid(targetUuid)        [sessionStorage.ts:871, §5.7]
                   → fast path  (tail-window match)
                     OR slow path (full rewrite, gated by 50 MB cap)
                     OR warn-and-skip (fileSize > MAX_TOMBSTONE_REWRITE_BYTES)
```

Three points a re-implementer must not miss:

1. **`/rewind` IS the conversation-transcript rewind.** It is *not* `removeLastFromHistory` (which is one-shot **prompt-history** undo for the up-arrow buffer — see §3.4 / §3.0 disambiguation table). The two share zero code paths.
2. **The 50 MB tombstone cap silently fails the slow path.** A `/rewind` against an old message in a >50 MB transcript will not raise an error to the user; the message remains on disk, but the in-process `messageSet` may already consider it removed — divergence between durable and in-memory state. Treat this as a known sharp edge; the warn-only behavior is intentional but undocumented in user-facing UX.
3. **`--rewind-files <user-message-id>`** (CLI flag, `main.tsx:991`) is a *files-only* sibling that restores worktree files to the snapshot at the named message and exits **without** invoking `removeMessageByUuid`. It does not tombstone anything in the transcript. The two surfaces share neither code nor semantics — only the verb "rewind".

Cross-spec: 21d owns picker invocation, hotkey, and `openMessageSelector` plumbing; 41 (this spec) owns every byte written/truncated under it.

### 5.8 Resume fan-out (`loadConversationForResume` → `processResumedConversation`)

Sourced from `utils/conversationRecovery.ts:456-...` (call sites `main.tsx:3112,3593,3634,3675`; `cli/print.ts:4912,5075`; `screens/ResumeConversation.tsx:191`). The post-load fan-out owned here (`sessionRestore.ts:409-551`):

```
processResumedConversation(result, opts, context):
    if feature('COORDINATOR_MODE'):
        modeWarning = modeApi?.matchSessionMode(result.mode)
        if modeWarning: messages.push(createSystemMessage(modeWarning, 'warning'))
    if !forkSession:
        sid = sessionIdOverride ?? result.sessionId
        if sid:
            switchSession(asSessionId(sid), opts.transcriptPath ? dirname(opts.transcriptPath) : null)
            await renameRecordingForSession()              # asciicast
            await resetSessionFilePointer()
            restoreCostStateForSession(sid)                # 06
    else if result.contentReplacements?.length:
        await recordContentReplacement(result.contentReplacements)   # seed fork

    restoreSessionMetadata(forkSession ? {...result, worktreeSession: undefined} : result)

    if !forkSession:
        restoreWorktreeForResume(result.worktreeSession)              # process.chdir + setCwd; saveWorktreeState(null) on ENOENT
        adoptResumedSessionFile()                                     # sessionFile = getTranscriptPath(); reAppendSessionMetadata(true)

    if feature('CONTEXT_COLLAPSE'):
        contextCollapse.restoreFromEntries(result.contextCollapseCommits ?? [], result.contextCollapseSnapshot)

    {agentDefinition, agentType} = restoreAgentFromSession(...)
    if feature('COORDINATOR_MODE'): saveMode(modeApi?.isCoordinatorMode() ? 'coordinator' : 'normal')

    restoredAttribution = opts.includeAttribution ? computeRestoredAttributionState(result) : undefined
    standaloneAgentContext = computeStandaloneAgentContext(result.agentName, result.agentColor)
    void updateSessionName(result.agentName)
    refreshedAgentDefs = await refreshAgentDefinitionsForModeSwitch(...)

    return {
        messages, fileHistorySnapshots, contentReplacements,
        agentName, agentColor, restoredAgentDef,
        initialState: { ...context.initialState, agent?, attribution?, standaloneAgentContext?, agentDefinitions: refreshedAgentDefs },
    }
```

Then REPL/print calls `restoreSessionStateFromLog(result, setAppState)` (`sessionRestore.ts:99-150`):

- Always: `fileHistoryRestoreStateFromLog(snapshots, …)` → `setAppState(prev => ({...prev, fileHistory: newState}))`.
- If `feature('COMMIT_ATTRIBUTION') && attributionSnapshots.length > 0`: `attributionRestoreStateFromLog`.
- If `feature('CONTEXT_COLLAPSE')`: `contextCollapse.restoreFromEntries(commits ?? [], snapshot)` (called unconditionally — even with empty inputs — because `restoreFromEntries` resets the store first to drop a prior `/resume`'s stale commit log).
- If `!isTodoV2Enabled() && messages.length > 0`: scan transcript backwards for the last `TodoWriteTool` `tool_use` and seed `AppState.todos[getSessionId()]` (SDK/non-interactive only — interactive uses file-backed v2 tasks; `:138-149`).

`restoreWorktreeForResume` (`sessionRestore.ts:332-366`) prefers a fresh `getCurrentWorktreeSession()` (set by `--worktree`); else `process.chdir(worktreeSession.worktreePath)` (TOCTOU-safe ENOENT check); on success: `setCwd`, `setOriginalCwd(getCwd())`, `restoreWorktreeSession(...)`, then clear memory-file caches, system-prompt sections, plans-dir cache (`projectRoot` is intentionally NOT updated — the transcript can't distinguish `--worktree` startup from mid-session EnterWorktreeTool entry). On ENOENT: `saveWorktreeState(null)` so the next metadata re-append records "exited" instead of resurrecting a dead path.

`exitRestoredWorktree` (`:380-400`) is the inverse for mid-session `/resume` switching to a non-worktree session.

### 5.9 `getLastSessionLog`

`sessionStorage.ts:3869-3932`:

```
getLastSessionLog(sid):
    {messages, summaries, customTitles, tags, agentSettings, worktreeStates,
     fileHistorySnapshots, attributionSnapshots, contentReplacements,
     contextCollapseCommits, contextCollapseSnapshot} = await loadSessionFile(sid)
    if messages.size === 0: return null
    if !getSessionMessages.cache.has(sid):
        getSessionMessages.cache.set(sid, Promise.resolve(new Set(messages.keys())))
    lastMessage = findLatestMessage(messages.values(), m => !m.isSidechain)
    if !lastMessage: return null
    transcript = buildConversationChain(messages, lastMessage)
    return convertToLogOption(transcript, ..., contentReplacements.get(sid) ?? []) +
           {worktreeSession, contextCollapseCommits, contextCollapseSnapshot}
```

`buildConversationChain` (`:2069-2094`) does a leaf→root walk via `parentUuid`, detects cycles (logs `tengu_chain_parent_cycle`, returns partial), then `recoverOrphanedParallelToolResults` recovers sibling assistant blocks from streaming's content_block_stop fan-out and orphaned `tool_result`s indexed by `parentUuid` (`:2096-...`). Recovery preserves group contiguity for `normalizeMessagesForAPI`.

### 5.10 `reAppendSessionMetadata`

`sessionStorage.ts:721-839`. Reads the trailing `LITE_READ_BUF_SIZE` window with `readFileTailSync`; absorbs any fresher SDK-written `custom-title` / `tag` into the cache (filter with `startsWith('{"type":"custom-title"')` to skip nested tool-input JSON); then unconditionally re-appends `last-prompt`, `custom-title`, `tag`, `agent-name`, `agent-color`, `agent-setting`, `mode`, `worktree-state`, and `pr-link` (when all three pr-link fields present), in that order. Empty-string SDK writes (`renameSession(id, null)` → `customTitle: ""`) clear the cache.

`skipTitleRefresh` is set by `adoptResumedSessionFile` because `restoreSessionMetadata` populated the cache from the same disk read microseconds prior — refreshing again would clobber a `--name`-supplied CLI title with the stale disk value (`:1530-1534`).

### 5.11 `recordSidechainTranscript` and `writeAgentMetadata`

Per-agent file `${projectDir}/${sessionId}/subagents[/<subdir>]/agent-<agentId>.jsonl` plus sidecar `agent-<agentId>.meta.json` (JSON-encoded `AgentMetadata`). The metadata sidecar is the only place `agentType` and `worktreePath` survive — without it, resuming a fork silently degrades to `general-purpose` (4 KiB system prompt, no inherited history) (`:274-289`).

Cited from 30: `tasks/LocalMainSessionTask.ts:360,416`, `tools/AgentTool/runAgent.ts:735,738,794`, `utils/forkedAgent.ts:531,588`.

**Forked-subagent lifecycle (writer-side, this spec; orchestration-side, 30; runtime mode, 09).** The two `forkedAgent.ts:531,588` writer call sites enter `recordSidechainTranscript` with `agentId !== undefined` and a `startingParentUuid` — this is the **exact** condition that bypasses the `messageSet` dedup in §5.4 so the forked transcript can re-emit the parent's tail under a fresh agent file without colliding. The forked subagent inherits parent transcript via the `startingParentUuid` arg (parent UUID threaded through `recordSidechainTranscript`); the resulting agent file is the durable record of the fork. This is distinct from `--fork-session` (§5.1, §5.3) — `--fork-session` mints a new top-level session id at resume time; forked subagents nest under an existing session as sidechain JSONL files. Bubble runtime mode (a 9.5/spec-09 concept governing prompt-permission expansion inside forked subagents) is invisible to this spec's writer path because §5.18's `toExternalPermissionMode` flatten erases it before any persistence-relevant diff fires — see §5.18 cross-ref to spec 09.

### 5.12 `BG_SESSIONS` background-task summary attachments

Cited from 04 (`query.ts:118` lazy module load via `feature('BG_SESSIONS')`; `query.ts:1685` injection). Persistence path owned here: the resulting `task-summary` entry (Entry type at `types/logs.ts:93-98`) is appended via `appendEntry` like any metadata entry (`sessionStorage.ts:1169-1170`). Last-wins on read; no chain participation.

`AppState.remoteBackgroundTaskCount` (`AppStateStore.ts:132`) is event-sourced from CCR `system/task_started` and `system/task_notification` WS events; the local `tasks` map is empty in viewer mode because the actual tasks live in the daemon child.

### 5.13 `AWAY_SUMMARY`

Cited from 04 (`hooks/useAwaySummary.ts:54` gates on `feature('AWAY_SUMMARY')`). The summary is generated by `services/awaySummary.ts.generateAwaySummary` and does not persist a dedicated entry type — it surfaces through the existing `task-summary` slot for `claude ps` / `claude logs` consumption.

### 5.14 `AGENT_MEMORY_SNAPSHOT`

`main.tsx:2258` checks `feature('AGENT_MEMORY_SNAPSHOT') && mainThreadAgentDefinition && isCustomAgent(mainThreadAgentDefinition) && mainThreadAgentDefinition.memory && mainThreadAgentDefinition.pendingSnapshotUpdate`. The actual snapshot path lives in `tools/AgentTool/loadAgentsDir.ts:348` (`feature('AGENT_MEMORY_SNAPSHOT') && isAutoMemoryEnabled()`). On-disk effect: writes a per-agent memory snapshot file alongside the agent's transcript; format is JSON with the agent definition's `memory` payload. The detailed memory algorithm is owned by 29.

### 5.15 Prompt history (`history.ts`) flush semantics

```
addToHistory(command):
    if CLAUDE_CODE_SKIP_PROMPT_HISTORY: return
    if !cleanupRegistered:
        cleanupRegistered = true
        registerCleanup(async () => {
            if currentFlushPromise: await currentFlushPromise
            if pendingEntries.length > 0: await immediateFlushHistory()
        })
    void addToPromptHistory(command)

addToPromptHistory(command):
    storedPastedContents = {}
    for [id, content] in entry.pastedContents:
        if content.type === 'image': continue
        if content.content.length <= MAX_PASTED_CONTENT_LENGTH:
            store inline
        else:
            hash = hashPastedText(content.content)
            store {contentHash: hash, ...}
            void storePastedText(hash, content.content)        # fire-and-forget
    logEntry = {...entry, timestamp: Date.now(), project: getProjectRoot(), sessionId: getSessionId()}
    pendingEntries.push(logEntry); lastAddedEntry = logEntry
    currentFlushPromise = flushPromptHistory(0)

flushPromptHistory(retries):
    if isWriting || pendingEntries.length === 0: return
    if retries > 5: return                         # bail until next prompt
    isWriting = true
    try:
        await immediateFlushHistory()
    finally:
        isWriting = false
        if pendingEntries.length > 0:
            await sleep(500)
            void flushPromptHistory(retries + 1)

immediateFlushHistory():
    historyPath = join(getClaudeConfigHomeDir(), 'history.jsonl')
    writeFile(historyPath, '', {mode: 0o600, flag: 'a'})        # ensure exists
    release = await lock(historyPath, {stale: 10000, retries: {retries: 3, minTimeout: 50}})
    jsonLines = pendingEntries.map(e => jsonStringify(e) + '\n'); pendingEntries = []
    await appendFile(historyPath, jsonLines.join(''), {mode: 0o600})
    await release()
```

Source: `history.ts:292-409`.

`getHistory()` orders entries: current-session first (yielded eagerly), then a buffered second pass for other-session entries — both still bounded by the same `MAX_HISTORY_ITEMS` window (`:190-217`). `getTimestampedHistory()` is current-project, deduped by `display`, lazy paste resolution.

`removeLastFromHistory()` pops from `pendingEntries` if present; otherwise `skippedTimestamps.add(entry.timestamp)` and `getHistory`/`makeHistoryReader` filter the timestamp out at read time. One-shot (`lastAddedEntry = null` after).

### 5.16 File-history three-phase commit

Each of `fileHistoryTrackEdit` and `fileHistoryMakeSnapshot` runs:

1. **Phase 1 — capture**: a no-op updater that records the current state without spreading (`fileHistory.ts:102-107`, `:213-218`).
2. **Phase 2 — async backup**: `createBackup(filePath, version)` outside the updater. Version = 1 for a track edit; version = `mostRecent.version + 1` for a snapshot.
3. **Phase 3 — commit**: a second updater that re-checks tracked-files (race-safe against concurrent edits), shallow-spreads `snapshots` and `trackedFiles`, fires `recordFileHistorySnapshot(messageId, snapshot, isSnapshotUpdate)` fire-and-forget.

`MAX_SNAPSHOTS = 100` is enforced by `allSnapshots.length > MAX_SNAPSHOTS ? allSnapshots.slice(-MAX_SNAPSHOTS) : allSnapshots` (`:309-310`). `snapshotSequence` is a monotonic counter incremented on every snapshot regardless of eviction — used by `useGitDiffStats` as activity signal once `snapshots.length` plateaus at 100.

Resume-time hard-link migration (`copyFileHistoryForResume`, `:922-...`): builds `${configDir}/file-history/${newSessionId}/`, then for each backup `link(oldPath, newPath)`; falls back to copy on hard-link failure; tolerates `EEXIST` (already migrated). All snapshots and links run in parallel via `Promise.allSettled`.

The "lazy 100ms write queue" cited in scope refers to the `Project` writer's `FLUSH_INTERVAL_MS = 100` (§5.4). File-history snapshots themselves go through the same queue via `recordFileHistorySnapshot` → `appendEntry({type:'file-history-snapshot', ...})`.

### 5.17 Hydration (remote / CCR v2)

- `hydrateRemoteSession(sid, ingressUrl)` (`:1587-1622`) replaces local JSONL with `sessionIngress.getSessionLogs` output via `writeFile` (truncates), then sets `remoteIngressUrl` on the project.
- `hydrateFromCCRv2InternalEvents(sid)` (`:1632-1723`) uses `setInternalEventReader` / `setInternalSubagentEventReader` callbacks (`bootstrap-`-registered by 35); writes foreground events to the session JSONL and per-agent events to `getAgentTranscriptPath(asAgentId(agent_id))`. Re-throws `'CCRClient: Epoch mismatch (409)'` so the worker can race against `gracefulShutdown`.

When a remote ingress URL or CCR v2 internal writer is registered, `FLUSH_INTERVAL_MS` drops to `REMOTE_FLUSH_INTERVAL_MS = 10` (`sessionStorage.ts:530`, `:1349-1360`).

### 5.18 onChangeAppState diff effects

`src/state/onChangeAppState.ts:43-171`. For each change between `oldState` and `newState`:

- **`toolPermissionContext.mode`** (single choke point):
  - `prevExternal = toExternalPermissionMode(prev)`; `newExternal = toExternalPermissionMode(new)`.
  - If `prevExternal !== newExternal`: compute `isUltraplan = (newExternal==='plan' && newState.isUltraplanMode && !oldState.isUltraplanMode) ? true : null` (RFC 7396 null-removes), call `notifySessionMetadataChanged({permission_mode: newExternal, is_ultraplan_mode: isUltraplan})`.
  - Always call `notifyPermissionModeChanged(newMode)` (raw mode for SDK status stream).
  - **Cross-spec 09 (bubble runtime mode):** the `toExternalPermissionMode` flatten at `onChangeAppState.ts:74-76` collapses `bubble → 'default'` and `ungated_auto → 'acceptEdits'` (or equivalent — see 09 for the canonical mapping). These two modes are exactly the ones invisible to CCR via this `prevExternal !== newExternal` guard: a `default → bubble → default` sequence flattens to `default → default → default` and the CCR `notifySessionMetadataChanged` notify is suppressed as noise. The **raw** `notifyPermissionModeChanged` SDK channel still fires (its `print.ts` listener applies its own filter). A reader debugging "why doesn't `bubble` mode appear in CCR `external_metadata`" should land here, then traverse to spec 09 for `bubble`/`ungated_auto` definitions and to spec 35 for CCR-side metadata schema.
- **`mainLoopModel`**: `null` → `updateSettingsForSource('userSettings', {model: undefined}); setMainLoopModelOverride(null)`. Non-null → save and override.
- **`expandedView`**: persisted as `globalConfig.{showExpandedTodos: ev==='tasks', showSpinnerTree: ev==='teammates'}`.
- **`verbose`**: persisted as `globalConfig.verbose`.
- **`tungstenPanelVisible`** (ANT-only via `process.env.USER_TYPE === 'ant'`): persisted as `globalConfig.tungstenPanelVisible`.
- **`settings`**: `clearApiKeyHelperCache()`, `clearAwsCredentialsCache()`, `clearGcpCredentialsCache()`. If `settings.env` changed: `applyConfigEnvironmentVariables()`.

---

## 6. Verbatim Assets

### 6.1 On-disk path constants

```
~/.claude/                                              # getClaudeConfigHomeDir()
~/.claude/history.jsonl                                 # global prompt history
~/.claude/file-history/<sessionId>/<backupFileName>     # per-session file backups
~/.claude/projects/                                     # getProjectsDir()
~/.claude/projects/<sanitizePath(cwd)>/                 # getProjectDir(cwd) (memoized)
~/.claude/projects/<dir>/<sessionId>.jsonl              # getTranscriptPath()
~/.claude/projects/<dir>/<sessionId>/subagents[/<subdir>]/agent-<agentId>.jsonl   # getAgentTranscriptPath
~/.claude/projects/<dir>/<sessionId>/subagents[/<subdir>]/agent-<agentId>.meta.json   # getAgentMetadataPath
~/.claude/projects/<dir>/<sessionId>/remote-agents/remote-agent-<taskId>.meta.json    # getRemoteAgentMetadataPath
```

### 6.2 Constants table (verbatim)

| Constant | Value | Citation |
|---|---|---|
| `MAX_TRANSCRIPT_READ_BYTES` | `50 * 1024 * 1024` | `sessionStorage.ts:229` |
| `MAX_TOMBSTONE_REWRITE_BYTES` | `50 * 1024 * 1024` | `sessionStorage.ts:123` |
| `Project.FLUSH_INTERVAL_MS` (default) | `100` | `sessionStorage.ts:567` |
| `REMOTE_FLUSH_INTERVAL_MS` | `10` | `sessionStorage.ts:530` |
| `Project.MAX_CHUNK_BYTES` | `100 * 1024 * 1024` | `sessionStorage.ts:568` |
| `LITE_READ_BUF_SIZE` (re-exported) | `64 * 1024` (sessionStoragePortable.ts) | `sessionStorage.ts:87` |
| `EPHEMERAL_PROGRESS_TYPES` | `Set([bash_progress, powershell_progress, mcp_progress, +sleep_progress when PROACTIVE\|KAIROS])` | `sessionStorage.ts:186-193` |
| `MAX_HISTORY_ITEMS` | `100` | `history.ts:19` |
| `MAX_PASTED_CONTENT_LENGTH` | `1024` | `history.ts:20` |
| `lock` retry policy | `{stale: 10000, retries: {retries: 3, minTimeout: 50}}` | `history.ts:308-314` |
| `flushPromptHistory` max retries | `5` | `history.ts:335` |
| `flushPromptHistory` backoff | `await sleep(500)` between rounds | `history.ts:347` |
| `MAX_SNAPSHOTS` (file history) | `100` | `fileHistory.ts:54` |
| `HISTORY_PAGE_SIZE` (remote) | `100` | `assistant/sessionHistory.ts:7` |
| `PANEL_GRACE_MS` | `30_000` (inlined; in sync with `framework.ts`) | `state/teammateViewHelpers.ts:8` |
| `SKIP_PRECOMPACT_THRESHOLD` (re-exported) | sessionStoragePortable.ts | `sessionStorage.ts:90` |
| `SKIP_FIRST_PROMPT_PATTERN` | `/^(?:\s*<[a-z][\w-]*[\s>]\|\[Request interrupted by user[^\]]*\])/` | `sessionStorage.ts:125-126` |
| `cleanupPeriodDays === 0` ⇒ skip persistence | gate | `sessionStorage.ts:966` |
| File mode | `0o600` for files, `0o700` for `mkdir` | `sessionStorage.ts:636-641,1601,1655,1689-1696` |
| Anthropic-beta header (remote events) | `ccr-byoc-2025-07-29` | `assistant/sessionHistory.ts:39` |

### 6.3 `AppState` shape (verbatim, abbreviated comments retained)

```ts
export type CompletionBoundary =
  | { type: 'complete'; completedAt: number; outputTokens: number }
  | { type: 'bash'; command: string; completedAt: number }
  | { type: 'edit'; toolName: string; filePath: string; completedAt: number }
  | {
      type: 'denied_tool'
      toolName: string
      detail: string
      completedAt: number
    }

export type SpeculationResult = {
  messages: Message[]
  boundary: CompletionBoundary | null
  timeSavedMs: number
}

export type SpeculationState =
  | { status: 'idle' }
  | {
      status: 'active'
      id: string
      abort: () => void
      startTime: number
      messagesRef: { current: Message[] }                            // Mutable ref
      writtenPathsRef: { current: Set<string> }                      // Mutable ref - relative paths
      boundary: CompletionBoundary | null
      suggestionLength: number
      toolUseCount: number
      isPipelined: boolean
      contextRef: { current: REPLHookContext }
      pipelinedSuggestion?: {
        text: string
        promptId: 'user_intent' | 'stated_intent'
        generationRequestId: string | null
      } | null
    }

export const IDLE_SPECULATION_STATE: SpeculationState = { status: 'idle' }

export type FooterItem =
  | 'tasks' | 'tmux' | 'bagel' | 'teams' | 'bridge' | 'companion'

export type AppState = DeepImmutable<{
  settings: SettingsJson
  verbose: boolean
  mainLoopModel: ModelSetting
  mainLoopModelForSession: ModelSetting
  statusLineText: string | undefined
  expandedView: 'none' | 'tasks' | 'teammates'
  isBriefOnly: boolean
  showTeammateMessagePreview?: boolean                                // ENABLE_AGENT_SWARMS only
  selectedIPAgentIndex: number
  coordinatorTaskIndex: number                                        // -1 = pill, 0 = main, 1..N = agent rows
  viewSelectionMode: 'none' | 'selecting-agent' | 'viewing-agent'
  footerSelection: FooterItem | null
  toolPermissionContext: ToolPermissionContext
  spinnerTip?: string
  agent: string | undefined                                           // --agent CLI flag or settings
  kairosEnabled: boolean
  remoteSessionUrl: string | undefined
  remoteConnectionStatus: 'connecting' | 'connected' | 'reconnecting' | 'disconnected'
  remoteBackgroundTaskCount: number
  replBridgeEnabled: boolean
  replBridgeExplicit: boolean
  replBridgeOutboundOnly: boolean
  replBridgeConnected: boolean
  replBridgeSessionActive: boolean
  replBridgeReconnecting: boolean
  replBridgeConnectUrl: string | undefined
  replBridgeSessionUrl: string | undefined
  replBridgeEnvironmentId: string | undefined
  replBridgeSessionId: string | undefined
  replBridgeError: string | undefined
  replBridgeInitialName: string | undefined
  showRemoteCallout: boolean
}> & {
  tasks: { [taskId: string]: TaskState }                              // not DeepImmutable (function fields)
  agentNameRegistry: Map<string, AgentId>
  foregroundedTaskId?: string
  viewingAgentTaskId?: string
  companionReaction?: string
  companionPetAt?: number
  mcp: {
    clients: MCPServerConnection[]
    tools: Tool[]
    commands: Command[]
    resources: Record<string, ServerResource[]>
    pluginReconnectKey: number                                        // bumped by /reload-plugins
  }
  plugins: {
    enabled: LoadedPlugin[]
    disabled: LoadedPlugin[]
    commands: Command[]
    errors: PluginError[]
    installationStatus: {
      marketplaces: Array<{ name: string; status: 'pending'|'installing'|'installed'|'failed'; error?: string }>
      plugins: Array<{ id: string; name: string; status: 'pending'|'installing'|'installed'|'failed'; error?: string }>
    }
    needsRefresh: boolean
  }
  agentDefinitions: AgentDefinitionsResult
  fileHistory: FileHistoryState
  attribution: AttributionState
  todos: { [agentId: string]: TodoList }
  remoteAgentTaskSuggestions: { summary: string; task: string }[]
  notifications: { current: Notification | null; queue: Notification[] }
  elicitation: { queue: ElicitationRequestEvent[] }
  thinkingEnabled: boolean | undefined
  promptSuggestionEnabled: boolean
  sessionHooks: SessionHooksState
  tungstenActiveSession?: { sessionName: string; socketName: string; target: string }
  tungstenLastCapturedTime?: number
  tungstenLastCommand?: { command: string; timestamp: number }
  tungstenPanelVisible?: boolean
  tungstenPanelAutoHidden?: boolean
  bagelActive?: boolean
  bagelUrl?: string
  bagelPanelVisible?: boolean
  computerUseMcpState?: {                                             // CHICAGO_MCP only
    allowedApps?: readonly { bundleId: string; displayName: string; grantedAt: number }[]
    grantFlags?: { clipboardRead: boolean; clipboardWrite: boolean; systemKeyCombos: boolean }
    lastScreenshotDims?: { width: number; height: number; displayWidth: number; displayHeight: number; displayId?: number; originX?: number; originY?: number }
    hiddenDuringTurn?: ReadonlySet<string>
    selectedDisplayId?: number
    displayPinnedByModel?: boolean
    displayResolvedForApps?: string
  }
  replContext?: {
    vmContext: import('vm').Context
    registeredTools: Map<string, { name: string; description: string; schema: Record<string, unknown>; handler: (args: Record<string, unknown>) => Promise<unknown> }>
    console: { log/error/warn/info/debug: (...args: unknown[]) => void; getStdout: () => string; getStderr: () => string; clear: () => void }
  }
  teamContext?: {
    teamName: string
    teamFilePath: string
    leadAgentId: string
    selfAgentId?: string
    selfAgentName?: string
    isLeader?: boolean
    selfAgentColor?: string
    teammates: { [teammateId: string]: { name: string; agentType?: string; color?: string; tmuxSessionName: string; tmuxPaneId: string; cwd: string; worktreePath?: string; spawnedAt: number } }
  }
  standaloneAgentContext?: { name: string; color?: AgentColorName }
  inbox: { messages: Array<{ id: string; from: string; text: string; timestamp: string; status: 'pending'|'processing'|'processed'; color?: string; summary?: string }> }
  workerSandboxPermissions: { queue: Array<{ requestId: string; workerId: string; workerName: string; workerColor?: string; host: string; createdAt: number }>; selectedIndex: number }
  pendingWorkerRequest: { toolName: string; toolUseId: string; description: string } | null
  pendingSandboxRequest: { requestId: string; host: string } | null
  promptSuggestion: { text: string | null; promptId: 'user_intent'|'stated_intent'|null; shownAt: number; acceptedAt: number; generationRequestId: string | null }
  speculation: SpeculationState
  speculationSessionTimeSavedMs: number
  skillImprovement: { suggestion: { skillName: string; updates: { section: string; change: string; reason: string }[] } | null }
  authVersion: number                                                 // incremented on login/logout
  initialMessage: { message: UserMessage; clearContext?: boolean; mode?: PermissionMode; allowedPrompts?: AllowedPrompt[] } | null
  pendingPlanVerification?: { plan: string; verificationStarted: boolean; verificationCompleted: boolean }
  denialTracking?: DenialTrackingState
  activeOverlays: ReadonlySet<string>
  fastMode?: boolean
  advisorModel?: string
  effortValue?: EffortValue
  ultraplanLaunching?: boolean
  ultraplanSessionUrl?: string
  ultraplanPendingChoice?: { plan: string; sessionId: string; taskId: string }
  ultraplanLaunchPending?: { blurb: string }
  isUltraplanMode?: boolean
  replBridgePermissionCallbacks?: BridgePermissionCallbacks
  channelPermissionCallbacks?: ChannelPermissionCallbacks
}

export type AppStateStore = Store<AppState>
```

Citation: `src/state/AppStateStore.ts:41-454`. `getDefaultAppState()` body at `:456-569` initializes every required field; `toolPermissionContext.mode` is `'plan'` when `isTeammate() && isPlanModeRequired()`, else `'default'` (`:463-466`).

### 6.4 Transcript JSON-line schemas (verbatim)

```ts
export type SerializedMessage = Message & {
  cwd: string
  userType: string
  entrypoint?: string                                                 // CLAUDE_CODE_ENTRYPOINT
  sessionId: string
  timestamp: string
  version: string
  gitBranch?: string
  slug?: string
}

export type TranscriptMessage = SerializedMessage & {
  parentUuid: UUID | null
  logicalParentUuid?: UUID | null                                     // preserved when parentUuid is nullified for session breaks
  isSidechain: boolean
  gitBranch?: string
  agentId?: string
  teamName?: string
  agentName?: string
  agentColor?: string
  promptId?: string                                                   // OTel correlation for user prompts
}

// Per-line metadata entries (one or more allowed; last-wins or append-replay per type):
export type SummaryMessage         = { type: 'summary';        leafUuid: UUID; summary: string }
export type CustomTitleMessage     = { type: 'custom-title';   sessionId: UUID; customTitle: string }
export type AiTitleMessage         = { type: 'ai-title';       sessionId: UUID; aiTitle: string }
export type LastPromptMessage      = { type: 'last-prompt';    sessionId: UUID; lastPrompt: string }
export type TaskSummaryMessage     = { type: 'task-summary';   sessionId: UUID; summary: string; timestamp: string }
export type TagMessage             = { type: 'tag';            sessionId: UUID; tag: string }
export type AgentNameMessage       = { type: 'agent-name';     sessionId: UUID; agentName: string }
export type AgentColorMessage      = { type: 'agent-color';    sessionId: UUID; agentColor: string }
export type AgentSettingMessage    = { type: 'agent-setting';  sessionId: UUID; agentSetting: string }
export type PRLinkMessage          = { type: 'pr-link';        sessionId: UUID; prNumber: number; prUrl: string; prRepository: string; timestamp: string }
export type ModeEntry              = { type: 'mode';           sessionId: UUID; mode: 'coordinator'|'normal' }
export type WorktreeStateEntry     = { type: 'worktree-state'; sessionId: UUID; worktreeSession: PersistedWorktreeSession | null }
export type ContentReplacementEntry= { type: 'content-replacement'; sessionId: UUID; agentId?: AgentId; replacements: ContentReplacementRecord[] }
export type FileHistorySnapshotMessage = { type: 'file-history-snapshot'; messageId: UUID; snapshot: FileHistorySnapshot; isSnapshotUpdate: boolean }
export type AttributionSnapshotMessage = {
  type: 'attribution-snapshot'; messageId: UUID; surface: string                  // 'cli'|'ide'|'web'|'api'
  fileStates: Record<string, FileAttributionState>
  promptCount?: number; promptCountAtLastCommit?: number
  permissionPromptCount?: number; permissionPromptCountAtLastCommit?: number
  escapeCount?: number; escapeCountAtLastCommit?: number
}
export type FileAttributionState = { contentHash: string; claudeContribution: number; mtime: number }
export type SpeculationAcceptMessage = { type: 'speculation-accept'; timestamp: string; timeSavedMs: number }

export type ContextCollapseCommitEntry = {
  type: 'marble-origami-commit'                                       // discriminator obfuscated to keep external builds clean
  sessionId: UUID
  collapseId: string                                                  // 16-digit; max across entries reseeds counter
  summaryUuid: string
  summaryContent: string                                              // full <collapsed id="...">text</collapsed>
  summary: string
  firstArchivedUuid: string
  lastArchivedUuid: string
}
export type ContextCollapseSnapshotEntry = {
  type: 'marble-origami-snapshot'
  sessionId: UUID
  staged: Array<{ startUuid: string; endUuid: string; summary: string; risk: number; stagedAt: number }>
  armed: boolean
  lastSpawnTokens: number
}

export type PersistedWorktreeSession = {
  originalCwd: string
  worktreePath: string
  worktreeName: string
  worktreeBranch?: string
  originalBranch?: string
  originalHeadCommit?: string
  sessionId: string
  tmuxSessionName?: string
  hookBased?: boolean
}

export type Entry =
  | TranscriptMessage
  | SummaryMessage | CustomTitleMessage | AiTitleMessage | LastPromptMessage
  | TaskSummaryMessage | TagMessage | AgentNameMessage | AgentColorMessage
  | AgentSettingMessage | PRLinkMessage
  | FileHistorySnapshotMessage | AttributionSnapshotMessage | QueueOperationMessage
  | SpeculationAcceptMessage | ModeEntry | WorktreeStateEntry | ContentReplacementEntry
  | ContextCollapseCommitEntry | ContextCollapseSnapshotEntry
```

Citation: `src/types/logs.ts:8-330`.

`AgentMetadata` (sidecar):

```ts
export type AgentMetadata = {
  agentType: string
  worktreePath?: string                                                // when isolation: 'worktree'
  description?: string                                                 // original AgentTool task description
}
```

Citation: `sessionStorage.ts:264-272`.

`RemoteAgentMetadata` (sidecar):

```ts
export type RemoteAgentMetadata = {
  taskId: string
  remoteTaskType: string
  sessionId: string                                                    // CCR session id (Sessions API)
  title: string
  command: string
  spawnedAt: number
  toolUseId?: string
  isLongRunning?: boolean
  isUltraplan?: boolean
  isRemoteReview?: boolean
  remoteTaskMetadata?: Record<string, unknown>
}
```

Citation: `sessionStorage.ts:305-318`.

### 6.5 Session/external-metadata schemas (verbatim)

```ts
export type SessionState = 'idle' | 'running' | 'requires_action'

export type RequiresActionDetails = {
  tool_name: string
  action_description: string                                          // human-readable
  tool_use_id: string
  request_id: string
  input?: Record<string, unknown>
}

export type SessionExternalMetadata = {
  permission_mode?: string | null
  is_ultraplan_mode?: boolean | null
  model?: string | null
  pending_action?: RequiresActionDetails | null
  post_turn_summary?: unknown                                         // typed at emit site
  task_summary?: string | null                                        // forked summarizer mid-turn output
}
```

Citation: `src/utils/sessionState.ts:1-45`.

### 6.6 `PROJECT_ONBOARDING_STATE` schema (verbatim)

The on-disk schema lives in the project config (`utils/config.ts:113-148`):

```ts
type ProjectConfig = {
  // …other fields…
  hasCompletedProjectOnboarding?: boolean
  projectOnboardingSeenCount: number                                  // default 0
  hasClaudeMdExternalIncludesApproved?: boolean
  hasClaudeMdExternalIncludesWarningShown?: boolean
  // …
}

const DEFAULT_PROJECT_CONFIG: ProjectConfig = {
  allowedTools: [],
  mcpContextUris: [],
  mcpServers: {},
  enabledMcpjsonServers: [],
  disabledMcpjsonServers: [],
  hasTrustDialogAccepted: false,
  projectOnboardingSeenCount: 0,
  hasClaudeMdExternalIncludesApproved: false,
  hasClaudeMdExternalIncludesWarningShown: false,
}
```

`projectOnboardingState.ts` step shape (verbatim):

```ts
export type Step = {
  key: string                                                          // 'workspace' | 'claudemd'
  text: string
  isComplete: boolean
  isCompletable: boolean
  isEnabled: boolean
}

export function getSteps(): Step[] {
  const hasClaudeMd = getFsImplementation().existsSync(join(getCwd(), 'CLAUDE.md'))
  const isWorkspaceDirEmpty = isDirEmpty(getCwd())
  return [
    { key: 'workspace',
      text: 'Ask Claude to create a new app or clone a repository',
      isComplete: false, isCompletable: true, isEnabled: isWorkspaceDirEmpty },
    { key: 'claudemd',
      text: 'Run /init to create a CLAUDE.md file with instructions for Claude',
      isComplete: hasClaudeMd, isCompletable: true, isEnabled: !isWorkspaceDirEmpty },
  ]
}
```

`shouldShowProjectOnboarding` short-circuits when `hasCompletedProjectOnboarding || projectOnboardingSeenCount >= 4 || process.env.IS_DEMO`. Threshold = 4 visible runs. (Citations: `projectOnboardingState.ts:19-83`.)

### 6.7 Critical regex assets

- `SKIP_FIRST_PROMPT_PATTERN = /^(?:\s*<[a-z][\w-]*[\s>]\|\[Request interrupted by user[^\]]*\])/` — anchored at line start; matches lowercase-tag XML opens (IDE/hook/notification messages) or the literal interrupt marker. Used by `getFirstMeaningfulUserMessageTextContent` to skip non-meaningful messages. (`sessionStorage.ts:125-126`.)
- Pasted-content reference parser: `/\[(Pasted text|Image|\.\.\.Truncated text) #(\d+)(?: \+\d+ lines)?(\.)*\]/g` — group 2 is the numeric id. (`history.ts:65-74`.)
- Newline counter for `+N lines` ref: `/\r\n|\r|\n/g` — the count is the number of separators, not the number of lines, by historical compatibility (`history.ts:47-49`).
- Tombstone needle: `"uuid":"<UUID>"` — full key-value pattern, not bare UUID, so `parentUuid` matches don't false-positive (`sessionStorage.ts:892-894`).

---

## 7. Side Effects & I/O

### 7.1 Filesystem

Reads:
- `~/.claude/history.jsonl` (`readLinesReverse` for prompt history; `history.ts:118`).
- `~/.claude/projects/<dir>/<sessionId>.jsonl` (single-pass for `loadSessionFile` / `loadTranscriptFile`; tail-only for `readFileTailSync` / `readLiteMetadata`).
- Per-agent `subagents/agent-*.jsonl` and `*.meta.json` sidecars (resume, AgentTool resume).
- `~/.claude/file-history/<sessionId>/<backupFileName>` (`copyFileHistoryForResume` reads via hard-link source).

Writes:
- `appendFile(path, lines, {mode: 0o600})` for the session JSONL, with `mkdir(dirname, {recursive: true, mode: 0o700})` retry.
- `writeFile` (truncate) on remote/CCRv2 hydration.
- `lock` file `<historyPath>.lock` for prompt history (`history.ts:308`).
- Tombstone via `fh.truncate(absLineStart)` + `fh.write(...)` (positional rewrite of trailing lines).
- `writeFile` of `*.meta.json` sidecars (no lock; race-tolerant).
- `link` (hard-link), fallback `copyFile` for file-history migration.

Cleanup:
- `registerCleanup` in `getProject()` flushes pending writes then `reAppendSessionMetadata` on graceful shutdown (`sessionStorage.ts:447-466`).
- `registerCleanup` in `addToHistory` awaits in-flight flush then issues a final `immediateFlushHistory` (`history.ts:418-431`).

### 7.2 Network

- `sessionIngress.appendSessionLog` / `getSessionLogs` — owned by 35 (cited `sessionStorage.ts:1333,1597`).
- CCR v2 internal-event reader/writer — registered by 35; this spec consumes their callable signatures only (`sessionStorage.ts:490-507`).
- `assistant/sessionHistory.ts` calls `axios.get` to `${BASE_API_URL}/v1/sessions/<sid>/events` with OAuth headers + `anthropic-beta: ccr-byoc-2025-07-29` + `x-organization-uuid`; `timeout: 15000`, `validateStatus: () => true`.

### 7.3 Process spawn / signals

- `gracefulShutdownSync(1, 'other')` on remote-persistence failure (`sessionStorage.ts:1341`).
- `process.chdir` in `restoreWorktreeForResume` — TOCTOU-safe ENOENT check (`sessionRestore.ts:343`).
- No spawns — this layer is filesystem + memory only.

### 7.4 Environment variables consumed

| Var | Effect | Citation |
|---|---|---|
| `CLAUDE_CODE_ENTRYPOINT` | Stamped into `SerializedMessage.entrypoint` | `sessionStorage.ts:423-425` (via getEntrypoint), `types/logs.ts:11` |
| `USER_TYPE` | `'ant'` gates `tungstenPanelVisible` mirror; voice provider gate; identity-selector throw | `state/onChangeAppState.ts:143`, `state/AppState.tsx:16`, `:142-163` |
| `CLAUDE_CODE_SKIP_PROMPT_HISTORY` | Skips both prompt history and session-storage persistence | `history.ts:414`, `sessionStorage.ts:968` |
| `TEST_ENABLE_SESSION_PERSISTENCE` | Allow session persistence under `NODE_ENV=test` | `sessionStorage.ts:961-965` |
| `NODE_ENV === 'test'` | Default-skip persistence (with above override) | `sessionStorage.ts:965` |
| `ENABLE_SESSION_PERSISTENCE` | Enable v1 session-ingress persistence path | `sessionStorage.ts:1326-1330` |
| `IS_DEMO` | Force-suppress project onboarding | `projectOnboardingState.ts:70` |
| `CLAUDE_CODE_ENABLE_SDK_FILE_CHECKPOINTING` | Enable file history in SDK | `fileHistory.ts:74` |
| `CLAUDE_CODE_DISABLE_FILE_CHECKPOINTING` | Disable file history (overrides everything) | `fileHistory.ts:69,76` |
| `CLAUDE_CODE_EMIT_SESSION_STATE_EVENTS` | Mirror `idle`/`running` to SDK event stream | `sessionState.ts:127` |

### 7.5 Permissions / trust

This layer enforces NO permission checks itself; it inherits filesystem permission via OS umask. Persistence skipping (`shouldSkipPersistence`) is the closest thing to a guard. Trust state (`sessionTrustAccepted`) lives in `STATE` and is consumed by other layers.

---

## 8. Feature Flags & Variants

| Flag / env | On | Off |
|---|---|---|
| `feature('VOICE_MODE')` | `VoiceProvider` from `context/voice.js` wraps children | `({children}) => children` passthrough (`state/AppState.tsx:14-18`) |
| `feature('PROACTIVE') \| feature('KAIROS')` | `EPHEMERAL_PROGRESS_TYPES` includes `'sleep_progress'` | excluded (`sessionStorage.ts:190-193`) |
| `feature('COMMIT_ATTRIBUTION')` (ANT-only) | `attributionRestoreStateFromLog` runs on resume; `recordAttributionSnapshot` callable | resume skips (`sessionRestore.ts:111-119`) |
| `feature('CONTEXT_COLLAPSE')` | `contextCollapse.restoreFromEntries` runs unconditionally (resets store first) and `marble-origami-*` entries are written | resume skips entirely (`sessionRestore.ts:127-136,494-502`) |
| `feature('COORDINATOR_MODE')` | `matchSessionMode` consulted on resume; `mode` entries persisted via `saveMode` | mode entry writes are no-ops (`sessionRestore.ts:428-432,514-516`) |
| `feature('BG_SESSIONS')` | `taskSummaryModule` lazy-loaded; `task-summary` entries written; `claude ps`/`logs`/`attach`/`kill` CLI verbs and `--bg` recognized | no entries (`query.ts:118,1685`; `entrypoints/cli.tsx:185`) |
| `feature('AWAY_SUMMARY')` | `useAwaySummary` runs and emits a `task-summary` slot via `services/awaySummary.generateAwaySummary` | hook returns early (`hooks/useAwaySummary.ts:54`) |
| `feature('AGENT_MEMORY_SNAPSHOT')` | Per-agent memory snapshot file written when `isCustomAgent(def) && def.memory && def.pendingSnapshotUpdate && isAutoMemoryEnabled()` | snapshot path skipped (`main.tsx:2258`, `tools/AgentTool/loadAgentsDir.ts:348`) |
| `feature('HISTORY_SNIP')` | `applySnipRemovals` runs on load; `snipMetadata.removedUuids` honored | older boundaries without `removedUuids` skip — pre-snip history loads (`sessionStorage.ts:1982-...`) |
| `feature('CHICAGO_MCP')` | `computerUseMcpState` populated in AppState | absent (`AppStateStore.ts:259`) |
| `process.env.USER_TYPE === 'ant'` | `tungstenPanelVisible` mirrored to `globalConfig`; identity-selector throws in `useAppState` (currently dead — see §3.1 footnote ‡); voice provider gate path active | external builds skip these branches (`onChangeAppState.ts:143`, `AppState.tsx:142-163`) |
| `process.env.USER_TYPE === 'ant'` (slash-command gate) | `/tag` slash command registered (`isEnabled: () => process.env.USER_TYPE === 'ant'` at `commands/tag/index.ts:7`); user can toggle searchable tags on the current session | `/tag` invisible in external builds. **Note:** the `Tag` Entry schema (§6.4), `tag` `appendEntry` dispatch (§3.3 / §5.4), and `reAppendSessionMetadata` re-emission (§5.10) are **all universal** — on-disk `tag` entries from prior ANT-internal sessions resume cleanly on external builds (the SDK or older recordings can also originate them); only the user-facing `/tag` command itself is gated. |
| `--no-session-persistence` | `setSessionPersistenceDisabled(true)` ⇒ all transcript writes skipped, `--resume` impossible. Allowed only with `--print` | normal persistence (`main.tsx:991`, `bootstrap/state.ts:1325-1331`) |
| Remote ingress URL set OR CCR v2 internal writer set | `FLUSH_INTERVAL_MS = 10` | `100` (`sessionStorage.ts:530,1349-1360`) |
| `globalConfig.fileCheckpointingEnabled === false` | File history disabled | enabled (`fileHistory.ts:67-70`) |

`-c, --continue`: resume the most recent session in current dir (`main.tsx:988`).
`-r, --resume [value]`: resume by session id, or open picker with optional search (`main.tsx:988`).
`--fork-session`: when resuming, mint a new session id instead of reusing (`main.tsx:988`).
`--resume-session-at <message id>`: only messages up to and including the assistant message with `<message.id>` (use with `--resume` in print mode; `main.tsx:991`).
`--rewind-files <user-message-id>`: restore files to state at the specified user message and exit (requires `--resume`; `main.tsx:991`).
`--from-pr [value]`: resume a session linked to a PR (KAIROS_GITHUB_WEBHOOKS gated; `main.tsx:991`).

---

## 9. Error Handling & Edge Cases

- **Empty pasted content / image refs** — `expandPastedTextRefs` only inlines text; image refs become content blocks elsewhere (`history.ts:81-100`).
- **Cycle in `parentUuid` chain** — `buildConversationChain` logs `tengu_chain_parent_cycle` and returns the partial transcript walked so far (`sessionStorage.ts:2076-2086`).
- **Preserved-segment walk broken** — `applyPreservedSegmentRelinks` logs `tengu_relink_walk_broken` (with tail/head/anchor presence flags + walkSteps + transcriptSize) and returns; resume loads full pre-compact history rather than corrupting the splice (`:1888-1902`).
- **Stale preserved segment** (manual `/compact` after reactive compact) — `segIsLive=false`; relink is skipped but absolute-prune still removes everything before the absolute-last boundary.
- **Tombstone outside tail window** — slow path bails when `fileSize > MAX_TOMBSTONE_REWRITE_BYTES = 50 MB`; logs `Skipping tombstone removal: session file too large` and returns (`:927-932`).
- **`appendEntry` for other-session** — when `sid !== currentSessionId` and the session file doesn't exist on disk, logs `appendEntry: session file not found for other session <sid>` and returns (`:1148-1153`).
- **Remote persistence failure** — `persistToRemote` calls `gracefulShutdownSync(1, 'other')` after `logEvent('tengu_session_persistence_failed', {})` (`:1339-1342`).
- **CCR v2 epoch mismatch** — re-throws `'CCRClient: Epoch mismatch (409)'` so the worker doesn't race against `gracefulShutdown` (`:1713-1718`).
- **Worktree dir gone on resume** — `restoreWorktreeForResume` swallows `process.chdir` ENOENT, calls `saveWorktreeState(null)` so next metadata re-append records exit (`sessionRestore.ts:343-350`).
- **Bypass-permissions race on mount** — `AppStateProvider` mount-only `useEffect` checks `isBypassPermissionsModeAvailable && isBypassPermissionsModeDisabled()` and applies `createDisabledBypassPermissionsContext` if true; logs `'Disabling bypass permissions mode on mount (remote settings loaded before mount)'` (`AppState.tsx:60-73`).
- **Settings change → cache clears** — wrapped in try/catch with `logError(toError(error))` so a failing cache clear doesn't break state propagation (`onChangeAppState.ts:156-170`).
- **Prompt history flush disk error** — `immediateFlushHistory` logs `Failed to write prompt history: <error>` via `logForDebugging` and continues (`history.ts:319-326`); flush retries up to 5 with 500 ms backoff between rounds before bailing.
- **Lock stale** — `lock(historyPath, {stale: 10000, retries: {retries: 3, minTimeout: 50}})` (`history.ts:308-314`); stale-detection at 10 s avoids permanent deadlock from crashed CLIs.
- **`--no-session-persistence` not with `--print`** — main.tsx enforces; otherwise rejected with stderr error.
- **`--session-id` with `--continue`/`--resume`** — only allowed when `--fork-session` also provided; else: `'Error: --session-id can only be used with --continue or --resume if --fork-session is also specified.'` (`main.tsx:1279-1282`).
- **`AppStateProvider` nested** — throws `'AppStateProvider can not be nested within another AppStateProvider'` (`AppState.tsx:46`).
- **`useAppState` outside provider** — `useAppStore` throws `'useAppState/useSetAppState cannot be called outside of an <AppStateProvider />'` (`AppState.tsx:120-123`).
- **Identity selector (ANT)** — `'Your selector in \`useAppState(${selector.toString()})\` returned the original state, which is not allowed. You must instead return a property for optimised rendering.'` (`AppState.tsx:151-153`).

---

## 10. Telemetry & Observability

Analytics events (`logEvent` from `services/analytics`):

| Event | Sites |
|---|---|
| `tengu_transcript_view_enter` / `tengu_transcript_view_exit` | `state/teammateViewHelpers.ts:51,91` |
| `tengu_chain_parent_cycle` | `sessionStorage.ts:2083` |
| `tengu_relink_walk_broken` (with `tailInTranscript`, `headInTranscript`, `anchorInTranscript`, `walkSteps`, `transcriptSize`) | `sessionStorage.ts:1894-1900` |
| `tengu_chain_parallel_tr_recovered` (with `recovered_count`) | `sessionStorage.ts:2195-2197` |
| `tengu_snip_resume_filtered` (with `removed_count`, `relinked_count`) | `sessionStorage.ts:2035-2038` |
| `tengu_session_persistence_failed` | `sessionStorage.ts:1319,1340` |
| `tengu_file_history_track_edit_success` / `_failed` | `fileHistory.ts:111,180,189` |

Diagnostics (`logForDiagnosticsNoPII`): `hydrate_remote_session_fail`, `hydrate_ccr_v2_completed` (with `duration_ms`, `event_count`, `subagent_event_count`), `hydrate_ccr_v2_read_fail`, `hydrate_ccr_v2_fail`.

OTel correlation: every user `TranscriptMessage` carries `promptId` from `getPromptId()`. The `lastMainRequestId` slot in bootstrap state is read at shutdown to send cache-eviction hints to inference.

SDK event mirror: `notifySessionStateChanged` enqueues `{type: 'system', subtype: 'session_state_changed', state}` when `CLAUDE_CODE_EMIT_SESSION_STATE_EVENTS` is truthy (`sessionState.ts:127-132`).

---

## 11. Reimplementation Checklist

Invariants a re-implementer MUST preserve:

- [ ] `createStore.setState` is `Object.is`-gated; identical refs do NOT trigger `onChange` or listeners (`store.ts:23`).
- [ ] `AppStateProvider` constructs the store exactly once with `useState(() => createStore(...))`; nesting throws.
- [ ] `recordTranscript`'s prefix-only dedup tracking (`!seenNewMessage`) — required for compact-boundary correctness.
- [ ] `insertMessageChain` re-stamps `userType`/`entrypoint`/`cwd`/`sessionId`/`version`/`gitBranch`/`slug` AFTER the `...message` spread, so resumed/forked messages get re-keyed.
- [ ] `parentUuid: null` for compact boundaries; `logicalParentUuid` carries the previous parent.
- [ ] `tool_result` parent override: `user.sourceToolAssistantUUID` becomes `effectiveParentUuid` when present.
- [ ] `appendEntry` buffers into `pendingEntries` while `sessionFile === null`; first user/assistant message triggers `materializeSessionFile`, which writes metadata first then drains.
- [ ] `appendEntry` dedup against `messageSet` is bypassed only when `entry.isSidechain && entry.agentId !== undefined` and only for the LOCAL write; remote ingress still observes single-chain semantics (do not push to remote from the sidechain bypass branch).
- [ ] `FLUSH_INTERVAL_MS = 100` default; drops to `10` whenever a remote ingress URL OR CCR v2 internal writer is registered.
- [ ] `MAX_CHUNK_BYTES = 100 MiB` per appendFile call; resolvers fire after their chunk's flush.
- [ ] `flush()` waits for both queue drain AND `pendingWriteCount` (non-queue tracked operations like `removeMessageByUuid`).
- [ ] Tombstone fast-path uses `"uuid":"<UUID>"` needle (key-value), NEVER bare UUID; slow path is gated by `MAX_TOMBSTONE_REWRITE_BYTES = 50 MB`.
- [ ] `applyPreservedSegmentRelinks`: validate tail→head BEFORE mutating; zero out `usage.input_tokens/output_tokens/cache_creation_input_tokens/cache_read_input_tokens` on every preserved assistant; absolute prune is independent of seg-live.
- [ ] `reAppendSessionMetadata` order is `last-prompt → custom-title → tag → agent-name → agent-color → agent-setting → mode → worktree-state → pr-link`. SDK-mutable fields (`custom-title`, `tag`) refresh from tail before re-append; empty-string clears the cache.
- [ ] `adoptResumedSessionFile` sets `sessionFile` and calls `reAppendSessionMetadata(skipTitleRefresh=true)` — required so `--name foo` survives a quit-before-message.
- [ ] `switchSession` is the ONLY way to atomically change `sessionId` + `sessionProjectDir`. Never set them independently.
- [ ] `regenerateSessionId({setCurrentAsParent})` deletes the outgoing plan-slug entry and resets `sessionProjectDir = null`.
- [ ] `MAX_HISTORY_ITEMS = 100`; `MAX_PASTED_CONTENT_LENGTH = 1024`. `addToHistory` skip via `CLAUDE_CODE_SKIP_PROMPT_HISTORY`. Pasted text > 1024 chars goes to `paste-store/<contentHash>`. Image entries skipped.
- [ ] `getHistory` orders current-session-first, then other-session, all bounded by `MAX_HISTORY_ITEMS`.
- [ ] `removeLastFromHistory` is one-shot; the second call is a no-op.
- [ ] `MAX_SNAPSHOTS = 100` evict-oldest; `snapshotSequence` is monotonic.
- [ ] `copyFileHistoryForResume` uses `link()` first, falls back to `copyFile`; tolerates `EEXIST`.
- [ ] `restoreSessionStateFromLog` calls `contextCollapse.restoreFromEntries` UNCONDITIONALLY when `feature('CONTEXT_COLLAPSE')` (so a `/resume` resets a stale prior-session commit log).
- [ ] `restoreWorktreeForResume` prefers fresh `--worktree` worktree over transcript-recorded worktree; ENOENT triggers `saveWorktreeState(null)`.
- [ ] `processResumedConversation` does NOT re-run `restoreWorktreeForResume`/`adoptResumedSessionFile` on `--fork-session` (uses fresh startup id).
- [ ] `--fork-session` strips `worktreeSession` from the metadata pass (fork doesn't take ownership).
- [ ] `getLastSessionLog` primes `getSessionMessages` cache only when empty (mid-session callers must not clobber a live cache).
- [ ] `shouldShowProjectOnboarding` cap: 4 visible runs.
- [ ] `PANEL_GRACE_MS = 30_000` for terminal-task lingering; `evictAfter = 0` is the immediate-dismiss sentinel.
- [ ] On-disk file mode is `0o600`; directories `0o700`.
- [ ] Anthropic-beta header for remote events: `ccr-byoc-2025-07-29` exact.

---

## 12. Open Questions / Unknowns

1. **Layout of pre-PR#24099 progress entries on disk** — bridged at load via `progressBridge` rewrite (cited at `sessionStorage.ts:151-156`) but the bridge implementation lives in `loadTranscriptFile` body which was not read fully. A reimplementer needs that body for legacy-transcript faithfulness.
2. **`getSessionMessages` cache topology** — referenced as `memoize`d (`getSessionMessages.cache.has/set/`) but its full body and eviction policy live elsewhere in `sessionStorage.ts`; only the call sites consumed here.
3. **`loadTranscriptFile` / `loadSessionFile` full pipeline** — these read-side functions are referenced from `loadFullLog`/`getLastSessionLog`/`hydrate*` but their internals (chain reconstruction, leaf-uuid set, `progressBridge`, `applyPreservedSegmentRelinks` invocation order) require dedicated read; out of budget for sub-H1's writer-focused scope. Spec 04/07 may need cross-citation.
4. **`AGENT_MEMORY_SNAPSHOT` snapshot file format** — `main.tsx:2258` triggers, `loadAgentsDir.ts:348` checks; the actual file path/format lives in code paths owned by 29 (memory).
5. **`snipMetadata.removedUuids` boundary subtype** — `applySnipRemovals` reads it but the boundary subtype is described as "in `excluded-strings.txt`" because `HISTORY_SNIP` is ANT-only; the literal must not leak. The exact discriminator is therefore obfuscated and not recorded here; cross-cite 19 (HISTORY_SNIP tool surface) for the canonical name.
6. **Plan/worktree state persistence detail** — `mode` and `worktree-state` entries are documented here; the actual `saveMode` / `saveWorktreeState` callers live in 18 (worktree mode) and 30 (coordinator) — those specs own when these are emitted.
7. **`BG_SESSIONS` task-summary cadence** — owned by 04 (`min(5 steps, 2min)` cited in `types/logs.ts:88-92`); this spec only documents the entry shape, not the trigger.
8. **`speculation-accept` entry write site** — entry type defined in `types/logs.ts:233-237`; the writer call site was not located in the read passes (likely `screens/REPL.tsx` speculation paths). Cross-cite 04/37.

---
