# 37c — Ink Primitives Catalog

## §0 Scope

Companion to spec **37 (ink-ui-shell)** and its sister catalogs **37a
(higher-level components)** and **37b (commands UI)**. This spec covers the
in-house Ink runtime under `src/ink/` — Claude Code does **not** depend on the
upstream `ink` package. It ships its own React reconciler, its own DOM-like
node tree, its own Yoga layout adapter, its own ANSI tokenizer/parser, its own
keypress parser, its own mouse-tracking and selection model, its own focus
manager, and its own renderer/diff/blit pipeline.

That's why the residual list under PHASE9 audit is so dense: 96 files in
`src/ink/`, 69 of them uncited by the original spec 37 (which focused on the
shell, not the primitive layer). This catalog enumerates them all.

The boundary is sharp:

- `src/ink/` = the renderer + primitives (the equivalent of Ink-the-library).
- `src/components/`, `src/screens/`, `src/ink/components/App.tsx` host = the
  application UI built **on top** of these primitives (37 / 37a).

The catalog is grouped by category. Each entry lists role, key exports, and —
where obvious from imports — who consumes it. Nothing here describes
application-level UI; that's spec 37/37a/37b's job.


## §1 Primitive Categories

The 96 files cluster into ten categories:

| § | Category | Count | Where |
|---|---|---:|---|
| §2.1 | Host components (Box / Text / Button / etc.) | 18 | `src/ink/components/*` |
| §2.2 | DOM + reconciler + renderer pipeline | 11 | `src/ink/{dom,reconciler,renderer,...}.ts` |
| §2.3 | Layout (Yoga adapter) | 4 | `src/ink/layout/*` |
| §2.4 | Screen buffer + diff + blit + selection | 7 | `src/ink/{screen,frame,output,...}.ts` |
| §2.5 | Text measurement + wrapping | 7 | `src/ink/{stringWidth,measure-*,wrap-*}.ts` |
| §2.6 | ANSI runtime — termio (parser + emit) | 9 | `src/ink/termio*` |
| §2.7 | Event system | 10 | `src/ink/events/*` |
| §2.8 | Hooks (animation, input, terminal capabilities) | 12 | `src/ink/hooks/*` |
| §2.9 | Terminal capability detection / IO | 6 | `src/ink/{terminal,terminal-querier,...}.ts` |
| §2.10 | Misc utilities (constants, focus, hit-test, warn) | 12 | various |

Conventions:

- **Bun runtime hooks.** `wrapAnsi.ts` and `stringWidth.ts` prefer
  `Bun.wrapAnsi` / `Bun.stringWidth` when available, falling back to npm
  packages otherwise. The Bun fallbacks let the bundle still run on Node for
  tests / SDK consumers.
- **Native Yoga.** Layout calls into `src/native-ts/yoga-layout` (a TS port
  of yoga that ships the bundle with no `.node` binary dependency) — see
  `src/ink/layout/yoga.ts`.
- **`react/compiler-runtime`.** Most components are pre-compiled by the React
  Compiler (the `_c` cache import) — that's why the bodies are full of
  `$[idx] !== prev ? recompute : reuse` patterns.
- **`react-reconciler` (LegacyRoot + ConcurrentRoot).** `reconciler.ts` is
  the host config; `ink.tsx` mounts the root and plumbs onCommit /
  onPostCommit into the renderer pipeline.
- **DOM-like terminology.** Nodes are `DOMElement`s (`ink-root` / `ink-box` /
  `ink-text` / `ink-virtual-text` / `ink-link` / `ink-progress` /
  `ink-raw-ansi`). Events bubble via `parentNode`. There's a `FocusManager`
  per root and a `Dispatcher` for events — both modeled after the browser.


## §2 Entries

### §2.1 Host components (`src/ink/components/`)

These are the JSX primitives the rest of the codebase composes. They are NOT
re-exported by a barrel — direct imports from `'../ink/components/Box.js'`
etc. are used throughout `src/components/` and `src/ink/components/App.tsx`.

#### `src/ink/components/Box.tsx`
**Role:** The fundamental flex container. Wraps a Yoga node and accepts the
full `Styles` surface (`flexDirection`, `flexGrow`, padding, margin, border,
gap, position, overflow, etc., minus `textWrap` which is `Text`-only). Also
the carrier of all event-handler props: `onClick`, `onKeyDown` /
`onKeyDownCapture`, `onFocus` / `onBlur`, `onPaste`, `tabIndex`, `autoFocus`.
`tabIndex >= 0` participates in Tab/Shift+Tab cycling driven by `FocusManager`
(see §2.10). `onClick` only fires inside `<AlternateScreen>` where SGR mouse
tracking is active.
**Exports:** `default Box`, `Props` (extends `Styles` minus `textWrap`).
**Used by:** Practically every component in `src/components/` and
`src/screens/`.

#### `src/ink/components/Text.tsx`
**Role:** Inline text leaf. Accepts `color`, `backgroundColor`, `italic`,
`underline`, `strikethrough`, `dimColor`, `bold`, `inverse`, `wrap`. Writes
through to an `ink-text` DOMElement; styles cascade to nested `Text` nodes via
`squashTextNodesToSegments`.
**Exports:** `default Text`, prop type with `color: Color`.
**Used by:** Everywhere — the universal text primitive.

#### `src/ink/components/Button.tsx`
**Role:** Focusable interactive button. Wraps `Box`, tracks
`{focused, hovered, active}` internal state, fires `onAction` on Enter / Space
/ click. Accepts most `Styles` (minus `textWrap`), plus `tabIndex` (default
0), `autoFocus`, and standard event-capture props.
**Used by:** Wizard footers, dialog buttons in `src/components/wizard/`,
`src/components/agents/`, etc.

#### `src/ink/components/Newline.tsx`
**Role:** Inserts `\n × count` (default 1). Must live inside a `<Text>`.
Tiny — pure render-from-prop, memoized via React Compiler cache.

#### `src/ink/components/Spacer.tsx`
**Role:** `<Box flexGrow={1} />` shorthand for filling the major axis of a
flex container. One-liner.

#### `src/ink/components/Link.tsx`
**Role:** OSC 8 hyperlink wrapper. If `supportsHyperlinks()` is true, emits
`<ink-link href={url}>...</ink-link>`; otherwise renders `fallback ?? children`.
Used inside `Text` and inside `Ansi.tsx`'s span renderer.

#### `src/ink/components/ScrollBox.tsx`
**Role:** Scroll viewport. Imperative handle exposes
`scrollTo / scrollBy / scrollToBottom / scrollToElement / getScrollTop /
getScrollHeight / getYogaScrollHeight / getPendingDelta`. The
`scrollToElement` variant is render-time-deferred (reads
`yogaNode.getComputedTop()` in the same Yoga pass that computes
scrollHeight) so it's stable under the throttled render. Marks scroll
activity via `bootstrap/state.js#markScrollActivity` so animations can pause
during scrolling. Backbone of the transcript / message-list views.

#### `src/ink/components/AlternateScreen.tsx`
**Role:** Enters DEC 1049 alt-screen, optionally enables SGR mouse tracking
(default on). While mounted, height is constrained to terminal rows so
overflow must be handled via `overflow: scroll` / flexbox. Exit-restores the
main screen content. Notifies the Ink instance via `setAltScreenActive()` so
the renderer keeps the cursor inside the viewport. The host for the
fullscreen UI (e.g. ctrl-O transcript overlay).

#### `src/ink/components/RawAnsi.tsx`
**Role:** Bypass for pre-rendered, already-width-wrapped ANSI lines. Emits a
single Yoga leaf with constant-time measure func and hands the joined string
straight to `output.write`. Skips the `<Ansi>` → React tree → squash →
re-emit roundtrip. Used by ColorDiff / NAPI module output, syntax-highlighted
diff transcripts, etc.

#### `src/ink/components/NoSelect.tsx`
**Role:** Marks contents non-selectable in fullscreen text selection. Cells
inside are skipped by both the highlight overlay and the copied-text walk.
`fromLeftEdge` extends the exclusion zone to column 0 — used for gutters
(line numbers, diff +/- sigils, list bullets) inside indented containers.

#### `src/ink/components/ErrorOverview.tsx`
**Role:** Last-ditch error surface — pretty-prints `error.message`, the
top stack frame's source excerpt (via `code-excerpt`), and a cleaned stack
trace (via `stack-utils`). Mounted by `App.tsx`'s ErrorBoundary on render
crash.

#### `src/ink/components/App.tsx`
**Role:** Root host wrapping the user's tree. Owns the InputEvent /
TerminalFocusEvent emitters (via `EventEmitter`), starts/stops the
`TerminalQuerier`, manages early-input capture handoff, the
`SelectionState`, kitty/modifyOtherKeys mode lifecycle, DECSET 1004 focus
reporting, suspend (Ctrl+Z) handling, and the ErrorBoundary. Provides
`AppContext`, `StdinContext`, `ClockProvider`, `CursorDeclarationContext`,
`TerminalFocusProvider`, `TerminalSizeContext` to children.

#### `src/ink/components/AppContext.ts`
**Role:** Context exposing `exit(error?)` to descendants. `useApp()` reads
this. Default value is a no-op so tests can render outside an Ink instance.

#### `src/ink/components/ClockContext.tsx`
**Role:** Shared animation clock. `Clock` interface:
`subscribe(onChange, keepAlive)`, `now()`, `setTickInterval(ms)`. Subscribers
declare keepAlive intent — the interval only runs when at least one keepAlive
exists. All instances share one tick so animations stay phase-locked. Used by
`useAnimationFrame` and `useAnimationTimer`.

#### `src/ink/components/CursorDeclarationContext.ts`
**Role:** Setter for "where the IME caret should be parked after this
frame." Carries a `{relativeX, relativeY, node}` declaration. The optional
`clearIfNode` argument prevents sibling list items from clobbering each
other's declarations during focus transitions. Read by `ink.tsx`'s
post-render to position the native terminal cursor.

#### `src/ink/components/StdinContext.ts`
**Role:** Carries `{stdin, setRawMode, isRawModeSupported,
internal_exitOnCtrlC, internal_eventEmitter, internal_querier}` to
descendants. `useStdin()` reads it. The `internal_*` fields are how
`useInput`, `useSelection`, `useSearchHighlight`, `useTabStatus` reach the
Ink instance without prop drilling.

#### `src/ink/components/TerminalFocusContext.tsx`
**Role:** `{isTerminalFocused, terminalFocusState}` from DECSET 1004 events.
`TerminalFocusProvider` subscribes via `useSyncExternalStore` to the
non-React `terminal-focus-state` signal so non-focus-consuming children
don't re-render on focus change. `terminalFocusState: 'unknown'` is the
default for terminals that don't report focus — treated identically to
`'focused'` (no animation throttling).

#### `src/ink/components/TerminalSizeContext.tsx`
**Role:** `{columns, rows}` context populated by `App.tsx` from
SIGWINCH / `process.stdout.columns`. Trivial — six-line file. Read by
`useTerminalSize` and most layout-aware components.


### §2.2 DOM + reconciler + renderer pipeline

This is the React-host plumbing equivalent to react-dom's reconciler, but for
a 2D character grid.

#### `src/ink/dom.ts`
**Role:** The "DOM" — `DOMElement`, `DOMNode`, `TextNode`, `ElementNames`
(`'ink-root' | 'ink-box' | 'ink-text' | 'ink-virtual-text' | 'ink-link' |
'ink-progress' | 'ink-raw-ansi'`). Mutation API: `createNode`,
`createTextNode`, `appendChildNode`, `insertBeforeNode`, `removeChildNode`,
`setAttribute`, `setStyle`, `setTextNodeValue`, `setTextStyles`, `markDirty`,
`scheduleRenderFrom`. Each `DOMElement` carries a Yoga `LayoutNode` reference
and pending-clear bookkeeping.
**Used by:** `reconciler.ts` (creates these from React fibers),
`render-node-to-output.ts` (walks them).

#### `src/ink/reconciler.ts`
**Role:** `react-reconciler` host config. Wires `appendChildNode` /
`insertBeforeNode` / `removeChildNode` / `setStyle` etc. as the hostConfig.
Uses `LegacyRoot` (default) but `ink.tsx` chooses ConcurrentRoot. Tracks
profile counters (`getLastCommitMs`, `getLastYogaMs`, `recordYogaMs`) for
the FPS overlay. Detects environment to skip devtools connection in
production. Exports `dispatcher` (a `Dispatcher` for events) and
`markCommitStart` (called by ScrollBox to time a commit).

#### `src/ink/ink.tsx`
**Role:** The `Ink` class — one per render target. Owns the FocusManager,
the LogUpdate writer, the optimizer, the screen buffer pair (front/back
frames), the FrameEvent emitter, alt-screen state, mouse-state, scroll
follow state. Schedules renders via queueMicrotask (test env uses
synchronous onImmediateRender). Bootstraps colorize (chalk) and terminal
notification provider. 30+ imports.

#### `src/ink/renderer.ts`
**Role:** The `Ink.render(node)` entry — creates an `Ink` instance, hands
the React tree to the reconciler, registers in `instances` so consecutive
calls reuse the same instance per WriteStream. Exposes `RenderOptions`
(`stdout`, `stdin`, `stderr`, `exitOnCtrlC`, `patchConsole`, `debug`, etc.).

#### `src/ink/root.ts`
**Role:** Helper that creates a detached `ink-root` DOMElement with its own
`FocusManager`, runs render-to-output and returns the produced screen. Used
for **off-tree measurement** — e.g. `useSearchHighlight.scanElement` paints
the existing DOM subtree to a fresh Screen at its natural height to compute
match positions.

#### `src/ink/render-node-to-output.ts`
**Role:** The walker that converts the laid-out DOM tree to terminal output.
Reads each node's Yoga rect, applies text styles via `applyTextStyles`,
draws borders via `renderBorder`, handles `overflow: scroll` translation,
populates `nodeCache` (rect + getComputedTop) for hit-testing and ScrollBox
viewport culling, sets `didLayoutShift` when any node moved (read by
`ink.tsx` to decide whether to force full damage), emits scroll-region
optimization hints (`getScrollHint`) for alt-screen DECSTBM scrolling.

#### `src/ink/render-to-screen.ts`
**Role:** Higher-level wrapper. Takes `{frontFrame, backFrame, isTTY,
terminalWidth, terminalRows, altScreen, prevFrameContaminated}`, calls
render-node-to-output, returns the new Frame. Also exports `MatchPosition`
(used by `useSearchHighlight`).

#### `src/ink/render-border.ts`
**Role:** Border drawing. `cli-boxes` styles plus a custom `dashed` style
that uses `╌` / `╎`. Supports inline border text (`BorderTextOptions`:
position top/bottom, align start/end/center, offset). Draws via direct
`Output.write` of box-drawing chars with applied colors.

#### `src/ink/output.ts`
**Role:** The character grid being written into. Manages a 2D buffer of
cells (interned via the shared `CharPool` / `StylePool` / `HyperlinkPool`),
exposes `write(col, row, text, styles, hyperlink)`, blit, etc.
~all-purpose writer that the renderer drives.

#### `src/ink/optimizer.ts`
**Role:** Diff post-processor — single-pass: drop empty stdout patches,
merge consecutive cursorMoves, drop no-op (0,0) moves, concat adjacent
style patches, dedupe consecutive same-URI hyperlinks, cancel cursor
hide/show pairs, drop count-0 clears. Reduces the byte-stream sent to the
terminal.

#### `src/ink/log-update.ts`
**Role:** The "log-update" writer — diffs `{frontFrame, backFrame}` into a
patch list, optimizes, and writes to stdout. Handles the full damage path
(`forceRedraw`), DECSET 2026 synchronized output (BSU/ESU bracketing),
cursor hide/show, alt-screen scroll-region optimization (DECSTBM with
csiScrollUp/csiScrollDown), OSC 8 hyperlink dedup. The bottom of the
render pipeline.


### §2.3 Layout (Yoga adapter)

Adapter pattern — `LayoutNode` is the abstract interface, Yoga is the
implementation. Lets the bundle ship a TS-port Yoga without coupling the
component layer to native Yoga's API surface.

#### `src/ink/layout/node.ts`
**Role:** Adapter types + enums: `LayoutEdge`, `LayoutGutter`,
`LayoutDisplay`, `LayoutFlexDirection`, `LayoutAlign`, `LayoutJustify`,
`LayoutOverflow`, `LayoutPositionType`, `LayoutWrap`, `LayoutMeasureMode`,
`LayoutMeasureFunc`, `LayoutNode` (the interface — getComputedWidth,
getComputedHeight, getComputedTop, getComputedLeft, getComputedPadding,
getComputedBorder, setMeasureFunc, calculateLayout, etc.).

#### `src/ink/layout/yoga.ts`
**Role:** Implementation — `createYogaLayoutNode()` returns a `LayoutNode`
backed by `Yoga` from `src/native-ts/yoga-layout`. Maps adapter enums to
Yoga's `Align` / `FlexDirection` / `Edge` / `Justify` / etc.

#### `src/ink/layout/engine.ts`
**Role:** One-line entrypoint: `createLayoutNode()` → calls the Yoga
factory. The seam where alternate engines could plug in.

#### `src/ink/layout/geometry.ts`
**Role:** Pure value types: `Point`, `Size`, `Rectangle`, `Edges`. Helpers:
`edges(...)` overload, `unionRect`, `clamp`. Used by hit-test, screen blit,
node-cache, selection.


### §2.4 Screen buffer + frame + selection

The cell-level model that sits between layout output and stdout writes.

#### `src/ink/screen.ts`
**Role:** The `Screen` data model — interned cells (via `StylePool`,
`CharPool`, `HyperlinkPool`), `CellWidth` enum (Single, Wide, SpacerTail —
the half-cell that follows a CJK / wide emoji), getters
`cellAt / cellAtIndex / charInCellAt / visibleCellAtIndex`, mutators
`setCellAt / setCellStyleId / shiftRows / blitRegion / resetScreen /
markNoSelectRegion`, OSC 8 hyperlink helpers. Diff iteration via
`diffEach`. Pools are shared across screens so blit copies cell-IDs
directly without re-interning.

#### `src/ink/frame.ts`
**Role:** `Frame = {screen, viewport, cursor, scrollHint?,
scrollDrainPending?}`. `emptyFrame(rows, cols, pools)` creates a blank
frame. The diff between two frames produces a `Diff` (patch list) which
optimizer + log-update consume.

#### `src/ink/selection.ts`
**Role:** Fullscreen text-selection state — `{anchor, focus, isDragging,
mode}` (mode is char | word | line). Cell-aware (handles wide chars).
Exposes `startSelection`, `extendSelection`, `finishSelection`,
`clearSelection`, `hasSelection`, `captureScrolledRows` (used when
keyboard-scrolling a selection: rows scrolled out are captured as plain
text so the eventual copy still includes them), `shiftAnchor`,
`shiftSelection`, `applySelectionOverlay` (paints the highlight onto a
Screen via SGR 7 inverse).

#### `src/ink/searchHighlight.ts`
**Role:** Inverts cells at all visible occurrences of `query`
(case-insensitive, wide-char-aware). Same damage machinery as selection —
the diff picks up inverted cells as ordinary changes, log-update stays a
pure diff engine. Returns true if any match was painted (caller forces
full-frame damage on transitions).

#### `src/ink/hit-test.ts`
**Role:** Given `(col, row)`, walks the DOMElement tree (using
`nodeCache` rects) to find the deepest hit node. Children traversed in
reverse so later siblings (painted on top) win. Also exposes
`dispatchClick` and `dispatchHover` — bubble the event up through
`parentNode`, recompute `localCol/localRow` per handler. Hit even on
nodes without `onClick` so dispatchClick can find ancestors with handlers.

#### `src/ink/node-cache.ts`
**Role:** `nodeCache: WeakMap<DOMElement, CachedLayout>` —
`{x, y, width, height, top?}`. Populated by `render-node-to-output`,
read by `hit-test`, `selection`, `ScrollBox`. Also exports
`pendingClears` (rects of removed children that need clearing) and
`addPendingClear` / `consumeAbsoluteRemovedFlag` — the
absolute-position-removed flag disables blit for the next frame because
absolute removals can paint over non-siblings.

#### `src/ink/measure-element.ts`
**Role:** `measureElement(node) → {width, height}`. Reads
`yogaNode.getComputedWidth()` / `getComputedHeight()`. Tiny.


### §2.5 Text measurement + wrapping

#### `src/ink/stringWidth.ts`
**Role:** Display width of a string in terminal cells. Prefers
`Bun.stringWidth` when available; falls back to a pure-JS implementation
that uses `eastAsianWidth` directly with `ambiguousAsWide: false` (per
Unicode recommendation for Western contexts) — more accurate than the
`string-width` package for chars like ⚠ (which `string-width` reports as
width 2). Strips ANSI first via `strip-ansi`.

#### `src/ink/widest-line.ts`
**Role:** `widestLine(s)` — max line width via `lineWidth` cache. Iterates
on `'\n'` boundaries with `indexOf` (no array allocation).

#### `src/ink/line-width-cache.ts`
**Role:** Per-line `stringWidth` cache — completed transcript lines are
immutable, so caching cuts ~50× the calls during streaming. Simple Map
with full-clear at 4096 entries.

#### `src/ink/measure-text.ts`
**Role:** Single-pass `{width, height}` for a multi-line text under a
`maxWidth`. Uses `lineWidth` and `Math.ceil(w / maxWidth)` for height.
Handles `maxWidth ≤ 0` / non-finite by treating each line as one visual
line. The Yoga measure-func backend for `ink-text` leaves.

#### `src/ink/wrap-text.ts`
**Role:** The text-wrapping helper used by `dom.ts` / leaf text nodes.
Combines `sliceAnsi`, `stringWidth`, `wrapAnsi`, ellipsis on overflow
(`…`), with `truncate(text, columns, position: 'start' | 'middle' | 'end')`
plus boundary handling for wide chars (sliceAnsi may overshoot by 1 cell;
retry once with a tighter bound).

#### `src/ink/wrapAnsi.ts`
**Role:** `wrapAnsi(input, columns, options?)` — prefers `Bun.wrapAnsi`,
falls back to npm `wrap-ansi`.

#### `src/ink/squash-text-nodes.ts`
**Role:** `squashTextNodesToSegments(node, inheritedStyles, hyperlink)` —
walks a `<Text>` subtree and produces `StyledSegment[]`
(`{text, styles, hyperlink?}`) with styles merged top-down. Used during
render to flatten nested `Text` markup into a styled string the screen
buffer can ingest.

#### `src/ink/tabstops.ts`
**Role:** `expandTabs(text, interval = 8)` — POSIX 8-column tabstops
(hardcoded in Ghostty, inspired by its `Tabstops.zig`). Skips ANSI
sequences via the tokenizer, only expands `\t` in the text stream.

#### `src/ink/bidi.ts`
**Role:** Software bidi reordering for RTL text on Windows /
WindowsTerminal (which lack native bidi). Uses `bidi-js`. macOS terminals
do bidi natively, so this is a no-op there. Detection: `WT_SESSION` for
Windows Terminal or `process.platform === 'win32'`.


### §2.6 ANSI runtime — termio (parser + emitter)

The full ANSI / VT500-series implementation. A semantic action-based parser
(inspired by ghostty / iTerm2 / tmux) sits next to constants for emitting
sequences. Anything that produces or consumes raw escape codes goes through
this directory.

#### `src/ink/termio.ts`
**Role:** Re-export façade — `Parser`, `Action`, `Color`, `CursorAction`,
`CursorDirection`, `Grapheme`, `NamedColor`, `TextStyle`, `defaultStyle`.
The `Parser` is a streaming class — `parser.feed(input)` returns
`Action[]` where actions are structured (`{type: 'text', graphemes,
style}`, `{type: 'cursor', action: ...}`, `{type: 'sgr', ...}`, …) not
string tokens. Used by `Ansi.tsx` and the keypress parser.

#### `src/ink/termio/types.ts`
**Role:** `NamedColor` (16-color names), `Color` (named | rgb | indexed),
`UnderlineStyle`, `TextStyle`, `defaultStyle`, action variants. The
semantic-not-string design.

#### `src/ink/termio/ansi.ts`
**Role:** C0 / C1 control character constants (`NUL`…`US`, `BEL`, `ESC`,
etc.), ESC types (`ESC_TYPE.CSI / OSC / DCS / SS3 / APC`), `SEP`, helpers
like `isEscFinal`. The byte-level vocabulary.

#### `src/ink/termio/csi.ts`
**Role:** CSI parameter byte ranges (`isCSIParam`, `isCSIIntermediate`,
`isCSIFinal`), `csi(...parts)` builder, named CSI sequences:
`CURSOR_HOME`, `cursorTo`, `cursorMove`, `eraseLines`, `ERASE_SCREEN`,
`ERASE_SCROLLBACK`, `setScrollRegion`, `RESET_SCROLL_REGION`,
`scrollUp`, `scrollDown`, `PASTE_START` / `PASTE_END`,
`ENABLE_KITTY_KEYBOARD` / `DISABLE_KITTY_KEYBOARD`,
`ENABLE_MODIFY_OTHER_KEYS` / `DISABLE_MODIFY_OTHER_KEYS`, `FOCUS_IN` /
`FOCUS_OUT`, `CSI` parser-side enums. The bulk of "emit ANSI" lives here.

#### `src/ink/termio/dec.ts`
**Role:** DEC private modes (`DEC.CURSOR_VISIBLE = 25`, `ALT_SCREEN = 47`,
`ALT_SCREEN_CLEAR = 1049`, `MOUSE_SGR = 1006`, `BRACKETED_PASTE = 2004`,
`SYNCHRONIZED_UPDATE = 2026`, `FOCUS_EVENTS = 1004`, etc.) plus emitters
`decset(mode)` / `decreset(mode)`. Re-exports common ones:
`SHOW_CURSOR`, `HIDE_CURSOR`, `ENTER_ALT_SCREEN` (1049 set),
`EXIT_ALT_SCREEN` (1049 reset), `ENABLE_MOUSE_TRACKING`,
`DISABLE_MOUSE_TRACKING`, `BSU` / `ESU` (synchronized output bracket),
`DBP` / `EBP` (bracketed-paste), `DFE` / `EFE` (focus events).

#### `src/ink/termio/esc.ts`
**Role:** Simple ESC sequence parser (RIS / DECSC / DECRC / NEL / IND /
RI / DECPAM / DECPNM). Returns an `Action` or null.

#### `src/ink/termio/osc.ts`
**Role:** OSC framing (`OSC_PREFIX`, `ST = ESC \\`), `osc(...parts)` —
uses `ST` for kitty (avoids beeps), `BEL` elsewhere; `wrapForMultiplexer`
— DCS-passthrough wrap for tmux 3.3+ (`allow-passthrough` gate);
`parseOSC` for the parser; constants `OSC.SET_TITLE_AND_ICON`,
`PROGRESS`, `ITERM2`, OSC 8 hyperlink (`link`, `LINK_END`,
`OSC8_PREFIX`), tabstatus (`tabStatus`, `supportsTabStatus`,
`CLEAR_TAB_STATUS`).

#### `src/ink/termio/sgr.ts`
**Role:** Parses SGR parameters (`;` and `:` separators) into a
`TextStyle`. Handles 8-color, bright (90+), 256-color (38;5;N), truecolor
(38;2;R;G;B), underline styles (single/double/curly/dotted/dashed via
SGR 4:N), strikethrough, blink, italic, reset (0). Produces semantic
`Color` values not raw codes.

#### `src/ink/termio/parser.ts`
**Role:** The streaming `Parser` class. Uses the tokenizer for boundary
detection, then dispatches to `applySGR`, `parseEsc`, `parseOSC`. Tracks
current `TextStyle` across calls — `feed(chunk)` emits actions tagged
with the style at chunk-time. Grapheme-aware via Intl.Segmenter
(`getGraphemeSegmenter`). The "interpret" half of the
tokenize→interpret split.

#### `src/ink/termio/tokenize.ts`
**Role:** State-machine tokenizer (states: `ground / escape /
escapeIntermediate / csi / ss3 / osc / dcs / apc`). `feed(input)` returns
`Token[]` of `{type: 'text' | 'sequence', value}`. Used by both the
Parser (interpretation) and `parse-keypress` (which only needs sequence
boundaries). The "boundary detection" half of the split.


### §2.7 Event system (`src/ink/events/`)

DOM-style event propagation — capture phase, target, bubble phase,
`stopPropagation` / `preventDefault` / `stopImmediatePropagation`.

#### `src/ink/events/event.ts`
**Role:** Base `Event` class — minimal,
`stopImmediatePropagation()` /  `didStopImmediatePropagation()`. Older
events (`InputEvent`, `TerminalFocusEvent`) extend this directly;
newer DOM-style ones extend `TerminalEvent` (which extends `Event`).

#### `src/ink/events/terminal-event.ts`
**Role:** Browser-Event-shaped base: `type`, `timeStamp`, `bubbles`,
`cancelable`, `target` / `currentTarget` (`EventTarget`), `eventPhase`
(`'none' | 'capturing' | 'at_target' | 'bubbling'`),
`stopPropagation()`, `preventDefault()`. The shared ancestor for
`KeyboardEvent` and `FocusEvent`.

#### `src/ink/events/keyboard-event.ts`
**Role:** `KeyboardEvent` extends `TerminalEvent`. `key` is a literal
char for printable keys ('a', '3', ' ') or a multi-char name for special
keys ('down', 'return', 'escape', 'f1'). Modifier flags
(`ctrl, shift, meta, superKey, fn`). Idiomatic check:
`e.key.length === 1` for printable. Bubbles, cancelable.
`Key` shape: a flag-bag derived from `ParsedKey`.

#### `src/ink/events/focus-event.ts`
**Role:** `FocusEvent` extends `TerminalEvent` — bubbles (matching
react-dom focusin/focusout). `relatedTarget` carries the
previously/newly-focused node. Type is `'focus'` or `'blur'`.

#### `src/ink/events/click-event.ts`
**Role:** `ClickEvent` extends Event (older shape). Carries `col`, `row`
(absolute screen coords), `localCol`, `localRow` (recomputed per handler
during bubble: `col - box.x`, `row - box.y`), `cellIsBlank` (true when
the clicked cell has no visible content — handlers can ignore accidental
clicks on empty space).

#### `src/ink/events/input-event.ts`
**Role:** `InputEvent` — emitted on stdin keypress/paste. Carries the
parsed `Key` flag-bag plus the raw input string. The `useInput` hook
listens for these via `StdinContext.internal_eventEmitter`.

#### `src/ink/events/terminal-focus-event.ts`
**Role:** `TerminalFocusEvent` — `'terminalfocus'` / `'terminalblur'`,
fired when the **terminal window** (not a component) gains/loses focus
via DECSET 1004 (`CSI I` / `CSI O`). Distinct from FocusEvent which is
intra-app.

#### `src/ink/events/dispatcher.ts`
**Role:** The central `Dispatcher`. Walks the tree from root → target
collecting capture handlers, then target → root collecting bubble
handlers. Uses `react-reconciler` priority constants
(`DiscreteEventPriority`, `ContinuousEventPriority`,
`DefaultEventPriority`) to tag dispatches, so React batches updates
correctly. Reads handlers from `node._eventHandlers` populated by
reconciler when host props are set.

#### `src/ink/events/emitter.ts`
**Role:** `EventEmitter extends NodeEventEmitter` — overrides `emit` to
respect `stopImmediatePropagation()` on `Event` instances. Sets
`maxListeners(0)` (unbounded) because many `useInput` consumers can
legitimately listen on the same emitter.

#### `src/ink/events/event-handlers.ts`
**Role:** `EventHandlerProps` type — every host-component event-handler
prop name (`onKeyDown`, `onKeyDownCapture`, `onFocus`, `onFocusCapture`,
`onBlur`, `onBlurCapture`, `onPaste`, `onResize`, `onClick`,
`onMouseEnter`, `onMouseLeave`, etc.). Plus
`HANDLER_FOR_EVENT[eventType]: {bubble, capture}` — the lookup the
dispatcher uses to find the prop name for an event type.


### §2.8 Hooks (`src/ink/hooks/`)

#### `src/ink/hooks/use-app.ts`
**Role:** `useApp() = useContext(AppContext)` — exposes `exit(error?)`.

#### `src/ink/hooks/use-stdin.ts`
**Role:** `useStdin() = useContext(StdinContext)`.

#### `src/ink/hooks/use-input.ts`
**Role:** Subscribes to `internal_eventEmitter`'s `'input'` events.
Callback `(input, key: Key, event: InputEvent)`. `isActive` flag avoids
duplicate handling when multiple `useInput` calls are mounted.
Application-level keybindings ride on top of this.

#### `src/ink/hooks/use-interval.ts`
**Role:** Two helpers: `useAnimationTimer(intervalMs)` — non-keepAlive
clock subscription, returns elapsed ms; updates whenever the shared
clock ticks (which only runs when at least one keepAlive subscriber
exists). Pure time-based animation driver (shimmer position, frame
index).

#### `src/ink/hooks/use-animation-frame.ts`
**Role:** `useAnimationFrame(intervalMs | null)` → `[ref, time]`. Ref
attaches to the animated element; the hook only ticks while that
element is within the terminal viewport (uses `useTerminalViewport`).
KeepAlive subscription — drives the clock to run. Pause via `null`;
time freezes and resumes from the new clock-time on next number.

#### `src/ink/hooks/use-declared-cursor.ts`
**Role:** `useDeclaredCursor({line, column, active})` returns a ref
callback. Sets a `CursorDeclaration` in `CursorDeclarationContext` so
the post-render cursor lands at the input caret — IME preedit text and
screen readers / magnifiers track the native cursor. Layout-effect
timing so it's read on the first frame (no one-keystroke lag).

#### `src/ink/hooks/use-search-highlight.ts`
**Role:** `setQuery / scanElement / setPositions`. `setQuery` paints
inverse on all visible matches; `scanElement` re-renders an existing
DOM subtree to a fresh Screen at its natural height to compute
match positions (zero context duplication); `setPositions` overlays
yellow on the current match — `currentIdx + rowOffset` (rowOffset
tracks scroll, positions stay message-relative).

#### `src/ink/hooks/use-selection.ts`
**Role:** `useSelection()` exposes `copySelection`,
`copySelectionNoClear` (copy-on-select), `clearSelection`,
`hasSelection`, `getState`, `subscribe`, `shiftAnchor`,
`shiftSelection`. Reads from the Ink instance via
`StdinContext.internal_querier`. No-op when fullscreen mode is off.

#### `src/ink/hooks/use-terminal-focus.ts`
**Role:** `useTerminalFocus(): boolean` — reads
`TerminalFocusContext.isTerminalFocused` (DECSET 1004). Returns true on
'unknown' (best-effort).

#### `src/ink/hooks/use-terminal-title.ts`
**Role:** `useTerminalTitle(title | null)` — strips ANSI, writes
`OSC.SET_TITLE_AND_ICON` (or `process.title` on Win32). null = opt-out.

#### `src/ink/hooks/use-terminal-viewport.ts`
**Role:** `useTerminalViewport()` → `[ref, {isVisible}]`. Layout-effect
visibility, no re-render on visibility change (callers already
re-rendering via animation tick or state pick up the latest value
naturally — avoids infinite loops with other layout effects).

#### `src/ink/hooks/use-tab-status.ts`
**Role:** OSC 21337 tab status. Three presets — idle (green
0,215,95), busy, waiting — with indicator color, status text, status
color. `wrapForMultiplexer` so tmux passthrough works. Exposes a
setter the app can call when busy/waiting state changes.


### §2.9 Terminal capability detection / IO

#### `src/ink/terminal.ts`
**Role:** Detection knobs: `setXtversionName`, `isXtermJs`,
`supportsExtendedKeys`, OSC 9;4 progress (`isProgressReportingAvailable`,
`Progress` shape — state is `'running' | 'completed' | 'error' |
'indeterminate'`, plus optional percentage). Supported terminals for
progress: ConEmu (Win), Ghostty 1.2.0+, iTerm2 3.6.6+. Windows Terminal
interprets OSC 9;4 as notifications (not progress) — explicitly excluded.

#### `src/ink/terminal-querier.ts`
**Role:** `TerminalQuerier` — tracks outstanding queries (DECRQM, DA1,
OSC 11, etc.). Each batch is terminated by a DA1 sentinel (`CSI c`):
every terminal since VT100 responds to DA1, so if your DECRQM response
arrives before DA1's, the terminal supports the feature; if DA1 arrives
first, it doesn't — no timeouts needed. `TerminalQuery<T>`:
`{outbound: string, matcher: (response) => T | undefined}`. Built via
`decrqm()`, `oscColor()`, `kittyKeyboard()` factories.

#### `src/ink/terminal-focus-state.ts`
**Role:** Non-React signal for DECSET 1004 focus state. Tri-state:
`'focused' | 'blurred' | 'unknown'`. Subscribers via
`subscribeTerminalFocus`; getters `getTerminalFocused` /
`getTerminalFocusState`. `TerminalFocusProvider` uses
`useSyncExternalStore` to bridge into React.

#### `src/ink/parse-keypress.ts`
**Role:** Keypress parser — converts terminal input bytes into
`ParsedKey` (`{name, ctrl, shift, meta, option, super, fn, sequence}`)
or `ParsedMouse` (mouse events via SGR 1006) or `TerminalResponse`
(query responses recognized as such). Handles: kitty keyboard
(`CSI codepoint [;mod] u`), xterm modifyOtherKeys
(`CSI 27;mod;keycode ~`), legacy single-char ESC sequences,
xterm function keys, paste markers (`CSI 200~` / `CSI 201~`),
focus markers (`CSI I` / `CSI O`), DA1 / DECRPM responses. Uses the
termio tokenizer for boundary detection.

#### `src/ink/clearTerminal.ts`
**Role:** `getClearTerminalSequence()` — picks the right escape sequence
for the current terminal (modern: `ERASE_SCREEN` + `ERASE_SCROLLBACK`;
legacy Windows conhost: HVP cursor home), with detection for Windows
Terminal (`WT_SESSION`), mintty (`TERM_PROGRAM=mintty` or MSYS2/MinGW
via `MSYSTEM`).

#### `src/ink/supports-hyperlinks.ts`
**Role:** Wraps the `supports-hyperlinks` library + an
`ADDITIONAL_HYPERLINK_TERMINALS` allowlist (ghostty, Hyper, kitty,
alacritty, iTerm.app, iTerm2) checked against both `TERM_PROGRAM` and
`LC_TERMINAL` (preserved through tmux). Returns boolean for OSC 8
support.


### §2.10 Misc utilities

#### `src/ink/colorize.ts`
**Role:** chalk wrapper. **Critical patches**:
`boostChalkLevelForXtermJs` — bumps chalk.level from 2 to 3 when
`TERM_PROGRAM === 'vscode'` (xterm.js / VS Code / Cursor / code-server
support truecolor since 2017 but often don't set
`COLORTERM=truecolor`; chalk's supports-color falls through to -256color
regex → level 2; at level 2 chalk.rgb() downgrades to 6×6×6 cube — Claude
orange becomes washed-out salmon). Tmux truecolor clamp also lives here.
Exports `applyColor`, `applyTextStyles` — the actual SGR-sequence
emitters used by render-border and render-node-to-output.

#### `src/ink/styles.ts`
**Role:** `Color` (RGBColor / HexColor / Ansi256Color / AnsiColor),
`TextStyles` (color, backgroundColor, italic, underline, etc.),
`Styles` (full surface — flexbox + spacing + border + dimensions +
position + overflow + textWrap). `applyStyles(node, styles)` walks the
declarative style object onto a Yoga node. The single source of truth
for what props `Box` accepts.

#### `src/ink/Ansi.tsx`
**Role:** Top-level component (note: not in `components/` — it sits
beside `ink.tsx`). Parses an ANSI-escaped string at render time via
`Parser`, emits one `<Text>` per style span with appropriate `color`,
`bold`, `italic`, etc. Memoized so re-renders with the same string
reuse the parsed segments. Used as an escape hatch for pre-formatted
ANSI from tools like `cli-highlight`.

#### `src/ink/focus.ts`
**Role:** `FocusManager` — DOM-like; pure state, no tree refs (callers
pass root for tree walks). Tracks `activeElement`, an internal focus
stack (cap 32), enabled flag. Methods: `focus(node)`, `blur()`,
`pushFocus()`, `popFocus()`. Stored on the root DOMElement so any node
reaches it via `parentNode` (cf. browser's `node.ownerDocument`).
`getFocusManager` / `getRootNode` helpers.

#### `src/ink/get-max-width.ts`
**Role:** `getMaxWidth(yogaNode)` = computedWidth − padding − border.
Documented quirk: can return wider than the parent under
column-direction `align-items: stretch` (Yoga's two-pass measurement —
AtMost gives wide, Exactly gives narrow). Callers should clamp to
actual screen width.

#### `src/ink/instances.ts`
**Role:** `Map<NodeJS.WriteStream, Ink>` — one Ink instance per output
stream. Lets repeat `render()` calls hit the same instance. In a
separate file so `ink.ts` (which creates) and the unmount path (which
deletes) don't import each other.

#### `src/ink/constants.ts`
**Role:** `FRAME_INTERVAL_MS = 16` — shared throttle / animation
target (~60fps). One line.

#### `src/ink/warn.ts`
**Role:** `ifNotInteger(value, name)` — a sanity guard that logs at
'warn' level if a numeric prop is non-integer. Used by `Box.tsx` for
flex/padding/etc. Tiny.

#### `src/ink/useTerminalNotification.ts`
**Role:** `TerminalWriteContext` (`createContext<WriteRaw | null>`),
`TerminalWriteProvider`, `useTerminalNotification()` hook returning
`{notifyITerm2, notifyKitty, notifyGhostty, notifyBell, progress}`.
The `progress(state, percentage?)` emits OSC 9;4. The bell uses BEL.
Per-terminal notify uses iTerm2's OSC 9, kitty's OSC 99 with id, ghostty's
OSC 777.


## §3 Theming model

Claude Code's "theme" is **not** in `src/ink/`. Ink primitives only know
about RAW colors — `Color = RGBColor | HexColor | Ansi256Color | AnsiColor`
in `styles.ts`. Components pick which color they want from a theme-token
layer that lives outside this directory: callers import a theme module
(typically under `src/components/` or `src/utils/`) which resolves a
semantic token (e.g. "claude orange") to a concrete `rgb(215,119,87)` —
which is then handed to `Box` / `Text` as the `color` / `borderColor` /
`backgroundColor` prop.

The crucial primitive support for theming lives in **`colorize.ts`**:

- **xterm.js boost.** If `TERM_PROGRAM === 'vscode'` and chalk.level === 2,
  it bumps to 3. Without this, every `rgb(R,G,B)` color call would
  downgrade to the nearest 6×6×6 cube color (washed-out salmon) on
  VS Code / Cursor / code-server / Coder containers — because xterm.js
  supports truecolor since 2017 but often doesn't set `COLORTERM`. This
  patch must run before the tmux clamp (tmux's passthrough limitation
  wins inside it).
- **tmux clamp.** Tmux parses truecolor SGR into its cell buffer correctly
  but only re-emits truecolor to the outer terminal under specific
  config — when not configured, colorize clamps to nearest 256-color so
  the user sees consistent output instead of incidental color drift.

Dark/light is not a primitive concern either — `terminalSizeContext` /
`terminalFocusContext` carry no light/dark signal; the app reads
terminal background color via `OSC 11` query through the
`TerminalQuerier` and exposes a token map elsewhere. `colorize.ts` is
the sole gate between semantic color → SGR bytes.

So the architecture is: `theme tokens (app)` → `Color (styles.ts)` →
`applyColor (colorize.ts → chalk → SGR)` → `output buffer (output.ts)` →
`log-update diff → stdout`.


## §4 Animation primitives

There is **no `Spinner` component in `src/ink/`** — the spinner family
lives in `src/components/Spinner/` (covered by spec 37 / 37a). What
`src/ink/` provides is the animation **infrastructure**:

1. **`ClockContext` / `createClock(tickIntervalMs)`** — one shared
   monotonic clock per Ink instance. `subscribe(onChange, keepAlive)`:
   keepAlive subscribers force the timer to run; non-keepAlive
   subscribers only get notified when someone else is keeping it alive.
   `tickTime` is captured once per tick so all subscribers in the same
   tick see identical time (animations stay phase-locked).

2. **`useAnimationFrame(intervalMs | null)`** —  keepAlive subscription
   that auto-pauses when the attached element scrolls off-viewport
   (via `useTerminalViewport`). Pass `null` to fully unsubscribe; time
   freezes, then resumes from the new clock time.

3. **`useAnimationTimer(intervalMs)`** — non-keepAlive driver. Use for
   pure time-based math (frame index, shimmer position) when the spinner
   itself isn't responsible for keeping the clock alive (something else
   is — e.g. a sibling already running an animation).

4. **`FRAME_INTERVAL_MS = 16`** — the universal ~60fps target. The clock
   default and the render throttle agree on this number.

5. **Auto-throttling on terminal blur.** `ClockContext` reads
   `useTerminalFocus()` and slows ticks while the terminal is blurred —
   consumers don't need to handle focus state. `terminalFocusState
   === 'unknown'` is treated as 'focused' so we don't penalize terminals
   that don't report focus.

6. **Auto-throttling under scroll.** `bootstrap/state.js#markScrollActivity`
   is fired by `ScrollBox.tsx`; spinners check this signal to skip ticks
   during user scrolling, so the transcript stays smooth.

The actual SHIMMERED / FLASHING / GLIMMER spinners (`FlashingChar`,
`GlimmerMessage`, `ShimmerChar`, `SpinnerAnimationRow`,
`useShimmerAnimation`, `useStalledAnimation`, `TeammateSpinnerLine`) all
sit in `src/components/Spinner/` and call `useAnimationFrame` /
`useAnimationTimer`. None of them are part of this catalog.


## §5 Cross-spec edges

- **Spec 37 (ink-ui-shell)** is the consumer. Its top-level `<App>` shell
  mounts the React tree using `Ink.render` (renderer.ts), and every
  component inside it imports `Box` / `Text` / `Button` / `ScrollBox`
  / `useInput` / `useAnimationFrame` from this catalog. The 372 files
  routed to "37-ink-ui-shell" in PHASE9-COVERAGE are the consumers; the
  69 files here are the primitives those components are built on.

- **Spec 37a (top-level components catalog)** — its design-system
  primitives (`ThemedBox`, `ThemedText`, `Pane`, `Divider`, `Dialog`,
  `Tabs`, `LoadingState`, `ProgressBar`) are thin wrappers around `Box`
  / `Text` here, adding theme tokens. Its message components
  (`AssistantTextMessage`, `UserTextMessage`, the `messages/` family)
  use `<Text>` and `<Markdown>` (which uses `<Ansi>` from this spec).

- **Spec 37b (commands UI)** — wizards / dialogs use `Button`,
  `ScrollBox`, `useInput`, `useTerminalFocus`. Plugin-management screens
  rely on ScrollBox's imperative handle (`scrollToElement`) for
  jump-to-search-result behavior.

- **Spec 38 (output styles / theming)** — extends the theme token
  layer above `styles.ts`. Doesn't modify primitives; only adds new
  semantic-token resolutions.

- **Spec 39 (vim mode) / 41 (session/state/history)** — both consume
  `useInput` + the keypress parser's modifier flags. Vim mode maps `Key`
  predicates to vim semantics; history-search uses
  `useSearchHighlight` for in-transcript find.

- **Spec 04 (turn pipeline)** is decoupled from this layer — it streams
  message events that the UI consumes; no direct ink imports.

- **Native dependency surface.** `layout/yoga.ts` →
  `src/native-ts/yoga-layout` (TS Yoga port, no .node binary). `Bun.*`
  fast paths in `stringWidth.ts` / `wrapAnsi.ts`. `react-reconciler` for
  the host-config seam. `chalk` / `cli-boxes` / `bidi-js` /
  `code-excerpt` / `stack-utils` / `supports-hyperlinks` /
  `emoji-regex` / `get-east-asian-width` / `wrap-ansi` /
  `@alcalzone/ansi-tokenize` are the npm dependencies that
  participate. Everything else is in-house.

- **Bundling discipline.** None of the files in this directory are
  feature-flag gated (per `bun:bundle` `feature(...)`). They're the
  bedrock — always present, regardless of `KAIROS` / `BRIDGE_MODE` /
  etc. The `USER_TYPE === 'ant'` gate doesn't appear here either; this
  is shared infrastructure.
