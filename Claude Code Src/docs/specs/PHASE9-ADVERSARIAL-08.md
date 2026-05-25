# Phase 9.5 Adversarial Review — Spec 08 (Tool Base & Registry)

Reviewer: Opus side, parallel review of 18 specs.
Spec: `docs/specs/08-tool-base-registry.md`
Verified against: `src/Tool.ts` (792 LOC), `src/tools.ts` (389 LOC), `src/types/permissions.ts` (441 LOC), `src/services/mcp/client.ts`, `src/components/permissions/PermissionRequest.tsx`, `src/tools/TaskOutputTool/TaskOutputTool.tsx`.

## Severity Counts

| Severity | Count |
|---|---|
| BLOCKER | 0 |
| MAJOR   | 1 |
| MINOR   | 4 |
| NIT     | 3 |

## Findings

### MAJOR-1 — `src/types/tools.ts` and `src/types/message.ts` cited as line-attributed imports actually do not exist
**Spec §2.3 / §12 Open Questions #1.** Spec lists imports of `Tool.ts` from `./types/tools.js` and `./types/message.js` and flags them as "file not present". I verified: `src/Tool.ts:58` imports `from './types/tools.js'`, but `ls src/types/` shows only `command.ts, generated/, hooks.ts, ids.ts, logs.ts, permissions.ts, plugin.ts, textInputTypes.ts`. The `generated/` subdir contains only `events_mono` and `google` — no `tools.ts` or `message.ts`.
The spec correctly *flags* this in §12 but uses the imports as authoritative type references in §3.1 / §4.4 (e.g., `ToolProgressData` union "from `types/tools.js`"). For a reimplementer, the listed union members (`AgentToolProgress`, `BashProgress`, `MCPProgress`, `REPLToolProgress`, `SkillToolProgress`, `TaskOutputProgress`, `WebSearchProgress`) are unverifiable from this tree. Spec should escalate this from "Open Question" to a documented missing-source artifact in §2 with explicit warning that the type member list is not source-grounded.
**Fix:** Add a §2.x "Missing Leaked Source" subsection enumerating both files and downgrade the §4.4 enumeration to "as cited by Tool.ts type imports — exact union members unverifiable in leak."

### MINOR-1 — `Tool.ts:362-695` cited line range vs actual `362-695`
Spec §3.1 cites line range `362-695` for the `Tool` type. Verified: type starts at line 362, closes at line 695. Range is exact. **No defect.** (Self-check.)

### MINOR-2 — `ToolPermissionContext` mirror at `src/types/permissions.ts:427-441` — verified, fix held
Spec §4.2 claims canonical owner is `Tool.ts:123-138`, mirror at `types/permissions.ts:427-441` "intentionally omits `isAutoModeAvailable`". Verified line-by-line:
- `Tool.ts:123-138` has `isAutoModeAvailable?: boolean` at line 130.
- `types/permissions.ts:427-441` has `mode, additionalWorkingDirectories, alwaysAllowRules, alwaysDenyRules, alwaysAskRules, isBypassPermissionsModeAvailable, strippedDangerousRules?, shouldAvoidPermissionPrompts?, awaitAutomatedChecksBeforeDialog?, prePlanMode?` — and **does** omit `isAutoModeAvailable`. Phase 9.4 fix held. **No defect.**
However the mirror uses `readonly` per-field rather than the `DeepImmutable<{...}>` wrapper. Spec doesn't note this divergence — minor doc gap.

### MINOR-3 — `getAllBaseTools()` order count: spec enumeration vs actual array
Spec §5.1 pseudocode lists ~35 tool slots. Actual `tools.ts:194-250` has 39 spread/conditional positions. Difference: spec omits the `getSendMessageTool()` direct call before `ListPeersTool` (it IS in spec line 604), and the final `ListMcpResourcesTool, ReadMcpResourceTool` pair (spec catches them at line 621). On careful re-read, spec enumeration is **complete**, just visually compressed. **No defect** — but spec does not state the actual element-count invariant ("39 positions in the leaked tree" would aid reimplementers).

### MINOR-4 — `Tool.ts:783-792` `buildTool` line range
Spec §1 cites `Tool.ts:783-792` for buildTool. Verified: function declared line 783, closes line 792. Exact. **No defect.**

### MINOR-5 — `TOOL_DEFAULTS` line range `757-769`
Spec §6.1 cites `Tool.ts:757-769`. Actual: `757-769` is the constant block (verified). **No defect.**

### NIT-1 — `Tool.ts:158-300` ToolUseContext "~74 leaf fields"
Spec claims ~74 leaf fields. I did not enumerate but the type spans 158-300 (143 lines), with ~50 visible top-level keys. The "74 leaf" figure includes nested `options.*` keys. Untestable as stated; would benefit from "≈50 top-level + ≈25 nested options.* fields" precision.

### NIT-2 — `Tool.ts:469-470` `tengu_tool_pear` reference
Verified at line 470: "Only applied when the tengu_tool_pear is enabled." Spec §12 Q6 flags this as needing investigation; the reference is real. No defect.

### NIT-3 — Aliased tools (TaskOutputTool)
Spec §5.1 claims `TaskOutputTool.ts:184` declares `aliases: ['AgentOutputTool','BashOutputTool']`. Actual file is `TaskOutputTool.tsx` (not `.ts`) and the alias is at line **150**, not 184: `aliases: ['AgentOutputTool', 'BashOutputTool']`. Path extension and line number both off. **MINOR-6** (re-classifying upward from NIT).

### MINOR-6 — TaskOutputTool path/line drift (see NIT-3)
Spec §5.1: "see spec 15 §3 / `TaskOutputTool.ts:184`". Actual path is `src/tools/TaskOutputTool/TaskOutputTool.tsx` (TSX not TS) and line is 150. Both wrong. Cross-spec impact: spec 15 is the authoritative owner; if spec 15 has the same drift, it propagates. **Recommend audit of spec 15 §3 against `TaskOutputTool.tsx:150`.**

### Verified-Correct Claims (for the record)
- `McpAuthTool` factory pattern: `createMcpAuthTool(name, config)` confirmed at `services/mcp/client.ts:55, 2318, 2331`. Spec §5.1 claim correct.
- `ReviewArtifactTool` flag-gated registration via `PermissionRequest.tsx:36` and `feature('REVIEW_ARTIFACT')`: verified. NOT in `tools.ts`. Spec §5.1 claim correct.
- USER_TYPE='ant' gates: `REPLTool` (16-19), `SuggestBackgroundPRTool` (20-24), `ConfigTool` (214), `TungstenTool` (215), `REPLTool` re-include (232) — all verified.
- `feature()` gates: `PROACTIVE`/`KAIROS` for SleepTool (25-28), `AGENT_TRIGGERS` cronTools (29-35), `AGENT_TRIGGERS_REMOTE` (36-38), `MONITOR_TOOL` (39-41), `KAIROS` SendUserFile (42-44), `KAIROS||KAIROS_PUSH_NOTIFICATION` PushNotification (45-49), `KAIROS_GITHUB_WEBHOOKS` SubscribePR (50-52), `OVERFLOW_TEST_TOOL` (107), `CONTEXT_COLLAPSE` (110), `TERMINAL_PANEL` (113), `WEB_BROWSER_TOOL` (117), `COORDINATOR_MODE` (120), `HISTORY_SNIP` (123), `UDS_INBOX` (126), `WORKFLOW_SCRIPTS` (129) — all line refs match.
- ANT import-order banner (`tools.ts:1`): exact match with spec §6.2.
- Lazy require getters at lines 63, 66, 69 (TeamCreate/TeamDelete/SendMessage): exact match.
- `getEmptyToolPermissionContext` exported at line 140: matches.
- Special tools set: `ListMcpResourcesTool.name, ReadMcpResourceTool.name, SYNTHETIC_OUTPUT_TOOL_NAME` at `tools.ts:301-305`: matches §5.2.
- `assembleToolPool` partition-then-sort with `uniqBy('name')` at `tools.ts:345-367`: matches.

## Verdict

**APPROVE WITH MINOR FIXES.** Spec is structurally accurate and source-grounded with high fidelity. The Phase 9.4 ToolPermissionContext mirror fix held cleanly. The only structural concern is **MAJOR-1** (missing-source escalation for `types/tools.ts` / `types/message.ts`) and **MINOR-6** (TaskOutputTool path/line drift). No fabricated enumerations, no contradictions with cross-spec claims I could detect.

## Cross-Spec Impact

- **Spec 15** (tool-tasks) likely shares the TaskOutputTool path/line drift in MINOR-6. Audit recommended.
- **Spec 09** (permissions): ToolPermissionContext type ownership claim ("canonical owner Tool.ts:123") must be consistent. Verified spec 08's claim; spec 09 should defer to spec 08 for the type itself.
- **Spec 19** (tool-misc): SyntheticOutputTool registered-by-name-only claim flagged in §12 Q2 — spec 19 must own that explanation.
- **Spec 16** (tool-mcp-lsp): McpAuthTool factory pattern referenced; spec 16 owns the per-server creation logic.
- **Spec 26** (analytics-flags): StatSig configs `claude_code_global_system_caching` and `claude_code_system_cache_policy` are spec-08 invariants but server-side artifacts. Flagged correctly in §12 Q4-5.
- **Spec 04** (turn pipeline): `validateInput` → `checkPermissions` ordering claim in §9 must match spec 04.

## Hardest-to-Verify Claim

§5.1 cache-invariance claim: *"The order is hand-maintained and **must match the upstream StatSig `claude_code_global_system_caching` config**. Reordering invalidates the global system prompt cache for every user."* This is a server-side invariant about an external StatSig config that does not ship with the leak. The textual cache-invariant comment at `tools.ts:191-192` confirms the *intent*, but the *fact* that reordering invalidates downstream cache cannot be proven from the source — it depends on the server's hash-based cache breakpoint logic. Spec correctly flags this in §12 but the claim is asserted as load-bearing throughout §5.1 and §5.3 without an empirically verifiable source within the leaked tree. A reimplementer cannot validate the invariant without server access.
