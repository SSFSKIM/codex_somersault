# Phase 9.5b Adversarial Review — Spec 37 (Ink UI Shell)

**Reviewer**: Skeptic · **Date**: 2026-05-10 · **Scope**: architectural skeleton (37); enumeration owned by 37a/b/c (not re-verified).

## Severity Counts

- **CRITICAL**: 0
- **HIGH**: 2
- **MEDIUM**: 3
- **LOW**: 2
- **NIT**: 1

Verdict: **PASS WITH FIXES** — spec 37's architectural claims are largely correct and well-cited, but it omits two compile/runtime-level facts that the Phase 9.5 catalog companions surfaced (React Compiler runtime, chalk truecolor patch) and contains one stale section header ("the LogoV2/welcome screen, ... InitV2/Splash" implied by review brief — actually only `LogoV2/WelcomeV2` is mentioned, no InitV2/Splash claim found). Source-line claims that were spot-checked all match.

---

## Top 5 Findings

### 1. [HIGH] React Compiler runtime not mentioned in spec 37

Spec 37a (catalog) flagged that compiled output uses `react/compiler-runtime`. Spec 37 itself never mentions React Compiler — but the runtime is unambiguously present:

```
/Users/new/Downloads/claude-code-main/src/components/Spinner.tsx:1
  import { c as _c } from "react/compiler-runtime";
/Users/new/Downloads/claude-code-main/src/components/Spinner.tsx:316-317
  function BriefSpinner(t0) {
    const $ = _c(31);
/Users/new/Downloads/claude-code-main/src/screens/REPL.tsx:1
  (compiler-runtime import)
```

Re-implementers reading spec 37 in isolation would not learn that the leaked tree is **post-compile output** with memo-cache slots (`_c(N)`) — they could mistake `t0` parameter naming and `$` cache arrays for hand-authored idioms. The architectural section §4.2 (render lifecycle) should say "all components in this tree are React-Compiler-emitted; props arrive as a single `t0` object that is destructured locally."

**Fix**: add a §4.x note "Build pipeline: React Compiler" citing `src/components/Spinner.tsx:1` and `src/hooks/useTypeahead.tsx:1` (compiler-runtime import).

### 2. [HIGH] Chalk truecolor VSCode patch (`ink/colorize.ts`) not in spec 37

Spec 37 §6.9 mentions `chalk.level=2 for Apple Terminal asciichart` (`utils/theme.ts:617-620`) but never cites the more impactful patch in `ink/colorize.ts:21-26`:

```
if (process.env.TERM_PROGRAM === 'vscode' && chalk.level === 2) {
  chalk.level = 3
}
```

…paired with the tmux clamp `if (process.env.TMUX && chalk.level > 2) chalk.level = 2` (`colorize.ts:52-54`). The brief's "spec 37c finding: chalk truecolor patch for VSCode (TERM_PROGRAM check)" is verified present in source but **absent from spec 37**. Spec 37 lists `colorize.ts` only as "grep-inspected" with no detail.

**Fix**: §7 (Side Effects) should add a bullet: "Process-wide chalk level mutations: VSCode/xterm.js boost (level 2→3), tmux clamp (>2→2). Order matters: VSCode first, tmux second." Both are global side-effects executed at module import time.

### 3. [MEDIUM] Phase 9.5 catalog companions referenced loosely

The brief says the spec must "verify it references [37a/b/c] correctly". Spec 37 frontmatter does not link to `37a-components-catalog.md`, `37b-hooks-catalog.md`, or `37c-ink-primitives-catalog.md` even though all three files exist (`docs/specs/37[abc]-*.md`). The "Adjacent (do not redocument)" header at line 4 lists numbered specs (09, 16, 34, 35, 36, 38, 39, 41) but never points readers to the companions for enumeration. Given that 37 explicitly delegates "~140-component widget library" enumeration to elsewhere, omitting the cross-link is a navigability bug.

**Fix**: header line 5 should include "Catalog companions: 37a (components), 37b (hooks), 37c (ink primitives)".

### 4. [MEDIUM] `useTypeahead.tsx` and `useReplBridge.tsx` missing from spec 37

Brief asks: "useTypeahead.tsx 213KB / useReplBridge.tsx 115KB — mentioned in spec 37?" Verified file sizes: `useTypeahead.tsx` = 207KB / 1384 LOC, `useReplBridge.tsx` = 113KB / 722 LOC (brief's "213KB/115KB" off by ~3% but order-of-magnitude correct). Neither file is referenced anywhere in spec 37. Both are co-located with `src/hooks/` (NOT `src/ink/hooks/`) so they belong to the feature layer the spec claims to cover ("the entire `src/components/` tree"). At 207KB, `useTypeahead` is one of the largest single files in `src/` and is the engine behind the prompt input's slash-command/file-mention/history typeahead — directly in scope of §6.8 PromptInputFooter.

Spec 37 acknowledges in §12.2 that PromptInput-pipeline details are "listed only" but does not name these two hooks. Catalog 37b reportedly enumerates them (per brief).

**Fix**: §2.4 source-gap notes should add "`hooks/useTypeahead.tsx` (1384 LOC) and `hooks/useReplBridge.tsx` (722 LOC) — enumeration owned by 37b; the architectural shell here does not redocument."

### 5. [MEDIUM] Screen registry undersized vs. brief assumption

Brief implies a screen registry of "REPL, InitV2, Splash, etc." — actual `src/screens/` contents are exactly **three files**: `REPL.tsx`, `Doctor.tsx`, `ResumeConversation.tsx`. There is no `InitV2`, no `Splash`, no `Onboarding` screen at this path (Onboarding is a `src/components/Onboarding.tsx`). Spec 37 §1 correctly says "the screen surfaces (`src/screens/`)" without enumerating, and §2.1 lists `Doctor.tsx (574)` and `ResumeConversation.tsx (398)`. So spec 37 is **correct**; the brief's premise was wrong. Worth flagging because it reflects a possible mental-model gap in Phase 9.5 review scope: there is no formal "screen registry" — `screens/` is just the directory holding the three top-level full-app components, and most full-screen UIs live in `src/components/` (e.g., `LogoV2/WelcomeV2.tsx`, `Onboarding.tsx`).

**Fix (NIT on spec 37)**: §1 could add "Note: `src/screens/` contains only three files; most full-screen UIs are under `src/components/<dir>/` (Onboarding, LogoV2, BridgeDialog, etc.). There is no central screen registry."

---

## Other Findings (terse)

- **[LOW]** §6.9 cites `frame onFrame quarter-interval ~250 fps setTimeout floor` at `ink/ink.tsx:755`. Verified: `ink.tsx:758` does `setTimeout(() => this.onRender(), FRAME_INTERVAL_MS >> 2)` — comment mismatch (line off by 3) but math correct (16 >> 2 = 4ms ≈ 250fps).
- **[LOW]** `ink/constants.ts` claimed "3 lines"; actual is 2 lines + comment (file is `// Shared frame interval...\nexport const FRAME_INTERVAL_MS = 16` plus trailing newline). Trivial off-by-one.
- **[NIT]** `ink/ink.tsx` line count: spec says "1722 lines", verified 1722. Match.

---

## Cross-Spec Impact

- **Spec 38 (output styles)**: spec 37 correctly defers `STREAMLINED_OUTPUT` print-mode delta. No change.
- **Spec 36 (voice STT)**: spec 37 owns `VoiceIndicator`/`VoiceModeNotice` rendering; 36 owns engine. Boundary clean.
- **Spec 09 (permissions)**: spec 37 cites `PermissionRequest.permissionComponentForTool` switch at `:47-82` — verified consistent with §5.2 enumeration.
- **Spec 39 (vim/keybindings)**: `MESSAGE_ACTIONS` flag dual-citation between 37 and 39 is correctly disambiguated.
- **Catalog companions 37a/b/c**: missing cross-link is the only structural gap (Finding #3).

---

## Hardest-to-Verify Claim

§5.1 `showSetupScreens` pseudocode (60+ lines, branch-order-preserving). The spec claims this evaluates "in that order" through ten distinct setup gates. The file `interactiveHelpers.tsx` is 365 lines (verified) and includes lazy `await import()` calls inside the flow — verifying branch order would require reading `:104-298` line-by-line and confirming each gate's `if` boundary. Within the 20-read budget I confirmed file size and the §3.2 export signatures, but the §5.1 ordering is taken on faith from grep+spot-checks. A future deep-read pass should diff the spec's pseudocode against the actual file to catch any reordering or missing `if/else` branches (especially the `feature('TRANSCRIPT_CLASSIFIER')` + `feature('KAIROS')` block at `:224-288`).

---

## Bottom Line

Spec 37 is **architecturally sound**: source line citations spot-checked all match, signature-surface §3.1 matches `ink.ts:18-85`, theme-key inventory matches `theme.ts:4-89`, and the ANT-only DCE-marker explanation (`"external" === 'ant'`) is correctly interpreted. Two omissions (React Compiler runtime, chalk-level mutations in `colorize.ts`) and one navigability gap (no link to 37a/b/c companions) are the actionable fixes. No CRITICAL or correctness-breaking issues found.
