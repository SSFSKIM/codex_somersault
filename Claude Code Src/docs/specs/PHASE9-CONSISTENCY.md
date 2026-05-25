# Phase 9 — Cross-Spec Consistency Review

Read-only audit of `docs/specs/00..42` for internal drift before adversarial per-spec review.
Evidence by `path:line` (spec lines, not source). Source spot-checks were not required.

---

## Executive Summary

| Severity | Count | Notes |
|---|---:|---|
| Critical | 3 | Canonical-source confusion (flag matrix), tool registry naming drift, `ToolPermissionContext` shape disagreement |
| Major   | 7 | Glossary contradictions on PermissionMode, missing tools in registry, gated-flag ownership double-claims, missing 21a/21b/21c flag mapping |
| Minor   | 11 | Typos, redundant flag rows, alias confusion, stale "see spec NN" hints |

The set is structurally sound. INDEX.md's adjacency graph is well-formed and **no dead spec-number links exist** (every `spec NN` reference falls inside `00..42`). Most drift is **terminology / canonical-source / table-completeness**, not algorithmic disagreement.

---

## Per-Check Findings Table

| # | Specs involved | Finding | Severity | Recommended fix |
|---|---|---|---|---|
| 1.0 | 26 vs 00 | The Phase 9 prompt says "spec 26 is the canonical flag source." It isn't. **Spec 26 §8.1 enumerates only 7 build-time flags** (those used by analytics: `PERFETTO_TRACING`, `ENHANCED_TELEMETRY_BETA`, `SHOT_STATS`, `SLOW_OPERATION_LOGGING`, `COWORKER_TYPE_TELEMETRY`, `CHICAGO_MCP`, `KAIROS`). The full flag matrix lives in **`00-overview.md` §8.1 + §8.1.B** (89 flags). Spec 26 §1 explicitly disclaims wider ownership ("each individual flag's behavior lives in its owning spec"). | Critical | Add a one-line note at the top of spec 26 §8 pointing readers to `00-overview.md` §8.1 as the matrix root, and rename §8.1 → "Telemetry/analytics flags (this spec's narrow scope)". |
| 1.1 | 00 §8.1, 30, 32, 21c | Spec 00 lists 89 flags. All flags I sampled outside spec 00 (`PROACTIVE`, `KAIROS`, `KAIROS_BRIEF`, `KAIROS_GITHUB_WEBHOOKS`, `KAIROS_PUSH_NOTIFICATION`, `KAIROS_CHANNELS`, `BRIDGE_MODE`, `DAEMON`, `VOICE_MODE`, `AGENT_TRIGGERS`, `AGENT_TRIGGERS_REMOTE`, `MONITOR_TOOL`, `WEB_BROWSER_TOOL`, `TERMINAL_PANEL`, `CONTEXT_COLLAPSE`, `OVERFLOW_TEST_TOOL`, `COORDINATOR_MODE`, `FORK_SUBAGENT`, `BG_SESSIONS`, `HISTORY_SNIP`, `WORKFLOW_SCRIPTS`, `CCR_REMOTE_SETUP`, `EXPERIMENTAL_SKILL_SEARCH`, `MCP_SKILLS`, `ULTRAPLAN`, `TORCH`, `UDS_INBOX`, `BUDDY`, `BUILTIN_EXPLORE_PLAN_AGENTS`, `VERIFICATION_AGENT`, `BASH_CLASSIFIER`, `POWERSHELL_AUTO_MODE`, `TREE_SITTER_BASH`, `TREE_SITTER_BASH_SHADOW`, `ANTI_DISTILLATION_CC`, `CONNECTOR_TEXT`, `PROMPT_CACHE_BREAK_DETECTION`, `UNATTENDED_RETRY`, `AUTO_THEME`, `HISTORY_PICKER`, `HOOK_PROMPTS`, `MESSAGE_ACTIONS`, `QUICK_SEARCH`, `MCP_RICH_OUTPUT`, `STREAMLINED_OUTPUT`, `REVIEW_ARTIFACT`, `EXTRACT_MEMORIES`, `FILE_PERSISTENCE`, `ULTRATHINK`, `CCR_AUTO_CONNECT`, `CCR_MIRROR`, `DOWNLOAD_USER_SETTINGS`, `UPLOAD_USER_SETTINGS`, `KAIROS_DREAM`, `MEMORY_SHAPE_TELEMETRY`, `AGENT_MEMORY_SNAPSHOT`, `LODESTONE`, `DIRECT_CONNECT`, `SSH_REMOTE`, `COMMIT_ATTRIBUTION`, `TEAMMEM`, `BREAK_CACHE_COMMAND`, `REACTIVE_COMPACT`, `TEMPLATES`, `TOKEN_BUDGET`, `CACHED_MICROCOMPACT`, `CHICAGO_MCP`, `TRANSCRIPT_CLASSIFIER`, `COMPACTION_REMINDERS`, `SKILL_IMPROVEMENT`, `KAIROS`) all appear in spec 00. | Minor | None — spec 00 is the matrix; no missing rows. |
| 1.2 | 00 vs 30/14 | `PROMPT_CACHE_BREAK_DETECTION` owner column says "14 / 30"; spec 30 §1 also claims it. Both list it as in-scope. The flag's *call sites* live in `runAgent.ts` (spec 30 territory), but spec 14 also covers it. | Minor | Move `PROMPT_CACHE_BREAK_DETECTION` to "30 only" in spec 00; remove from spec 14 §8 if duplicated there. |
| 1.3 | 00 / 21c | Spec 00 §8.1 row for `BUDDY` lists owning spec **42**, not 21c. Spec 21c lists `/buddy` command. Both cite the same flag. Acceptable but reader may miss the cross-reference. | Minor | Add "(commands in 21c, runtime in 42)" to the BUDDY row in spec 00. |
| 1.4 | 21b/21c vs 00 | The ant- and flagged-command catalogs (`21b`, `21c`) do **not** explicitly include a "Required flag" column for every command. Several commands in `21c` are flagged-only by name but the gating flag is implicit (e.g. `/proactive` ↔ `PROACTIVE`, `/voice` ↔ `VOICE_MODE`, `/bridge` ↔ `BRIDGE_MODE`). | Major | Add a `Gate` column to the `21c` table mapping each flagged command to its `feature('…')` predicate. |
| 2.0 | 08 vs 11 | Spec 11 uses tool *display* names `Edit`/`Write` interchangeably with the canonical class names `FileEditTool`/`FileWriteTool`. Spec 08's registry list uses the class names. Reading spec 11 in isolation, "EditTool" / "WriteTool" appear (substrings of FileEditTool), but the actual exported names are `FileEditTool` / `FileWriteTool`. | Minor | None required — substring match was a false positive. |
| 2.1 | 08 vs 15 | Spec 15 mentions `AgentOutputTool` and `BashOutputTool` but spec 08's registry does not list them. Spec 15 line 184 clarifies these are **aliases** declared in `aliases: ['AgentOutputTool','BashOutputTool']` for `TaskOutputTool`. | Minor | Add a parenthetical "(aliased on `TaskOutputTool`)" in spec 08's registry list to prevent future drift. |
| 2.2 | 08 vs 19 | Spec 19 introduces `ScheduleCronTool` and `ReviewArtifactTool`. Neither appears in spec 08's registry list. `REVIEW_ARTIFACT` is mentioned in spec 00 §8.1.B as "(gated, not in tools.ts list)" — meaning it is gated through a different surface. `ScheduleCronTool` looks like a typo or alias for `CronCreateTool`. | Major | Confirm `ScheduleCronTool` is a typo in spec 19 (likely `CronCreateTool`). If `ReviewArtifactTool` is real but registered elsewhere, document its registration site in spec 08 §5 with a "see spec 19" pointer. |
| 2.3 | 08 vs 16 | Spec 16 lists `McpAuthTool`. It does not appear in spec 08's flat registry. It is a per-server tool created on demand by `createMcpAuthTool(serverName, config)`. | Minor | Add a footnote to spec 08's registry list: "`McpAuthTool` is created per server by `createMcpAuthTool()` (spec 16) and not statically listed in `tools.ts`." |
| 2.4 | 08 vs 19/14 | `TungstenTool` and `MergedTool` are referenced in spec 08 §5 (TODO note) and 19/30/32. `TungstenTool` is listed but not described in any one tool spec — it lacks an owning §3 description. | Major | Either add a `TungstenTool` subsection to spec 19 (§3) or explicitly note that `TungstenTool` is a synthetic registry node owned by spec 30 (coordinator). |
| 2.5 | 20 vs 21a/b/c | Spec 20 (command system) lists registry types and lookup, but the per-command catalogs in 21a/b/c are pure tables. Some commands (`/buddy`, `/torch`, `/ultraplan`) are referenced in spec 21 (the splitter file) but not in spec 20's `INTERNAL_ONLY_COMMANDS` verbatim list. | Minor | Cross-reference 21a/b/c commands against spec 20 §5.2 `INTERNAL_ONLY_COMMANDS` — confirm the verbatim list is current. |
| 3.0 | INDEX.md adjacency | All "see spec NN" references point inside 00..42. **No dead links.** | — | none |
| 3.1 | 30 ↔ 14 | Spec 30 lists `tools/AgentTool/built-in/*.ts` as IN scope; spec 14 lists `AgentTool` IN scope. The shared boundary: spec 14 owns the *tool surface* (input schema, dispatch, prompt body), spec 30 owns the *runner* (`runAgent.ts`, coordinator orchestration). Both reference `forkSubagent.ts` as IN scope. Real overlap — see seam findings. | Major | Add explicit handoff bullet in spec 30 §1: "AgentTool input schema + prompt assembly = spec 14; this spec owns runAgent + forkSubagent execution + builtInAgents.ts." |
| 4.0 | Glossary: "Turn" | Defined consistently as "user-message → assistant-response cycle" in `00-overview.md:391`. Specs 03 and 04 use it the same way. No drift. | — | none |
| 4.1 | Glossary: "Permission mode" | **Drift.** Spec 00 §6.2 lists the **runtime** set as `default, acceptEdits, bypassPermissions, dontAsk, plan, +auto (TC flag)`. Spec 09 §3 (line ~158) lists the **external** set as `acceptEdits, bypassPermissions, default, plan` (4) and the **internal** type union adds `'auto'` and `'bubble'`. Spec 00 mentions `dontAsk` in the runtime set; spec 09 does not list `dontAsk` in `EXTERNAL_PERMISSION_MODES` — it appears only as an "internal sentinel". | Major | Reconcile: confirm whether `dontAsk` is part of the `INTERNAL_PERMISSION_MODES` runtime validation set. Update spec 00 §6.2 to match spec 09 §3 exactly, since spec 09 cites `permissions.ts:33-36` directly. |
| 4.2 | Glossary: "Plan Mode" | Consistent: `mode='plan'`, only Read/Glob/Grep/AskUserQuestion + EnterPlanMode/ExitPlanMode allowed. Specs 09 and 18 agree on enforcement details. | — | none |
| 4.3 | Glossary: "AcceptEdits" | Used as a `PermissionMode` literal in 09, referenced in 00, 02, 03, 04, 08, 10, 14, 19, 30, 34, 35, 37, 41. No semantic drift. | — | none |
| 4.4 | Glossary: "Compact" | Consistent. Spec 07 defines compact (manual `/compact`), microcompact (per-turn cache-edit), reactive compact (413 response handler), context-collapse drain. Spec 00's glossary references it correctly. | — | none |
| 4.5 | Glossary: "Subagent" | Consistent: spec 00 §6.2 says "Agent (subagent)"; spec 14 / 30 use "subagent" interchangeably with "agent" (forks are also subagents per spec 30 §5.5). | — | none |
| 4.6 | Glossary: "Skill frontmatter" | Spec 17 §6.5 owns the frontmatter parser fields verbatim. No conflicting definitions elsewhere. Spec 28 (plugins) references the same fields without redefining. | — | none |
| 4.7 | Glossary: "MCP tool" | Consistent. Spec 23 owns server config, spec 16 owns tool surface and `MCPTool`. No definitional drift. | — | none |
| 4.8 | Glossary: "Session" | "Session" is used heavily but never crisply glossed in spec 00. Specs 41, 35, 04 each use it slightly differently (a turn record file vs. an Ink REPL run vs. a remote-server connection lifetime). | Major | Add "Session" to spec 00 §6.2 glossary with the three contexts disambiguated (transcript file, Ink REPL run, remote connection). |
| 4.9 | Glossary: "ToolUseBlock" | Used in 03, 04, 08, 22 without definition in spec 00. Concrete shape lives in spec 04 (turn pipeline). | Minor | Add "ToolUseBlock" to spec 00 glossary as "an SDK message content block of type `tool_use` carrying `id`, `name`, `input` — see spec 04 §5". |
| 5.0 | Tool interface (`Tool<…>`): 08 vs 10..19 | Spec 08 defines the canonical `Tool<Input,Output,P>` shape verbatim from `Tool.ts`. Tool specs 10–19 redocument the *fields they implement* but do not redeclare the type. No drift. | — | none |
| 5.1 | `ToolPermissionContext`: 08 vs 09 | **Drift.** Spec 08 §4.2 includes `isAutoModeAvailable?: boolean` in the type. Spec 09 §4.4 explicitly says `isAutoModeAvailable` is **not** declared in the type — only injected at `permissionSetup.ts:987`. Both cite different `Tool.ts` line ranges (`123-138` vs `permissions.ts:427-441`). One reads from `Tool.ts`, the other from `types/permissions.ts`. The two sources may genuinely differ; the audit should pick the authoritative file. | Critical | Verify against `src/types/permissions.ts` and `src/Tool.ts` which one exports `ToolPermissionContext`. The other should re-export. Make spec 08 cite the same file as spec 09 (or note re-export chain) and pick a single canonical field set. Spec 09 is more precise on the `isAutoModeAvailable` injection — keep that; spec 08 should drop the field from its verbatim block and add a footnote. |
| 5.2 | `PermissionResult`: 08 vs 09 | Spec 08 references it; spec 09 §3 owns the full union (`{behavior:'allow'|'ask'|'deny', …}`). Consistent. | — | none |
| 5.3 | `QueryEngine.run` signature: 03 vs 04 | Spec 03 owns `class QueryEngine` and the `ask()` wrapper. Spec 04 references `QueryEngine` only as caller. No signature redeclared. | — | none |
| 5.4 | AgentTool input schema: 14 vs 30 | Spec 14 §3 declares `{description, prompt, subagent_type?, model?, run_in_background?}` from `AgentTool.tsx:645-647`. Spec 30 references the same fields, mentions `subagent_type` becomes optional under `FORK_SUBAGENT`. Consistent. | — | none |
| 5.5 | `McpServerConfig`: 02 vs 23 | Spec 23 §6 owns `McpStdioServerConfigSchema` / `McpServerConfigSchema` / `ScopedMcpServerConfig` verbatim. Spec 02 references them via `pluginConfigs.<id>.mcpServers` and uses the same names. Consistent. Spec 16 uses `ScopedMcpServerConfig` once in `createMcpAuthTool` signature — agrees. | — | none |
| 6.0 | Phase seam 08↔09 | `ToolPermissionContext` defined twice (see 5.1). `PermissionMode` re-exported through `Tool.ts` per spec 08 §2. Owner is `permissions.ts` — clearly spec 09 territory. | Critical (rolled into 5.1) | Spec 08 should explicitly note "type re-exported from `permissions.ts`; canonical owner = spec 09". |
| 6.1 | Phase seam 19↔20 | Spec 19 (misc tools) lists tools that look command-like (`AskUserQuestionTool`, `BriefTool`). Spec 20 (command system) describes slash commands. The boundary is clean: 19 = tool definitions; 20/21 = `/`-prefixed commands. No overlap. | — | none |
| 6.2 | Phase seam 21c↔22 | Spec 21c lists flagged commands (e.g., `/api`, `/messages`); these typically issue API requests through spec 22's clients. No double-ownership of the API client. | — | none |
| 6.3 | Phase seam 29↔30 | Spec 29 (memory service) and spec 30 (coordinator) overlap on `services/AgentSummary/`, `services/awaySummary.ts`, `services/toolUseSummary/`, `services/autoDream/`. Spec 30 §1 IN-scope claims all of these. Spec 29 covers the memory service proper. The four summary services are coordinator-owned; spec 29 should explicitly delegate. | Major | Add a delegation bullet in spec 29 §1: "Agent/away/toolUse/autoDream summary services live in spec 30; this spec owns memory service and MEMORY.md only." |
| 6.4 | Phase seam 36↔37 | Spec 36 (voice) and spec 37 (Ink UI shell) both touch input. Boundary appears clean: 36 owns voice input pipeline; 37 owns the Ink REPL surface that consumes it. No conflicts spotted. | — | none |
| 6.5 | Phase seam 42↔end | Spec 42 (misc) is a catch-all; covers `BUDDY`, `companion`, etc. No other spec claims these. | — | none |

---

## Flag Matrix Delta (rows added/missing in spec 26 vs. canonical 00)

Spec 26 §8.1 lists exactly 7 build-time flags. **None of the other 82** flags grep-confirmed across the spec set are missing from spec 00 §8.1 + §8.1.B. Therefore there is **no flag missing from any canonical matrix** — but the prompt's premise that spec 26 is the matrix is wrong.

Flags in spec 26 §8.1 but not in spec 00 §8.1: **none**. Spec 00 covers all 7.
Flags in spec 00 §8.1 but not in spec 26: **82** (expected — spec 26 is narrowly scoped).

---

## Dead-Link List

`grep -rn "spec [0-9]\{2\}"` of every spec returned only references in the range `00..42`. **No dead spec links.**

INDEX.md `Adjacent` columns reference spec IDs that all exist. No orphaned adjacency entries detected.

---

## Glossary Drift Table

| Term | Owning spec | Drift type | Specs that disagree |
|---|---|---|---|
| Turn | 00 §6.2 | none | — |
| ToolUseBlock | 04 (de facto) | **missing from spec 00 glossary** | used by 03, 04, 08, 22 without canonical definition |
| PermissionMode | 09 §3 | **runtime-set list disagreement** | spec 00 §6.2 includes `dontAsk`; spec 09 EXTERNAL list does not |
| Plan Mode | 18 + 09 | none | — |
| AcceptEdits | 09 | none | — |
| Compact | 07 | none | — |
| Subagent | 00 §6.2 / 30 | none (inclusive of forks per 30) | — |
| Skill frontmatter | 17 §6.5 | none | — |
| MCP tool | 23 + 16 | none | — |
| Session | (none) | **missing canonical definition** | 41 (transcript), 35 (remote connection), 04 (turn-loop run) all use it differently |

---

## Signature Drift Table

| Signature | Owning spec | Other spec(s) | Drift |
|---|---|---|---|
| `Tool<Input,Output,P>` | 08 §3 | 10..19 (per-tool `prompt`/`call` only) | none |
| `ToolPermissionContext` | 08 §4.2 + 09 §4.4 | both | **`isAutoModeAvailable?` field present in 08, absent in 09** — pick one canonical file |
| `PermissionResult` | 09 | 08 references | none |
| `QueryEngine.run` / `QueryEngine` class | 03 | 04 callers | none |
| AgentTool input zod schema | 14 §3 | 30 references | none |
| `McpServerConfigSchema` / `ScopedMcpServerConfig` | 23 §6 | 02, 16 | none — all use same names + structure |

---

## Phase-Seam Observations

| Seam | Status | Note |
|---|---|---|
| 08 ↔ 09 (tool registry vs permissions) | **Issue** | `ToolPermissionContext` redefinition (see 5.1). Permission types are re-exported from `Tool.ts` for backwards compat — both specs need a "canonical owner" pointer. |
| 19 ↔ 20 (misc tools vs command system) | OK | Clean separation. |
| 21c ↔ 22 (flagged commands vs API) | OK | `/api`, `/messages` use 22's client without re-declaring it. |
| 29 ↔ 30 (memory vs coordinator) | **Issue** | `awaySummary.ts`, `AgentSummary/`, `toolUseSummary/`, `autoDream/` claimed by spec 30; spec 29 should explicitly delegate. |
| 36 ↔ 37 (voice vs Ink UI) | OK | Clean separation. |
| 42 ↔ end (misc catch-all) | OK | No collisions. |
| 14 ↔ 30 (AgentTool surface vs runner) | **Issue** | `forkSubagent.ts` is in scope for 30 but spec 14's IN-scope list also references it via `runAgent`/forking; needs a single-line handoff sentence in 30 §1. |

---

## Surprises

1. **Spec 26 is not the flag matrix.** The Phase 9 prompt assumed it was; spec 00 §8 is the actual canonical matrix and is *very* well-built (89 flags catalogued in two tables, with owning-spec column).
2. **Tool registry "missing" entries are mostly aliases or per-instance factories** — `AgentOutputTool`/`BashOutputTool` are aliases on `TaskOutputTool`; `McpAuthTool` is per-server. The single real mismatch is `ScheduleCronTool` in spec 19 (likely typo for `CronCreateTool`).
3. **`ToolPermissionContext` divergence** is the only genuine type-shape drift — and it sits exactly on the 08↔09 phase seam, which is the most-warned-about boundary.
4. **`Session` term has no canonical definition** despite being used in three structurally different ways across the spec set.

---

## Patterns Worth Flagging for the Adversarial Phase

- Tool/command name drift will cluster at registries (08, 20, 21*) — adversarial review of those specs should diff against `src/tools.ts` and `src/commands.ts` directly.
- Type-shape drift will cluster at the `Tool.ts` ↔ `types/permissions.ts` re-export chain. Phase 9 adversarial reviewers of 08 and 09 should be paired.
- "Spec X owns flag Y but spec Z mentions it" is mostly fine — the owning-spec column in spec 00 §8.1 is the single source of truth and is well-curated.
