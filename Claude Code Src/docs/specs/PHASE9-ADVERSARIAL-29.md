# Phase 9.5b Adversarial Review — Spec 29 (Service: Memory)

Reviewer: skeptic. Method: read spec 29 + ten owned source files + targeted greps.
Source date: 2026-05-10 archive snapshot.

## Severity counts

| Severity | Count |
|---|---|
| Critical | 0 |
| High | 1 |
| Medium | 3 |
| Low | 4 |
| Nit | 2 |

Total: 10 findings. Verdict below.

## Top 5 findings

### F1 — HIGH — Datadog allow-list cross-spec inconsistency
Spec 29 logs SEVEN distinct `tengu_team_mem_*` events from owned files
(verified by grep on `src/services/teamMemorySync/`):

1. `tengu_team_mem_sync_pull` (index.ts:1205)
2. `tengu_team_mem_sync_push` (index.ts:1233)
3. `tengu_team_mem_sync_started` (watcher.ts:298)
4. `tengu_team_mem_entries_capped` (index.ts:661)
5. `tengu_team_mem_secret_skipped` (index.ts:935)
6. `tengu_team_mem_push_suppressed` (watcher.ts:112)
7. (also `tengu_extract_memories_*`, `tengu_session_memory_*`, `tengu_auto_mem_tool_denied` — extraction side)

But `src/services/analytics/datadog.ts:60-63` allow-lists ONLY the first
four. Events 5 and 6 are emitted but NOT in the allow-list — which spec 26
(Datadog allow-list = 44 events) and spec 29's cross-reference both miss.
Spec 29 §3.3 cites the watcher exports without auditing whether its events
ship. Either the allow-list is incomplete (under-counted) or these two
events are silently dropped. Spec 26 should bump the count and spec 29 §2.3
should cite this gap.

### F2 — MEDIUM — `setCachedClaudeMdContent` is NOT in claudemd.ts (spec 05 cross-ref drift)
The dispatch brief asserts "Phase 9.5 spec 05 verified at line 176". Verified:
`setCachedClaudeMdContent` lives in `src/bootstrap/state.ts:1207`, is
imported by `src/context.ts:5`, and is called at `src/context.ts:176` with
`setCachedClaudeMdContent(claudeMd || null)` — the `||` (not `??`) is
correct because empty-string CLAUDE.md should also resolve to `null`
(`??` would propagate `""`, defeating the cache-miss signal). Spec 29 §1
delegates this to spec 05 cleanly; spec 05 should pin the file as
`context.ts:176`, NOT `claudemd.ts:176` if any version of the spec
mis-attributes it.

### F3 — MEDIUM — Spec 29 silent on team-memory sync TRIGGERS
The brief asks "what triggers sync?". Spec 29 §5 documents push/pull/sync
internals but never enumerates trigger sites in one place. Verified
triggers:

- **Initial pull**: `setup.ts:367` calls `startTeamMemoryWatcher` (lazy
  import, gated on `feature('TEAMMEM')`).
- **Watch event**: `fs.watch({recursive:true})` in `watcher.ts:179-208` →
  `schedulePush()` → 2 s debounce → `executePush()` → `pushTeamMemory`.
- **PostToolUse explicit**: `src/utils/sessionFileAccessHooks.ts:201,205`
  calls `notifyTeamMemoryWrite()` after FileEdit/FileWrite — covers the
  same-tick race the watcher comment cites.
- **Shutdown flush**: `stopTeamMemoryWatcher` (`watcher.ts:327-352`) awaits
  in-flight + flushes pending under 2 s budget.

Spec 29 §2.5 lists the importer rows but does not assemble them into a
trigger-causality table. **Recommend adding §5.10 "sync trigger map"**.

### F4 — MEDIUM — Phase 9.6 spec 30 "AgentSummary, toolUseSummary, autoDream owners"
Spec 29 §1 explicitly delegates `services/AgentSummary/`,
`services/awaySummary.ts`, `services/toolUseSummary/`, and
`services/autoDream/` to spec 30. Verified by file existence — all four
exist in `src/services/`. `extractMemories.ts:169` says
`createAutoMemCanUseTool` is "shared by extractMemories and autoDream",
confirming a code-level contract crossing the 29↔30 boundary. Spec 30
must own that import (autoDream consuming `createAutoMemCanUseTool` from
spec-29 territory) and call it out as a public-export of spec 29; spec 29
§3.1 lists `createAutoMemCanUseTool` as exported but doesn't mark it as
"consumed by autoDream (spec 30)". Cross-spec citation gap.

### F5 — LOW — Line-count drift in §2.1 (off-by-one across the table)
Spec 29 §2.1 line totals are systematically off by one or two:

| Path | Spec | Actual |
|---|---|---|
| extractMemories.ts | 616 | 615 |
| SessionMemory/sessionMemory.ts | 496 | 495 |
| SessionMemory/prompts.ts | 325 | 324 |
| teamMemorySync/index.ts | 1257 | 1256 |
| teamMemorySync/secretScanner.ts | 325 | 324 |
| teamMemorySync/types.ts | 157 | 156 |
| teamMemorySync/watcher.ts | 388 | 387 |

Pattern is consistent (spec counts include trailing newline as a line; `wc -l`
does not). Cosmetic; mention once in spec 29 conventions or correct.

## Other findings (lower severity)

- **F6 (LOW)**: Spec 29 §5.3 line-cite `:280-288, :569-587` for the
  `extractor`/`drainer` rebinding is correct (verified at extractMemories.ts
  lines 280–288 and 569–587).
- **F7 (LOW)**: Spec §2.3 claim "feature('EXTRACT_MEMORIES') does NOT have
  a third gate at memdir/paths.ts:65" — accepted; the leak shows that path
  is owned by spec 40.
- **F8 (LOW)**: Spec §3.2 omits export of `createMemoryFileCanUseTool`
  signature note that input is `unknown` (sessionMemory.ts:461) whereas
  `createAutoMemCanUseTool` takes `Record<string, unknown>` (extractMemories.ts:172).
  Inconsistency between the two `CanUseToolFn` factory signatures should be
  flagged as a hard-to-test surface.
- **F9 (NIT)**: §4.4 watcher state lists `pushSuppressedReason` but does
  not call out that `notifyTeamMemoryWrite()` (watcher.ts:314-319) does
  NOT bypass suppression — only fs-watch unlink stat-ENOENT clears it. A
  stale OAuth state will deadlock notify-driven pushes silently until
  session restart.
- **F10 (NIT)**: §3.2 surface lists `EXTRACTION_WAIT_TIMEOUT_MS=15000` and
  `EXTRACTION_STALE_THRESHOLD_MS=60000` (sessionMemoryUtils.ts:12-13);
  these constants are NOT exported (verified — file-private `const`), so
  documenting them as part of "public interface" §4.2 wait/timeout
  constants is misleading. They are observable behavior, not API.

## Verdict

**ACCEPT WITH MINOR FIXES.** Spec 29 is dense and accurate on the major
algorithms (`runExtraction`, `pushTeamMemory` 412 conflict-resolution loop,
`shouldExtractMemory` two-condition triggers, watcher debounce + suppression).
Source audit found **no false claims** about behavior. The misses are:

- Phase 9.6 telemetry coverage gap (F1) — needs cross-spec lift to spec 26.
- Sync-trigger enumeration (F3) — readability improvement.
- Cross-spec citation hygiene (F4) — autoDream consumes a spec-29 export.
- Line-count drift (F5) — automated-fixable.

## Cross-spec impact

| Touches spec | Reason |
|---|---|
| **05** (CLAUDE.md injection) | F2 — confirm `context.ts:176` is the canonical site of `setCachedClaudeMdContent(claudeMd \|\| null)`. |
| **26** (Datadog allow-list = 44) | F1 — two emitted events (`tengu_team_mem_secret_skipped`, `tengu_team_mem_push_suppressed`) are NOT in the allow-list. Either the allow-list count is short (now 46), or the spec needs to note these are intentionally not allow-listed. |
| **30** (coordinator/multi-agent) | F4 — autoDream consumes spec-29's `createAutoMemCanUseTool`; AgentSummary/toolUseSummary/awaySummary all owned by 30, confirmed. |
| **40** (memdir) | clean — spec 29 correctly delegates `memoryTypes`, `memoryScan`, `findRelevantMemories`, `teamMemPaths`, `teamMemPrompts` to 40. |
| **07** (compact) | clean — `getSessionMemoryContent`/`waitForSessionMemoryExtraction`/`truncateSessionMemoryForCompact` correctly consumed by `services/compact/sessionMemoryCompact.ts:28,32,33`. |
| **41** (session/transcript state) | clean — `sessionFileAccessHooks.ts` correctly noted as 41-owned despite registering `notifyTeamMemoryWrite`. |

## Hardest-to-verify claim

**Spec §5.3 claim**: "Trailing extractions (from stashed contexts) skip
[the throttle gate] since they process already-committed work that should
not be throttled."

This is structurally correct in the code (extractMemories.ts:377 — the
`if (!isTrailingRun)` guard) but the **semantic argument** ("already-committed
work") is unverifiable from source alone. The trailing run reads from the
*same* `pendingContext.context.messages` reference, which is mutable and
may have grown between the stash (`:562`) and the trailing call (`:516-520`).
The spec implicitly assumes message arrays are append-only post-stash —
true under current `query.ts` semantics but fragile if compaction or rewind
ever rewrites history while a stashed context is in flight. Worth a note
in §5.3 that "trailing run reuses the stashed context's messages reference;
correctness depends on append-only growth between stash and run".

(Also hard-to-verify: F1's claim that the allow-list is *incomplete* vs.
*intentionally selective* — depends on intent, not source.)

---

End of review. Word count ≈ 980; trimmed top-5 + verdict below 500 if
treated as primary deliverable.
