# PHASE9-FIXES-38.md

Fixes applied to `docs/specs/38-output-styles.md` from Phase 9.6c adversarial review (`PHASE9-ADVERSARIAL-38.md`).

Date: 2026-05-10. Agent: general-purpose (a50182f623161bf0f), retry pass after quota reset.

---

## Findings addressed

### HIGH — §5.5 mcp_instructions cross-cut to mcp_instructions_delta (spec 16 + 05)

- **Issue.** Spec §5.5 transcribes the `dynamicSections` list verbatim including `DANGEROUS_uncachedSystemPromptSection('mcp_instructions', …)` but omits the inline routing comment at `prompts.ts:480-481, 509, 516` explaining that when `isMcpInstructionsDeltaEnabled()` is true the live MCP server-instructions text is delivered via `mcp_instructions_delta` *attachments* (attachments.ts) rather than via this prompt slot. A reader of the spec alone could not reconstruct that `mcp_instructions` (in-prompt) and `mcp_instructions_delta` (attachment-side) are two halves of the same feature.
- **Verification.** `grep -n "mcp_instructions_delta\|isMcpInstructionsDeltaEnabled" src/constants/prompts.ts` → matches at `:62, :480, :481, :509, :516`. Confirmed.
- **Fix.** Inlined a comment block immediately after the `mcp_instructions` slot in §5.5's pseudocode, naming the runtime flag, the attachments.ts wiring, and the joint owners (spec 16 MCP, spec 05 assembly). Spec 38 retains only positional ownership of the slot.

### MED — §12 footnote: source comment at outputStyles.ts:158 is wrong

- **Issue.** `outputStyles.ts:158` comment lists priority order as `built-in, plugin, managed, user, project`, but the array literal at `:159` is `[pluginStyles, userStyles, projectStyles, managedStyles]` — last writer wins, so the actual order is built-in < plugin < user < project < **managed**. Spec 38 already transcribed the array correctly but did not flag the comment-vs-code drift.
- **Verification.** Read `src/constants/outputStyles.ts:137-175`. Comment at `:158`: `// Add styles in priority order (lowest to highest): built-in, plugin, managed, user, project`. Array at `:159`: `[pluginStyles, userStyles, projectStyles, managedStyles]`. Confirmed: comment swaps managed and project.
- **Fix (spec 38).** Added a callout under the §5.3 precedence sentence pointing readers to BUGS-IN-SOURCE.md §8.
- **Fix (BUGS-IN-SOURCE.md).** Promoted total from 7 → 8 confirmed; added entry §8 (`getAllOutputStyles` priority-order comment contradicts the array literal, severity `cosmetic`, surfaced Phase 9.6c spec 38 fix agent), with reproduction (managed-overrides-project on a same-named style) and suggested fix (rewrite comment to `built-in, plugin, user, project, managed`).

### MED — Custom-style attachment-vs-reminder asymmetry

- **Issue.** `attachments.ts:1597-1611` queues an `output_style` attachment for any non-default style name (covers `Explanatory`, `Learning`, *and* any custom or plugin-namespaced name). `messages.ts:3796-3810` resolves only names present in `OUTPUT_STYLE_CONFIG` and returns `[]` otherwise — so custom and plugin styles produce the attachment chip in the conversation history but no `<system-reminder>` reminder text. Was previously buried in §12 Open Question 3; the adversarial review asked it be promoted to §9 edge-cases since it is a documented user-visible asymmetry rather than an open question.
- **Verification.** Spec 38 §6.5 already cites `messages.ts:3796-3810` returning `[]` for unknown names. Spec §6.5 already calls this out in passing; lacked an §9 entry.
- **Fix.** Replaced the lone `§9` line on the lookup-miss with an expanded edge-case bullet enumerating the asymmetry: where the attachment is enqueued, where the resolution drops it, what the user sees in each surface, and the two consistent reimplementations (drop at queue OR widen the lookup). Existing §12 #3 entry kept (it discusses the policy choice; §9 now documents the runtime symptom).

### MED — getSessionStartDate ownership clarification

- **Issue.** `prompts.ts:8` imports `getSessionStartDate` and uses it at `prompts.ts:452` inside `getSimpleSystemSection()` (the *static* intro: `"… CWD: …\nDate: ${getSessionStartDate()}"`). It is **not** invoked from `getOutputStyleSection` or any dynamic section that varies with output style. Spec 38 was silent, leaving readers unsure whether session-start date rendering was an output-style concern.
- **Verification.** `grep -n "getSessionStartDate" src/constants/prompts.ts` → only matches at `:8` (import) and `:452` (call inside `getSimpleSystemSection`'s template). Not invoked in the output-style fragment.
- **Fix.** Added a dedicated §12 entry "Not owned here (→ spec 05): `getSessionStartDate()`" disclaiming ownership and crediting spec 05 (system-prompt assembly). Renumbered subsequent entries.

### LOW — Verbatim trailing-whitespace in Learning prompt

- **Issue.** The Learning built-in prompt at `outputStyles.ts:64`, `:71`, `:90` ends with literal trailing spaces (`encouraging.…implementation yourself.   `, `…multiple valid approaches  `, `…Learn by Doing request      `). These survive verbatim into the system prompt, but a stock `trailing-whitespace` pre-commit hook would silently strip them in any reimplementation, producing a byte-different prompt. §11 reimplementation checklist did not warn about this.
- **Verification.** `grep -n " $" src/constants/outputStyles.ts` → matches at `:64`, `:70`, `:90`. (Adversarial brief cited 64/71/90; the `:71` trailing space was actually on `:70` — note in fix log preserved both citations to keep the audit honest.)
- **Fix.** Extended the §11 first checklist item ("Built-in registry shape") with a "Trailing-whitespace caveat" sub-clause naming the lines and the disable-the-lint instruction.

---

## Files modified

- `docs/specs/38-output-styles.md` — five inline edits (§5.5 cross-cut comment, §5.3 footnote, §9 expanded asymmetry bullet, §11 trailing-whitespace caveat, §12 new entry #6 + renumber).
- `docs/specs/BUGS-IN-SOURCE.md` — total bumped (7→8); added confirmed bug §8.

## Files NOT modified

- `src/` (read-only archive — no source patches per repo policy).
- `docs/specs/PHASE9-ADVERSARIAL-38.md` — historical record, untouched.

## Cross-references

- Spec 16 (MCP server connection/management) — joint owner of `mcp_instructions_delta` routing.
- Spec 05 (system-prompt assembly) — joint owner of the `mcp_instructions` slot positioning AND sole owner of `getSessionStartDate` rendering.
- BUGS-IN-SOURCE.md §8 — the new comment-vs-code drift entry.
