# Phase 9.6c Fixes — Spec 32 (Mode: Kairos)

Sourced from `docs/specs/PHASE9-ADVERSARIAL-32.md`. Verify-before-edit performed for every change.

## HIGH — Pattern A2 phantom missing-source: RemoteTriggerTool/

**Verification.** `ls src/tools/RemoteTriggerTool/` returns:
- `RemoteTriggerTool.ts`
- `UI.tsx`
- `prompt.ts`

The tool surface is fully present in the leaked tree. The `AGENT_TRIGGERS_REMOTE` flag gates the `require()`, NOT the source files.

This is the same Pattern A2 phantom-row defect documented in:
- spec 19 — ScheduleCronTool (resolved Phase 9.6)
- spec 00 §2.5 — Pattern A2 catalog entry

**Edits applied to `docs/specs/32-mode-kairos.md`:**

1. §1.4 row `AGENT_TRIGGERS_REMOTE`:
   - Status: `partial` → `present`
   - Owned-source column extended with `src/tools/RemoteTriggerTool/{RemoteTriggerTool.ts,UI.tsx,prompt.ts}`
   - Annotation: "Phase 9.6c: tool source verified present — was phantom missing-source row"

2. §2.5 missing-source ledger table: row `src/tools/RemoteTriggerTool/RemoteTriggerTool.js` REMOVED.

3. §12 item 10: original "RemoteTriggerTool/ — remote scheduled trigger" replaced with strikethrough + RESOLVED Phase 9.6c marker citing the three present source files. Cross-ref to spec 19 retained for surface ownership.

## MEDIUM — §3.3 description field source verified

Adversarial concern: spec §5.3 pseudocode wrote `description: tool.description(...)` without callsite citation.

**Verification:**
- `src/services/mcp/channelPermissions.ts:36` declares `description: string` in `ChannelPermissionRequestParams`.
- `src/hooks/toolPermission/handlers/interactiveHandler.ts:9` imports `CHANNEL_PERMISSION_REQUEST_METHOD`.
- `:250` materializes the `description` string (per-callsite, not direct `tool.description()` invocation).
- `:336-338` constructs the params: `tool_name: ctx.tool.name, description, input_preview: truncateForPreview(displayInput)`.
- `:345` sends the notification: `method: CHANNEL_PERMISSION_REQUEST_METHOD`.
- `:350` logs failure path.

**Edit applied:** §3.3 schema annotated with inline reference to interactiveHandler.ts:250,337; one-line outbound-callsite paragraph added below the schema block citing `interactiveHandler.ts:345` and the param-build sites.

## MEDIUM — workerType cross-spec connection added (Phase 9.5 spec 34 ripple)

**Verification:**
- `src/bridge/initReplBridge.ts:476-484` — `workerType: BridgeWorkerType = 'claude_code'`; if `feature('KAIROS')` then lazy-`require('../assistant/index.js')`, call `isAssistantMode()`, and on true set `workerType = 'claude_code_assistant'`. Bit-exact match to Phase 9.5 spec 34 finding.
- `src/bridge/types.ts:79` — `export type BridgeWorkerType = 'claude_code' | 'claude_code_assistant'`. Union confirmed.

**Edits applied:**
- §1.2 ("In scope") — appended cross-cite line to spec 34 with file:line citations.
- §5.1 — boot-time activation pseudocode followed by NOTE block: same KAIROS+isAssistantMode predicate flips bridge workerType; full citations included.

## LOW — `commands.ts:324-325` line citation cleanup

**Verification:** `commands.ts:324` is `...(briefCommand ? [briefCommand] : [])` and `:325` is `...(assistantCommand ? [assistantCommand] : [])`. The spec previously listed only `:324`.

**Edit applied:** §2.2 line "src/commands.ts:324" → "src/commands.ts:324-325" with both pushes documented.

## LOW — `tengu_kairos` GB unfalsifiability noted

**Status:** Already correctly footnoted in §6.5 ("(comment)" annotation at `main.tsx:1034`). The actual call site is in missing-source `assistant/gate.js:isKairosEnabled()` (per §12 item 2). Phase 9.7 §13.2 unfalsifiability already applies; no new edit needed.

## Files modified

- `docs/specs/32-mode-kairos.md` — 5 in-place edits (§1.2 cross-cite, §1.4 row, §2.2 line cite, §2.5 ledger row removed, §3.3 callsite cite, §5.1 NOTE, §12 item 10)
- `docs/specs/PHASE9-FIXES-32.md` — this file

## Pattern catalog impact

Spec 32 §12.10 was the third confirmed Pattern A2 ripple in Phase 9.6:
- spec 19 ScheduleCronTool (Phase 9.6 original)
- spec 32 RemoteTriggerTool (Phase 9.6c — this file)

Spec 00 §2.5 should reflect both as canonical exemplars. No new pattern; Pattern A2 (registry-citation-misread-as-missing-source) catalog entry stands.
