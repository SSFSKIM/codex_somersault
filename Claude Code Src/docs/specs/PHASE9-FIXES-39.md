# PHASE 9.6c — Fix log: spec 39 (vim & keybindings)

Reviewer findings source: `docs/specs/PHASE9-ADVERSARIAL-39.md`
Pattern: B2 (in-place spec edits) + F2 (BUGS-IN-SOURCE candidate).

## Findings status

| ID | Severity | Status | Notes |
|---|---|---|---|
| H1 | HIGH | **APPLIED** | `KEYBINDING_ACTIONS` count corrected `78 → 86` at §6.3 and §11. Verified actual count via `awk` count of single-quoted entries in `schema.ts:64-172` = 86. |
| H2 | HIGH | **APPLIED** | `Scroll`/`MessageActions` context inconsistency flagged in §6.3 (new paragraph), §11 (extended bullet), and added as confirmed bug #9 in `BUGS-IN-SOURCE.md`. |
| M1 | MEDIUM | **APPLIED** | §5.5 chord context "priority" illusion documented — order is dropped at the resolver `new Set()` boundary; only `useKeybinding(s)` dedup path preserves order. |
| M2 | MEDIUM | **APPLIED** | §5.5 wheel-event bypass extended: explicit "routed only via component-level `useKeybinding(s)`, never the global ChordInterceptor". |
| M3 | MEDIUM | **APPLIED** | §5.6 non-consume case documented — when `result.action !== action`, hook silently no-ops without `stopImmediatePropagation`. |
| Cross-36 | MEDIUM | **APPLIED** | §12 new open-question #8 records the vim INSERT-mode × `voice:pushToTalk` `space` collision as a confirmed gap from spec 36 review; deferred to spec 36 follow-up. |
| LOW × 3 / Nits × 2 | LOW/NIT | NOT IN SCOPE | B2 charter is HIGH + MEDIUM. |

## Verification before edit

- **H1 count.** `awk 'NR>=64 && NR<=172' src/keybindings/schema.ts | grep -c "^  '"` → `86`. Spec asserted `78`.
- **H2 inconsistency.** `grep "context: 'Scroll'" src/keybindings/defaultBindings.ts` → match at `:196`. `grep "'Scroll'" src/keybindings/schema.ts` → no match in `KEYBINDING_CONTEXTS` (`:12-32`). Same for `MessageActions` (only in flag-gated default block at `:268-295`).
- **M1 chord wrap.** `resolver.ts:193` confirmed: `const ctxSet = new Set(activeContexts)` discards order.
- **M2 wheel bypass.** `KeybindingProviderSetup.tsx:238-240` confirmed: `if ((key.wheelUp || key.wheelDown) && pending === null) return`.
- **M3 useKeybinding.** `useKeybinding.ts:64-72` confirmed: `if (result.action === action) { if (handler() !== false) event.stopImmediatePropagation() }` — outer `if` skipped (no consumption) when actions differ.

## Edits

### 39-vim-keybindings.md

1. **§6.3** — `78 entries` → `86 entries; canonical list at schema.ts:64-172` + new "Inconsistency: `Scroll` context" paragraph covering `Scroll` and `MessageActions` enum gap, references `BUGS-IN-SOURCE.md`.
2. **§5.5 Chord lifetime** — appended two bullets: (a) wheel-event routing clarification, (b) "Context-order illusion" explaining the `new Set` wrap and last-binding-wins semantics in chord/single-key resolvers vs. order-preserving dedup in `useKeybinding(s)`.
3. **§5.6 Active contexts and priority** — appended "Non-consume case" sentence explaining only the matching `action` instance calls `stopImmediatePropagation`; sibling `useInput` handlers later in render order can still see and re-resolve the keystroke.
4. **§11 Reimplementation checklist** — `78-action` → `86-action`; appended note that `Scroll`/`MessageActions` appear as `context:` values in defaults despite being absent from the enum.
5. **§12 Open questions** — added new item 8 documenting the vim INSERT × `voice:pushToTalk` `space` collision as confirmed gap from spec 36 review.

### BUGS-IN-SOURCE.md

6. Header total `8 confirmed` → `9 confirmed`.
7. New entry **#9: `Scroll` and `MessageActions` contexts in `DEFAULT_BINDINGS` are absent from `KEYBINDING_CONTEXTS`** with severity `minor`, paths, reproduction, and suggested fix (add both names to `KEYBINDING_CONTEXTS` + `KEYBINDING_CONTEXT_DESCRIPTIONS`).

## Phase 10 ripple

- **Spec 36** must own the resolution of open-question #8 (vim INSERT × voice space). Likely fix: `voice:pushToTalk` handler early-return when `editorMode === 'vim' && vimState.mode === 'INSERT'`. Not visible in leaked tree.
- **Spec 02 / settings** unchanged — keybindings.json remains documented as separate from config.json.
- No other specs ripple from these edits.
