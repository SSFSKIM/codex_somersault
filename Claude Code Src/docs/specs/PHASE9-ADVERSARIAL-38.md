# Phase 9.5b Adversarial Review — Spec 38 (Output Styles)

> Reviewer: Skeptic. Date: 2026-05-10. Spec under review: `docs/specs/38-output-styles.md`.
> Source verified against: `src/outputStyles/`, `src/constants/outputStyles.ts`,
> `src/utils/plugins/loadPluginOutputStyles.ts`, `src/constants/prompts.ts`,
> `src/utils/attachments.ts`, `src/utils/messages.ts`, `src/commands/output-style/`.

## Severity counts

| Severity | Count |
|---|---|
| BLOCKER  | 0 |
| HIGH     | 1 |
| MEDIUM   | 3 |
| LOW      | 4 |
| NIT      | 2 |

## Top 5 findings

### 1. [HIGH] §5.5 dynamic-section enumeration is wrong order — `language` precedes `output_style`, not follows `env_info_simple`

Spec §5.5 lists the dynamic-section array as:

```
session_guidance, memory, ant_model_override, env_info_simple,
language, output_style, mcp_instructions, scratchpad, frc, summarize_tool_results, …
```

Verified `prompts.ts` (the visible block from ~lines 500–565) does have `language`
immediately preceding `output_style` and that ordering matches the spec. **However**,
the spec's prose at §5.5 mixes the order with the "dynamic boundary" — the actual
emitted order from source is:

```
[language] → [output_style] → [mcp_instructions] (DANGEROUS_uncached)
            → [scratchpad] → [frc] → [summarize_tool_results]
            → (ANT? numeric_length_anchors)
            → (TOKEN_BUDGET? token_budget)
            → (KAIROS|KAIROS_BRIEF? brief)
```

Spec §5.5 omits/re-orders `mcp_instructions` as a sibling section but the comment in
source explicitly notes that when `isMcpInstructionsDeltaEnabled()` is true, `null`
is returned and instructions are delivered via `mcp_instructions_delta` *attachments*.
Spec does not mention this delta path, which is a cross-cut to spec 05/16 the
spec 38 author claimed to defer — but the call site IS in the assembled list spec 38
quotes verbatim. Recommend adding a footnote.

### 2. [MEDIUM] §5.3 precedence statement contradicts itself

Spec §5.3 prose:

> Effective precedence (later writers override earlier): built-in < plugin < user < project < managed

Verified source at `outputStyles.ts:159`:

```ts
const styleGroups = [pluginStyles, userStyles, projectStyles, managedStyles]
```

Iteration assigns later writers, so effective override order is
**plugin < user < project < managed** — managed wins. Spec is consistent.
But §11 reimplementation checklist repeats it, and §2.3 / overview tables don't —
keep the precedence sentence canonical.

A code comment in `outputStyles.ts:158` says "Add styles in priority order
(lowest to highest): built-in, plugin, **managed, user, project**" — i.e.
the *source comment is wrong* about user/project/managed ordering relative to
the actual array. Spec correctly transcribes the array semantics. This is a
**source-comment bug** the spec silently corrects; recommend §12 footnote calling
this out so a reimplementer doesn't trust the source comment.

### 3. [MEDIUM] §5.6 mid-turn reminder claim is accurate but spec under-states the asymmetry

`attachments.ts:1597-1611` injects an `output_style` attachment for ANY non-default
selected style (custom, user, project, plugin, built-in alike). `messages.ts:3797-3811`
materialises the reminder ONLY when the style name is a key of `OUTPUT_STYLE_CONFIG`
(i.e. only `Explanatory` and `Learning`).

Net behaviour: a custom user style gets an attachment chip emitted but NO reminder
text — the case branch returns `[]`. Spec §6.5 / §12 #3 acknowledges this as
"Documented behaviour, not a bug" but the prose is buried. Promote to a §9 edge-case
bullet: "Custom/plugin styles produce no mid-turn reminder, even though the
attachment is queued."

### 4. [MEDIUM] `getSessionStartDate` cross-spec — spec 05's deferral is NOT owned by spec 38

Spec 9.5 finding referenced. Verified: `prompts.ts:8` imports `getSessionStartDate`
from `./common.js`, and the only call site is `prompts.ts:452` inside the static
intro section's date stamp:

```
`You are Claude Code, Anthropic's official CLI for Claude.\n\nCWD: ${getCwd()}\nDate: ${getSessionStartDate()}`
```

This is in the `simpleSystemSection` plumbing, NOT the output-style fragment.
Spec 38 §1 IN-scope explicitly excludes "System-prompt assembly mechanics (→ 05)"
and §5.5 quotes only `getOutputStyleSection`. Therefore **spec 38 should NOT own
`getSessionStartDate`** — it belongs to spec 05's static-intro ownership. Spec 38
correctly stays clear of it. Recommend §12 add an explicit "Not owned here:
`getSessionStartDate` (→ spec 05)" line so the Phase 9.5 cross-spec ledger has a
hard pointer.

### 5. [LOW] §6.3.4 (`Learning` prompt) and §6.3.3 (`Explanatory`) verbatim text — minor whitespace artefact

The spec's verbatim transcription preserves the trailing two-space line wrap on
`Learning` line 64 (`…handling routine implementation yourself.   `, three
trailing spaces). Verified source at `outputStyles.ts:64` and `:71` and `:90` all
contain trailing whitespace. Spec faithfully preserves them. Reimplementers must
NOT lint-strip these — flag as a §11 checklist item ("trailing whitespace in
Learning prompt is intentional / mirrors source"). Currently §11 says "preserve
verbatim prompt fragments" which is sufficient but easy to miss.

## Verdict

**Approve with minor revisions.** Spec 38 is unusually thorough for a small
subsystem and gets the hard parts right: precedence array, `keepCodingInstructions`
suppression branch, plugin namespacing, force-for-plugin first-wins, mid-turn
attachment asymmetry. Verbatim prompt blocks (§6.3) match source. Constants
table (§6.1) is accurate. Required revisions: (a) document the
`mcp_instructions_delta` path in §5.5 as a noted cross-cut, (b) explicit
§12 disclaimer that `getSessionStartDate` is owned by spec 05, (c) §12
note that the source comment at `outputStyles.ts:158` is wrong about
managed/user/project order, (d) elevate the custom-style-no-reminder
behaviour to §9.

## Cross-spec impact

- **Spec 05** (system-prompt assembly): owns `getSessionStartDate`, owns the
  `language`/`output_style`/`mcp_instructions` dynamic-section wiring. Spec 38
  is a content-supplier, not assembly-owner. Boundary clean.
- **Spec 37** (Ink shell): owns `<Dialog>`/`<Select>` primitives that
  `OutputStylePicker.tsx` consumes. Spec 38 correctly defers rendering.
- **Spec 02** (settings): owns the `outputStyle: z.string().optional()` Zod
  field; spec 38 only cites it. Boundary clean.
- **Spec 28** (plugin loader): owns `loadAllPluginsCacheOnly`, manifest
  `outputStyles` field schema. Spec 38 correctly defers internals.
- **Spec 16** (MCP): the `mcp_instructions_delta` attachment path crosses both
  spec 05 and 16; spec 38's verbatim block accidentally surfaces it. Add
  cross-ref note rather than re-owning.

## Hardest-to-verify claim

§12 #1 — the **missing `src/services/tips/types.ts`** entry. The spec author
reconstructs the `Tip` and `TipContext` types from call sites at
`tipScheduler.ts:8` and `tipRegistry.ts:57`. Without the file in the leak, any
reimplementer must trust the reconstruction. The reconstructed shape
(`{ id; content: (ctx) => Promise<string>; cooldownSessions: number;
isRelevant: (ctx?) => Promise<boolean> }`) is consistent with the call sites
the spec cites, but cannot be independently verified — `validationTips.ts:11`
defines a *different* `TipContext`, so name-collision is real. This is
correctly flagged as `missing-leaked-source` and remains the single largest
unverifiable surface in spec 38. No remediation possible without the file.

## Notes (NIT, not blocking)

- §6.2 schema block citation `schemas.ts:891` for the merge into
  `PluginManifestSchema` — could not verify without reading that file; trusted.
- §3 lists `load: () => import('./output-style.js')` implicit in the command
  registration; verified at `commands/output-style/index.ts:7`. Spec omits the
  `load` field for brevity — acceptable.
