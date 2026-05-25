# Phase 9.6c Fixes — Spec 23 (`docs/specs/23-service-mcp.md`)

Applied 5 fixes from `PHASE9-ADVERSARIAL-23.md` (H1, H2, M1, M2, M3). Each was verified against source before editing. No critical findings; spec was technically accurate, fixes are clarifications and cross-spec cross-cites.

## H1 — §4.1 state diagram: disabled? branch annotation

**Source verified:** `src/services/mcp/useManageMCPConnections.ts:346` calls `isMcpServerDisabled(client.name)` after onclose, which reads disk state (not AppState). Comment at `useManageMCPConnections.ts:345` reads `// check the disk state. We may want to refactor some of this.`.

**Fix:** Added `(disk re-read; AppState may be stale — see §5.5)` annotation under the post-onclose `isMcpServerDisabled?` branch in the §4.1 state diagram so reimplementers don't conflate AppState-disabled with disk-disabled.

## H2 — ElicitationDialog.tsx referenced but never named

**Source verified:** `src/components/mcp/ElicitationDialog.tsx` exists, 1168 lines (`wc -l` confirmed). It is the consumer of the §5.6 elicitation-queue contract.

**Fix:** Added a row in §2.1 source-map table for `src/components/mcp/ElicitationDialog.tsx` with explicit cross-cite to spec 37 (UI shell) and a note that modifying the queue payload shape here breaks the dialog.

## M1 — `_NOT_CODE_OR_FILEPATHS` redaction policy defer to spec 09

**Source verified:** marker `AnalyticsMetadata_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS` and `TelemetrySafeError_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS` appear ~25× across `client.ts` and `auth.ts` (sample: `client.ts:177, 1061, 1698, 2702, 2746`; `auth.ts:828, 838, 880, 1245, 1323`). Confirmed `type: 'tools' as AnalyticsMetadata_…` cast at `useManageMCPConnections.ts:638, 651, 675, 713`.

**Fix:** Added an inline blockquote at the top of §10 deferring redaction-policy ownership to spec 09, listing the affected fields and naming the marker class explicitly.

## M2 — `vscodeSdkMcp.ts` mis-classified under spec 25; should be spec 34

**Source verified:** `src/services/mcp/vscodeSdkMcp.ts:44` reads `if (process.env.USER_TYPE !== 'ant' || !vscodeMcpClient) return`. Lines 76–97 fire `tengu_vscode_${eventName}` events and read `tengu_vscode_review_upsell` / `tengu_vscode_onboarding` / `tengu_vscode_cc_auth` Statsig gates. This is IDE-bridge plumbing, not OAuth.

**Fix:**
- Removed `vscodeSdkMcp.ts` from the spec-25 row in §2.1 (officialRegistry.ts/oauthPort.ts/xaa.ts/xaaIdpLogin.ts).
- Added a new dedicated row for `vscodeSdkMcp.ts` in §2.1 with explicit `USER_TYPE === 'ant'` gate, `tengu_vscode_*` event note, and **owned by spec 34** cross-cite.
- Added a second bullet in §8 ANT-only behavior covering `vscodeSdkMcp.ts:44` gate.
- Updated §12 Q4 to explicitly state the file is ANT-only and owned by spec 34, not 25.

## M3 — `MCP_SKILLS` analytics: no separate `type:'skills'` event

**Source verified:** `useManageMCPConnections.ts:638, 651, 675, 713` only ever set `type` to `'tools' | 'prompts' | 'resources'`. Skill cache is invalidated alongside `prompts/list_changed` and `resources/list_changed` (§8 and lines 678–710 of source) but no distinct event fires.

**Fix:** Extended the §10 `tengu_mcp_list_changed` row to explicitly state "**No separate `type:'skills'` event**" with source line cites, so a reader doesn't expect one.

## Verification

- All five edits succeeded (Edit tool returned success on each call).
- Source line cites preserved verbatim.
- No bit-exact constants, schemas, or pseudocode were touched — fixes are annotations, cross-cites, and clarifications.
- Spec line count grows from 1115 to ~1125; structure (§1–§12) preserved.

## Verdict

**Spec 23 patched per all 5 adversarial findings.** Approved-equivalent state. No spec 17/25/28/32/34/37 ripple required beyond the cross-cites already added in this pass.
