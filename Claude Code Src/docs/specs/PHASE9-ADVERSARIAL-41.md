# Phase 9.5b Adversarial Review — Spec 41: Session State, History & Resume

**Spec:** `docs/specs/41-session-state-history.md` (1258 lines)
**Reviewer role:** Skeptic
**Source files cross-checked:** `src/state/{store,AppStateStore,AppState,onChangeAppState,selectors,teammateViewHelpers}.ts(x)`, `src/history.ts`, `src/assistant/sessionHistory.ts`, `src/projectOnboardingState.ts`, plus targeted greps into `src/utils/sessionStorage.ts`, `src/utils/sessionRestore.ts`, `src/utils/fileHistory.ts`, `src/types/logs.ts`, `src/commands/{resume,rewind,tag}/`.
**Reads consumed:** ~17.

---

## Severity Counts

| Severity | Count |
|---|---:|
| BLOCKER | 0 |
| HIGH    | 1 |
| MEDIUM  | 3 |
| LOW     | 4 |
| Nits    | 2 |

---

## Top 5 Findings

### F1 — HIGH: `/rewind` slash-command path is invisible in spec 41

The skeptic prompt asked specifically about `/rewind` cross-spec to 21d. Spec 41's §1 scope and §8 only enumerate the **CLI flag** `--rewind-files <user-message-id>` (`main.tsx:991`), describing it as a *files-only restore*. The interactive `/rewind` slash command (`src/commands/rewind/rewind.ts`) is a thin wrapper that calls `context.openMessageSelector()` and returns `{type:'skip'}` — it surfaces a UI that ultimately drives `removeTranscriptMessage` / `removeMessageByUuid` (the tombstone path documented in §5.7 / §3.3).

Spec 41 owns the tombstone primitive but never connects it to the user-facing `/rewind` command path. A reader of spec 41 cannot answer "where does the conversation rewind UI write its tombstones?" without traversing 21d. There is also no statement that `/rewind` reuses the `removeMessageByUuid` slow/fast-path gating (`MAX_TOMBSTONE_REWRITE_BYTES = 50MB`). The `removeLastFromHistory` path (history.ts:453) is documented for **prompt history** undo only — distinct from `/rewind` of the conversation transcript — and the spec doesn't disambiguate.

**Impact:** A re-implementer building `/rewind` from spec 41 alone will miss the tombstone gating semantics. Cross-ref to 21d alone is insufficient because 21d won't redocument the tombstone fast/slow path.

**Fix:** Add a §5.x "/`/rewind` slash command" subsection that maps `openMessageSelector → removeTranscriptMessage(targetUuid) → removeMessageByUuid` and references §5.7. Distinguish the *conversation* rewind from the *prompt-history* `removeLastFromHistory` undo.

### F2 — MEDIUM: Forked-session lifecycle conflates `--fork-session` (resume-time fork) with forked subagents (Phase 9.5/spec 09 bubble runtime mode)

Spec 41 §5.1, §5.3, §11 cover `--fork-session` thoroughly: outgoing id stashed via `regenerateSessionId({setCurrentAsParent})`, fresh `randomUUID()`, no worktree-takeover. But **forked subagents** — the `utils/forkedAgent.ts:531,588` writer call sites cited in §5.11 — are documented only as *transcript writers*, not as a session lifecycle concern.

The skeptic prompt explicitly asks: "Phase 9.5 spec 09 finding: bubble runtime mode in forked subagents — does spec 41 reference forked-session lifecycle?" Answer: **no**. Spec 41 mentions `forkedAgent.ts` exactly twice (in `2.4 Imported by` and §5.11 citation), and the word "bubble" appears nowhere. There is no statement that:
- forked subagents inherit parent transcript via `recordSidechainTranscript(..., startingParentUuid)` with `agentId !== undefined` (the *exact* condition that bypasses the `messageSet` dedup in §5.4);
- the bubble permission-mode (a 9.5/spec-09 concept) does NOT reach CCR via `onChangeAppState` because `toExternalPermissionMode()` flattens `bubble → 'default'` (`onChangeAppState.ts:74-76` — verified in source).

**Impact:** Spec 41 *implicitly* describes the bubble mode flatten in the §5.18 "external mode didn't change" comment, but never names "bubble" or links to spec 09. A reader investigating "why doesn't bubble mode show up in CCR external_metadata" cannot find the answer in 41.

**Fix:** Add an explicit cross-ref to spec 09 (`bubble` runtime mode) inside §5.18 noting that `bubble` and `ungated_auto` are exactly the modes flattened by `toExternalPermissionMode` and that the resulting "external mode didn't change" guard is what makes them invisible to CCR. Add a §5.11 sentence noting `forkedAgent.ts:531,588` calls inherit parent UUID via the `startingParentUuid` arg and rely on the §5.4 sidechain bypass.

### F3 — MEDIUM: `/tag` slash command — ANT-only gating not surfaced in spec 41 contract

Spec 41 §3.3 lists the `tag` entry type in the `appendEntry` dispatch table and §6.4 schemas it as `TagMessage`. But the user-facing `/tag` slash command (`src/commands/tag/index.ts:7`) is **ANT-only**: `isEnabled: () => process.env.USER_TYPE === 'ant'`.

Spec 41 documents `USER_TYPE === 'ant'` gates for `tungstenPanelVisible` (§5.18), voice provider (§4 / §8), and identity-selector (§3.1) — but not for `/tag`. A re-implementer reading §3.3 / §6.4 will assume `tag` entries are emitted by all builds; they are not. The asymmetric persistence — `reAppendSessionMetadata` always re-appends a cached tag (§5.10) regardless of build — is correct but only writes back what was already on disk, so external builds will never produce one.

**Impact:** Glossary disambiguation for "tag" is incomplete. SDK callers writing `tag` entries via the SDK still work (entry schema is universal), but `/tag` itself is invisible externally.

**Fix:** Add a row to §8 feature-flag table: `process.env.USER_TYPE === 'ant'` enables `/tag` slash command. Note that on-disk `tag` entries from prior ANT-internal sessions are still re-appended by `reAppendSessionMetadata` even on external builds.

### F4 — MEDIUM: Three-place ownership boundary (`history.ts` vs `state/` vs `assistant/sessionHistory.ts`) is documented but not summarized

The skeptic prompt asks: "assistant/sessionHistory.ts vs state/ vs history.ts — three places, what do they own?" Spec 41 §3.4, §3.5, §3.1 cover each individually, but there is no consolidated table. Verified ownership from source:

| File | Owns |
|---|---|
| `src/state/*` | In-process `AppState` Zustand-style store + diff effects + selectors. Zero on-disk persistence. |
| `src/history.ts` | **Prompt** history (the up-arrow / ctrl+r picker). On-disk: `~/.claude/history.jsonl` (cross-project), `paste-store/<contentHash>`. NOT the conversation transcript. |
| `src/assistant/sessionHistory.ts` | **Remote SDK pagination** of CCR session events. Read-only HTTP client; no on-disk side effects. `ccr-byoc-2025-07-29` beta header. |

The fact that `history.ts` is **prompt** history and not **conversation/transcript** history is load-bearing — these are easy to confuse. Spec 41 §1.1, §1.2, §1.3 imply the split but never state it as a one-line disambiguation.

**Fix:** Add a one-line ownership table at the top of §3 explicitly distinguishing prompt-history-of-the-CLI-input-line vs transcript-of-the-conversation vs remote-SDK-pagination.

### F5 — LOW: Glossary cleanliness — spec 00 owns the 3-way "session" disambiguation; spec 41 owns (a) "transcript-file session" cleanly

Spec 00 line 396 defines three meanings of "session": (a) transcript-file, (b) Ink REPL, (c) remote-server. Verified.

Spec 41 §1.1 says: "in-process app state … that every UI surface, hook, and tool reads/writes through" (this is **(b)** Ink REPL territory) and §1.2 "on-disk session transcripts" (clearly **(a)**). The split between (a) and (b) is real and legitimate but the spec's title "Session State, History & Resume" doesn't pick a side. **§1 line 1** should explicitly say "this spec owns (a) and the *persistence-relevant slice* of (b); (c) is owned by 35."

The remote-server session **(c)** is mostly handled correctly: §3.5 (remote SDK pagination) and §5.17 (hydrate) are clearly cited as 35-owned-but-this-spec-consumes. Good.

The transcript-file ownership claim is clean — every JSONL path / Entry type / writer is here, no overlap with 35.

**Verdict on glossary question:** Spec 41 owns "transcript-file session" cleanly; the (a)/(b) overlap on AppState is *technically* split but not labeled.

---

## Other findings (LOW / Nits)

- **LOW:** §5.8 references `sessionRestore.ts:442-445` for `switchSession`. Verified at `:442` (call site uses `switchSession(asSessionId(sid), …)`). OK.
- **LOW:** §3.3 cites `recordTranscript` body at `:1408-1449`; §5.2 cites `:1408-1449` and `:1392-1407`. Not double-checked line-by-line (would exceed read budget) but the constants `MAX_TOMBSTONE_REWRITE_BYTES = 50 MB` (`:123`), `REMOTE_FLUSH_INTERVAL_MS = 10` (`:530`), `FLUSH_INTERVAL_MS = 100` (`:567`), `MAX_CHUNK_BYTES = 100 MiB` (`:568`) all verify exactly.
- **LOW:** Spec 41 §11 invariant "`removeLastFromHistory` is one-shot" verified at `history.ts:454-456` (`lastAddedEntry = null` after read).
- **LOW:** `isChainParticipant(m) := m.type !== 'progress'` cited at `:154-156` — verified exactly at `sessionStorage.ts:154-156`.
- **Nit:** §6.2 lock retry policy `{stale: 10000, retries: {retries: 3, minTimeout: 50}}` cited at `history.ts:308-314` — verified exact at `history.ts:308-315`.
- **Nit:** §3.1 says `useAppState`'s identity-selector throw is gated by an "ANT-only sentinel guard"; the source at `AppState.tsx:150` actually evaluates `false && state === selected` — i.e., the guard is **dead** post-bundling (`"external" === 'ant'` always false in external builds; the literal `false` here suggests this was source-bundled in a way that disables the check entirely). Spec 41 says "ANT-only sentinel guard rejects identity selectors" — this is *aspirationally* true but is presently a no-op in the leaked source. Worth a footnote.

---

## Verdict

**ACCEPT WITH MINOR REVISIONS.**

Spec 41 is structurally sound. Constants, schemas, and writer-side algorithms verify against source with no factual errors detected in this read budget. The `recordTranscript` / `insertMessageChain` / `applyPreservedSegmentRelinks` algorithm captures (the load-bearing logic for compaction correctness) appears faithful.

The gaps are **completeness gaps**, not correctness gaps: the spec under-documents the user-facing slash commands (`/rewind`, `/tag`) that drive its primitives, and under-cross-references spec 09's bubble runtime mode. F1 is the only finding I'd call near-shipping-blocker; F2–F4 are quality-of-life improvements for the cross-spec network.

---

## Cross-spec impact

- **Spec 21d (slash commands)** must own `/resume`, `/rewind`, `/tag` user surfaces but should explicitly defer to 41 for tombstone gating, transcript writers, and `tag` entry semantics. Bidirectional pointer required.
- **Spec 09 (bubble runtime mode)** is the missing piece for §5.18's `toExternalPermissionMode` flatten. A one-line "see 09 for `bubble`/`ungated_auto` definition" in §5.18 would close the loop.
- **Spec 30 (coordinator / agent topology)** owns *when* `recordSidechainTranscript` is called and the `forkedAgent.ts:531,588` lifecycle. Spec 41 §5.11 cross-cites correctly. No change needed.
- **Spec 35 (remote / CCR)** ownership of `sessionIngress` and `setInternalEventReader` is clean — §5.17 and §2.5 explicitly defer.
- **Spec 04 (turn pipeline)** owns `recordTranscript` / `flushSessionStorage` call sites; §2.2 cites the line numbers correctly. No change.
- **Spec 00 (overview/glossary)** the 3-way "session" disambiguation works; spec 41 doesn't break it but could state which definitions it owns at §1 line 1.

---

## Hardest-to-verify claim

**§5.6 `applyPreservedSegmentRelinks` invariant 6: "Zero stale usage on every preserved assistant: `input_tokens=0, output_tokens=0, cache_creation_input_tokens=0, cache_read_input_tokens=0` — otherwise on-disk tokens reflect pre-compact ~190K context and resume immediately autocompacts."**

This is a behavioral claim about the interaction between (a) the load-side splice in `sessionStorage.ts:1839-1956`, (b) the autocompaction trigger threshold (likely owned by 07), and (c) the tokens-on-disk encoding in `TranscriptMessage.usage` (which is itself derived from API responses, owned by 03/06).

To verify, I would need to:
1. Read `applyPreservedSegmentRelinks` body to confirm the four `usage.*` fields are zeroed (not just one or two).
2. Cross-check `services/contextCollapse/persist.ts` and the autocompaction threshold logic in 07 to confirm that pre-zeroing is actually load-bearing for not-immediately-recompacting on resume.
3. Confirm that the on-disk `usage` field is even consulted on resume (vs recomputed from message content).

This single claim spans 4 files and 3 specs (07, 06, 41). The `~190K` figure is a magic number; if the autocompaction threshold is something else (e.g., 160K or 200K), the spec is misleading even if the zero-out behavior is correct. Without reading 07's compaction threshold AND the relinker body, I cannot rule out that the zero-out is *defensive* (no-op in practice) rather than load-bearing.

This is the most likely place for a subtle bug-or-spec-mismatch to hide.
