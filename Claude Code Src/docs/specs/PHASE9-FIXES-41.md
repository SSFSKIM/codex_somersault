# Phase 9.6c — Spec 41 Fix Log

**Spec:** `docs/specs/41-session-state-history.md`
**Adversarial review:** `docs/specs/PHASE9-ADVERSARIAL-41.md`
**Verdict applied:** ACCEPT WITH MINOR REVISIONS — completeness fixes only; no correctness changes.

---

## Findings addressed

| # | Sev | Finding | Edit |
|---|---|---|---|
| F1 | HIGH | `/rewind` slash-command path invisible in spec 41 (only `--rewind-files` CLI flag was documented) | New §5.7a "/rewind slash command — user-facing path into the tombstone primitive". Maps `commands/rewind/rewind.ts:8-12` → `openMessageSelector` (21d UI) → `removeTranscriptMessage` (`sessionStorage.ts:1472`) → `removeMessageByUuid` (`:871`, §5.7) with explicit `MAX_TOMBSTONE_REWRITE_BYTES = 50 MB` slow-path gate (`:123`, `:927`). Distinguishes `/rewind` (transcript tombstone) from `removeLastFromHistory` (one-shot prompt-history undo at `history.ts:453`) and from `--rewind-files` (files-only, no tombstone). |
| F2 | MED | Forked-subagent lifecycle conflated with `--fork-session`; bubble runtime mode (spec 09) never named | §5.11 expanded with explicit lifecycle paragraph: `forkedAgent.ts:531,588` writer call sites pass `agentId !== undefined` + `startingParentUuid` → §5.4 sidechain bypass. Distinct from `--fork-session` (top-level new id at resume) vs forked subagents (sidechain JSONL nesting). §5.18 gains explicit cross-spec 09 reference: `toExternalPermissionMode` flatten at `onChangeAppState.ts:74-76` collapses `bubble → 'default'` (verified in source) and `ungated_auto`, suppressing CCR notify; raw SDK channel still fires. |
| F3 | MED | `/tag` ANT-only gate (`commands/tag/index.ts:7`) absent from §8 feature-flag table | New row in §8: `process.env.USER_TYPE === 'ant'` (slash-command gate) → `/tag` registered/invisible. Notes that the `Tag` Entry schema, `appendEntry` dispatch, and `reAppendSessionMetadata` re-emission are universal — only the user-facing command is gated. |
| F4 | MED | Three-place ownership (`state/` vs `history.ts` vs `assistant/sessionHistory.ts`) not summarized | New §3.0 "Three-place ownership boundary" table. Aligns with spec 00:396 3-way "session" glossary (a)/(b)/(c). Explicitly calls out that `history.ts` is **prompt** history (`~/.claude/history.jsonl`), NOT transcript. |
| F5 | LOW | Spec 00 3-way "session" disambiguation: which definitions does spec 41 own? | New paragraph at top of §1: spec 41 owns (a) transcript-file cleanly + persistence-relevant slice of (b) Ink-REPL; rendering-only slice of (b) → 38; (c) remote-server → 35 (consumed read-only). Forward-references §3.0. |
| Nit | — | §3.1 says ANT-only sentinel guard rejects identity selectors — actually dead branch (`false &&` short-circuit at `AppState.tsx:150`) | §3.1 row updated with footnote ‡ noting the source bug. Cross-references existing `BUGS-IN-SOURCE.md` entry #14 (already covers this verbatim — no new bug entry needed). |

## Verifications performed before edit

- `commands/rewind/rewind.ts:8-12` → 4-line wrapper calling `context.openMessageSelector()` then `{type: 'skip'}`. Confirmed.
- `commands/rewind/index.ts` → `aliases: ['checkpoint']`, `type: 'local'`, `supportsNonInteractive: false`. Confirmed.
- `commands/tag/index.ts:7` → `isEnabled: () => process.env.USER_TYPE === 'ant'`. Confirmed.
- `sessionStorage.ts:123` → `MAX_TOMBSTONE_REWRITE_BYTES = 50 * 1024 * 1024`. Confirmed.
- `sessionStorage.ts:871` → `removeMessageByUuid`. Confirmed.
- `sessionStorage.ts:927` → `if (fileSize > MAX_TOMBSTONE_REWRITE_BYTES)` slow-path gate. Confirmed.
- `sessionStorage.ts:1472` → `removeTranscriptMessage` thin wrapper around `getProject().removeMessageByUuid`. Confirmed.
- `history.ts:453-456` → `removeLastFromHistory` one-shot (sets `lastAddedEntry = null`). Confirmed.
- `onChangeAppState.ts:74-76` → `toExternalPermissionMode(prevMode/newMode)` flatten + `prevExternal !== newExternal` guard. Confirmed in source comment block ("default→bubble→default is noise from CCR's POV").
- `AppState.tsx:150` → `if (false && state === selected)` literal-false short-circuit. Confirmed.
- `BUGS-IN-SOURCE.md` entry #14 already documents the dead-branch bug. No new entry required.

## Files modified

- `docs/specs/41-session-state-history.md` — six in-place edits (§1 glossary note, §3.0 new ownership table, §3.1 footnote ‡, §5.7 slow-path warning emphasis, §5.7a new subsection, §5.11 forked-subagent lifecycle expansion, §5.18 spec-09 cross-ref, §8 `/tag` gate row).

## Files NOT modified

- `BUGS-IN-SOURCE.md` — entry #14 already covers `AppState.tsx:150`.
- Specs 09, 21d, 30, 35 — cross-refs added one-way (from 41 outward); reverse cross-refs deferred to those specs' own next pass per the adversarial review's "Cross-spec impact" section.

## Out of scope (deferred)

- F-LOW §5.6 `applyPreservedSegmentRelinks` invariant 6 verification (the "hardest-to-verify claim" in adversarial review): would require reading 07's autocompaction threshold + relinker body + `services/contextCollapse/persist.ts`. Out of scope for completeness pass.
