# 39 — Vim Mode and Keybindings System Specification

Last updated: 2026-05-09 · Owner: sub-H9 · Adjacent: 36, 37, 41

## 1. Purpose & Scope

The keybindings system maps terminal key events (Ink `Key` + `input`) to
named actions (`app:*`, `chat:*`, `select:*`, …) per UI context, with
multi-key chord support, user JSON overrides, hot-reload, validation,
and platform-aware display formatting. Vim mode is an independent input
layer inside the prompt input only: a NORMAL/INSERT state machine that
parses vim commands (operators × motions × text objects, counts,
find/replace/indent, register, dot-repeat) and falls back to the
non-vim text input. Vim is gated by the user-set
`config.editorMode === 'vim'` runtime flag (`utils/config.ts:177,231`,
`PromptInput/utils.ts:12-15`), independent of `feature()` flags.

**In scope.** `src/keybindings/*` (parser, resolver, match, schema,
default bindings, validation, reserved shortcuts, template, provider
setup, hooks); `src/vim/*` (types, transitions, motions, operators, text
objects); the `/vim` command toggle; user override file
(`~/.claude/keybindings.json`) load/watch/precedence; the full default
bindings table verbatim including flag-gated entries.

**Out of scope.** Voice push-to-talk semantics → 36 (only the binding
entry is documented here). Ink shell mounting/wiring, `useInput`
plumbing, prompt-input rendering → 37. Persisted vim preference vs.
session state → 41.

## 2. Source Map

### 2.1 Inventory

| Path | LOC | Coverage | Role |
|---|--:|---|---|
| `src/keybindings/defaultBindings.ts` | 340 | full | `DEFAULT_BINDINGS` table |
| `src/keybindings/schema.ts` | 236 | full | Zod schema, contexts, action enum |
| `src/keybindings/parser.ts` | 203 | full | `parseKeystroke`/`parseChord`/display |
| `src/keybindings/match.ts` | 120 | full | Ink `Key` → keystroke equality |
| `src/keybindings/resolver.ts` | 244 | full | Single & chord resolution |
| `src/keybindings/validate.ts` | 498 | full | Warnings, dup-key scan |
| `src/keybindings/reservedShortcuts.ts` | 127 | full | Non-rebindable + reserved |
| `src/keybindings/loadUserBindings.ts` | 472 | full | File load, watcher, cache |
| `src/keybindings/template.ts` | 52 | full | `keybindings.json` template |
| `src/keybindings/shortcutFormat.ts` | 63 | full | Non-React display lookup |
| `src/keybindings/useKeybinding.ts` | 196 | full | `useKeybinding(s)` hooks |
| `src/keybindings/useShortcutDisplay.ts` | 59 | full | React display hook |
| `src/keybindings/KeybindingContext.tsx` | 242 | full | Provider + context |
| `src/keybindings/KeybindingProviderSetup.tsx` | 307 | full | Setup + ChordInterceptor |
| `src/keybindings/types.ts` | — | **MISSING** (registry refs only) | `ParsedKeystroke`, `ParsedBinding`, `KeybindingBlock`, `Chord`, `KeybindingContextName` are imported from `./types.js` everywhere but the file is absent from the leaked tree. |
| `src/vim/types.ts` | 199 | full | `VimState`, `CommandState`, key groups |
| `src/vim/transitions.ts` | 490 | full | NORMAL-mode state machine |
| `src/vim/operators.ts` | 556 | full | d/c/y/x/r/~/J/p/>/< execution |
| `src/vim/motions.ts` | 82 | full | hjkl/wbe/WBE/0/^/$/G/gj/gk |
| `src/vim/textObjects.ts` | 186 | full | `iw`/`aw`, quotes, brackets |
| `src/hooks/useVimInput.ts` | 317 | full | INSERT/NORMAL hook + dot-repeat |
| `src/commands/vim/index.ts` | 12 | full | `/vim` command stub |
| `src/commands/vim/vim.ts` | 38 | full | `editorMode` toggle |
| `src/components/PromptInput/utils.ts:12-15` | — | grep | `isVimModeEnabled()` runtime gate |
| `src/components/PromptInput/PromptInput.tsx:2243` | — | grep | `<VimTextInput>` mount site |
| `src/utils/config.ts:177,231,593,636` | — | grep | `EditorMode`, default, settable |
| `src/utils/configConstants.ts:15` | — | grep | `EDITOR_MODES = ['normal','vim']` |
| `src/types/textInputTypes.ts:222` | — | grep | `type VimMode = 'INSERT'\|'NORMAL'` |

### 2.2 Imports / imported by

**Imports.** `chokidar`, `zod/v4` (schema), `react` (provider/hooks),
`Ink` (`useInput`, `Key`), services: `analytics/index`,
`analytics/growthbook`, `utils/cleanupRegistry`, `utils/debug`,
`utils/envUtils`, `utils/errors`, `utils/signal`,
`utils/slowOperations`, `utils/lazySchema`, `utils/platform`,
`utils/bundledMode`, `utils/semver`. Vim imports `utils/Cursor`,
`utils/intl`, `utils/stringUtils`.

**Imported by.** `KeybindingSetup` is mounted somewhere in the app
state tree (per `KeybindingProviderSetup.tsx:38-46` JSDoc;
mount call site is in 37). `useKeybinding(s)` are consumed throughout
`components/`, `screens/`, `hooks/`. `getShortcutDisplay` is used by
non-React callers (`query/stopHooks.ts` per `shortcutFormat.ts:23-29`).
`useVimInput` is consumed by `VimTextInput` (gated by
`isVimModeEnabled()` at `PromptInput.tsx:2243`).

### 2.3 Missing source

`src/keybindings/types.ts` is referenced from every keybindings module
(`./types.js` imports throughout) and re-exported as
`KeybindingContextName`, `ParsedKeystroke`, `ParsedBinding`,
`KeybindingBlock`, `Chord`. The file is **not present** in the leaked
tree — recorded in §12.

## 3. Public Interface (Contract)

### 3.1 Keybindings module

```ts
// loadUserBindings.ts
export function loadKeybindings(): Promise<KeybindingsLoadResult>
export function loadKeybindingsSync(): ParsedBinding[]
export function loadKeybindingsSyncWithWarnings(): KeybindingsLoadResult
export function initializeKeybindingWatcher(): Promise<void>
export function disposeKeybindingWatcher(): void
export const subscribeToKeybindingChanges // = signal.subscribe
export function getKeybindingsPath(): string
export function isKeybindingCustomizationEnabled(): boolean
export function getCachedKeybindingWarnings(): KeybindingWarning[]
export function resetKeybindingLoaderForTesting(): void
export type KeybindingsLoadResult = {
  bindings: ParsedBinding[]; warnings: KeybindingWarning[]
}

// resolver.ts
export type ResolveResult =
  | { type: 'match'; action: string }
  | { type: 'none' }
  | { type: 'unbound' }
export type ChordResolveResult =
  | { type: 'match'; action: string }
  | { type: 'none' }
  | { type: 'unbound' }
  | { type: 'chord_started'; pending: ParsedKeystroke[] }
  | { type: 'chord_cancelled' }
export function resolveKey(input, key, activeContexts, bindings): ResolveResult
export function resolveKeyWithChordState(input, key, activeContexts, bindings, pending): ChordResolveResult
export function getBindingDisplayText(action, context, bindings): string|undefined
export function keystrokesEqual(a, b): boolean

// React surface
export function useKeybinding(action, handler, options?): void
export function useKeybindings(handlers, options?): void
export function useShortcutDisplay(action, context, fallback): string
export function useRegisterKeybindingContext(context, isActive=true): void
export function useKeybindingContext(): KeybindingContextValue
export function useOptionalKeybindingContext(): KeybindingContextValue|null
export function KeybindingProvider(props): ReactNode
export function KeybindingSetup({ children }): ReactNode

// Non-React
export function getShortcutDisplay(action, context, fallback): string

// Schema (verbatim)
export const KEYBINDING_CONTEXTS: readonly[...]
export const KEYBINDING_CONTEXT_DESCRIPTIONS: Record<...>
export const KEYBINDING_ACTIONS: readonly[...]
export const KeybindingBlockSchema, KeybindingsSchema  // lazy z.object
export type KeybindingsSchemaType = z.infer<...>

// Validation
export type KeybindingWarning = {
  type: KeybindingWarningType; severity: 'error'|'warning';
  message: string; key?: string; context?: string;
  action?: string; suggestion?: string }
export function validateBindings(userBlocks, parsedBindings): KeybindingWarning[]
export function checkDuplicateKeysInJson(jsonString): KeybindingWarning[]
export function formatWarning(w): string
export function formatWarnings(ws): string

// Reserved
export const NON_REBINDABLE: ReservedShortcut[]
export const TERMINAL_RESERVED: ReservedShortcut[]
export const MACOS_RESERVED: ReservedShortcut[]
export function getReservedShortcuts(): ReservedShortcut[]
export function normalizeKeyForComparison(key): string

// Template
export function generateKeybindingsTemplate(): string
```

### 3.2 Vim module

```ts
// vim/types.ts
export type Operator = 'delete'|'change'|'yank'
export type FindType = 'f'|'F'|'t'|'T'
export type TextObjScope = 'inner'|'around'
export type VimState =
  | { mode:'INSERT'; insertedText:string }
  | { mode:'NORMAL'; command:CommandState }
export type CommandState = … (see §4)
export type PersistentState = {
  lastChange: RecordedChange|null
  lastFind: { type:FindType; char:string }|null
  register: string
  registerIsLinewise: boolean }
export type RecordedChange = … (see §4)
export const OPERATORS = { d:'delete', c:'change', y:'yank' }
export const SIMPLE_MOTIONS = Set([h,l,j,k,w,b,e,W,B,E,0,^,$])
export const FIND_KEYS = Set([f,F,t,T])
export const TEXT_OBJ_SCOPES = { i:'inner', a:'around' }
export const TEXT_OBJ_TYPES = Set([w,W,",',`,(,),b,[,],{,},B,<,>])
export const MAX_VIM_COUNT = 10000
export function createInitialVimState(): VimState
export function createInitialPersistentState(): PersistentState

// transitions.ts
export type TransitionContext = OperatorContext & {
  onUndo?: () => void; onDotRepeat?: () => void }
export type TransitionResult = { next?: CommandState; execute?: () => void }
export function transition(state, input, ctx): TransitionResult

// operators.ts (all return void, mutate via ctx.set*)
executeOperatorMotion / executeOperatorFind / executeOperatorTextObj
executeLineOp / executeX / executeReplace / executeToggleCase
executeJoin / executePaste / executeIndent / executeOpenLine
executeOperatorG / executeOperatorGg

// motions.ts
export function resolveMotion(key, cursor, count): Cursor
export function isInclusiveMotion(key): boolean   // 'eE$'
export function isLinewiseMotion(key): boolean    // 'jkG' || 'gg'

// textObjects.ts
export function findTextObject(text, offset, objectType, isInner): {start,end}|null

// hooks/useVimInput.ts
export function useVimInput(props: UseVimInputProps): VimInputState
// VimInputState extends TextInputState with
//   mode: VimMode; setMode: (mode)=>void
```

### 3.3 `VimMode` (`types/textInputTypes.ts:222`)

```ts
export type VimMode = 'INSERT' | 'NORMAL'
```

## 4. Data Model & State

### 4.1 Vim state machine (verbatim ASCII from `vim/types.ts:7-26`)

```
                             VimState
  ┌──────────────────────────────┬──────────────────────────────────────┐
  │  INSERT                      │  NORMAL                              │
  │  (tracks insertedText)       │  (CommandState machine)              │
  │                              │                                      │
  │                              │  idle ──┬─[d/c/y]──► operator        │
  │                              │         ├─[1-9]────► count           │
  │                              │         ├─[fFtT]───► find            │
  │                              │         ├─[g]──────► g               │
  │                              │         ├─[r]──────► replace         │
  │                              │         └─[><]─────► indent          │
  │                              │                                      │
  │                              │  operator ─┬─[motion]──► execute     │
  │                              │            ├─[0-9]────► operatorCount│
  │                              │            ├─[ia]─────► operatorTextObj
  │                              │            └─[fFtT]───► operatorFind │
  └──────────────────────────────┴──────────────────────────────────────┘
```

### 4.2 `CommandState` (verbatim, `vim/types.ts:59-75`)

```ts
export type CommandState =
  | { type: 'idle' }
  | { type: 'count'; digits: string }
  | { type: 'operator'; op: Operator; count: number }
  | { type: 'operatorCount'; op: Operator; count: number; digits: string }
  | { type: 'operatorFind'; op: Operator; count: number; find: FindType }
  | { type: 'operatorTextObj'; op: Operator; count: number;
      scope: TextObjScope }
  | { type: 'find'; find: FindType; count: number }
  | { type: 'g'; count: number }
  | { type: 'operatorG'; op: Operator; count: number }
  | { type: 'replace'; count: number }
  | { type: 'indent'; dir: '>' | '<'; count: number }
```

### 4.3 `RecordedChange` (`vim/types.ts:92-119`)

Discriminated union: `insert | operator | operatorTextObj | operatorFind
| replace | x | toggleCase | indent | openLine | join`. Captures
exactly what `executeOperator*` / `executeX` / etc. record so dot
(`.`) can replay. Insert tracks plain text; line-ops record motion as
`'d'|'c'|'y'`.

### 4.4 Persistent state

`{ lastChange, lastFind: {type, char}, register: string,
registerIsLinewise: boolean }` — survives across commands inside one
mounted `useVimInput`. Linewise is detected by `register.endsWith('\n')`
(`operators.ts:302`), so `setRegister(content, true)` always appends
`\n` if missing (`operators.ts:122-125, 502-504`).

### 4.5 Keybindings runtime state (`KeybindingProviderSetup.tsx:1-90`)

- `bindings` (`ParsedBinding[]`) — `useState`, replaced on hot-reload.
- `warnings` (`KeybindingWarning[]`) — surfaced via `useNotifications`.
- `pendingChordRef` + `pendingChord` state — both kept in sync; ref is
  for synchronous reads from `resolve()`, state for re-render.
- `chordTimeoutRef` — `setTimeout` handle for chord cancel.
- `handlerRegistryRef`: `Map<action, Set<{action,context,handler}>>`.
- `activeContextsRef`: `Set<KeybindingContextName>`; updated
  synchronously on context mount/unmount (`KeybindingContext.tsx:215-242`).

Module-level cache in `loadUserBindings.ts`: `cachedBindings`,
`cachedWarnings`, `watcher`, `initialized`, `disposed`,
`lastCustomBindingsLogDate` (rate-limit telemetry to once/day).

## 5. Algorithm / Control Flow

### 5.1 Loading and merge precedence

```
loadKeybindings():
  default = parseBindings(DEFAULT_BINDINGS)               # Phase A
  if not isKeybindingCustomizationEnabled():              # GrowthBook
      return { default, [] }
  read ~/.claude/keybindings.json
  if ENOENT: return { default, [] }
  parsed = jsonParse(content)
  if not object or !'bindings' in parsed:
      return { default, [parse_error] }
  userBlocks = parsed.bindings
  if !isKeybindingBlockArray(userBlocks):
      return { default, [parse_error] }
  user = parseBindings(userBlocks)
  merged = [...default, ...user]                          # last wins
  warnings = checkDuplicateKeysInJson(content)            # raw scan
            ++ validateBindings(userBlocks, merged)
  fire tengu_custom_keybindings_loaded once/day
  return { merged, warnings }
```

`loadKeybindingsSyncWithWarnings()` mirrors the async path with
`readFileSync`. Sync path is called from React `useState` initializer
(`KeybindingProviderSetup.tsx:5-11`).

### 5.2 Watcher

`initializeKeybindingWatcher()` (idempotent via `initialized` flag,
`loadUserBindings.ts:353-403`):

- skips entirely when `!isKeybindingCustomizationEnabled()`.
- resolves `dirname(getKeybindingsPath())`; bails if missing.
- `chokidar.watch(userPath, { persistent:true, ignoreInitial:true,
   awaitWriteFinish:{ stabilityThreshold:500, pollInterval:200 },
   ignorePermissionErrors:true, usePolling:false, atomic:true })`.
- `add`/`change` → `handleChange` reloads via `loadKeybindings()`,
  emits to `keybindingsChanged` signal.
- `unlink` → reset to defaults, emit.
- `registerCleanup(disposeKeybindingWatcher)`.

### 5.3 Single-key resolution (`resolver.ts:32-61`)

```
resolveKey(input, key, activeContexts, bindings):
  ctxSet = Set(activeContexts)
  match = undefined
  for binding in bindings:                # order = default ++ user
    if binding.chord.length != 1: skip
    if binding.context not in ctxSet: skip
    if matchesBinding(input, key, binding): match = binding   # last wins
  if !match: return {none}
  if match.action === null: return {unbound}
  return {match, action}
```

### 5.4 Chord resolution (`resolver.ts:166-244`)

```
resolveKeyWithChordState(input, key, activeContexts, bindings, pending):
  if key.escape and pending != null: return chord_cancelled
  current = buildKeystroke(input, key)
  if !current:
      if pending != null: return chord_cancelled
      return none
  testChord = pending ? [...pending, current] : [current]
  contextBindings = bindings.filter(b => activeContexts ⊇ b.context)
  # group prefix candidates by stringified chord; later null overrides default
  chordWinners: Map<string, action|null>
  for b in contextBindings:
      if b.chord.length > testChord.length and chordPrefixMatches(testChord, b):
          chordWinners.set(chordToString(b.chord), b.action)
  hasLongerChords = any(action != null for action in chordWinners)
  if hasLongerChords: return chord_started(pending=testChord)
  exactMatch = lastBinding satisfying chordExactlyMatches(testChord, b)
  if exactMatch:
      if action === null: return unbound
      return match(exactMatch.action)
  return pending != null ? chord_cancelled : none
```

Note: prefix preference is unconditional — even if a single-key exact
match exists, an active longer chord on the same prefix wins
(`resolver.ts:217-221`). Null overrides on the prefix shadow the
default they unbind so `ctrl+x` can fall through to a single-key
binding when its chord is unbound (`resolver.ts:198-208`).

### 5.5 Chord lifetime (`KeybindingProviderSetup.tsx`)

- `setPendingChord(p)` clears any prior `chordTimeoutRef`; if `p !==
  null`, starts a fresh `setTimeout(…, CHORD_TIMEOUT_MS=1000)` that
  nulls both ref and state on fire (line 30, 53-68).
- `ChordInterceptor` registers `useInput` BEFORE children
  (line 92-187): it builds contexts as
  `[...handlerContexts, ...activeContexts, 'Global']`, calls
  `resolveKeyWithChordState`, and on any chord/unbound/match-when-in-
  chord result it calls `event.stopImmediatePropagation()` so prompt
  input never sees the second key. On `match` while previously in a
  chord, it walks the registry directly to invoke a handler whose
  context is in the active set; otherwise the regular `useKeybinding`
  hook handles the match.
- Wheel events bypass interception when no chord is pending (line
  119-121, verified 238-240): `if ((key.wheelUp || key.wheelDown) &&
  pending === null) return`. Consequence: wheel-up/down events are
  routed only via component-level `useKeybinding(s)` (e.g. the Scroll
  context handler) and never through the global ChordInterceptor or
  chord resolution.
- **Context-order illusion.** The `[...handlerContexts, ...activeContexts,
  'Global']` array is passed to `resolveKeyWithChordState`, which wraps
  it in `new Set(activeContexts)` (`resolver.ts:193`) — order is
  dropped at the resolver boundary and **last-binding-wins** decides.
  Order only matters in the `useKeybinding(s)` dedup path
  (`[...new Set(contextsToCheck)]`, §5.6). For ChordInterceptor /
  `resolveKey` / `resolveKeyWithChordState`, "context priority" is an
  illusion — there is no priority, only set membership.

### 5.6 Active contexts and priority

`useKeybinding(action, handler, {context='Global', isActive=true})`:

1. registers `{action, context, handler}` into `handlerRegistryRef`.
2. attaches its own `useInput`.
3. on input, builds `contextsToCheck = [...activeContexts, context,
   'Global']` then dedupes preserving order — first occurrence wins
   precedence.
4. `resolve()` returns `match | chord_started | chord_cancelled |
   unbound | none`. The hook calls `setPendingChord` on chord
   transitions; on `match` invokes the handler iff `result.action ===
   action`. Returning `false` lets the event propagate
   (`useKeybinding.ts:69-73`); `Promise<void>` is fire-and-forget.
   **Non-consume case.** When `result.type === 'match'` but
   `result.action !== action`, the hook silently no-ops and does NOT
   call `event.stopImmediatePropagation()`. Only the `useKeybinding`
   instance whose `action` parameter matches the resolved action
   consumes the event; any other instance running its own `useInput`
   later in the render order will still see the keystroke, re-run
   `resolveKey`, and may also fire if it handles the same action.

`useRegisterKeybindingContext(name, isActive?)` adds/removes the
context from `activeContextsRef` via `useLayoutEffect`. Registered
contexts take precedence over `'Global'` in resolution
(`KeybindingContext.tsx:215-242`).

### 5.7 Vim per-keystroke pipeline (`hooks/useVimInput.ts:175-295`)

```
handleVimInput(rawInput, key):
  state = vimStateRef.current
  filtered = inputFilter ? inputFilter(rawInput,key) : rawInput
  input = state.mode==='INSERT' ? filtered : rawInput
  if key.ctrl: textInput.onInput(input,key); return
  if key.escape and INSERT: switchToNormalMode(); return
  if key.escape and NORMAL: command := idle; return
  if key.return: textInput.onInput(input,key); return        # always
  if INSERT:
      track insertedText (subtract last grapheme on backspace/delete)
      textInput.onInput(input,key); return
  if !NORMAL: return
  if command.type==='idle' and arrow key:
      textInput.onInput(input,key); return                   # cursor / history
  expectsMotion = command.type in {idle,count,operator,operatorCount}
  vimInput = input
       leftArrow→'h', rightArrow→'l', upArrow→'k', downArrow→'j'
       expectsMotion && backspace → 'h'
       expectsMotion && type!='count' && delete → 'x'
  result = transition(command, vimInput, ctx)
  if result.execute: result.execute()
  if still NORMAL: command := result.next ?? (result.execute ? idle : command)
  if input==='?' and was idle: props.onChange('?')           # search trigger
```

`switchToNormalMode` (line 61-80): if INSERT had `insertedText`,
record it as `{type:'insert', text}`. Then move cursor left by 1
when `offset>0 && value[offset-1] !== '\n'` (vim semantics), and reset
`vimStateRef.current = NORMAL/idle`.

### 5.8 `transition()` dispatch table (`vim/transitions.ts:64-87`)

| state.type | function |
|---|---|
| `idle` | `fromIdle` |
| `count` | `fromCount` |
| `operator` | `fromOperator` |
| `operatorCount` | `fromOperatorCount` |
| `operatorFind` | `fromOperatorFind` |
| `operatorTextObj` | `fromOperatorTextObj` |
| `find` | `fromFind` |
| `g` | `fromG` |
| `operatorG` | `fromOperatorG` |
| `replace` | `fromReplace` |
| `indent` | `fromIndent` |

Shared `handleNormalInput(input, count, ctx)` covers `d/c/y → operator`,
SIMPLE_MOTIONS → execute, FIND_KEYS → `find` state, `g→g` state, `r→
replace`, `> | < → indent`, and execute-immediate keys: `~ x J p P D C
Y G . ; , u i I a A o O` (transitions.ts:98-200).

`handleOperatorInput(op,count,input,ctx)`: `i|a → operatorTextObj`,
FIND_KEYS → `operatorFind`, SIMPLE_MOTIONS → `executeOperatorMotion`,
`G → executeOperatorG`, `g → operatorG` state (lines 206-242).

Counts: `fromIdle` rejects `0` as a count (it's the line-start motion,
line 250-257). `fromCount` accumulates digits, clamped to
`MAX_VIM_COUNT=10000` (line 270-274). `fromOperatorCount` multiplies
outer × motion count for vim's `2d3w` semantics (line 326-328).

`fromG`: `gj/gk` are visual-line motions; `gg` (with optional count) is
go-to-line — count=1 means "no count" → first line; >1 → line N
(transitions.ts:386-417). `fromOperatorG` mirrors for `dgg`/`dG` (line
420-435; cancels on any other key).

`fromReplace`: empty-string input cancels (literal `r<BS>`); else
`executeReplace(input, count, ctx)` (line 438-447).

`fromIndent`: only `state.dir` confirms (`>>` or `<<`); else cancel
(line 450-458).

`executeRepeatFind` (`;`/`,`): looks up `lastFind`; on `,` flips
`fFtT` direction via the verbatim map; calls
`cursor.findCharacter(char, type, count)` (line 465-490).

### 5.9 Operator range computation

`getOperatorRange` (`operators.ts:429-475`): start = min(cursor,target),
end = max(cursor,target).

- `cw`/`cW`: range = end-of-vim-word (count-1 forward then end), so
  trailing whitespace is preserved.
- `isLinewiseMotion(motion)` → linewise; extend `to` to include the
  next newline (or EOF, also subsuming the preceding newline).
- `isInclusiveMotion(motion) && cursor<=target` → `to =
  measuredText.nextOffset(to)`.
- Always snap range out of `[Image #N]` chips at both ends.

`getOperatorRangeForFind` (line 482-491): always inclusive (find
adjusts for `t/T` already).

`applyOperator` (line 493-522): `setRegister(content, linewise)` then
yank moves cursor to `from`; delete splices and clamps cursor to
`min(from, lastValidOffset)`; change splices and `enterInsert(from)`.

Linewise content is normalized to end with `\n` for paste detection
(line 502-504).

### 5.10 Line operations (`executeLineOp`, `operators.ts:102-166`)

`dd`/`cc`/`yy`: compute current logical line by counting `\n` before
cursor offset (cursor.getPosition uses wrapped lines — wrong for
this). `linesToAffect = min(count, lines.length-currentLine)`.
`yank` rewinds offset to `lineStart`; `delete` includes preceding
newline when deleting through EOF; `change` blanks line(s) and
`enterInsert(lineStart)`. Records `{type:'operator', op, motion: op[0],
count}`.

### 5.11 Paste (`executePaste`, `operators.ts:294-343`)

If register is empty: noop. Detect linewise via trailing `\n`. Linewise:
insert `count` repetitions of register lines at `currentLine` (P) or
`currentLine+1` (p); set offset to start of inserted block.
Charwise: insert at `cursor.offset` (P) or `nextOffset(cursor.offset)`
(p); set offset to end of inserted text minus the last grapheme.

### 5.12 Indent (`executeIndent`, `operators.ts:348-392`)

Two-space indent. `>` prepends; `<` strips: try the two-space prefix
first; else single tab; else strip up to 2 leading whitespace chars.
Cursor lands at first non-blank of current line.

### 5.13 Text objects (`textObjects.ts`)

Word: grapheme-aware. If at word char → expand both sides over word
chars; if at whitespace (always returns the whitespace run); if at
punctuation → over punctuation. `around` extends through trailing or
leading whitespace.

Quote: pair quotes 0-1, 2-3, … on the current line; cursor must lie
between a pair. Inner: between quotes; around: includes both.

Bracket: walk back tracking depth to find matching open; walk forward
to matching close. Inner: strict interior; around: inclusive of both.

`PAIRS`: `( ) b → ()`; `[ ] → []`; `{ } B → {}`; `< > → <>`; `" → ""`;
`' → ''`; `` ` `` → `` `` ``.

## 6. Verbatim Assets

### 6.1 Default keybindings table (`defaultBindings.ts:32-340`, every entry)

Computed prologue: `IMAGE_PASTE_KEY = platform==='windows' ? 'alt+v' :
'ctrl+v'`. `SUPPORTS_TERMINAL_VT_MODE = platform!=='windows' ||
(bun ? semver≥1.2.23 : semver(node)≥22.17.0 <23.0.0 || ≥24.2.0)`.
`MODE_CYCLE_KEY = SUPPORTS_TERMINAL_VT_MODE ? 'shift+tab' : 'meta+m'`.

| Context | Key | Action | Gate |
|---|---|---|---|
| Global | `ctrl+c` | `app:interrupt` | (special, see §6.2) |
| Global | `ctrl+d` | `app:exit` | (special, see §6.2) |
| Global | `ctrl+l` | `app:redraw` | — |
| Global | `ctrl+t` | `app:toggleTodos` | — |
| Global | `ctrl+o` | `app:toggleTranscript` | — |
| Global | `ctrl+shift+b` | `app:toggleBrief` | `feature('KAIROS') \|\| feature('KAIROS_BRIEF')` |
| Global | `ctrl+shift+o` | `app:toggleTeammatePreview` | — |
| Global | `ctrl+r` | `history:search` | — |
| Global | `ctrl+shift+f` | `app:globalSearch` | `feature('QUICK_SEARCH')` |
| Global | `cmd+shift+f` | `app:globalSearch` | `feature('QUICK_SEARCH')` |
| Global | `ctrl+shift+p` | `app:quickOpen` | `feature('QUICK_SEARCH')` |
| Global | `cmd+shift+p` | `app:quickOpen` | `feature('QUICK_SEARCH')` |
| Global | `meta+j` | `app:toggleTerminal` | `feature('TERMINAL_PANEL')` |
| Chat | `escape` | `chat:cancel` | — |
| Chat | `ctrl+x ctrl+k` | `chat:killAgents` | — |
| Chat | `MODE_CYCLE_KEY` (`shift+tab` or `meta+m`) | `chat:cycleMode` | platform-derived |
| Chat | `meta+p` | `chat:modelPicker` | — |
| Chat | `meta+o` | `chat:fastMode` | — |
| Chat | `meta+t` | `chat:thinkingToggle` | — |
| Chat | `enter` | `chat:submit` | — |
| Chat | `up` | `history:previous` | — |
| Chat | `down` | `history:next` | — |
| Chat | `ctrl+_` | `chat:undo` | — |
| Chat | `ctrl+shift+-` | `chat:undo` | — |
| Chat | `ctrl+x ctrl+e` | `chat:externalEditor` | — |
| Chat | `ctrl+g` | `chat:externalEditor` | — |
| Chat | `ctrl+s` | `chat:stash` | — |
| Chat | `IMAGE_PASTE_KEY` (`ctrl+v` or `alt+v`) | `chat:imagePaste` | platform-derived |
| Chat | `shift+up` | `chat:messageActions` | `feature('MESSAGE_ACTIONS')` |
| Chat | `space` | `voice:pushToTalk` | `feature('VOICE_MODE')` |
| Autocomplete | `tab` | `autocomplete:accept` | — |
| Autocomplete | `escape` | `autocomplete:dismiss` | — |
| Autocomplete | `up` | `autocomplete:previous` | — |
| Autocomplete | `down` | `autocomplete:next` | — |
| Settings | `escape` | `confirm:no` | — |
| Settings | `up` | `select:previous` | — |
| Settings | `down` | `select:next` | — |
| Settings | `k` | `select:previous` | — |
| Settings | `j` | `select:next` | — |
| Settings | `ctrl+p` | `select:previous` | — |
| Settings | `ctrl+n` | `select:next` | — |
| Settings | `space` | `select:accept` | — |
| Settings | `enter` | `settings:close` | — |
| Settings | `/` | `settings:search` | — |
| Settings | `r` | `settings:retry` | — |
| Confirmation | `y` | `confirm:yes` | — |
| Confirmation | `n` | `confirm:no` | — |
| Confirmation | `enter` | `confirm:yes` | — |
| Confirmation | `escape` | `confirm:no` | — |
| Confirmation | `up` | `confirm:previous` | — |
| Confirmation | `down` | `confirm:next` | — |
| Confirmation | `tab` | `confirm:nextField` | — |
| Confirmation | `space` | `confirm:toggle` | — |
| Confirmation | `shift+tab` | `confirm:cycleMode` | — |
| Confirmation | `ctrl+e` | `confirm:toggleExplanation` | — |
| Confirmation | `ctrl+d` | `permission:toggleDebug` | — |
| Tabs | `tab` | `tabs:next` | — |
| Tabs | `shift+tab` | `tabs:previous` | — |
| Tabs | `right` | `tabs:next` | — |
| Tabs | `left` | `tabs:previous` | — |
| Transcript | `ctrl+e` | `transcript:toggleShowAll` | — |
| Transcript | `ctrl+c` | `transcript:exit` | — |
| Transcript | `escape` | `transcript:exit` | — |
| Transcript | `q` | `transcript:exit` | — |
| HistorySearch | `ctrl+r` | `historySearch:next` | — |
| HistorySearch | `escape` | `historySearch:accept` | — |
| HistorySearch | `tab` | `historySearch:accept` | — |
| HistorySearch | `ctrl+c` | `historySearch:cancel` | — |
| HistorySearch | `enter` | `historySearch:execute` | — |
| Task | `ctrl+b` | `task:background` | — |
| ThemePicker | `ctrl+t` | `theme:toggleSyntaxHighlighting` | — |
| Scroll | `pageup` | `scroll:pageUp` | — |
| Scroll | `pagedown` | `scroll:pageDown` | — |
| Scroll | `wheelup` | `scroll:lineUp` | — |
| Scroll | `wheeldown` | `scroll:lineDown` | — |
| Scroll | `ctrl+home` | `scroll:top` | — |
| Scroll | `ctrl+end` | `scroll:bottom` | — |
| Scroll | `ctrl+shift+c` | `selection:copy` | — |
| Scroll | `cmd+c` | `selection:copy` | — |
| Help | `escape` | `help:dismiss` | — |
| Attachments | `right` | `attachments:next` | — |
| Attachments | `left` | `attachments:previous` | — |
| Attachments | `backspace` | `attachments:remove` | — |
| Attachments | `delete` | `attachments:remove` | — |
| Attachments | `down` | `attachments:exit` | — |
| Attachments | `escape` | `attachments:exit` | — |
| Footer | `up` | `footer:up` | — |
| Footer | `ctrl+p` | `footer:up` | — |
| Footer | `down` | `footer:down` | — |
| Footer | `ctrl+n` | `footer:down` | — |
| Footer | `right` | `footer:next` | — |
| Footer | `left` | `footer:previous` | — |
| Footer | `enter` | `footer:openSelected` | — |
| Footer | `escape` | `footer:clearSelection` | — |
| MessageSelector | `up` | `messageSelector:up` | — |
| MessageSelector | `down` | `messageSelector:down` | — |
| MessageSelector | `k` | `messageSelector:up` | — |
| MessageSelector | `j` | `messageSelector:down` | — |
| MessageSelector | `ctrl+p` | `messageSelector:up` | — |
| MessageSelector | `ctrl+n` | `messageSelector:down` | — |
| MessageSelector | `ctrl+up` | `messageSelector:top` | — |
| MessageSelector | `shift+up` | `messageSelector:top` | — |
| MessageSelector | `meta+up` | `messageSelector:top` | — |
| MessageSelector | `shift+k` | `messageSelector:top` | — |
| MessageSelector | `ctrl+down` | `messageSelector:bottom` | — |
| MessageSelector | `shift+down` | `messageSelector:bottom` | — |
| MessageSelector | `meta+down` | `messageSelector:bottom` | — |
| MessageSelector | `shift+j` | `messageSelector:bottom` | — |
| MessageSelector | `enter` | `messageSelector:select` | — |
| MessageActions | `up` | `messageActions:prev` | `feature('MESSAGE_ACTIONS')` |
| MessageActions | `down` | `messageActions:next` | `feature('MESSAGE_ACTIONS')` |
| MessageActions | `k` | `messageActions:prev` | `feature('MESSAGE_ACTIONS')` |
| MessageActions | `j` | `messageActions:next` | `feature('MESSAGE_ACTIONS')` |
| MessageActions | `meta+up` | `messageActions:top` | `feature('MESSAGE_ACTIONS')` |
| MessageActions | `meta+down` | `messageActions:bottom` | `feature('MESSAGE_ACTIONS')` |
| MessageActions | `super+up` | `messageActions:top` | `feature('MESSAGE_ACTIONS')` |
| MessageActions | `super+down` | `messageActions:bottom` | `feature('MESSAGE_ACTIONS')` |
| MessageActions | `shift+up` | `messageActions:prevUser` | `feature('MESSAGE_ACTIONS')` |
| MessageActions | `shift+down` | `messageActions:nextUser` | `feature('MESSAGE_ACTIONS')` |
| MessageActions | `escape` | `messageActions:escape` | `feature('MESSAGE_ACTIONS')` |
| MessageActions | `ctrl+c` | `messageActions:ctrlc` | `feature('MESSAGE_ACTIONS')` |
| MessageActions | `enter` | `messageActions:enter` | `feature('MESSAGE_ACTIONS')` |
| MessageActions | `c` | `messageActions:c` | `feature('MESSAGE_ACTIONS')` |
| MessageActions | `p` | `messageActions:p` | `feature('MESSAGE_ACTIONS')` |
| DiffDialog | `escape` | `diff:dismiss` | — |
| DiffDialog | `left` | `diff:previousSource` | — |
| DiffDialog | `right` | `diff:nextSource` | — |
| DiffDialog | `up` | `diff:previousFile` | — |
| DiffDialog | `down` | `diff:nextFile` | — |
| DiffDialog | `enter` | `diff:viewDetails` | — |
| ModelPicker | `left` | `modelPicker:decreaseEffort` | (ant-only context) |
| ModelPicker | `right` | `modelPicker:increaseEffort` | (ant-only context) |
| Select | `up` | `select:previous` | — |
| Select | `down` | `select:next` | — |
| Select | `j` | `select:next` | — |
| Select | `k` | `select:previous` | — |
| Select | `ctrl+n` | `select:next` | — |
| Select | `ctrl+p` | `select:previous` | — |
| Select | `enter` | `select:accept` | — |
| Select | `escape` | `select:cancel` | — |
| Plugin | `space` | `plugin:toggle` | — |
| Plugin | `i` | `plugin:install` | — |

### 6.2 Reserved / non-rebindable (`reservedShortcuts.ts:16-67`)

```
NON_REBINDABLE   = [{ctrl+c, error}, {ctrl+d, error}, {ctrl+m, error}]
TERMINAL_RESERVED= [{ctrl+z, warning, SIGTSTP}, {ctrl+\, error, SIGQUIT}]
MACOS_RESERVED   = [cmd+c, cmd+v, cmd+x, cmd+q, cmd+w, cmd+tab, cmd+space]
```

`ctrl+c`/`ctrl+d` ARE in `DEFAULT_BINDINGS` (so the resolver finds
them) but `validate.ts` reports an error if a user tries to override
them — see `defaultBindings.ts:36-41` comment and `validate.ts` flow
through `checkReservedShortcuts`.

### 6.3 User config Zod schema (verbatim, `schema.ts`)

```ts
export const KEYBINDING_CONTEXTS = [
  'Global','Chat','Autocomplete','Confirmation','Help','Transcript',
  'HistorySearch','Task','ThemePicker','Settings','Tabs',
  'Attachments','Footer','MessageSelector','DiffDialog',
  'ModelPicker','Select','Plugin'
] as const

export const KeybindingBlockSchema = lazySchema(() =>
  z.object({
    context: z.enum(KEYBINDING_CONTEXTS).describe(
      'UI context where these bindings apply. Global bindings work everywhere.'),
    bindings: z.record(
      z.string().describe('Keystroke pattern (e.g., "ctrl+k", "shift+tab")'),
      z.union([
        z.enum(KEYBINDING_ACTIONS),
        z.string().regex(/^command:[a-zA-Z0-9:\-_]+$/)
          .describe('Command binding (e.g., "command:help", "command:compact"). Executes the slash command as if typed.'),
        z.null().describe('Set to null to unbind a default shortcut'),
      ]).describe('Action to trigger, command to invoke, or null to unbind'),
    ).describe('Map of keystroke patterns to actions'),
  }).describe('A block of keybindings for a specific context'))

export const KeybindingsSchema = lazySchema(() =>
  z.object({
    $schema: z.string().optional().describe('JSON Schema URL for editor validation'),
    $docs: z.string().optional().describe('Documentation URL'),
    bindings: z.array(KeybindingBlockSchema()).describe('Array of keybinding blocks by context'),
  }).describe('Claude Code keybindings configuration. Customize keyboard shortcuts by context.'))

export type KeybindingsSchemaType = z.infer<ReturnType<typeof KeybindingsSchema>>
```

`KEYBINDING_ACTIONS` (86 entries; canonical list at `schema.ts:64-172`)
is the verbatim union accepted as a non-`command:`/non-null action.

**Inconsistency: `Scroll` context.** `defaultBindings.ts:196` defines a
`context: 'Scroll'` block (`pageup`, `pagedown`, `wheelup`, `wheeldown`,
`ctrl+home`, `ctrl+end`, `ctrl+shift+c`, `cmd+c`) but `Scroll` is **not**
in `KEYBINDING_CONTEXTS` above. The default block is fed through
`parseBindings(DEFAULT_BINDINGS)` which never runs Zod, so defaults work,
but a user attempting to override e.g. `wheeldown → scroll:lineDown` in
`~/.claude/keybindings.json` fails Zod with `Unknown context "Scroll"`.
Same pattern applies to `MessageActions` when `feature('MESSAGE_ACTIONS')`
is on. Reimplementers should either add both contexts to
`KEYBINDING_CONTEXTS` or document the override gap. See
`BUGS-IN-SOURCE.md` candidate.

### 6.4 Constants

| Name | Value | Source |
|---|---|---|
| `CHORD_TIMEOUT_MS` | 1000 | `KeybindingProviderSetup.tsx:30` |
| `FILE_STABILITY_THRESHOLD_MS` | 500 | `loadUserBindings.ts:51` |
| `FILE_STABILITY_POLL_INTERVAL_MS` | 200 | `loadUserBindings.ts:56` |
| `MAX_VIM_COUNT` | 10000 | `vim/types.ts:182` |
| Vim indent step | `'  '` (two spaces) | `operators.ts:357` |
| Default `editorMode` | `'normal'` | `utils/config.ts:593` |
| `EDITOR_MODES` | `['normal','vim']` | `utils/configConstants.ts:15` |
| Notification timeout for warnings | 60000 ms | `KeybindingProviderSetup.tsx:90` |
| User config path | `join(getClaudeConfigHomeDir(), 'keybindings.json')` | `loadUserBindings.ts:115-117` |
| Template `$schema` | `https://www.schemastore.org/claude-code-keybindings.json` | `template.ts:46` |
| Template `$docs` | `https://code.claude.com/docs/en/keybindings` | `template.ts:47` |

### 6.5 Modifier alias map (`parser.ts:23-71`)

```
ctrl|control                          → ctrl
alt|opt|option                        → alt
shift                                 → shift
meta                                  → meta
cmd|command|super|win                 → super
esc                                   → key='escape'
return                                → key='enter'
space                                 → key=' '
↑ ↓ ← →                               → key='up|down|left|right'
otherwise                             → key=part.toLowerCase()
```

Display map (`parser.ts:105-138`): `escape→Esc`, ` →Space`,
`tab→tab`, `enter→Enter`, `backspace→Backspace`, `delete→Delete`,
arrows→Unicode arrows, `pageup→PageUp`, `pagedown→PageDown`,
`home→Home`, `end→End`. macOS displays `alt|meta` as `opt`, `super`
as `cmd`; other platforms as `alt`/`super`.

Chord separator: `parseChord` splits on `\s+`, but a literal lone
space is parsed as the space key (`parser.ts:80-84`).

There is **no leader key**. Chords are explicit ASCII space-separated
multi-keystroke sequences; the only multi-key defaults are
`ctrl+x ctrl+k` and `ctrl+x ctrl+e` (Chat).

### 6.6 Vim normal-mode key glossary (commands.md style index)

Operators: `d` delete, `c` change, `y` yank.
Motions: `h l j k`, `w b e W B E`, `0 ^ $`, `G`, `gj gk`, `gg`.
Find: `f F t T` (then char), `; ,` (repeat / reverse).
Text objects: `iw aw iW aW i" a" i' a' i\` a\` i( a( i) a) ib ab i[ a[ i] a] i{ a{ i} a} iB aB i< a< i> a>`.
Inserts: `i I a A o O`.
Counts: any `1-9` then digits.
Other: `r<char>` replace, `~` toggle case, `x` delete char,
`J` join, `p` paste after, `P` paste before, `D` delete-to-EOL,
`C` change-to-EOL, `Y` yank line, `>>` indent, `<<` outdent, `.`
dot-repeat, `u` undo (delegated to `props.onUndo`).

## 7. Side Effects & I/O

- **FS read.** `~/.claude/keybindings.json` — sync (`readFileSync`
  inside React `useState` initializer) and async (`readFile`
  inside watcher callbacks).
- **FS watch.** `chokidar.watch(userPath)` with `awaitWriteFinish`
  (500/200 ms). Cleaned up via `registerCleanup`.
- **Config FS.** `/vim` reads/writes `~/.claude/config.json` via
  `getGlobalConfig`/`saveGlobalConfig` to flip `editorMode`
  between `normal` and `vim` (`commands/vim/vim.ts:8-22`).
- **Process / signals.** None directly. Vim never spawns; keybindings
  never spawn.
- **Env vars.** No direct env reads in this subsystem; indirect via
  `getClaudeConfigHomeDir()` and `getPlatform()`.
- **External binaries.** None.
- **Trust boundary.** User keybindings.json loading is gated by
  `tengu_keybinding_customization_release` GrowthBook
  (`loadUserBindings.ts:41-46`) — when off, defaults only and the
  watcher never starts (lines 137-139, 357-362).

## 8. Feature Flags & Variants

| Gate | Effect when on | Effect when off | Source |
|---|---|---|---|
| `feature('KAIROS') \|\| feature('KAIROS_BRIEF')` | Adds Global `ctrl+shift+b → app:toggleBrief` | Binding absent | `defaultBindings.ts:45-47` |
| `feature('QUICK_SEARCH')` | Adds Global `ctrl+shift+f`, `cmd+shift+f`, `ctrl+shift+p`, `cmd+shift+p` | All four absent | `defaultBindings.ts:52-59` |
| `feature('TERMINAL_PANEL')` | Adds Global `meta+j → app:toggleTerminal` | Absent | `defaultBindings.ts:60` |
| `feature('MESSAGE_ACTIONS')` | Adds Chat `shift+up → chat:messageActions` and the entire `MessageActions` context block | All absent | `defaultBindings.ts:88-90, 268-295` |
| `feature('VOICE_MODE')` | Adds Chat `space → voice:pushToTalk` | Absent | `defaultBindings.ts:96` |
| GrowthBook `tengu_keybinding_customization_release` (default `false`) | User keybindings.json loaded + watched | Defaults only; watcher no-ops | `loadUserBindings.ts:41-46, 137-139, 357-362` |
| `config.editorMode === 'vim'` | `<VimTextInput>` mounted; `useVimInput` runs | `<TextInput>` mounted; vim FSM never instantiated | `PromptInput/utils.ts:12-15`, `PromptInput.tsx:2243` |

ANT-only path: `ModelPicker` context is annotated `// ant-only`
(`defaultBindings.ts:309`) but ships unconditionally; the UI gating of
that picker lives outside this module. The `editorMode === 'emacs'`
legacy value is mapped to `'normal'` for backward compatibility
(`commands/vim/vim.ts:13-15`).

`USER_TYPE === 'ant'` does not gate any binding here. The doc comment
in `loadUserBindings.ts:6-10` claims "User keybinding customization is
currently only available for Anthropic employees", but the runtime
gate is the GrowthBook flag, not `USER_TYPE`.

## 9. Error Handling & Edge Cases

- **`keybindings.json` ENOENT** → defaults, no warnings
  (`loadUserBindings.ts:217-220`).
- **JSON parse error** → defaults + error warning
  `Failed to parse keybindings.json: <msg>` (line 222-235).
- **Wrapper missing `bindings`** → defaults + error
  `keybindings.json must have a "bindings" array` with suggestion
  `Use format: { "bindings": [ ... ] }` (line 152-167).
- **`bindings` not an array** → `'"bindings" must be an array'`.
- **Block structure invalid** → `keybindings.json contains invalid
  block structure`; suggestion `Each block must have "context"
  (string) and "bindings" (object)` (line 170-189).
- **Duplicate JSON keys in same block** → warning per second
  occurrence; the message says JSON uses the last value
  (`validate.ts:289-302`). Cross-context duplicates are allowed.
- **Duplicate key→action within a context** (post-parse) → warning
  with `Previously bound to "<old>". Only the last binding will be
  used.` (`validate.ts:336-368`).
- **Reserved shortcut binding** → severity per `reservedShortcuts.ts`
  (`ctrl+c`/`ctrl+d`/`ctrl+m`/`ctrl+\` are errors; `ctrl+z` and macOS
  shortcuts vary).
- **Invalid context name** → `Unknown context "X"`; suggestion lists
  the 18 valid names (`validate.ts:156-163`).
- **`command:` binding outside Chat** → warning (must be in `Chat`
  context, line 209-218).
- **`command:` action with bad characters** → warning, regex
  `/^command:[a-zA-Z0-9:\-_]+$/` (line 198-207).
- **`voice:pushToTalk` bound to bare a–z** → warning that the binding
  prints into the input during warmup; suggest `space` or modifier
  combo (`validate.ts:220-242`). Cross-references spec 36.
- **Empty key part / unparseable keystroke** → parse errors
  (`validate.ts:91-122`).
- **Notifications.** When warnings exist, a single notification
  `keybinding-config-warning` is added with text `Found N keybinding
  error(s) and M warning(s) · /doctor for details` (or singular
  variants), severity `error`/`warning`, priority `immediate`/`high`,
  60 s timeout (`KeybindingProviderSetup.tsx:60-91`).
- **Chord timeout.** After 1 s of no follow-up keystroke, the chord
  is cancelled (debug log `[keybindings] Chord timeout - cancelling`).
- **Chord cancel via Escape.** While `pending !== null`, Escape always
  returns `chord_cancelled`; `KeybindingProviderSetup.ChordInterceptor`
  stops propagation (lines 161-166).
- **Wheel events.** `ChordInterceptor` no-ops wheel events when
  no chord is pending (line 119-121).
- **`r<BS>`** cancels the replace; absent guard, `executeReplace('')`
  would delete under cursor (`transitions.ts:444-446`).
- **`G` in vim** with `count===1` means "no count" (last line),
  otherwise line N (`transitions.ts:146-156`).
- **`x` at EOF** is a no-op (`operators.ts:174`).
- **Word motions and Image chips.** Operator ranges always snap out of
  `[Image #N]` chips at both ends (`operators.ts:471-473`).
- **macOS Cmd bindings on non-kitty terminals.** `cmd+c` etc. simply
  never fire because the modifier never reaches the pty
  (`match.ts:54-58`, `defaultBindings.ts:206-211`).
- **Ink quirk: escape sets `key.meta`** is suppressed at two sites:
  `match.ts:96-101` zeroes `meta` when matching escape, and
  `resolver.ts:88-89` zeroes effective `meta` when building a chord
  keystroke for the escape key.
- **Customization disabled fallback.** External users always get
  `defaultBindings`, no merge, no watcher. The cache is still populated
  so subsequent calls are O(1) (`loadUserBindings.ts:266-271`).

## 10. Telemetry & Observability

| Event | Trigger | Source |
|---|---|---|
| `tengu_custom_keybindings_loaded` | Once per UTC day on first user-bindings load; `{user_binding_count}` | `loadUserBindings.ts:83-90` |
| `tengu_keybinding_fallback_used` | `getShortcutDisplay` / `useShortcutDisplay` returns the fallback. `{action, context, fallback, reason: 'action_not_found' \| 'no_context'}` | `shortcutFormat.ts:48-58`, `useShortcutDisplay.ts:42-55` |
| `tengu_editor_mode_changed` | `/vim` toggles editor mode; `{mode, source:'command'}` | `commands/vim/vim.ts:24-28` |

Debug logs (`logForDebugging`): `[keybindings] Loaded N user bindings
from <path>`, `[keybindings] Watching for changes to <path>`,
`[keybindings] Detected change to <path>`, `[keybindings] Detected
deletion of <path>`, `[keybindings] Chord timeout - cancelling`,
`[keybindings] KeybindingSetup initialized with N bindings, M
warnings`, `[keybindings] Reloaded: N bindings, M warnings`,
`[keybindings] Skipping file watcher - user customization disabled`.

## 11. Reimplementation Checklist

- Provide a `types.ts` exporting `Chord`, `ParsedKeystroke`,
  `ParsedBinding`, `KeybindingBlock`, `KeybindingContextName`
  consistent with consumers (registry references, see §12).
- Preserve the 86-action `KEYBINDING_ACTIONS` enum and 18-context
  `KEYBINDING_CONTEXTS` enum verbatim. Note that `Scroll` (and
  `MessageActions` when flag-on) appear as `context:` values in
  `DEFAULT_BINDINGS` despite being absent from `KEYBINDING_CONTEXTS` —
  reimplementers must either add them to the enum or accept the
  user-override gap (see §6.3 inconsistency note).
- Preserve every entry of the default bindings table including all
  five flag-gated branches and the platform-derived `IMAGE_PASTE_KEY`
  / `MODE_CYCLE_KEY` / `SUPPORTS_TERMINAL_VT_MODE` derivations.
- Modifier alias map matches `parser.ts:23-71` exactly; chord parser
  splits on `\s+` except a literal `' '` is the space key.
- Resolver semantics: last-binding-wins for same-context same-chord;
  prefix-with-non-null-action wins over exact single-key match;
  `null` user binding produces `unbound`; chord null on prefix is
  shadowed so the prefix may resolve as a single-key default.
- Ink modifier match: `alt`/`meta` collapse to `key.meta`; Escape
  zeroes meta; `super` is distinct.
- `loadKeybindings` precedence: defaults first, user bindings appended
  (so later wins). Sync and async paths must agree on cache.
- File watcher: chokidar with `awaitWriteFinish: { 500, 200 }`,
  `ignoreInitial: true`, `atomic: true`. Cleanup via
  `registerCleanup`.
- `tengu_custom_keybindings_loaded` rate-limited to once per UTC day
  (`new Date().toISOString().slice(0,10)` key).
- `KeybindingSetup` mounts a `<ChordInterceptor>` that registers
  `useInput` BEFORE children and stops propagation for chord
  starts/cancels/unbound matches and chord-completing matches.
- Chord pending state lives in both a ref (synchronous read in
  `resolve()`) and React state (re-render); 1 s timeout cancels.
- `useRegisterKeybindingContext` uses `useLayoutEffect` so registration
  precedes the first input event after mount.
- Active context priority in `useKeybinding(s)`: registered active
  contexts → declared `context` → `'Global'`, dedup preserving first
  occurrence.
- Validation: parse-error / duplicate / reserved / invalid_context /
  invalid_action; deduplicate by `${type}:${key}:${context}`.
- Reserved shortcut list: `NON_REBINDABLE` always; `TERMINAL_RESERVED`
  always; `MACOS_RESERVED` only on macOS.
- Template embeds `$schema` and `$docs` URLs and excludes
  non-rebindable shortcuts.
- Vim FSM: 11 `CommandState` variants and exact transitions per
  `transitions.ts`; `MAX_VIM_COUNT = 10000`; `0` is line-start motion
  not a count digit; `cw`/`cW` special-cases to end-of-word; linewise
  detection via trailing `\n`; image-chip snap-out at both ends.
- Vim execute: dot-repeat replays `RecordedChange` exactly; INSERT
  exit moves cursor left by 1 unless at column 0 or after `\n`.
- `/vim` toggles `config.editorMode` between `'normal'` and `'vim'`,
  treating legacy `'emacs'` as `'normal'`. UI gating uses
  `getGlobalConfig().editorMode === 'vim'`.

## 12. Open Questions / Unknowns

1. **Missing `src/keybindings/types.ts`.** All keybindings modules
   import `Chord`, `ParsedKeystroke`, `ParsedBinding`,
   `KeybindingBlock`, `KeybindingContextName` from `./types.js`, but
   the file is absent from the leaked tree. Field shapes are inferable
   from usage but the canonical declarations cannot be cited verbatim.
2. **Where `KeybindingSetup` is mounted.** The provider's call site is
   in the Ink shell (spec 37); not cited here.
3. **`isVimModeEnabled` reactivity.** `PromptInput.tsx:2243` reads
   `isVimModeEnabled()` synchronously on each render via
   `getGlobalConfig()`. Whether toggling `/vim` mid-session re-renders
   the prompt input (and therefore swaps `<TextInput>` ↔
   `<VimTextInput>`) without a manual reload depends on
   global-config-change propagation outside this module — see spec 41.
4. **`tengu_keybinding_customization_release` rollout.** The default
   passed to `getFeatureValue_CACHED_MAY_BE_STALE` is `false`
   (`loadUserBindings.ts:42-44`); whether ANT users get the flag on by
   default is a GrowthBook policy question, not visible here, despite
   the doc comment claiming ANT-only customization.
5. **`ModelPicker` ant-only annotation.** Inline comment at
   `defaultBindings.ts:309` says "(ant-only)" but the binding ships
   unconditionally — the UI hosting this context is ANT-gated
   elsewhere; spec not cross-checked here.
6. **Visual mode.** The state-machine ASCII in `vim/types.ts` only
   shows INSERT and NORMAL. `VimMode` is `'INSERT'|'NORMAL'`
   (`textInputTypes.ts:222`). No visual mode is implemented.
7. **Undo backing.** `u` delegates to `props.onUndo` from
   `useVimInput` callers; the actual undo store lives in `useTextInput`
   and is out of scope here.
8. **Vim INSERT mode × voice push-to-talk on `space`.** Default Chat
   binding `space → voice:pushToTalk` (gated by `feature('VOICE_MODE')`)
   collides with vim INSERT-mode space-insertion. `useVimInput`
   processes input *after* `useInput` keybinding handlers register, so
   in INSERT mode a `space` keystroke would first match the Chat
   `voice:pushToTalk` action and only reach `useVimInput.handleVimInput`
   if the handler returns `false` (event propagates) or
   `stopImmediatePropagation` is not called. Confirmed gap from spec 36
   review — neither spec 36 nor spec 39 documents the resolution.
   Likely: `voice:pushToTalk` handler must early-return when
   `editorMode === 'vim'` and `vimState.mode === 'INSERT'`, but that
   guard is not visible in the leaked tree. Defer to spec 36 follow-up.
