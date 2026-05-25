# Phase 9.5 Adversarial Review — Spec 09 (Permission System)

**Reviewer:** Opus fallback (codex agent died mid-review with auth failure; first finding inherited from codex's partial output and verified independently)
**Spec:** `/Users/new/Downloads/claude-code-main/docs/specs/09-permission-system.md`
**Review depth:** ~25 src files inspected; full spec read; key verbatim quotes checked.

## Severity counts

- **Critical:** 0
- **Major:** 2
- **Minor:** 4

No security holes (denial-claim vs. src) and no race-condition gaps were found. Cross-spec drift with 08 is intact (the §4.6 mirror disclosure holds). The major findings are factual contradictions about `bubble` mode and one stale "Open Question" that the source already answers.

---

## Findings

### F1 — `bubble` IS a runtime permission mode (Major)

**Claim quote, §3 Glossary:**
> "**Permission mode** — A per-session enum gating the decision tree's defaults. Six values defined at type level; runtime-validatable subset excludes `bubble` and conditionally `auto` (see §4.1)."

**Claim quote, §4.1:**
> "Empirically `bubble` is referenced only in the type union — no runtime literal `'bubble'` produces or consumes it visibly in the leaked source. Treat as type-level placeholder for future use; flag in §12 Open Questions."

**Claim quote, §12.1 Open Questions:**
> "The string literal `'bubble'` is referenced only in the type union and in `isExternalPermissionMode`'s exclusion check (`PermissionMode.ts:104`). No code in the leaked tree appears to *return* or *test for* `bubble` at runtime."

**Source verification (contradictions):**

1. `src/tools/AgentTool/forkSubagent.ts:67` — `FORK_AGENT` literally assigns `permissionMode: 'bubble'` as the default for forked subagents (the `BuiltInAgentDefinition` for synthetic fork agents).
2. `src/tools/AgentTool/forkSubagent.ts:50` — comment: *"`permissionMode: 'bubble'` surfaces permission prompts to the parent terminal."* (clearly a runtime contract, not a placeholder).
3. `src/tools/AgentTool/runAgent.ts:443` — runtime branch: `agentPermissionMode === 'bubble' ? false : isAsync` — used to set `shouldAvoidPrompts` (and consequently `shouldAvoidPermissionPrompts` on `ToolPermissionContext`).
4. `src/tools/AgentTool/runAgent.ts:457-463` — `isAsync && !shouldAvoidPrompts` then sets `awaitAutomatedChecksBeforeDialog: true`. Comment says: *"This applies to bubble mode (always)..."*.
5. `src/tools/AgentTool/runAgent.ts:430-433` — `agentPermissionMode` overrides `toolPermissionContext.mode` (unless parent is `bypassPermissions`/`acceptEdits`/`auto`), so `'bubble'` is actually written into `ToolPermissionContext.mode` for forked subagents at runtime.
6. `src/tools/AgentTool/AgentTool.tsx:613` — comment: *"workerTools is rebuilt under permissionMode 'bubble' which differs from the parent's mode..."*.

**Severity:** Major. Codex's partial finding is correct. This is a security-spec contradiction — the spec claims `bubble` cannot reach `ToolPermissionContext.mode` at runtime, but for fork subagents it actually does. The decision tree in §6.1 then runs with `mode === 'bubble'`, which is **not handled** by any branch (steps 2a, 2b, dontAsk check, auto-mode classifier, shouldBypass). The mode quietly degrades to "default-equivalent" because none of the predicates match — but a reader trusting the spec would never know. Also, `INTERNAL_PERMISSION_MODES` does not include `'bubble'`, so any code path that round-trips the mode through `permissionModeFromString()` would coerce it back to `'default'` (security-relevant: a fork's mode could silently change on reload).

**Recommended fix:**
- In §3 and §4.1, replace "type-level placeholder" claim with: *"`bubble` is a runtime permission mode used exclusively by forked subagents (`src/tools/AgentTool/forkSubagent.ts:67`). It surfaces permission prompts to the parent terminal by setting `awaitAutomatedChecksBeforeDialog=true` and `shouldAvoidPermissionPrompts=false` on the child's `ToolPermissionContext`. Excluded from `INTERNAL_PERMISSION_MODES` so users cannot select it via settings/CLI."*
- Add a new row to the §4.1 mode metadata table for `bubble` (note that `PERMISSION_MODE_CONFIG` does not contain a `bubble` entry, so `getModeConfig` falls back to `default` config for title/symbol — that's the actual UI behavior).
- §6.1 decision tree: document that when `mode === 'bubble'`, none of the mode-predicates match, so behavior is the same as `default` for the inner decision tree; the bubble-specific behavior happens at the outer `useCanUseTool` level via `awaitAutomatedChecksBeforeDialog` (§6.3 coordinator path).
- Delete/rewrite §12.1 — this is no longer an open question.

---

### F2 — §12.3 "Open Question" about `isAutoModeAvailable` is contradicted by §4.6 itself (Minor)

**Claim quote, §12.3:**
> "`isAutoModeAvailable` is read by `getNextPermissionMode.ts:21,42` and set by `permissionSetup.ts:987` but is not declared on `ToolPermissionContext` in `types/permissions.ts:427-441`. Either an intentional `any`-typed extension or a TS gap. Consumers should check for the field defensively."

**Source verification:** `src/Tool.ts:130` declares `isAutoModeAvailable?: boolean` on the canonical `ToolPermissionContext`. The spec's own §4.6 already explains this: *"Canonical owner = `src/Tool.ts:123-138`... `isAutoModeAvailable` IS declared on the canonical Tool.ts type (line 130)... The mirror in `types/permissions.ts` omits the field because that file is the no-runtime-deps cycle breaker."*

**Severity:** Minor. Internal contradiction — §4.6 (Phase 9.4 fix) already resolved this, but §12.3 was not updated to match. Confusing for readers.

**Recommended fix:** Delete §12.3 entirely (or rewrite it as a cross-reference to §4.6's resolution).

---

### F3 — `dontAsk` is excluded from Shift+Tab cycle but spec doesn't note CLI/settings entry path clearly (Minor)

**Claim quote, §6.16:**
> "dontAsk → default        # not exposed in UI cycle today"

**Source verification:** `dontAsk` IS in `EXTERNAL_PERMISSION_MODES` (`types/permissions.ts:16-22`), so it can be set via `--permission-mode dontAsk` CLI and `defaultMode: "dontAsk"` in settings.json (Zod accepts it). But the cycle order in `getNextPermissionMode.ts` never targets it — only `transitionPermissionMode` can land on it via `setMode`. Spec is correct but the comment "not exposed in UI cycle today" undersells: the only way users reach `dontAsk` is via settings.json / CLI / SDK `set_permission_mode` control message. Flag because security-conscious readers might assume `dontAsk` requires a special bypass.

**Severity:** Minor. Documentation completeness.

**Recommended fix:** Add to §6.16: *"`dontAsk` is reachable only via `defaultMode: 'dontAsk'` in settings.json, `--permission-mode dontAsk` CLI flag, or SDK `set_permission_mode` control message — never via Shift+Tab. Once entered, the only auto-cycle path out is `dontAsk → default`."*

---

### F4 — §4.1 mode metadata table contains `dontAsk` row but `PERMISSION_MODE_CONFIG` map matches; verify accuracy (Minor — verified clean)

**Claim quote, §4.1 table:**
> `| `dontAsk` | "Don't Ask" | "DontAsk" | `⏵⏵` | `dontAsk` |`

**Source verification:** `src/utils/permissions/PermissionMode.ts:73-79` confirms exactly:
```
dontAsk: { title: "Don't Ask", shortTitle: 'DontAsk', symbol: '⏵⏵', color: 'error', external: 'dontAsk' }
```
**Severity:** N/A — clean.

---

### F5 — §6.1 step 1g lists "shell configs" as bypass-immune but spec defers actual list to §11 (Minor)

**Claim quote, §6.1 step 1g:**
> "# 1g. Safety-check 'ask' (bypass-immune): .git/, .claude/, .vscode/, shell configs"

**Source verification:** Type discriminator `'safetyCheck'` is in `types/permissions.ts:312-320`; the comment on `classifierApprovable` mentions "`.claude/`, `.git/`, shell configs" — matches. Path-safety body deferred to §11. Self-consistent.

**Severity:** Minor — note that "shell configs" is intentionally vague (matches src comment) but reader cannot verify the exact glob without §11.

**Recommended fix:** None required if §11 enumerates them; otherwise add inline list (`.bashrc`, `.zshrc`, `.profile`, `.fishrc` etc.) for self-containment.

---

### F6 — Auto-mode `'plan' && isAutoModeActive()` re-enters classifier in §6.1; verify that prePlanMode preserved (Minor)

**Claim quote, §6.1:**
> "if feature('TRANSCRIPT_CLASSIFIER') && (mode == 'auto' || (mode == 'plan' && isAutoModeActive()))"

**Source verification:** `prePlanMode` field on `ToolPermissionContext` (`types/permissions.ts:441`) records pre-plan-mode for restoration. Without `prePlanMode === 'auto'` check, the spec's logic could spuriously trigger classifier when user enters plan from `default`. `isAutoModeActive()` is described in §12.5 as a "global flag" — confirm via `autoModeState.ts`. Without verifying that flag's setter call graph (deferred to §27/§32 per §12.5), claim is plausible but untestable from this spec alone.

**Severity:** Minor — testability gap, not a bug claim.

**Recommended fix:** Cross-link §12.5 inline at §6.1 step where `isAutoModeActive()` is invoked; consumer of spec needs to know the flag's lifecycle.

---

## Spec-level verdict

**Minor revise.** The spec is fundamentally sound and security claims hold (no fail-open paths discovered, all denial routes verified, decision tree branch order is exact, the §4.6 ToolPermissionContext mirror disclosure correctly resolves the 08-09 seam). The single major issue (F1) is a factual contradiction about `bubble` being type-only — easily corrected by adding two paragraphs and editing §12.1. The minor issues are documentation completeness items.

---

## Cross-spec impact

- **Spec 30..36 (Modes & Coordinator)** / **Spec 35 (AgentTool fork subagent)**: must reference the corrected §4.1 entry for `bubble`. Currently §6.1 of the AgentTool spec likely describes fork subagents but cannot cite spec 09 for `bubble` semantics if 09 says it's type-only.
- **Spec 08 (Tool.checkPermissions)**: §4.6 mirror seam is intact. No drift.
- **Spec 02 (Settings precedence)**: must continue to confirm that `defaultMode: 'bubble'` is rejected by Zod (since `INTERNAL_PERMISSION_MODES` excludes it).
- **Spec 21 (Command catalog) — `/permissions`, `/permission-mode`**: must continue to use `ExternalPermissionMode` only; `setMode` PermissionUpdate is correctly typed to exclude `'bubble'` and `'auto'` (verified `PermissionUpdateSchema.ts:63`).
- **Spec 22 (Hooks/SDK control messages)**: SDK `set_permission_mode` cannot supply `'bubble'` (External-only) — confirm 22's schema.
- **Spec 26 (Telemetry)**: ensure `tengu_tool_use_*` events for `mode == 'bubble'` are categorized correctly (probably bucketed as `'default'` since `PERMISSION_MODE_CONFIG.bubble` is undefined and falls back).

---

## Hardest-to-verify claim

**§6.4 + §12.2** — the YOLO classifier system prompt body. The `.txt` files (`auto_mode_system_prompt.txt`, `permissions_external.txt`, `permissions_anthropic.txt`) are absent from the leak (`require('./yolo-classifier-prompts/<name>.txt')` references files not present in the tree). The substitution mechanism (§6.4) is verifiable from `yoloClassifier.ts:484-540`, but the actual *content* — which determines how aggressively auto-mode blocks — is unauditable from this snapshot. A spec reader cannot assess fail-closed behavior of the classifier without those prompts. This is acknowledged in §12.2 as an open question; no fix possible without the bundled `.txt` files. Recommend §12.2 explicitly state "audit gap — classifier policy is opaque from leaked source alone."
