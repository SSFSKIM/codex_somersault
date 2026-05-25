# Phase 10d — Final Cleanup Report

**Status**: complete · **Date**: 2026-05-09 · **Scope**: 40 permissive-audit residuals

After the Phase 10a/b/c enumeration passes, a permissive coverage audit
(basename-only match) found 40 source files whose basenames appeared
nowhere across the spec corpus. They clustered into five thematic groups.
This phase applied minimal catalog edits to existing specs so every
residual basename is mentioned at least once. **No new spec files were
created.**

## Per-cluster summary

### Cluster 1 — `src/skills/bundled/*` (11 files → spec 17)

- **Target spec**: `17-tool-skill.md`
- **New section**: `## 11.5 Bundled skills enumeration (catalog)` (inserted between §11 invariants and §12 open questions)
- **Sample entry**:
  ```
  | `src/skills/bundled/batch.ts` | `batch` | Orchestrates large parallelizable changes via plan mode + Agent worker fan-out (5..30 agents)... |
  ```
- **Files covered**: `batch.ts`, `claudeApi.ts`, `claudeApiContent.ts`, `claudeInChrome.ts`, `loremIpsum.ts`, `remember.ts`, `simplify.ts`, `stuck.ts`, `updateConfig.ts`, `verify.ts`, `verifyContent.ts`.

### Cluster 2 — `src/constants/*` (8 files → spec 42)

- **Target spec**: `42-misc.md`
- **New section**: `## §A Appendix — src/constants/ enumeration (Phase 10 cleanup)` (appended after the self-check)
- **Reasoning**: Spec 42 §5 already states constants belong to consuming specs. The appendix records each file's name + consumer spec to satisfy basename coverage without rehoming constants away from their callers.
- **Sample entry**:
  ```
  | `src/constants/figures.ts` | 37 (Ink UI) | Unicode glyphs: BLACK_CIRCLE, BULLET_OPERATOR, ... |
  ```
- **Files covered**: `cyberRiskInstruction.ts`, `errorIds.ts`, `figures.ts`, `product.ts`, `spinnerVerbs.ts`, `systemPromptSections.ts`, `toolLimits.ts`, `turnCompletionVerbs.ts`.

### Cluster 3 — `src/tools/*` sub-files (8 files → spec 19)

- **Target spec**: `19-tool-misc.md`
- **New section**: `## §13. Tool sub-file catalogs (Phase 10 cleanup)` with three sub-sections (PowerShellTool, BriefTool, ScheduleCronTool); appended above `End of spec.`
- **Sample entry**:
  ```
  | `src/tools/PowerShellTool/clmTypes.ts` | CLM_ALLOWED_TYPES set — Microsoft Constrained Language Mode allowlist... |
  ```
- **Files covered**: `BriefTool/upload.ts`, `PowerShellTool/{clmTypes, commonParameters, gitSafety, powershellPermissions, powershellSecurity}.ts`, `ScheduleCronTool/{CronDeleteTool, CronListTool}.ts`.

### Cluster 4 — `src/utils/*` and friends (7 files → spec 42a)

- **Target spec**: `42a-utils-long-tail.md`
- **New section**: `## §7 Phase 10 cleanup additions` (appended after §6 LOC histogram)
- **Sample entry**:
  ```
  | `src/utils/userAgent.ts` | getClaudeCodeUserAgent() — returns claude-code/${MACRO.VERSION}. Kept dependency-free... |
  ```
- **Files covered**: `activityManager.ts`, `bash/specs/nohup.ts`, `bash/specs/time.ts`, `claudeDesktop.ts`, `dxt/helpers.ts`, `shellConfig.ts`, `userAgent.ts`.

### Cluster 5 — `src/cli/transports/*` (6 files → spec 35)

- **Target spec**: `35-mode-remote-server.md`
- **Reasoning**: Spec 35 §3 already gates on `WebSocketTransport.ts:771` and the transport variants are remote-session machinery (CCR v2, hybrid POST). Spec 22 (service-api) deals with the Anthropic Files / model API, not the session-ingress transport.
- **New section**: `## §13. src/cli/transports/ catalog (Phase 10 cleanup)` appended after §12 (open questions)
- **Sample entry**:
  ```
  | `src/cli/transports/HybridTransport.ts` | WS-for-reads + HTTP-POST-for-writes hybrid. Activated when CLAUDE_CODE_POST_FOR_SESSION_INGRESS_V2 is truthy... |
  ```
- **Files covered**: `HybridTransport.ts`, `SSETransport.ts`, `SerialBatchEventUploader.ts`, `WorkerStateUploader.ts`, `ccrClient.ts`, `transportUtils.ts`.

## Verification

Permissive audit (basename-only match across `docs/specs/*.md` excluding `PHASE9-COVERAGE`/`PHASE10-COVERAGE`) re-run after edits:

```
Permissive residuals: 0
```

Down from 40 to 0. No files left uncategorized.

## Structural changes vs simple appending

- **Spec 17**: structural — inserted a new §11.5 between two existing top-level sections (§11 / §12). The other four edits were pure appends.
- **Spec 19, 42, 42a, 35**: simple appends after existing trailing markers (`End of spec.`, `Self-check`, `§6 LOC histogram`, `§12 open questions`).

## Notes on cluster-assignment validation

The original task brief proposed candidate target specs but asked me to verify by reading the files. Two notes:

- **Constants → spec 42, not spec 00.** Spec 42 §5 already enumerates constants policy ("Strings/regexes belong to consuming specs ... Cite-only here"). Adding the appendix to spec 42 keeps this policy in one place. Spec 00 (overview) is not the right home for a per-file catalog.
- **Transports → spec 35, not spec 22.** Spec 22 (service-api) focuses on the Anthropic API client, retries, beta headers, and Files API. The `src/cli/transports/` files implement remote-session transport selection (WS / SSE / HTTP-POST hybrid) gated by `CLAUDE_CODE_USE_CCR_V2` and `CLAUDE_CODE_POST_FOR_SESSION_INGRESS_V2` — pure remote-session machinery. Spec 35 already cites `WebSocketTransport.ts:771`, so the catalog completes the family there.

## Files that could not be categorized

None. All 40 fit cleanly into existing spec scopes.
