# 00 — Overview Specification

> **Authoritative anchor for the reverse-spec project.** Every per-subsystem spec under `docs/specs/` references this document for architecture, glossary, conventions, the canonical 12-section template, and the feature-flag matrix. Sub-agents writing later specs MUST read this file first.

Master plan: `../superpowers/specs/2026-05-08-claude-code-reverse-spec-design.md`.

---

## 1. Purpose & Scope

This spec describes the **leaked Claude Code CLI** (Anthropic's official terminal coding agent) at a level sufficient to anchor the 42 deeper specs that follow. It is intentionally not a deep dive into any single subsystem; subsystem detail lives in the layer-specific specs (01..42).

### What "Claude Code" is

A locally-run, terminal-based coding agent built on:

- **Runtime**: Bun (with build-time dead-code elimination via `bun:bundle`)
- **Language**: TypeScript (strict)
- **UI**: React + [Ink](https://github.com/vadimdemedes/ink) — React for the terminal
- **CLI parser**: Commander.js (extra-typings)
- **Schema validation**: Zod v4 (imported from `'zod/v4'`, not `'zod'`)
- **Code search**: ripgrep (via `GrepTool`)
- **Protocols**: Anthropic SDK, MCP (Model Context Protocol), LSP (Language Server Protocol)
- **Telemetry**: OpenTelemetry + gRPC (lazy-loaded; ~700KB-1MB combined)
- **Feature flags**: GrowthBook
- **Auth**: OAuth 2.0, JWT, macOS Keychain
- **Scale**: ~1,902 source files / ~512K LOC (excluding `.DS_Store`)

### Project scope (from the master plan)

- **Fidelity target**: bit-exact reimplementation. Algorithms, decision trees, system prompts, regexes, schemas, and constants are inlined verbatim.
- **Internal scope**: ALL feature flags including `USER_TYPE === 'ant'` paths.
- **Output**: 43 canonical specs in this directory; this is spec **00**.

### Caveat: bundled artifacts

`src/main.tsx` is ~803KB / 4683 lines and is bundled minified output. Bit-exact applies to behavior reachable via string searches and adjacent unbundled files (`entrypoints/`, `bootstrap/`, `cli/`, `setup.ts`). Residual gaps go to §12 of the relevant spec.

### What this spec is NOT

- Not the place to read system prompts verbatim — those live in their owning spec's §6.
- Not a substitute for reading the source — every claim here that a sub-agent depends on must be re-verified by that sub-agent against the source citations.

---

## 2. Source Map

### 2.1 The 9 Layers

```
Layer 0  Foundation         00
Layer A  Boot & Config      01, 02
Layer B  Query Loop & Ctx   03, 04, 05, 06, 07
Layer C  Tool System        08, 09, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19
Layer D  Slash Commands     20, 21
Layer E  Services           22, 23, 24, 25, 26, 27, 28, 29
Layer F  Modes (cross-cut)  30, 31, 32, 33, 34, 35, 36
Layer G  UI / Output        37, 38, 39
Layer H  Persistence        40, 41
Layer I  Long Tail          42
```

### 2.2 Source Ownership Map (top-level `src/`)

Every path in `src/` is owned by exactly one spec. Sub-agents use this map to set `IN scope` and `OUT of scope` boundaries.

| Path | Owner | Notes |
|---|---|---|
| `src/main.tsx` | 01 | Bundled; treat as read-mostly |
| `src/setup.ts` | 01 | Setup-time hooks (UDS_INBOX, CONTEXT_COLLAPSE, COMMIT_ATTRIBUTION, TEAMMEM, ant-only paths) |
| `src/entrypoints/` | 01 | `agentSdkTypes.ts`, `cli.tsx`, `init.ts`, `mcp.ts`, `sandboxTypes.ts`, `sdk/` |
| `src/bootstrap/` | 01 | `state.ts` (parallel-prefetch state) |
| `src/cli/` | 01 | CLI surface residual |
| `src/schemas/` | 02 | Zod schemas |
| `src/migrations/` | 02 | Settings migrations |
| `src/QueryEngine.ts` | 03 | 1295 lines — LLM API loop, streaming, retries, thinking, token counting |
| `src/query.ts` | 04 | 1729 lines — turn pipeline (message → tool-use → result), system-reminder injection, hook fan-out |
| `src/query/` | 04 | Pipeline helpers |
| `src/services/tools/` | 04 | Tool dispatch service surface |
| `src/context.ts` | 05 | 189 lines — system/user context memoized assembly |
| `src/context/` | 05 | Notifications and context helpers |
| `src/cost-tracker.ts`, `src/costHook.ts` | 06 | Token/cost tracking |
| `src/services/tokenEstimation.ts` | 06 | Token count estimation |
| `src/services/compact/` | 07 | Context compression (with `microcompact`/`snip` variants) |
| `src/Tool.ts` | 08 | 792 lines — `Tool` interface, `ToolUseContext`, `buildTool` factory |
| `src/tools.ts` | 08 | 389 lines — registry (`getAllBaseTools`, `getTools`, `assembleToolPool`, `getMergedTools`) |
| `src/tools/utils.ts` | 08 | Shared utilities |
| `src/hooks/toolPermission/` | 09 | Decision tree + permission UI |
| `src/types/permissions.ts` | 09 | Permission types verbatim |
| `src/utils/permissions/` | 09 | Permission rule matchers |
| `src/tools/BashTool/` | 10 | + `tools/shared/gitOperationTracking.ts` (per master plan) |
| `src/tools/{FileRead,FileWrite,FileEdit,NotebookEdit}Tool/` | 11 | |
| `src/tools/{Glob,Grep,ToolSearch}Tool/` | 12 | |
| `src/tools/{WebFetch,WebSearch}Tool/` | 13 | |
| `src/tools/{Agent,TeamCreate,TeamDelete,SendMessage}Tool/` | 14 | Tool surface only — algorithm in `tools/shared/spawnMultiAgent.ts` owned by 30 |
| `src/tools/Task*Tool/`, `src/Task.ts`, `src/tasks.ts`, `src/tasks/`, `src/tools/TodoWriteTool/` | 15 | |
| `src/tools/{MCP,LSP,ListMcpResources,ReadMcpResource,McpAuth}Tool/` | 16 | |
| `src/tools/SkillTool/`, `src/skills/` | 17 | |
| `src/tools/{EnterPlan,ExitPlan,EnterWorktree,ExitWorktree}*/` | 18 | |
| `src/tools/{Sleep,SyntheticOutput,ScheduleCron,RemoteTrigger,Brief,AskUserQuestion,Config,REPL,PowerShell}Tool/` | 19 | + `testing/TestingPermissionTool.tsx` + missing-source registry refs (Monitor, Workflow, WebBrowser, Snip, Tungsten, VerifyPlanExecution, PushNotification, SendUserFile, SubscribePR, etc.) |
| `src/commands.ts` | 20 | 754 lines — registration, argument parsing, lazy import, dynamic skill/plugin/workflow integration |
| `src/commands/` | 21 | 101 commands |
| `src/services/api/` | 22 | Anthropic client, file API, bootstrap, preconnect |
| `src/services/mcp/`, `src/services/mcpServerApproval.tsx` | 23 | MCP lifecycle, transports, auth/approval |
| `src/services/lsp/` | 24 | LSP server manager |
| `src/services/oauth/` | 25 | OAuth + JWT + Keychain |
| `src/services/analytics/` | 26 | GrowthBook + OTel + gRPC |
| `src/services/policyLimits/`, `src/services/remoteManagedSettings/`, `src/services/settingsSync/` | 27 | |
| `src/plugins/`, `src/services/plugins/` | 28 | |
| `src/services/extractMemories/`, `src/services/teamMemorySync/`, `src/services/SessionMemory/`, `src/memdir/` (interaction surface) | 29 | |
| `src/coordinator/`, `src/tools/shared/spawnMultiAgent.ts` (35KB primary spawn algorithm) | 30 | |
| (`PROACTIVE` flag delta surfaces) | 31 | |
| (`KAIROS*`, `AGENT_TRIGGERS*` flag delta surfaces) | 32 | |
| (`DAEMON` flag delta surfaces) | 33 | |
| `src/bridge/` | 34 | IDE bridge (VS Code, JetBrains), JWT, REPL bridge |
| `src/remote/`, `src/server/`, `src/commands/remote-setup/` | 35 | |
| `src/voice/`, `src/services/voice.ts`, `src/services/voiceKeyterms.ts`, `src/services/voiceStreamSTT.ts`, `src/commands/voice/` | 36 | Voice context wiring location is unresolved at overview time — sub-agent records actual location in spec 36 §2 |
| `src/ink/`, `src/ink.ts`, `src/components/` (~140), `src/screens/`, `src/dialogLaunchers.tsx`, `src/interactiveHelpers.tsx`, `src/replLauncher.tsx` | 37 | |
| `src/outputStyles/` | 38 | |
| `src/vim/`, `src/keybindings/` | 39 | |
| `src/memdir/` (storage + workflow) | 40 | |
| `src/state/`, `src/history.ts`, `src/assistant/sessionHistory.ts`, `src/projectOnboardingState.ts` | 41 | |
| `src/buddy/`, `src/upstreamproxy/`, `src/native-ts/`, `src/moreright/`, `src/assistant/` (residual not owned by 41), `src/constants/`, `src/types/` (residual), `src/utils/` (residual not owned by tool/service specs), CLI residual | 42 | Catch-all sweep |

### 2.3 Unclaimed `src/services/` paths (RESIDUAL — surface here)

The Layer E decomposition (22..29) covers the largest service modules but the actual `src/services/` directory is wider than 8 modules. These paths must be claimed by their natural service spec (or land in 42) during deep-dive; the Phase 9 coverage audit will enforce assignment:

```
AgentSummary/        autoDream/           awaySummary.ts
claudeAiLimits.ts    claudeAiLimitsHook.ts
diagnosticTracking.ts internalLogging.ts  MagicDocs/
mockRateLimits.ts    notifier.ts          preventSleep.ts
PromptSuggestion/    rateLimitMessages.ts rateLimitMocking.ts
tips/                toolUseSummary/      vcr.ts
```

Recommended host specs (sub-agents should confirm):

| Path | Suggested owner |
|---|---|
| `claudeAiLimits.ts`, `claudeAiLimitsHook.ts`, `rateLimitMessages.ts`, `rateLimitMocking.ts`, `mockRateLimits.ts` | 27 (policy/limits) or 22 (api) |
| `diagnosticTracking.ts`, `internalLogging.ts`, `notifier.ts` | 26 (analytics/observability) |
| `awaySummary.ts`, `AgentSummary/`, `toolUseSummary/`, `autoDream/` | 30 (coordinator) or 04 (turn pipeline) |
| `preventSleep.ts`, `vcr.ts` | 42 (long tail) |
| `tips/`, `MagicDocs/`, `PromptSuggestion/` | 38 (output styles) or 42 |

### 2.4 Source files imported by `Tool.ts` but not present at expected path

`Tool.ts` imports `./types/message.js` (line 40) and `./types/tools.js` (line 58); the `src/types/` directory contains `command.ts`, `generated/`, `hooks.ts`, `ids.ts`, `logs.ts`, `permissions.ts`, `plugin.ts`, `textInputTypes.ts` — but no `message.ts` or `tools.ts`. Spec 08 must locate these (`types/generated/`?) and either re-cite the actual location or record this as an unresolved source gap (§12).

### 2.5 Missing-Source Ledger (registry references → absent paths)

Tool/command registries reference modules whose source is absent from the leaked tree. The downstream spec is not allowed to silently drop these — record each as `missing-leaked-source` in the spec's §2 and cite the registry line. Confirmed missing as of Phase 0:

**Absent tool dirs referenced by `tools.ts`**:
| Symbol | Registry citation | Gate |
|---|---|---|
| `MonitorTool` | `tools.ts:39-41` | `feature('MONITOR_TOOL')` |
| `WorkflowTool` (incl. `bundled/`) | `tools.ts:129-134`; `commands.ts:401-405` | `feature('WORKFLOW_SCRIPTS')` |
| `WebBrowserTool` | `tools.ts:117-119` | `feature('WEB_BROWSER_TOOL')` |
| `SnipTool` | `tools.ts:123-125` | `feature('HISTORY_SNIP')` |
| `TungstenTool` | `tools.ts:60`, `:215` | ANT-only (`USER_TYPE === 'ant'`) |
| `VerifyPlanExecutionTool` | `tools.ts:91-95` | `CLAUDE_CODE_VERIFY_PLAN === 'true'` |
| `PushNotificationTool` | `tools.ts:46-49` | `feature('KAIROS') \|\| feature('KAIROS_PUSH_NOTIFICATION')` |
| `SendUserFileTool` | `tools.ts:42-44` | `feature('KAIROS')` |
| `SubscribePRTool` | `tools.ts:50-52` | `feature('KAIROS_GITHUB_WEBHOOKS')` |
| `SuggestBackgroundPRTool` | `tools.ts:21-24` | ANT-only |
| `OverflowTestTool` | `tools.ts:107-109` | `feature('OVERFLOW_TEST_TOOL')` |
| `CtxInspectTool` | `tools.ts:110-112` | `feature('CONTEXT_COLLAPSE')` |
| `TerminalCaptureTool` | `tools.ts:113-116` | `feature('TERMINAL_PANEL')` |
| `ListPeersTool` | `tools.ts:126-128` | `feature('UDS_INBOX')` |
| `ReviewArtifactTool` | `components/permissions/PermissionRequest.tsx:36` | `feature('REVIEW_ARTIFACT')` |

**Note (Phase 9.6 verification 2026-05-09):** `ScheduleCronTool/` was previously listed as
absent. Source verification (`ls src/tools/ScheduleCronTool/`) shows `CronCreateTool.ts`,
`CronDeleteTool.ts`, `CronListTool.ts`, `UI.tsx`, `prompt.ts` — all present. Row removed.
Phase 9.4 fix log already noted this; this catalog row was the lagging artifact.

**Absent type files referenced by `Tool.ts`**:
| Path | Citation | Likely actual location |
|---|---|---|
| `./types/message.ts` | `Tool.ts:40` | `src/types/generated/`? |
| `./types/tools.ts` | `Tool.ts:58` | `src/types/generated/`? |

Spec 08 must enumerate these in its §12 with the registry citation; if any are later found in `src/types/generated/`, the citation moves there.

---

## 3. Public Interface — N/A

The overview has no callable interface. See per-subsystem specs.

---

## 4. Data Model & State (High-Level)

The harness keeps most state in **immutable React-style stores** addressable by the `ToolUseContext` and the `AppState` Zustand-style store under `src/state/`. Major shared types:

| Type | Location | Owned by spec |
|---|---|---|
| `Tool<Input,Output,P>` | `src/Tool.ts:362` | 08 |
| `Tools = readonly Tool[]` | `src/Tool.ts:701` | 08 |
| `ToolUseContext` | `src/Tool.ts:158-300` | 08 |
| `ToolPermissionContext` | `src/Tool.ts:123-138` (`DeepImmutable<…>`) | 09 |
| `ValidationResult`, `PermissionResult`, `PermissionRule`, `PermissionMode` | `src/types/permissions.ts` | 09 |
| `Command` | `src/types/command.ts` | 20 |
| `AppState` | `src/state/AppState.tsx` | 41 |
| `Message`, `UserMessage`, `AssistantMessage`, `SystemMessage`, `ProgressMessage` | `src/types/message.ts` (location to verify — see §2.4) | 04 / 08 |
| `HookProgress`, `PromptRequest`, `PromptResponse` | `src/types/hooks.ts` | 09 |
| `AgentId`, branded session IDs | `src/types/ids.ts` | 14 / 41 |
| `FileStateCache` | `src/utils/fileStateCache.ts` | 11 |
| `FileHistoryState` | `src/utils/fileHistory.ts` | 41 |
| `AttributionState` | `src/utils/commitAttribution.ts` | 10 |
| `DenialTrackingState` | `src/utils/permissions/denialTracking.ts` | 09 |
| `ContentReplacementState` | `src/utils/toolResultStorage.ts` | 04 / 11 |
| `SystemPrompt` | `src/utils/systemPromptType.ts` | 05 |
| `ContextReplacementState` (per-thread tool result budget) | `src/utils/toolResultStorage.ts` | 04 |
| `RenderedSystemPrompt` (for fork subagents) | (frozen at turn start) | 05 / 30 |

`ToolUseContext` is the **god object** of every tool call: it threads ~70+ fields (74 leaf fields counted in `Tool.ts:158-300`) including app-state setters, message list, MCP clients, permission context, abort controller, callbacks for UI, agent identity, denial tracking, and the rendered system prompt for fork sharing. Spec 08 documents its full shape verbatim.

---

## 5. Algorithm / Control Flow (High-Level)

### 5.1 Boot Lifecycle

Side effects fired BEFORE heavy module evaluation begins (warm parallelism for boot acceleration). The header of `main.tsx:1-12` documents this explicitly:

```
main.tsx (top-level, before heavy imports)
  ├── profileCheckpoint('main_tsx_entry')   ← startup profiler entry (main.tsx:12)
  ├── startMdmRawRead()                     ← MDM (managed device) settings prefetch via plutil/reg query
  ├── startKeychainPrefetch()               ← macOS Keychain reads (OAuth + legacy API key, parallel; ~65ms saved on every macOS startup)

(then heavy module imports run, ~135ms)

  ├── Commander.js parses argv
  ├── Lazy-load OTel/gRPC iff telemetry enabled (~400-700KB deferred)
  ├── Initialize Ink renderer (REPL or non-interactive)
  ├── Load settings (precedence low → high: pluginSettingsBase < userSettings < projectSettings < localSettings < flagSettings < policySettings; policySettings internally cascades remote > HKLM/plist > file > HKCU. There is NO env source. See spec 02 for the verified chain — earlier overview claim of "env > project > user > MDM > defaults" was incorrect.)
  ├── Load CLAUDE.md chain (memory) via getUserContext()
  ├── Connect MCP servers (services/mcp/)
  ├── Load plugins, skills (commands.ts.loadAllCommands, memoized)
  └── Enter REPL / SDK / batch mode (depends on entrypoint)
```

GrowthBook is **not** part of the pre-module-eval prefetch; its `feature(...)` calls are resolved by `bun:bundle` at build time (DCE) and runtime initialization happens later via the analytics service (spec 26). Earlier draft of this section listed GrowthBook init in parallel — that was incorrect.

### 5.2 Turn Pipeline

```
user input
  ↓
query.ts                                  ← message normalization, system-reminder injection
  ↓
QueryEngine.ts                            ← Anthropic API streaming call
  ↓
(stream events: text, thinking, tool_use)
  ↓
For each tool_use:
  hooks/toolPermission/                   ← decision tree (allow/deny/ask)
    ↓
  tools/<Tool>/                           ← validateInput → checkPermissions → call
    ↓
  ToolResult<Output>                      ← optional newMessages, contextModifier, mcpMeta
  ↓
turn pipeline emits tool_result
  ↓ (loop continues until model stops calling tools)
```

### 5.3 Tool Dispatch

```
tools.ts:getAllBaseTools()                ← exhaustive list (ANT + feature gates evaluated)
  ↓
filterToolsByDenyRules(permissionContext) ← deny-rule prefilter
  ↓
REPL mode? → strip REPL_ONLY_TOOLS
  ↓
isEnabled() per tool                      ← runtime gate
  ↓
assembleToolPool(builtin + mcp)           ← partition-then-sort for cache stability
  ↓
sent to API (some `defer_loading: true` if shouldDefer && !alwaysLoad)
```

**Tool ordering is server-side cache invariant**, controlled by **two related StatSig configs**:
- `claude_code_global_system_caching` — `getAllBaseTools()` order MUST match this config. Reordering tools invalidates the global system prompt cache for all users (`tools.ts:191` comment).
- `claude_code_system_cache_policy` — places the global cache breakpoint after the last prefix-matched built-in tool. Drives `assembleToolPool`'s partition-then-sort: built-ins as a sorted contiguous prefix, then MCP tools sorted, then `uniqBy` so name conflicts resolve to built-in (`tools.ts:354-365` comment).

Together these explain why a flat sort across built-in + MCP would invalidate downstream cache keys whenever an MCP tool happens to sort between two built-ins.

### 5.4 Multi-Agent Spawn (preview; spec 30)

`tools/shared/spawnMultiAgent.ts` (35KB) is the primary spawn algorithm; `AgentTool`, `TeamCreateTool`, `SendMessageTool` are the user/model-facing surfaces. The coordinator (`coordinator/`, gated by `COORDINATOR_MODE`) orchestrates parallel sub-agents and applies prompt/context deltas. ANT users get a different default agent model (`'inherit'` vs `'haiku'` for non-ant — see `tools/AgentTool/built-in/exploreAgent.ts:78`).

---

## 6. Verbatim Assets

This section serves as the authoritative source for two artifacts that every later spec depends on: the canonical 12-section template (so sub-agents have one place to mirror) and the project glossary.

### 6.1 Canonical 12-Section Spec Template (authoritative for all sub-agents)

```markdown
# <Subsystem> Specification

## 1. Purpose & Scope
- What problem this subsystem solves and how it affects the user/system.
- Boundaries with adjacent subsystems (in scope / out of scope).

## 2. Source Map
- Primary files: `src/<path>:<line-range>` inventory of authoritative code.
- Feature-flag and ANT guard locations.
- Source coverage table: every owned file/dir, whether it exists, and whether
  it was read fully, sampled, or only grep-inspected.
- Registry references whose source file is absent from the leaked tree must be
  called out explicitly; do not silently drop them.
- Imports from: list of upstream modules.
- Imported by: list of downstream consumers.

## 3. Public Interface (Contract)
- Function/class/event signatures exposed to the rest of the harness.
- Runtime entrypoints and call sites that invoke the subsystem.
- TypeScript types verbatim (or contract-equivalent representation).
- Zod schemas verbatim where input/output is validated.

## 4. Data Model & State
- Core types/interfaces (verbatim).
- Persistent state schema and on-disk location, if any.
- In-memory state machine: state diagram with transitions, async lifecycle,
  cancellation, and cleanup.

## 5. Algorithm / Control Flow
- Pseudocode for every non-trivial routine, at a level sufficient for
  bit-exact rebuild.
- Decision trees for branching logic (esp. permission, routing, retry, cache).
- Retry, timeout, backoff, ordering, concurrency, debounce/throttle, and
  cache invalidation policy.

## 6. Verbatim Assets
- System prompts and message templates: full text.
- Interpolated templates: static text verbatim plus every variable slot and
  formatter.
- Critical regexes, with anchor explanations.
- Permission decision-tree pseudocode.
- Tables of constants (token caps, timeouts, cache TTLs, limits).

## 7. Side Effects & I/O
- Filesystem, network, process spawn, signal handling.
- Environment variables consumed (`USER_TYPE`, `ANTHROPIC_API_KEY`, etc.).
- External binaries required (`rg`, `git`, `node`, `bun`, etc.).
- Trust boundaries and permission checks for each side effect.

## 8. Feature Flags & Variants
- Per-flag behavioral diff: `feature('X')` on vs. off.
- ANT-only behavior vs. production behavior.
- Env/runtime gates that are not `feature()` calls, e.g. `CLAUDE_CODE_*`,
  `NODE_ENV`, provider/auth checks.

## 9. Error Handling & Edge Cases
- Known failure modes and recovery strategies.
- User-facing error messages (verbatim).
- Throw/catch/fallback paths, partial-failure behavior, and swallowed/log-only
  errors.

## 10. Telemetry & Observability
- Log points, metric names, OpenTelemetry spans, analytics events.

## 11. Reimplementation Checklist
- Bullet list of invariants and components a reimplementer must preserve.
- "Spec is complete when" checklist tied to concrete source-owned files and
  public behaviors.

## 12. Open Questions / Unknowns
- Items not resolvable from source alone.
- Estimates clearly marked as such.
- Cross-cutting concerns flagged for the dispatcher.
```

#### N/A Rule

For minor subsystems, write `## N. <Section Name> — N/A` on a single line. Do not invent content. **§1 Purpose and §2 Source Map MUST always have real content.**

#### Traceability Rule

Every non-trivial behavioral claim cites `src/path:line-range`. Prefer ≤25-line ranges unless the cited asset is a contiguous prompt/schema. Multi-file inferences cite all required files. Registry references whose source is absent are recorded as unresolved source gaps in §12 with the registry citation.

### 6.2 Glossary (canonical)

#### Core concepts

- **Turn**: One user-message → assistant-response cycle, possibly containing many tool calls. Driven by `query.ts`.
- **Session**: Context-dependent. Three structurally different uses across the spec set: (a) **Transcript-file session** — a persisted `~/.claude/projects/<repo>/<sessionId>.jsonl` log of one CLI invocation's full message history (spec 41 owns the format and lifecycle). (b) **Ink REPL session** — one foreground run of the CLI from launch to exit, owned by spec 04 / spec 37 (the running turn-loop instance). (c) **Remote-server session** — one connection lifetime over the remote/CCR transport, owned by spec 35 (per-connection state, mailbox, auth). When a spec just says "session" without qualification, prefer the transcript-file meaning.
- **ToolUseBlock**: An SDK message content block of type `tool_use` carrying `{id, name, input}`. Emitted by the model in a turn's assistant message and consumed by the turn pipeline to dispatch a tool. Concrete shape and parsing live in spec 04 §5; downstream consumers are spec 03 (turn loop), 08 (registry dispatch), and 22 (API client serialization).
- **Hook** (overloaded — three distinct subsystems share this name; cross-spec hazard):
  - (a) **User-config hook** — subscribable event point (`PreToolUse`, `PostToolUse`, `Stop`, `SubagentStop`, `SessionStart`, `SessionEnd`, `UserPromptSubmit`, `PreCompact`, `Notification`) configured in `settings.json` under the `hooks` field. Source: `src/utils/hooks/`. Spec 02 owns the settings field; spec 09 owns `PreToolUse`/`PostToolUse` permission integration via `src/hooks/toolPermission/`.
  - (b) **React hook (Ink UI)** — `useTypeahead`, `useReplBridge`, `useAwaySummary`, etc. Source: `src/hooks/`. Spec 37b catalogs the full set. These are NOT user-configurable; they are React hooks consumed by Ink components.
  - (c) **Internal/runtime hook** — services-layer programmatic event dispatcher used inside the harness (e.g., `src/services/hooks/`, `src/costHook.ts`, `src/query/stopHooks.ts`). Distinct from (a) — these are not user-configurable and have no settings.json schema.
  - When a spec says "hook" without qualification, prefer meaning (a). Sub-agents MUST disambiguate when crossing subsystems.
- **Skill / SkillTool / skill_listing** (three related but distinct concepts; per spec 17 §11.5 finding):
  - (a) **Skill** — the entity registered via `registerBundledSkill()` (or via plugin/dir discovery); a reusable workflow with metadata + body. Lives under `skills/bundled/`, in plugins, or via dynamic discovery. Spec 17 owns.
  - (b) **`skill_listing`** — an attachment kind emitted by `utils/attachments.ts:2661-2751`. NOT a skill itself; it is the catalog-style attachment that informs the model of available skills (name + when-to-use blurbs only). Spec 17 §11.5 + spec 05 (attachments) cross-reference.
  - (c) **`SkillTool`** — the registered tool that, when invoked, returns the full content of a specific skill named in its input. Source: `src/tools/SkillTool/`. Distinct from the listing — `SkillTool` is the *fetch* tool; `skill_listing` is the *catalog*.
  - Reimplementer hazard: confusing the catalog (`skill_listing`) with the tool (`SkillTool`) breaks the discover-then-load flow.
- **Plugin**: A bundle of commands, agents, skills, hooks, and MCP servers loaded via `plugins/`.
- **Agent (subagent)**: A sub-conversation with its own context window, dispatched via `AgentTool`. Inherits or overrides parent model (ANT default `inherit`, non-ANT default `haiku`).
- **Team**: A coordinated group of agents managed via `TeamCreateTool` / `coordinator/`.
- **Coordinator**: Multi-agent orchestrator (gated by `COORDINATOR_MODE`).
- **Workflow**: Scripted multi-step routine (gated by `WORKFLOW_SCRIPTS`); lives in `tools/WorkflowTool/`.
- **Context** (overloaded — five distinct meanings across the spec set; do not conflate):
  - (a) **`ToolUseContext`** — the per-tool-call god object threaded through every `Tool.call` invocation; ~70+ fields including `messages`, `abortController`, `mcpClients`, `setAppState`, `requestPrompt`, `agentId`, `renderedSystemPrompt`. Source: `src/Tool.ts:158-300`. Spec 08.
  - (b) **`ToolPermissionContext`** — `DeepImmutable<{ mode, alwaysAllowRules, alwaysDenyRules, askRules, ... }>`. Source: `src/Tool.ts:123-138` (re-exported from `src/types/permissions.ts`). Spec 09. Distinct from `ToolUseContext` — `ToolPermissionContext` is a *field* of `ToolUseContext`.
  - (c) **System context** — `getSystemContext()` output: git status, branch, default branch, log -5, user.name; truncated at 2000 chars. Source: `src/context.ts`. Spec 05.
  - (d) **User context** — `getUserContext()` output: CLAUDE.md chain, currentDate, memory files. Source: `src/context.ts`. Spec 05.
  - (e) **AsyncLocalStorage context** — Node's `AsyncLocalStorage` used for request-scoped values (e.g., agent identity, abort propagation across async boundaries). Spec 04 / 30.
  - When a spec just says "context" unqualified, the meaning is usually (a) inside tool implementations and (c)+(d) inside boot/turn pipeline discussion. Disambiguate explicitly when ambiguous.

#### Protocols

- **MCP** (Model Context Protocol): External tool/resource server protocol. Transports: stdio, SSE, HTTP, WebSocket. Integration in `services/mcp/` and `MCPTool`.
- **LSP** (Language Server Protocol): Code intelligence integration in `services/lsp/` and `LSPTool`.

#### Modes

- **Output style**: Assistant's response-shape preset under `outputStyles/` (`default`, `explanatory`, etc.).
- **Permission mode**: User-addressable runtime set is `INTERNAL_PERMISSION_MODES` (`src/types/permissions.ts:33-36`): `default`, `acceptEdits`, `bypassPermissions`, `dontAsk`, `plan`, plus `auto` only when `feature('TRANSCRIPT_CLASSIFIER')`. The type-level union `InternalPermissionMode` (`permissions.ts:28`) ALSO includes `bubble`. **Two-tier classification**: (a) Zod rejects `defaultMode: 'bubble'` in settings.json and `--permission-mode bubble` from CLI (it's not in the user-addressable set); BUT (b) `bubble` IS used at runtime as a permission mode for forked subagents — assigned at `forkSubagent.ts:67` (`permissionMode: 'bubble'`), branched on at `runAgent.ts:443`, and written into `ToolPermissionContext.mode` at `runAgent.ts:430-433`. See spec 09 §3 + spec 14 for forked-subagent flow. **Phase 9.6 fix**: prior versions (post-Phase-9.4) incorrectly described `bubble` as type-only; corrected here.
- **Plan mode**: User-toggled or model-initiated reasoning-only mode entered via `EnterPlanModeTool`.
- **Worktree mode**: Per-tool git worktree isolation via `EnterWorktreeTool` / `ExitWorktreeTool` (gated by `isWorktreeModeEnabled()`).
- **Bridge mode**: IDE integration channel in `bridge/` (gated by `BRIDGE_MODE`).
- **Daemon**: Background server (gated by `DAEMON`).
- **Remote (CCR)**: Remote session/server (gated by `CCR_REMOTE_SETUP`); env `CLAUDE_CODE_REMOTE`.
- **Voice**: Speech input mode (gated by `VOICE_MODE`).
- **Kairos**: Scheduled-trigger / cron-driven assistant mode (gated by `KAIROS` family).
- **Proactive**: Auto-wakeup mode where agent self-schedules continuations via `SleepTool` (gated by `PROACTIVE`).
- **Coordinator mode**: Parallel multi-agent execution (gated by `COORDINATOR_MODE`; env `CLAUDE_CODE_COORDINATOR_MODE`).
- **REPL mode**: Constrained mode where primitive tools are wrapped behind a `REPLTool` VM context.
- **Simple mode**: Reduced toolset (Bash + FileRead + FileEdit only). Triggered by env `CLAUDE_CODE_SIMPLE`.
- **Bare mode** (`isBareMode()`): Skips auto-discovery walks but honors explicit `--add-dir`.

#### Build / runtime

- **Bun bundle feature flag**: `import { feature } from 'bun:bundle'` — build-time dead-code elimination. See §8.
- **ANT guard**: `process.env.USER_TYPE === 'ant'` block, gating Anthropic-internal-only code.
- **Lazy require**: `const X = (() => { const m = require(...); return m.X })()` or getter functions to break circular imports without changing module shape.
- **Tool registry**: `getAllBaseTools()` / `getTools()` / `assembleToolPool()` / `getMergedTools()` in `tools.ts`.
- **Command registry**: `COMMANDS()` / `getCommands(cwd)` / `loadAllCommands(cwd)` in `commands.ts`.

#### Caching

- **Prompt cache**: Anthropic-side cache keyed off the system prompt + tool schemas. Tool ordering changes invalidate it.
- **System cache policy**: `claude_code_global_system_caching` StatSig dynamic config. Tool order in `getAllBaseTools()` MUST match it.
- **Cache breaker**: ANT-only injection into the system prompt for forcing cache miss (gated by `BREAK_CACHE_COMMAND`).
- **Cache breakpoint**: Server-side semantic boundary between built-in and MCP tools (drives `assembleToolPool`'s partition-sort).

#### Memory

- **MEMORY.md / memdir**: Persistent file-based memory under `src/memdir/`. Spec 40.
- **CLAUDE.md chain**: Project memory walked from `cwd` upward (and via `--add-dir`); fed into system context.
- **Auto memory extraction**: `services/extractMemories/` distills conversation into `MEMORY.md` updates.
- **Team memory sync**: `services/teamMemorySync/`.
- **Session memory**: `services/SessionMemory/`.

#### Observability

- **GrowthBook**: Feature flag and analytics platform; `services/analytics/`.
- **OpenTelemetry / gRPC**: Lazy-loaded telemetry stack.
- **Diagnostics no-PII**: `logForDiagnosticsNoPII` — privacy-safe logging used in context/git status assembly.

---

## 7. Side Effects & I/O — High-Level

Each subsystem documents its own; the cross-cutting picture:

- **Filesystem**: project `cwd` walking (CLAUDE.md), settings dirs (`~/.claude/...`), MEMORY.md, transcript persistence, plugin manifests, on-disk tool result overflow files.
- **Network**: Anthropic API (`services/api/`), MCP servers, GrowthBook, OAuth providers, OTel collectors.
- **Process spawn**: `git`, `rg`, `bash`, optionally `pwsh`, IDE bridges, MCP stdio servers, sub-agents.
- **Signals**: `SIGINT`/`SIGTERM` propagation through `AbortController` in every `ToolUseContext`.
- **Sockets**: `UDS_INBOX` Unix domain sockets (peers); IDE bridge IPC.
- **Keychain**: macOS keychain for OAuth tokens.
- **Required external binaries**: `git` (always), `rg` (search; bundled `bfs`/`ugrep` for ANT), `node`/`bun` (runtime).

---

## 8. Feature Flags & Variants

### 8.1 `feature('X')` Flag Matrix

Verified by `grep -rE "feature\\('[A-Z_]+'\\)" src/`. CLAUDE.md's flag list is incomplete; the authoritative set is below.

| Flag | Primary effect | Sites (representative) | Owning specs |
|---|---|---|---|
| `PROACTIVE` | Auto-wakeup, `SleepTool` | tools.ts:26; commands.ts:63; main.tsx:2197 | 31 |
| `KAIROS` | Scheduled assistant mode | tools.ts:42,46; commands.ts:67,70; main.tsx:80,1058,1642,2184,2206; assistant/ | 32 |
| `KAIROS_BRIEF` | `/brief` command | commands.ts:67; main.tsx:1728,2184,2201 | 32 |
| `KAIROS_GITHUB_WEBHOOKS` | `SubscribePRTool`, `/subscribe-pr` | tools.ts:50; commands.ts:101 | 32 |
| `KAIROS_PUSH_NOTIFICATION` | `PushNotificationTool` | tools.ts:46 | 32 |
| `KAIROS_CHANNELS` | Channel UI surface | interactiveHelpers.tsx:241; main.tsx:1642 | 32 |
| `AGENT_TRIGGERS` | Cron tools, scheduled | tools.ts:29 | 32 |
| `AGENT_TRIGGERS_REMOTE` | `RemoteTriggerTool` | tools.ts:36 | 32 |
| `BRIDGE_MODE` | IDE bridge | commands.ts:73,77; main.tsx:2246 | 34 |
| `DAEMON` | Background server | commands.ts:77 | 33 |
| `VOICE_MODE` | Voice input | commands.ts:80 | 36 |
| `CCR_REMOTE_SETUP` | `/remote-setup` | commands.ts:91 | 35 |
| `HISTORY_SNIP` | `SnipTool` + microcompact-snip variant | tools.ts:123; commands.ts:83; QueryEngine.ts:122,125,1276; query.ts:115,401 | 07, 19 |
| `WORKFLOW_SCRIPTS` | `WorkflowTool` + `/workflows` | tools.ts:129; commands.ts:86,401 | 19 |
| `EXPERIMENTAL_SKILL_SEARCH` | Local skill index | commands.ts:96; query.ts:66 | 17 |
| `MCP_SKILLS` | Treat MCP prompts as skills | commands.ts:550 | 17, 23 |
| `ULTRAPLAN` | `/ultraplan` | commands.ts:104 | 21 |
| `TORCH` | `/torch` (debug) | commands.ts:107 | 21 |
| `UDS_INBOX` | `ListPeersTool`, peers commands, `setup.ts:95` | tools.ts:126; commands.ts:108; setup.ts:95; main.tsx:1910,1945 | 19, 35 |
| `FORK_SUBAGENT` | `/fork` | commands.ts:113 | 21, 30 |
| `BUDDY` | Companion sprite + `/buddy` | commands.ts:118 | 42 (runtime) / 21c (command) |
| `MONITOR_TOOL` | `MonitorTool` | tools.ts:39 | 19 |
| `WEB_BROWSER_TOOL` | `WebBrowserTool` | tools.ts:117; main.tsx:1571 | 19 |
| `TERMINAL_PANEL` | `TerminalCaptureTool` | tools.ts:113 | 19 |
| `CONTEXT_COLLAPSE` | `CtxInspectTool` + collapsing pipeline | tools.ts:110; query.ts:18,440,616,800,1090,1176; setup.ts:295 | 04, 19 |
| `OVERFLOW_TEST_TOOL` | `OverflowTestTool` | tools.ts:107 | 19 |
| `COORDINATOR_MODE` | Coordinator spawn | tools.ts:120,280,292; QueryEngine.ts:115; main.tsx:76,1872 | 30 |
| `BREAK_CACHE_COMMAND` | System prompt cache breaker | context.ts:131,143 | 05 |
| `REACTIVE_COMPACT` | Reactive auto-compact | query.ts:15 | 07 |
| `TEMPLATES` | Job classifier | query.ts:69 | 04 |
| `BG_SESSIONS` | Background-task summaries | query.ts:118,1685; main.tsx:1116 | 30, 41 |
| `TOKEN_BUDGET` | Per-turn budget tracker | query.ts:280,1308 | 06 |
| `CACHED_MICROCOMPACT` | Cached microcompact | query.ts:423,870 | 07 |
| `CHICAGO_MCP` | MCP-related dispatcher path | query.ts:1033,1489; main.tsx:1477,1608 | 23 |
| `TRANSCRIPT_CLASSIFIER` | `auto` permission mode + classifier | types/permissions.ts:35; main.tsx:171,337,1399,1769; interactiveHelpers.tsx:224 | 09 |
| `COMMIT_ATTRIBUTION` | Git commit attribution | setup.ts:350 | 10 |
| `TEAMMEM` | Team memory sync activation | setup.ts:365 | 29 |
| `LODESTONE` | (UI surface) | interactiveHelpers.tsx:176; main.tsx:647 | 37 |
| `DIRECT_CONNECT` | Pending direct connection | main.tsx:548,612 | 35 |
| `SSH_REMOTE` | SSH-based remote | main.tsx:577,706 | 35 |
| `AGENT_MEMORY_SNAPSHOT` | Per-agent memory snapshot | main.tsx:2258 | 14, 40 |
| `UPLOAD_USER_SETTINGS` | Upload settings | main.tsx:963 | 27 |

### 8.1.B Additional Flag Catalog (47 more)

The table above covers the 42 most behaviorally distinctive flags in the harness's main pipelines. **`grep -hoE "feature\\('[A-Z_]+'\\)" src/ -r | sort -u | wc -l` returns 89 total.** The remaining 47 are catalogued below grouped by concern. Owning specs MUST surface any of these that affect their subsystem in their own §8.

#### Boot / CLI / install
| Flag | Effect | Site | Owning specs |
|---|---|---|---|
| `ABLATION_BASELINE` | Baseline-mode short-circuit | entrypoints/cli.tsx:21 | 01 |
| `ALLOW_TEST_VERSIONS` | Permits version `99.99.*` install | utils/nativeInstaller/download.ts:124 | 01 / 42 |
| `BYOC_ENVIRONMENT_RUNNER` | `environment-runner` subcommand | entrypoints/cli.tsx:226 | 01 |
| `DUMP_SYSTEM_PROMPT` | `--dump-system-prompt` debug flag | entrypoints/cli.tsx:53 | 01 / 05 |
| `SELF_HOSTED_RUNNER` | `self-hosted-runner` subcommand | entrypoints/cli.tsx:238 | 01 / 35 |
| `NEW_INIT` | Reworked `/init` flow | commands/init.ts:230 | 21 |
| `IS_LIBC_GLIBC` | Force-claim glibc | utils/envDynamic.ts:54 | 01 |
| `IS_LIBC_MUSL` | Force-claim musl | utils/envDynamic.ts:53 | 01 |
| `HARD_FAIL` | Hard-fail on unhandled error | main.tsx:3870 | 01 / 42 |
| `NATIVE_CLIENT_ATTESTATION` | Adds attestation header | constants/system.ts:82 | 22 / 25 |
| `NATIVE_CLIPBOARD_IMAGE` | Native clipboard paste | utils/imagePaste.ts:101 | 11 / 37 |

#### Telemetry / observability / metadata
| Flag | Effect | Site | Owning specs |
|---|---|---|---|
| `COWORKER_TYPE_TELEMETRY` | Coworker-type analytics field | services/analytics/metadata.ts:603 | 26 |
| `ENHANCED_TELEMETRY_BETA` | Enhanced session tracing | utils/telemetry/sessionTracing.ts:9 | 26 |
| `MEMORY_SHAPE_TELEMETRY` | MEMORY.md shape analytics | memdir/findRelevantMemories.ts:66 | 40 / 26 |
| `PERFETTO_TRACING` | Perfetto trace dump | utils/telemetry/perfettoTracing.ts:260 | 26 |
| `SHOT_STATS` | Shot distribution map | utils/stats.ts:131 | 26 |
| `SLOW_OPERATION_LOGGING` | ANT vs external slow-op logger | utils/slowOperations.ts:157 | 26 |
| `COMPACTION_REMINDERS` | Compact reminder attachments | utils/attachments.ts:922 | 07 |

#### Skills / agents
| Flag | Effect | Site | Owning specs |
|---|---|---|---|
| `BUILTIN_EXPLORE_PLAN_AGENTS` | Adds explore/plan built-in agents | tools/AgentTool/builtInAgents.ts:14 | 14 / 30 |
| `BUILDING_CLAUDE_APPS` | "Building Claude Apps" bundled skill | skills/bundled/index.ts:64 | 17 |
| `RUN_SKILL_GENERATOR` | "Run skill generator" bundled skill | skills/bundled/index.ts:73 | 17 |
| `SKILL_IMPROVEMENT` | Skill improvement hook | utils/hooks/skillImprovement.ts:177 | 17 |
| `KAIROS_DREAM` | Kairos dream skill | skills/bundled/index.ts:35 | 32 / 17 |
| `VERIFICATION_AGENT` | Verification-agent dispatch in TaskUpdate | tools/TaskUpdateTool/TaskUpdateTool.ts:335 | 15 |

#### Bash / permissions
| Flag | Effect | Site | Owning specs |
|---|---|---|---|
| `BASH_CLASSIFIER` | Bash command classifier path | tools/BashTool/bashPermissions.ts:1576,1645 (gates); :84,:631,:1429 are comments | 10 / 09 |
| `POWERSHELL_AUTO_MODE` | YOLO classifier PowerShell guidance | utils/permissions/yoloClassifier.ts:498 | 09 / 19 |
| `TREE_SITTER_BASH` | Tree-sitter Bash parser | utils/bash/parser.ts:51 | 10 |
| `TREE_SITTER_BASH_SHADOW` | Shadow tree-sitter Bash parser | utils/bash/parser.ts:51 | 10 |

#### API / cache / retry
| Flag | Effect | Site | Owning specs |
|---|---|---|---|
| `ANTI_DISTILLATION_CC` | Anti-distillation cache-control header | services/api/claude.ts:303 | 22 |
| `CONNECTOR_TEXT` | Beta header for connector text | constants/betas.ts:23 | 22 |
| `PROMPT_CACHE_BREAK_DETECTION` | Cache-break detection in agent runs | tools/AgentTool/runAgent.ts:824 | 30 (cited by 14) |
| `UNATTENDED_RETRY` | Unattended retry behavior in API | services/api/withRetry.ts:101 | 22 |

#### UI / UX / input
| Flag | Effect | Site | Owning specs |
|---|---|---|---|
| `AUTO_THEME` | Theme auto-mode in ConfigTool settings | tools/ConfigTool/supportedSettings.ts:34 | 19 / 38 |
| `HISTORY_PICKER` | Prompt history picker | components/PromptInput/PromptInput.tsx:1721 | 37 |
| `HOOK_PROMPTS` | Hook-driven request-prompt callback in REPL | screens/REPL.tsx:2520 | 37 / 09 |
| `MESSAGE_ACTIONS` | Per-message keybindings | keybindings/defaultBindings.ts:88 | 39 |
| `QUICK_SEARCH` | Quick-search keybindings | keybindings/defaultBindings.ts:52 | 39 |
| `MCP_RICH_OUTPUT` | Rich output in MCP tool UI | tools/MCPTool/UI.tsx:51 | 16 |
| `STREAMLINED_OUTPUT` | Streamlined CLI print output | cli/print.ts:857 | 38 |
| `REVIEW_ARTIFACT` | `ReviewArtifactTool` (gated, not in tools.ts list) | components/permissions/PermissionRequest.tsx:36 | 19 / 09 |

#### Memory / files / extraction
| Flag | Effect | Site | Owning specs |
|---|---|---|---|
| `EXTRACT_MEMORIES` | Auto-memory extraction caller-side gate | utils/backgroundHousekeeping.ts:7,34; query/stopHooks.ts:142 (gates); memdir/paths.ts:65 (comment only) | 29 / 40 |
| `FILE_PERSISTENCE` | Persistent file storage | utils/filePersistence/filePersistence.ts:279 | 11 / 41 |

#### Reasoning / thinking
| Flag | Effect | Site | Owning specs |
|---|---|---|---|
| `ULTRATHINK` | Extended thinking budget | utils/thinking.ts:20 | 03 |

#### Bridge / remote / connect
| Flag | Effect | Site | Owning specs |
|---|---|---|---|
| `CCR_AUTO_CONNECT` | Bridge auto-connect to CCR | bridge/bridgeEnabled.ts:186 | 34 / 35 |
| `CCR_MIRROR` | CCR mirror activation | main.tsx:2918 | 35 |

#### Settings / sync
| Flag | Effect | Site | Owning specs |
|---|---|---|---|
| `DOWNLOAD_USER_SETTINGS` | Download user settings on print | cli/print.ts:511 | 27 |

#### Away summary
| Flag | Effect | Site | Owning specs |
|---|---|---|---|
| `AWAY_SUMMARY` | Away-summary hook | hooks/useAwaySummary.ts:54 | 30 / 41 |

That brings the catalog to **89 confirmed flags**, including 64 not listed in the leaked CLAUDE.md (which lists ~25).

### 8.2 Non-`feature()` Runtime Gates

Environment variables and predicate-style gates that are NOT `feature()` calls but DO change behavior at runtime:

| Gate | Type | Effect | Site |
|---|---|---|---|
| `process.env.USER_TYPE === 'ant'` | env | ANT-internal code | 30+ sites (see §8.3) |
| `process.env.NODE_ENV === 'test'` | env | Adds `TestingPermissionTool`; skips git status memo | tools.ts:244; context.ts:37 |
| `process.env.IS_DEMO` | env | Disables `INTERNAL_ONLY_COMMANDS` | commands.ts:343 |
| `process.env.CLAUDE_CODE_SIMPLE` | env (truthy) | Reduces tools to Bash/Read/Edit (+ optional REPL/coordinator) | tools.ts:273 |
| `process.env.CLAUDE_CODE_REMOTE` | env (truthy) | Skip git status assembly | context.ts:125 |
| `process.env.CLAUDE_CODE_DISABLE_CLAUDE_MDS` | env (truthy) | Hard disable CLAUDE.md loading | context.ts:166 |
| `process.env.CLAUDE_CODE_VERIFY_PLAN === 'true'` | env | Adds `VerifyPlanExecutionTool` | tools.ts:92 |
| `process.env.CLAUDE_CODE_COORDINATOR_MODE` | env (truthy) | Activates coordinator mode together with `feature('COORDINATOR_MODE')` | main.tsx:1872 |
| `process.env.ENABLE_LSP_TOOL` | env (truthy) | Adds `LSPTool` to base tools | tools.ts:224 |
| `isToolSearchEnabledOptimistic()` | helper | Adds `ToolSearchTool` | tools.ts:249 |
| `isPowerShellToolEnabled()` | helper | Adds `PowerShellTool` | tools.ts:151 |
| `isReplModeEnabled()` | helper | Filters out `REPL_ONLY_TOOLS` after wrapping in `REPLTool` | tools.ts:314 |
| `isWorktreeModeEnabled()` | helper | Adds Enter/Exit worktree tools | tools.ts:225 |
| `isAgentSwarmsEnabled()` | helper | Adds Team tools | tools.ts:228 |
| `hasEmbeddedSearchTools()` | helper | Drops `GlobTool`/`GrepTool` (ANT bun bundle has bfs/ugrep) | tools.ts:201 |
| `isTodoV2Enabled()` | helper | Adds `TaskCreate`/`Get`/`Update`/`List`Tool | tools.ts:218 |
| `isUsing3PServices()` | helper | Hides `/login`/`/logout` for 3P provider users | commands.ts:337 |
| `isClaudeAISubscriber()` / `isFirstPartyAnthropicBaseUrl()` | helper | Gates `availability: 'claude-ai' \| 'console'` | commands.ts:417 |
| `isUndercover()` | helper (ANT) | BashTool prompt variant | tools/BashTool/prompt.ts:49 |
| `isBareMode()` | helper | `--bare` skips auto-discovery | context.ts:167 |

### 8.3 ANT-Only Path Index

Sites where `process.env.USER_TYPE === 'ant'` changes behavior (representative; full list in spec 08 §2):

| Concern | Site | Behavior |
|---|---|---|
| ConfigTool inclusion | tools.ts:214 | Only ANT |
| TungstenTool inclusion | tools.ts:215 | Only ANT |
| REPLTool top-level import | tools.ts:17 | Only ANT |
| SuggestBackgroundPRTool top-level import | tools.ts:21 | Only ANT |
| INTERNAL_ONLY_COMMANDS in command list | commands.ts:343 | Only ANT && !IS_DEMO |
| agentsPlatform command | commands.ts:48 | Only ANT |
| Auto-mode classifier path in `query.ts` | query.ts:927 | Only ANT |
| Setup-time ANT repo classification (auto-undercover prime) | setup.ts:337-348 | ANT only; primes commitAttribution/internal-repo cache |
| Setup-time bypass-permission safety gate | setup.ts:417-422 | ANT only; with `CLAUDE_CODE_ENTRYPOINT` exclusions for local-agent / claude-desktop |
| UDS_INBOX peer/socket setup | setup.ts:95 (`feature('UDS_INBOX')`); main.tsx:1910,1945 | feature gate, not ANT-only |
| WebFetch behavior diffs | tools/WebFetchTool/utils.ts:400 | Variant |
| EnterPlanMode prompt | tools/EnterPlanModeTool/prompt.ts:167 | Variant |
| TaskStopTool userFacingName | tools/TaskStopTool/TaskStopTool.ts:46 | `''` (hidden in transcript) for ANT |
| AgentTool prompt + isolation defaults | tools/AgentTool/prompt.ts:273; loadAgentsDir.ts:94,610; runAgent.ts:362 | Variant + extra `'remote'` isolation option |
| exploreAgent default model | tools/AgentTool/built-in/exploreAgent.ts:78 | `inherit` (ANT) vs `haiku` |
| ToolSearch prompt variant | tools/ToolSearchTool/prompt.ts:37 | Variant |
| FileEdit prompt | tools/FileEditTool/prompt.ts:17 | Variant |
| SkillTool prompt + telemetry (5+ sites) | tools/SkillTool/SkillTool.ts:171,379,494,607,694,1051 | Telemetry/behavior delta |
| ConfigTool supported settings list | tools/ConfigTool/supportedSettings.ts:134 | Extended list |
| Bash safe env vars list | tools/BashTool/bashPermissions.ts:174,250,329,591 | `ANT_ONLY_SAFE_ENV_VARS` admitted |
| Bash sandbox decision | tools/BashTool/shouldUseSandbox.ts:23 | Variant |
| Bash readonly validation | tools/BashTool/readOnlyValidation.ts:1211 | Variant |
| Terminal OSC features | ink/termio/osc.ts:468 | Variant |

---

## 9. Error Handling — N/A

(Per-subsystem.)

---

## 10. Telemetry & Observability — High-Level

- **Logging**: `logForDiagnosticsNoPII`, `logError`, `logForDebugging` (`utils/log.ts`, `utils/diagLogs.ts`, `utils/debug.ts`).
- **Analytics**: GrowthBook backed; `services/analytics/`. Spec 26.
- **OTel**: Lazy-loaded; ~400KB+ deferred. Activated via env (per spec 26).
- **gRPC**: Telemetry transport, ~700KB lazy.
- **Internal logging**: `services/internalLogging.ts`, `services/diagnosticTracking.ts`.
- **Notifier**: `services/notifier.ts` for OS-level desktop notifications.

Sub-agents must list every concrete log/metric/span emitted by their subsystem in §10 of their spec.

---

## 11. Reimplementation Checklist

A reimplementer of Claude Code, working from the full `docs/specs/` set, must preserve the following invariants. (Detailed invariants per subsystem live in each spec's §11.)

- [ ] `getAllBaseTools()` ordering is part of the prompt-cache key and must match the upstream `claude_code_global_system_caching` StatSig config (or the reimplementer must provide their own equivalent stable ordering).
- [ ] `assembleToolPool()` keeps built-in tools as a sorted contiguous prefix before MCP tools (also sorted), with `uniqBy` preserving insertion order so name conflicts resolve to built-in. The global cache breakpoint sits after the last prefix-matched built-in tool, controlled by `claude_code_system_cache_policy`.
- [ ] Every tool implements at minimum: `name`, `inputSchema`, `prompt`, `call`, `description`, `userFacingName`, `isReadOnly`, `isConcurrencySafe`, `isEnabled`, `checkPermissions`, `mapToolResultToToolResultBlockParam`, `renderToolUseMessage`, `maxResultSizeChars`, `toAutoClassifierInput`. `buildTool()` provides safe defaults for `isEnabled` (→ `true`), `isConcurrencySafe`, `isReadOnly`, `isDestructive`, `checkPermissions`, `toAutoClassifierInput`, `userFacingName`.
- [ ] `ToolUseContext` threads ~30+ fields including `messages`, `abortController`, `mcpClients`, `setAppState`, `requestPrompt`, `appendSystemMessage`, `agentId`, `renderedSystemPrompt`, `contentReplacementState`. Every tool call constructs and passes this context.
- [ ] `ToolPermissionContext` is `DeepImmutable`; permission updates produce a new context, never mutate.
- [ ] Permission modes: `default`, `acceptEdits`, `bypassPermissions`, `dontAsk`, `plan`, optional `auto` (gated by `TRANSCRIPT_CLASSIFIER`), and `bubble` (runtime-only mode for forked subagents — not user-addressable; see spec 09 §3 and spec 14).
- [ ] Permission rules carry `source ∈ {userSettings, projectSettings, localSettings, flagSettings, policySettings, cliArg, command, session}` and `behavior ∈ {allow, deny, ask}`.
- [ ] System context (memoized): git status (`git --no-optional-locks ...`), branch, default branch, log -5, user.name; truncated at 2000 chars; skipped under `CLAUDE_CODE_REMOTE` or `!shouldIncludeGitInstructions()`.
- [ ] User context (memoized): CLAUDE.md chain via `getMemoryFiles()` → `filterInjectedMemoryFiles` → `getClaudeMds`; cached in `setCachedClaudeMdContent`; disabled by `CLAUDE_CODE_DISABLE_CLAUDE_MDS` or `--bare` (with `--add-dir` exception).
- [ ] Currentdate is part of the user context.
- [ ] Boot fires `startMdmRawRead` and `startKeychainPrefetch` BEFORE heavy module evaluation.
- [ ] All `feature('FOO')` branches are dead-code-eliminated at build time; runtime `if (feature('FOO'))` short-circuits to false in non-ANT/non-flag builds.
- [ ] `import { feature } from 'bun:bundle'` is the canonical import.
- [ ] `.js` import suffixes for `.ts` files (NodeNext/ESM resolution).
- [ ] `import type { z } from 'zod/v4'` (not `'zod'`).
- [ ] Lazy `require()` patterns for circular-dep breaking (e.g., `getTeamCreateTool`, `getSendMessageTool`).
- [ ] Top-level `process.env.USER_TYPE === 'ant'` reads are intentional; the `eslint-disable custom-rules/no-process-env-top-level` markers must remain to allow build-time DCE to strip ANT branches.
- [ ] ANT import order rule: `// biome-ignore-all assist/source/organizeImports: ANT-ONLY import markers must not be reordered` — keeping conditional imports adjacent to their gates so the bundler strips them.
- [ ] Workflow / Bundled / Built-in / Plugin / Skill loading order in `commands.ts:loadAllCommands` is `bundledSkills, builtinPluginSkills, skillDirCommands, workflowCommands, pluginCommands, pluginSkills, COMMANDS()`.
- [ ] Memoization: `loadAllCommands` and several context functions use `lodash-es/memoize` and expose `.cache.clear?.()` for invalidation.
- [ ] Remote-safe / Bridge-safe command sets are explicit allowlists (`REMOTE_SAFE_COMMANDS`, `BRIDGE_SAFE_COMMANDS`) — default deny.
- [ ] `availability: 'claude-ai' | 'console'` on commands gates by auth/provider state and is re-evaluated every `getCommands()` call.
- [ ] `Tool.shouldDefer` and `Tool.alwaysLoad` control `defer_loading: true` in the API request; ToolSearch is the deferred-load companion.
- [ ] MCP tools: `mcpInfo: { serverName, toolName }`; name may be prefixed (`mcp__server__tool`) or unprefixed (`CLAUDE_AGENT_SDK_MCP_NO_PREFIX`).
- [ ] Tool result overflow: `maxResultSizeChars` triggers persistence to disk with a path-only preview returned to Claude (FileRead overrides to `Infinity` to avoid loops).

---

## 12. Open Questions / Unknowns

Items not resolvable from the overview pass; deferred to subsystem specs.

> **Phase 9.7 sweep (2026-05-09)**: items below were re-audited. RESOLVED items cite where the answer landed; DEFERRED items remain genuinely unresolvable from this leak.

1. ~~**`src/types/message.ts` and `src/types/tools.ts` location**~~ — **RESOLVED Phase 9.7 (missing-leaked-source)**: `src/types/generated/` exists but contains only `events_mono/` and `google/` proto-binding subdirectories — neither `tools.ts` nor `message.ts` is present. `src/types/hooks.ts:15` still imports `from 'src/types/message.js'`, confirming the file is referenced but stripped. Recorded in spec 08 §12 Q1 and 00 §13 missing-source ledger.
2. ~~**Full `setup.ts` semantics**~~ — **RESOLVED Phase 9.7**: spec 01 §4 + §6 cover `UDS_INBOX`, `CONTEXT_COLLAPSE`, `COMMIT_ATTRIBUTION`, `TEAMMEM`, and ANT setup paths.
3. ~~**`services/` residual modules** (§2.3)~~ — **RESOLVED Phase 9.7**: owner specs claimed during Phases 7–9; remaining unclaimed live in spec 42 §2 and spec 42a. Phase 10b coverage matrix (`PHASE10-COVERAGE.md`) confirms 100% claim.
4. ~~**`types/generated/` content**~~ — **RESOLVED Phase 9.7**: contents are bun-protobuf-gen output (`events_mono/{claude_code,common,growthbook}/v1/` + `google/`); enumerated at directory level in spec 42 §A and spec 26 §6.7. No SDK-type generation present.
5. **StatSig dynamic config schema (`claude_code_global_system_caching`)** — **DEFERRED**: schema is a server-side StatSig artifact (per spec 26 §12 — UI-only at `https://console.statsig.com/...`; not source-derivable).
6. ~~**`CLAUDE_AGENT_SDK_MCP_NO_PREFIX`**~~ — **RESOLVED Phase 9.7**: live at `services/mcp/client.ts:1763` (`isEnvTruthy(process.env.CLAUDE_AGENT_SDK_MCP_NO_PREFIX)` controls MCP tool name prefixing). Cross-refs: `Tool.ts:453-454` doc, `utils/permissions/permissions.ts:248`. Documented in spec 23 (and §11.5 of this overview's MCP entry).
7. ~~**`isUndercover()`**~~ — **RESOLVED Phase 9.7**: bool predicate at `src/utils/undercover.ts:28`. Consumers: `tools/BashTool/prompt.ts:16,49` (ANT-only prompt variant), `constants/prompts.ts:138,621,660,694-700`, `utils/attribution.ts:53` (commit attribution gate). Documented in spec 10 §8 and spec 04/05 prompt assembly.
8. ~~**Voice context wiring location**~~ — **RESOLVED Phase 9.7**: spec 36 confirms voice integrates via `commands/voice/` + `services/voice*.ts`; `context/voice/` does not exist. Voice state attaches to the turn pipeline through the standard command-result path (no special wiring).
9. **Testing scaffolding** — **DEFERRED**: broader test infrastructure (Vitest config, fixtures, harness) is outside the leaked `src/` tree. `src/tools/testing/TestingPermissionTool.tsx` is the only in-tree artifact.

---

## Appendix A. Repo Conventions (cited in §11)

These are stable patterns observable across the codebase. Sub-agents should preserve them and their rationale when reimplementing.

1. **`.js` extensions in imports**. NodeNext/ESM resolution. Do not "fix" to `.ts`.
2. **Zod v4 import**. `import { z } from 'zod/v4'` (not `'zod'`). Affects schema definition idioms.
3. **Bun bundle feature flags**. `import { feature } from 'bun:bundle'` is the canonical place; build-time DCE strips falsy branches.
4. **ANT guard at top level**. `process.env.USER_TYPE === 'ant'` is read at the top level so the bundler can DCE the unused branch. The accompanying `eslint-disable` set varies by file: `tools.ts:15` disables `custom-rules/no-process-env-top-level, @typescript-eslint/no-require-imports` because the env read is the bare predicate; `commands.ts:47` only disables `@typescript-eslint/no-require-imports` because the env read sits inside an immediately-evaluated `require()` ternary that the codebase's lint rule does not flag. The pattern is: include whichever disable comments are actually needed for the file's specific shape — there is no single canonical incantation.
5. **Conditional require for build-time strip**. The pattern `const X = feature('FLAG') ? require('./X.js').X : null` is used hundreds of times; bundler strips the inactive branch entirely. Equivalent dynamic `import()` does NOT strip and must not be substituted.
6. **Lazy getter for circular deps**. `const getX = () => require('./X.js').X as typeof import('./X.js').X` — keeps types while breaking module-evaluation cycle. Exit ESLint via `@typescript-eslint/no-require-imports` disable.
7. **ANT-ONLY import order rule**. `// biome-ignore-all assist/source/organizeImports: ANT-ONLY import markers must not be reordered` at the file head where conditional imports interleave with static imports. The bundler relies on textual position to strip.
8. **Centralized types in `src/types/`**. Re-exported from concrete modules for backwards compatibility. Import from `src/types/` in new code.
9. **Tool directories are self-contained**. `src/tools/<Name>/` houses input schema, permission, execution, prompt, and rendering side-by-side.
10. **Permissions in two layers**. Tool-specific logic in `Tool.checkPermissions`; general matching/decision in `utils/permissions/permissions.ts` and `hooks/toolPermission/`.
11. **`memoize` for expensive lazy work**. `lodash-es/memoize` is the standard. Caches expose `.cache.clear?.()` for manual invalidation.
12. **System context vs user context separation**. `getSystemContext` (git/cache breaker) and `getUserContext` (CLAUDE.md, currentDate). Both memoized; both feed the conversation prefix.
13. **Status truncation at 2000 chars**. `MAX_STATUS_CHARS` in `context.ts:20`. Source-exact truncation suffix (note the leading newline; `context.ts:84-89`):
```
\n... (truncated because it exceeds 2k characters. If you need more information, run "git status" using BashTool)
```
Concatenated as `status.substring(0, MAX_STATUS_CHARS) + '<above>'`.
14. **Allowlists for remote/bridge**. Default deny + explicit `REMOTE_SAFE_COMMANDS` / `BRIDGE_SAFE_COMMANDS` sets.
15. **`buildTool()` factory**. New tools must be constructed via `buildTool({...})` so `TOOL_DEFAULTS` (fail-closed `isReadOnly: false`, `isConcurrencySafe: false`, `checkPermissions: 'allow'`, etc.) apply. Direct object literals lose defaults.
16. **Avoid `Array.toSorted`**. Codebase targets Node 18 in places; copy-then-sort with `[...arr].sort()`.

---

## Appendix B. Phase 0 Source Coverage Note

Phase 0 read fully:
- `README.md`
- `Tool.ts` (792 lines)
- `tools.ts` (389 lines)
- `commands.ts` (754 lines)
- `context.ts` (189 lines)
- `types/permissions.ts` (first 120 lines, sufficient for permission mode/rule glossary)

Phase 0 sampled:
- `types/command.ts`, `types/hooks.ts`, `types/ids.ts` (top-of-file imports/headers)
- Directory listings of `src/`, `src/tools/`, `src/commands/`, `src/services/`, `src/state/`, `src/types/`, `src/entrypoints/`, `src/bootstrap/`

Phase 0 grep-inspected:
- `feature('FOO')` sites across `src/`
- `USER_TYPE === 'ant'` sites across `src/`
- Path existence checks for review-agent claims

Phase 0 deliberately deferred to deeper specs:
- `main.tsx` (4683 lines, bundled — defer to spec 01 with string-search strategy)
- `QueryEngine.ts` (1295 lines — spec 03)
- `query.ts` (1729 lines — spec 04)
- All tool dirs (spec 10..19)
- All service dirs (spec 22..29)

---

## 13. Epistemic Boundary — Leak-External Unfalsifiable Contracts

**Phase 9.5 adversarial review** (18 specs × 2 model lenses) identified contracts that the spec set asserts as invariants but which **cannot be verified from the leaked tree alone**. They depend on artifacts that ship in production but were not present in the leaked source: server-side configs (StatSig, GrowthBook), generated proto schemas, the `bin/claude` shim, the Bun bundler config, the `src/ssh/` subtree, Anthropic-server-side tokenization, and so on.

This section catalogs these so a reimplementer working from the spec set knows exactly which "MUST" claims are **black-box invariants** (preserve behavior; the spec cannot prove from source what the behavior must be) versus **falsifiable invariants** (verifiable against `src/`).

### 13.1 Cost-Multiplier Invariants (≥10× blast radius — elevated callout)

A wrong reimplementation of either of these silently inflates cost by ~10× before any user notices. **Treat as critical implementation invariants.**

| # | Spec | Section | Contract | Why unfalsifiable | Reimplementer guidance |
|---|---|---|---|---|---|
| C1 | 22 | §4 | **Session-stable header latch** — once a request header set is computed for a session, subsequent requests in the session must reuse the identical header set or the prompt cache is busted. | Verifying the latch holds across all call-sites requires exhaustive enumeration of `services/api/` mutators against a contract no test enforces. | Treat the per-session header set as immutable for the session lifetime. Add a unit-test harness that captures the first request's header set and asserts byte-equality on every subsequent request. Cross-reference from spec 06 (cost-token-tracking). |
| C2 | 05 | §5.3 | **Stale-prefix-date-wins / midnight crossing** — `getUserContext()` memoization holds the prefix's `currentDate` field for the session; crossing midnight does NOT invalidate it. Re-evaluating `currentDate` would emit a fresh prefix and force an estimated ~920K tokens of `cache_creation` at the next turn. | Three-source date reconciliation (system clock, prefix-baked date, server reconciliation) has no telemetry in the leak that proves the actual cache bust cost. | Memoize `currentDate` at the prefix layer for the session lifetime; do NOT re-render the system context just because the wall-clock day rolled over. Expect prefix-date drift on long-lived sessions; surface to the user via separate UI rather than re-emitting the prefix. |

### 13.2 Server-Side Configs Not in Leak

| # | Spec | Section | Contract | Why unfalsifiable | Reimplementer guidance |
|---|---|---|---|---|---|
| 1 | 00 | §5.3 | **Two StatSig configs gate tool ordering for cache invariance**: `claude_code_global_system_caching` (sets `getAllBaseTools()` order) and `claude_code_system_cache_policy` (places the global cache breakpoint after the last prefix-matched built-in tool). | StatSig configs are server-side; not in leaked tree. The leaked code references them by string but does not contain their schema or values. | Treat as black-box invariants. Reimplementer must either (a) match upstream StatSig configs exactly, or (b) provide their own equivalent stable ordering and cache-breakpoint policy and accept that they are decoupling from upstream cache hits. |
| 2 | 08 | §5.1 | **Tool ordering must match upstream StatSig config** for cache stability. | Same as above. | Same as above. The static order encoded in `getAllBaseTools()` IS the contract; do not alphabetize or otherwise reorder. |
| 3 | 14 | §5.3 | **`getBuiltInAgents` evaluation order** depends on GrowthBook defaults (`BUILTIN_EXPLORE_PLAN_AGENTS` and others). | GrowthBook configs absent from leak. | Treat the order observed in `tools/AgentTool/builtInAgents.ts` as the contract. Off-by-one in default-eval order would silently change which agents users see. |

### 13.3 Boot / Build Surfaces Not in Leak

| # | Spec | Section | Contract | Why unfalsifiable | Reimplementer guidance |
|---|---|---|---|---|---|
| 4 | 01 | §2.6 | **Pre-`main.tsx:12` boot ordering** — `bin/claude` shim runs Node-flag selection, version printing, autoupdater hand-off, and arg-parsing PRIOR to `main.tsx` first byte. | `package.json`, `bin/`, and `scripts/` are absent from the leak. The shim's exact contents must be inferred from runtime observation. | Reimplementer must provide a CLI shim that (a) selects the Bun/Node runtime, (b) handles autoupdater handoff before main bundle eval, and (c) routes `--version` and similar fast-path args. Match observed runtime behavior of the installed `claude` CLI. |
| 5 | 10 | §11.16 | **`ANT_ONLY_*` constants are DCE'd via Bun bundler constant-fold** — `process.env.USER_TYPE === 'ant'` is a literal-string compare that the bundler resolves at build time when `USER_TYPE` is baked into the bundle. | Bun build config (the `bun build` invocation, the `--define` flags, the bundle-time substitution map) is absent from the leak. | Reimplementer must configure their bundler to define-substitute `process.env.USER_TYPE` at build time so the predicate constant-folds; failure to do so leaks ANT-only code paths (and ANT-only constants like the safe-env-var allowlist) into external builds. |

### 13.4 Generated / External Schema Surfaces

| # | Spec | Section | Contract | Why unfalsifiable | Reimplementer guidance |
|---|---|---|---|---|---|
| 6 | 26 | §6.6 | **`to1PEventFormat` proto-field hoisting** — events for the 1P telemetry format are hoisted into specific proto-schema fields. | The proto schema (`src/types/generated/events_mono/`) is absent from the leak. Field shapes must be inferred from the JS that constructs them. | Treat the field-naming and nesting in `to1PEventFormat` as the de-facto schema. If migrating to a different telemetry sink, the field map must be re-derived; do not assume the leaked JS is normative for the wire format. |
| 7 | 11 | §6.4 | **Image base64 → token ratio of 0.125 (1 token per 8 base64 chars)** for image cost estimation. | Anthropic server-side tokenization is the source of truth; the leaked client uses 0.125 as an estimate that may differ from the actual server tokenization. | Treat as a heuristic for client-side cost preview only; never bill on or hard-cap based on this ratio. Server's `usage` blocks are authoritative. |

### 13.5 Cross-Module Conventions Not Locally Verifiable

| # | Spec | Section | Contract | Why unfalsifiable | Reimplementer guidance |
|---|---|---|---|---|---|
| 8 | 35 | §5.10 | **SSH reverse-forwarded unix-socket auth proxy** for CCR remote sessions. | `src/ssh/` subtree is absent from the leak. The auth-forwarding path is described by call sites but the implementation is missing. | Reimplementer must build their own SSH-tunnel auth-forwarding equivalent; spec 35 §5.10 describes the contract surface (forward a unix socket from local → remote, proxy `getauth` calls back) but cannot prove the wire protocol. |
| 9 | 34 | §9.3 | **WebSocket close codes 4090 / 4091 / 4092** are client-synthesized cross-module conventions (not IANA, not server-reserved). | Verifying the convention requires reading 3+ files simultaneously (bridge sender, bridge receiver, daemon dispatcher); no central registry. | Add a `BRIDGE_CLOSE_CODES` constants module that hard-codes the three values with comments explaining each meaning. Cross-reference from any module that close-encodes or close-handles. |

### 13.6 Notes for Reimplementers

- **"Treat as black-box invariant"** = preserve the observable behavior; do not derive it from leaked source. Match upstream by snapshotting current behavior, not by re-deriving from first principles.
- **Cost-multiplier invariants (§13.1)** SHOULD be elevated in their owning specs to a "Critical implementation invariants" section, and cross-referenced from spec 06 (cost-token-tracking).
- **Per Phase 9.5 review**, this list is **not exhaustive** — additional unfalsifiable contracts likely exist in specs not in the 18 reviewed. Sub-agents writing or reviewing later specs should flag any "MUST" assertion that depends on a server-side config, generated schema, build-time substitution, or external subtree absent from the leak.

### 13.7 Provenance

This appendix aggregates the "Leak-external unfalsifiable contracts" table from `PHASE9-ADVERSARIAL-SUMMARY.md` (2026-05-09). The 11 enumerated contracts above came from 18 spec adversarial reviews × 2 model lenses; each was verified at the cited spec section. Phase 9.7 final-pass added the §13.1 elevation of C1/C2 as cost-multiplier invariants per the summary's "Cost-multiplier invariants" callout.
