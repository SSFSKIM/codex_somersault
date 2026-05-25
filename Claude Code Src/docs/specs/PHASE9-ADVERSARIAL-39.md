# Phase 9.5b Adversarial Review — Spec 39 (Vim & Keybindings)

Reviewer role: Skeptic. Source-of-truth: `src/keybindings/*`, `src/vim/*`,
`src/hooks/useVimInput.ts`, `src/commands/vim/vim.ts`,
`src/components/PromptInput/utils.ts`. Read-only spot-checks (~13 reads).

## Severity counts

| Severity   | Count |
|------------|------:|
| CRITICAL   | 0 |
| HIGH       | 2 |
| MEDIUM     | 3 |
| LOW        | 3 |
| Nits       | 2 |

Net verdict: **APPROVE WITH FIXES**. The spec is meticulous and almost
entirely accurate. Two factual mistakes need correction; the rest are
minor wording/coverage issues.

---

## Top 5 findings

### 1. HIGH — `KEYBINDING_ACTIONS` count is wrong (78 → actually 86)

Spec §6.3 says: "`KEYBINDING_ACTIONS` (78 entries) is the verbatim
union…" and the reimplementation checklist (§11) says "Preserve the
78-action `KEYBINDING_ACTIONS` enum".

Counting non-context string-literal entries inside the
`KEYBINDING_ACTIONS = […] as const` block in
`src/keybindings/schema.ts:64-172` (`awk` over the literal range,
filtering rows that begin with a single-quoted lowercase identifier)
yields **86**. The 78 figure is repeated twice in the spec and would
mislead a reimplementer. **Fix:** change both 78 → 86 (or, safer,
"86 entries; see schema.ts:64–172 for the canonical list").

### 2. HIGH — `Scroll` context is in `DEFAULT_BINDINGS` but **not** in `KEYBINDING_CONTEXTS`

`defaultBindings.ts:196` defines a full `context: 'Scroll'` block
(`pageup`, `pagedown`, `wheelup`, `wheeldown`, `ctrl+home`, `ctrl+end`,
`ctrl+shift+c`, `cmd+c`). But `schema.ts:12-32` lists only 18 contexts —
`Scroll` is **absent**. Spec §6.3 cites the 18-context array verbatim
and §6.1 lists Scroll defaults verbatim, so the spec faithfully
mirrors the source — but it does **not flag** the inconsistency.

Consequence: a user attempting to override `wheeldown → scroll:lineDown`
in `~/.claude/keybindings.json` will fail Zod validation with
`Unknown context "Scroll"`, even though defaults rely on it. The
default block is parsed via `parseBindings(DEFAULT_BINDINGS)` which
does not run the Zod schema, so defaults work fine; but user
customization of scroll bindings is silently impossible.

The spec's reimplementation checklist must call this out: either add
`Scroll` to `KEYBINDING_CONTEXTS` or document that the default
`Scroll` block is non-overridable. Same applies to `MessageActions`
(only present in defaults when `feature('MESSAGE_ACTIONS')` is on,
not in `KEYBINDING_CONTEXTS`).

### 3. MEDIUM — ChordInterceptor's contexts include `'Global'` but the spec is ambiguous about ordering

Spec §5.5 says: `[...handlerContexts, ...activeContexts, 'Global']`.
Verified at `KeybindingProviderSetup.tsx:250`:
`const contexts = [...handlerContexts, ...activeContexts, "Global"]`.
However, ChordInterceptor passes the array to
`resolveKeyWithChordState`, which wraps it in `new Set(activeContexts)`
(`resolver.ts:193`) — so **order is dropped** at the resolver
boundary. Spec §5.6 separately claims `useKeybinding` "dedupes
preserving order — first occurrence wins precedence" via
`[...new Set(contextsToCheck)]`, which is correct for the
`useKeybinding(s)` path. But the chord interceptor's use of the
same array is fed into a Set and "precedence" is meaningless — last-
binding-wins (per `resolveKey`/`resolveKeyWithChordState`) is what
actually decides. The spec's §5.5 wording suggests context priority
matters in chord resolution; in code it does not. Tighten the prose.

### 4. MEDIUM — The "Wheel events bypass interception" claim is correct but understated

Spec §5.5 / §9 state wheel events bypass when no chord is pending.
Verified `KeybindingProviderSetup.tsx:238-240`. But this means
**wheel-up/down never trigger `scroll:lineUp/Down`** through the
chord interceptor — they pass through to ScrollKeybindingHandler's
own `useKeybindings` hook. The spec doesn't explicitly say this; a
reader could assume wheel events go through chord resolution. Add a
sentence: "wheel events are routed only via component-level
`useKeybinding(s)`, never the global ChordInterceptor."

### 5. MEDIUM — `useKeybinding` does not always `stopImmediatePropagation` on `match`

Spec §5.6 says: "on `match` invokes the handler iff `result.action ===
action`. Returning `false` lets the event propagate." Verified in
`useKeybinding.ts:64-72`:

```ts
case 'match':
  keybindingContext.setPendingChord(null)
  if (result.action === action) {
    if (handler() !== false) {
      event.stopImmediatePropagation()
    }
  }
  break
```

But: if `result.action !== action`, the hook **silently no-ops** and
does NOT `stopImmediatePropagation`. The spec is correct as written
but underspecifies the consequence: only the `useKeybinding`
instance whose `action` matches consumes the event. A different
instance running its own `useInput` later in the same render will
still see the keystroke, run `resolveKey` again, and may also fire.
That's intentional but worth a sentence in §5.6.

---

## Cross-spec impact

- **Spec 36 (voice mode):** spec 39 §9 correctly cross-refs the
  `voice:pushToTalk` bare-letter validation warning at
  `validate.ts:220-242`. Verified action exists in
  `KEYBINDING_ACTIONS:171`. No issue.
- **Spec 02 (settings):** spec 39 correctly puts `editorMode` in
  `utils/config.ts:177,231,593` and `EDITOR_MODES` in
  `configConstants.ts:15`. Verified via `vim.ts:6` reading
  `getGlobalConfig()` and `saveGlobalConfig()`. The
  `~/.claude/keybindings.json` file is **separate** from
  `config.json` and managed by its own loader/watcher — spec 39 is
  clear about this; no cross-spec contradiction. However, neither
  spec 02 nor spec 39 explicitly states whether `keybindings.json`
  participates in the settings migration system (`src/migrations/`).
  Likely out of scope but worth a one-liner in spec 02.
- **Spec 37 (Ink useInput):** ChordInterceptor uses raw
  `useInput` (with the eslint-disable comment for the
  `prefer-use-keybindings` lint rule, verified at line 15). Spec 39
  flags this correctly. The ESLint custom rule
  `custom-rules/prefer-use-keybindings` is mentioned but spec 37
  should own its definition. No contradiction here.
- **Spec 41 (session/persisted state):** spec 39 §12 #3 explicitly
  defers `isVimModeEnabled` reactivity to spec 41. Verified
  `PromptInput/utils.ts:12-15` reads `getGlobalConfig().editorMode`
  synchronously per render. Cross-spec deferral is correct.

---

## Hardest-to-verify claim

**`src/keybindings/types.ts` is missing from the leaked tree, but the
exported names listed in §2.3 (`Chord`, `ParsedKeystroke`,
`ParsedBinding`, `KeybindingBlock`, `KeybindingContextName`) are
inferable from usage.**

Confirmed missing via `ls`. The names are inferable only
indirectly from import sites (`parser.ts:1-6`, `resolver.ts:4-8`,
`match.ts:2`, `useKeybinding.ts:5`, `KeybindingProviderSetup.tsx:23`).
Field shapes for `ParsedKeystroke` are reconstructible from
`parser.ts:13-22` (literal initializer in `parseKeystroke`) and the
keys touched in `match.ts modifiersMatch` (`ctrl/shift/meta/alt/super`
+ `key`) — those are the only six fields. `ParsedBinding` is
reconstructible from `parser.ts:194-200`: `{chord, action, context}`
where `chord` is `ParsedKeystroke[]`. `KeybindingBlock` is
reconstructible from `loadUserBindings.ts:95-103`: `{context: string,
bindings: object}`. `Chord` is `ParsedKeystroke[]`.

`KeybindingContextName` — the spec implies it equals
`(typeof KEYBINDING_CONTEXTS)[number]` from `schema.ts`, but `Scroll`
and (when MESSAGE_ACTIONS is on) `MessageActions` are used as
`b.context` values that satisfy a `KeybindingContextName` parameter
in `resolver.ts`/`match.ts`. So either (a) the actual type is
broader than the schema enum (likely just `string`), or (b) defaults
do an unsafe cast. Cannot be verified without `types.ts`. **This is
the single largest gap in spec 39.**

---

## Other findings (LOW / Nits)

- **LOW:** §6.1 — the spec lists `Scroll | wheelup | scroll:lineUp`
  but `match.ts:41` returns `'wheelup'` from `getKeyName` only for
  `key.wheelUp`; `parser.ts:69` lowercases `'wheelup'` to
  `'wheelup'`, fine. Verified.
- **LOW:** §4.5 spec calls `chordTimeoutRef` a `setTimeout` handle.
  Verified type is `NodeJS.Timeout | null`
  (`KeybindingProviderSetup.tsx:143`). Match.
- **LOW:** §5.7 spec's `if key.ctrl: textInput.onInput(input,key);
  return` is verified at `useVimInput.ts:184-187` BEFORE the escape
  check. So `Ctrl+Esc` (rare) bypasses INSERT-exit logic. Spec
  doesn't mention this corner; very low priority.
- **Nit:** §6.3 the `KeybindingsSchema` is described as `z.object`,
  but the source is `lazySchema(() => z.object(...))`. Spec already
  shows `lazySchema` correctly in the verbatim block; the prose
  shorthand is fine.
- **Nit:** §5.8 spec calls `0` a "line-start motion not a count
  digit". Verified `transitions.ts:250-257` — exactly correct.
