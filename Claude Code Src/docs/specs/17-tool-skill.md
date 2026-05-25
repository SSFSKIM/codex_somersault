# 17 — SkillTool & Skill Loading Subsystem

> The `Skill` tool — how Claude invokes a slash-command skill from inside a turn — and the loaders that materialize bundled, plugin, dir, MCP, and (ANT-only) remote-canonical skills into `Command` objects. Read 08, 09, 20 first.

---

## 1. Purpose & Scope

### Subsystem responsibility
- Expose `Skill` as a Tool: receive a `{ skill, args }` input, validate that the skill name resolves to a prompt-based `Command`, ask permission, and **expand** the resolved skill into either (a) injected `newMessages` (inline execution) or (b) a forked sub-agent (`context: 'fork'`) (`tools/SkillTool/SkillTool.ts:331-869`).
- Provide the renderer (`formatCommandsWithinBudget`) that the **attachment** layer uses to format the per-turn skill listing under a 1%-of-context-window character budget, with bundled skills protected from truncation (`tools/SkillTool/prompt.ts:21-171`). The listing itself is *not* part of the SkillTool's system prompt — it is emitted as `skill_listing` attachments by `src/utils/attachments.ts:2661-2751` (see §5.1 and **05**). The SkillTool's `getPrompt()` is fixed text only.
- Materialize skills from five sources into `Command` objects: bundled (in-binary), plugin built-ins, plugin user-installed, on-disk skill dirs (`/skills/SKILL.md` + legacy `/commands/`), and MCP-provided prompts (`MCP_SKILLS`-gated). Resolve manifest frontmatter, hooks, paths, model, effort, allowed-tools (`skills/loadSkillsDir.ts:407-480, 270-401, 638-804`; `skills/bundledSkills.ts:53-100`; `skills/mcpSkillBuilders.ts:1-44`).
- Maintain the **dynamic** and **conditional** skill registries that are populated as files are touched mid-session (`skills/loadSkillsDir.ts:818-1075`).
- Track per-skill usage with a 7-day half-life ranking signal (`utils/suggestions/skillUsageTracking.ts:1-55`).
- (`SKILL_IMPROVEMENT`-gated) Run a side-channel post-sampling hook that proposes edits to the user's project skill file based on recent corrections (`utils/hooks/skillImprovement.ts:175-267`).
- (`EXPERIMENTAL_SKILL_SEARCH` + `USER_TYPE === 'ant'`) Resolve `_canonical_<slug>` skill names against discovered remote skills, fetch SKILL.md from a remote backend, inject the body as a meta user message (`tools/SkillTool/SkillTool.ts:108-116, 374-396, 488-504, 600-613, 957-1108`).

### IN scope
- Entire `src/tools/SkillTool/`.
- Entire `src/skills/`.
- `src/utils/hooks/skillImprovement.ts`.
- `src/utils/suggestions/skillUsageTracking.ts`.
- The `getMcpSkillCommands`, `getSkillToolCommands`, `getSlashCommandToolSkills` exports of `src/commands.ts:540-608` (the parts that constitute the skill-side surface; the broader `loadAllCommands` ordering belongs to 20 and is only **cited** here).
- The `formatCommandsWithinBudget` listing renderer in `tools/SkillTool/prompt.ts`. The **attachment-emission** wrapper `getSkillListingAttachments` (and `sentSkillNames`, `suppressNext`, `resetSentSkillNames`) lives in `src/utils/attachments.ts:2607-2751` and is owned by **05** — cited here, not duplicated.
- All feature-flag deltas: `EXPERIMENTAL_SKILL_SEARCH`, `MCP_SKILLS`, `BUILDING_CLAUDE_APPS`, `RUN_SKILL_GENERATOR`, `SKILL_IMPROVEMENT`, `KAIROS_DREAM`, plus `USER_TYPE === 'ant'` paths (≥6 ANT-only sites in `SkillTool.ts`).

### OUT of scope (refer by spec)
- The `Tool` interface, `buildTool`, registry assembly → **08**.
- Permission decision tree, deny/allow rule resolution machinery → **09**. Only the SkillTool-specific `ruleMatches` and `SAFE_SKILL_PROPERTIES` allowlist are in scope here.
- `loadAllCommands` ordering, `findCommand`, `processPromptSlashCommand`, slash-command UX, frontmatter parser → **20**. Cite-only.
- Per-tool catalog → **21**.
- Plugin discovery / marketplace plumbing → **28**. (Plugin **skill** integration is referenced.)
- Memory subsystem (`addInvokedSkill`, `clearInvokedSkillsForAgent`, `getInvokedSkillsForAgent`) → **29**. Cite-only.
- ToolSearch and the model-facing `DiscoverSkills` tool → **12**. The integration points (`discoveredSkillNames`, `isSkillSearchEnabled`, "skill_discovery" attachments) are referenced; the implementation is owned by 12.
- The forked-agent runner `runAgent` → **14**.
- MCP client lifecycle that produces MCP commands → **23**.

### Glossary delta (additions, see 00 §6.2)
- **Skill** — a `PromptCommand` (`type: 'prompt'`) that can be invoked by either the user (slash) or the model (Skill tool). Distinguished from a built-in command by `loadedFrom ∈ {'skills','plugin','bundled','mcp','commands_DEPRECATED'}` and/or `disableModelInvocation` (`commands.ts:586-603`).
- **Inline execution** — default; the resolved skill body is injected as `newMessages` into the parent turn, optionally adjusting `alwaysAllowRules.command`, `mainLoopModel`, and `effortValue` via a `contextModifier` (`SkillTool.ts:767-840`).
- **Forked execution** — `frontmatter.context === 'fork'`; the skill runs in a fresh sub-agent via `runAgent` and only its result string flows back (`SkillTool.ts:122-289, 622-632`).
- **Conditional skill** — a skill whose frontmatter declares `paths: …`; held in `conditionalSkills` until a file path operated on in-session matches one of its gitignore-style patterns, then promoted to `dynamicSkills` (`loadSkillsDir.ts:771-797, 997-1058`).
- **Dynamic skill** — a skill discovered from a nested `.claude/skills/` directory walked from a touched file path; deepest path wins (`loadSkillsDir.ts:861-975`).
- **Remote canonical skill** — ANT-only; name shape `_canonical_<slug>`; SKILL.md is fetched on-demand from an external backend and never appears in the local registry (`SkillTool.ts:374-396, 957-1108`).
- **Skill listing** — the per-turn enumeration of skills + descriptions emitted as a `skill_listing` **attachment** (`utils/attachments.ts:2661-2751`), formatted by `formatCommandsWithinBudget` and sized by `getCharBudget` (`prompt.ts:31-41, 70-171`). NOT appended to the SkillTool prompt — the SkillTool prompt is fixed instruction text. The attachment layer composes the listing from `getSkillToolCommands(cwd) ∪ getMcpSkillCommands(...)`, suppresses already-sent skills via per-agent `sentSkillNames`, and is owned by **05**.
- **Skill (in glossary terms)** — the term covers two slightly different surfaces:
  - **SkillTool-invocable command** — what `getSkillToolCommands` returns (used to list and to resolve invocation): `loadedFrom ∈ {'bundled','skills','commands_DEPRECATED'}` OR `(hasUserSpecifiedDescription \|\| whenToUse)` for plugin/MCP entries.
  - **System-prompt skill** — what `getSlashCommandToolSkills` counts (used by 03/05 builders): strictly tighter — `loadedFrom ∈ {'skills','plugin','bundled'}` OR `disableModelInvocation`, **excluding** `commands_DEPRECATED` from the SkillTool form. (`commands.ts:563-608`.)

---

## 2. Source Map

### 2.1 Primary files

| Path | Lines | Role |
|---|---|---|
| `src/tools/SkillTool/SkillTool.ts` | 1108 | Tool definition; input/output Zod schemas; `validateInput`; `checkPermissions` with `SAFE_SKILL_PROPERTIES` auto-allow; `call`; `executeForkedSkill`; `executeRemoteSkill`; `extractUrlScheme`; `mapToolResultToToolResultBlockParam` |
| `src/tools/SkillTool/prompt.ts` | 241 | `getPrompt` (memoized); `formatCommandsWithinBudget` (skill-listing renderer); `getCharBudget`; `getSkillToolInfo`; `getLimitedSkillToolCommands`; `getSkillInfo`; constants `SKILL_BUDGET_CONTEXT_PERCENT`, `CHARS_PER_TOKEN`, `DEFAULT_CHAR_BUDGET`, `MAX_LISTING_DESC_CHARS`, `MIN_DESC_LENGTH` |
| `src/tools/SkillTool/constants.ts` | 1 | `SKILL_TOOL_NAME = 'Skill'` |
| `src/tools/SkillTool/UI.tsx` | 127 | Ink renderers for `Skill` invocation: `renderToolUseMessage`, `renderToolUseProgressMessage` (caps at 3 messages), `renderToolResultMessage` (forked vs inline byline), error/rejected fallbacks |
| `src/skills/loadSkillsDir.ts` | 1086 | `parseSkillFrontmatterFields`; `createSkillCommand`; `loadSkillsFromSkillsDir`; `loadSkillsFromCommandsDir` (legacy); `transformSkillFiles`; `getSkillDirCommands` (memoized merge of managed/user/project/--add-dir/legacy with realpath dedup); conditional-skill state; dynamic skill discovery (`discoverSkillDirsForPaths`, `addSkillDirectories`, `activateConditionalSkillsForPaths`); `clearSkillCaches`; `clearDynamicSkills`; `getSkillsPath`; `estimateSkillFrontmatterTokens`; `onDynamicSkillsLoaded`; bottom-of-file write-once registration of MCP builders |
| `src/skills/bundledSkills.ts` | 220 | `BundledSkillDefinition` type; `registerBundledSkill`; `getBundledSkills` / `clearBundledSkills`; `getBundledSkillExtractDir`; secure on-disk extraction of `files: Record<string,string>` (umask-resistant `O_NOFOLLOW`/`O_EXCL`/`0o600`); `prependBaseDir` |
| `src/skills/bundled/index.ts` | 79 | `initBundledSkills()` — unconditional registrations + flag-gated registrations: `KAIROS \|\| KAIROS_DREAM`, `REVIEW_ARTIFACT`, `AGENT_TRIGGERS`, `AGENT_TRIGGERS_REMOTE`, `BUILDING_CLAUDE_APPS`, `shouldAutoEnableClaudeInChrome`, `RUN_SKILL_GENERATOR` |
| `src/skills/mcpSkillBuilders.ts` | 44 | Write-once leaf registry that lets `mcpSkills.ts` (in `src/services/mcp/`) call into `loadSkillsDir.ts` without forming an import cycle. Registered at `loadSkillsDir.ts:1083-1086` |
| `src/utils/hooks/skillImprovement.ts` | 267 | `initSkillImprovement` (registers a post-sampling hook iff `SKILL_IMPROVEMENT && tengu_copper_panda` GrowthBook flag); `createSkillImprovementHook` (every 5th user turn analyses recent transcript against the active project skill); `applySkillImprovement` (out-of-band rewrite of SKILL.md via `getSmallFastModel`) |
| `src/utils/suggestions/skillUsageTracking.ts` | 55 | `recordSkillUsage` with 60s in-process debounce + persisted `skillUsage[name].{usageCount,lastUsedAt}`; `getSkillUsageScore` with 7-day half-life decay floored at 0.1× |

### 2.2 Adjacent owners (not duplicated here)

| Concern | Spec | Anchor |
|---|---|---|
| `loadAllCommands` ordering, `findCommand`, command-source merge | 20 | `commands.ts:449-470` |
| `processPromptSlashCommand`, `getMessagesForPromptSlashCommand` (the inline-execution machinery the SkillTool dispatches into) | 20 | `utils/processUserInput/processSlashCommand.ts` |
| `parseFrontmatter`, `coerceDescriptionToString`, `splitPathInFrontmatter` | 20 | `utils/frontmatterParser.ts` |
| Permission rule evaluation, settings sources, deny/allow ordering | 09 | `utils/permissions/permissions.ts` |
| `runAgent`, sub-agent lifecycle, `prepareForkedCommandContext` | 14 | `tools/AgentTool/runAgent.ts`, `utils/forkedAgent.ts` |
| `addInvokedSkill`, `clearInvokedSkillsForAgent`, `getInvokedSkillsForAgent` (post-compact survival) | 29 | `bootstrap/state.ts` |
| MCP prompt → skill conversion, `MCP_SKILLS` plumbing on the MCP side | 23 | `services/mcp/client.ts:1392, 1670, 2174, 2348` |
| ToolSearch / DiscoverSkills | 12 | `tools/DiscoverSkillsTool/`; `services/skillSearch/*` (DCE'd in non-experimental builds — see §2.3) |
| `getBundledSkillsRoot` (per-process nonced parent) | 28 (plugins) / 11 | `utils/permissions/filesystem.ts` |

### 2.3 Conditional / absent paths (DCE'd in this leak)

The leaked tree does not contain the following directories that `SkillTool.ts:108-116`, `commands.ts:96`, and `query.ts:66` `require()` under `feature('EXPERIMENTAL_SKILL_SEARCH')`. This is expected: Bun's `bun:bundle` strips inactive `feature()` branches at build time, and the leak ships the non-experimental build.

| Required path | Caller | Symbol(s) used |
|---|---|---|
| `src/services/skillSearch/remoteSkillState.js` | `SkillTool.ts:110` | `stripCanonicalPrefix`, `getDiscoveredRemoteSkill` |
| `src/services/skillSearch/remoteSkillLoader.js` | `SkillTool.ts:111` | `loadRemoteSkill` |
| `src/services/skillSearch/telemetry.js` | `SkillTool.ts:112` | `logRemoteSkillLoaded` |
| `src/services/skillSearch/featureCheck.js` | `SkillTool.ts:113` | `isSkillSearchEnabled` |
| `src/tools/DiscoverSkillsTool/prompt.js` | `constants/prompts.ts:90-92` | `DISCOVER_SKILLS_TOOL_NAME` |
| `src/skills/bundled/dream.js` | `skills/bundled/index.ts:37` | `registerDreamSkill` |
| `src/skills/bundled/hunter.js` | `skills/bundled/index.ts:43` | `registerHunterSkill` |
| `src/skills/bundled/loop.js` | `skills/bundled/index.ts:49` | `registerLoopSkill` |
| `src/skills/bundled/scheduleRemoteAgents.js` | `skills/bundled/index.ts:59-60` | `registerScheduleRemoteAgentsSkill` |
| `src/skills/bundled/claudeApi.js` | `skills/bundled/index.ts:66` | `registerClaudeApiSkill` |
| `src/skills/bundled/runSkillGenerator.js` | `skills/bundled/index.ts:75` | `registerRunSkillGeneratorSkill` |

These are listed in the master Missing-Source Ledger (00 §2.5).

### 2.4 Source coverage

| Source | Read fully | Notes |
|---|---|---|
| `tools/SkillTool/SkillTool.ts` | ✅ | All 1108 lines |
| `tools/SkillTool/prompt.ts` | ✅ | All 241 lines |
| `tools/SkillTool/constants.ts` | ✅ | 1 line |
| `tools/SkillTool/UI.tsx` | ✅ | All 127 lines |
| `skills/loadSkillsDir.ts` | ✅ | All 1086 lines |
| `skills/bundledSkills.ts` | ✅ | All 220 lines |
| `skills/bundled/index.ts` | ✅ | All 79 lines |
| `skills/mcpSkillBuilders.ts` | ✅ | All 44 lines |
| `utils/hooks/skillImprovement.ts` | ✅ | All 267 lines |
| `utils/suggestions/skillUsageTracking.ts` | ✅ | All 55 lines |
| `commands.ts` skill exports | ✅ | Lines 449-605 (relevant subset) |

---

## 3. Public Interface (Contract)

### 3.1 Skill Tool (model-facing)

| Surface | Source | Shape |
|---|---|---|
| `name` | `constants.ts:1` | literal `'Skill'` |
| `searchHint` | `SkillTool.ts:333` | `'invoke a slash-command skill'` |
| `maxResultSizeChars` | `SkillTool.ts:334` | `100_000` |
| `inputSchema` | `SkillTool.ts:291-298` | `{ skill: string, args?: string }` (Zod, lazy) |
| `outputSchema` | `SkillTool.ts:301-326` | discriminated union: `inline` (`{ success, commandName, allowedTools?, model?, status?: 'inline' }`) ∪ `forked` (`{ success, commandName, status: 'forked', agentId, result }`) |
| `description({skill})` | `SkillTool.ts:342` | `Execute skill: ${skill}` |
| `prompt()` | `SkillTool.ts:344` | `getPrompt(getProjectRoot())` (memoized at `prompt.ts:173`) |
| `toAutoClassifierInput({skill})` | `SkillTool.ts:352` | `skill ?? ''` (records that the skill fired; downstream tool calls in the expanded prompt are classified separately) |
| `validateInput`, `checkPermissions`, `call` | `SkillTool.ts:354-841` | see §5 |
| `mapToolResultToToolResultBlockParam` | `SkillTool.ts:843-862` | inline → `Launching skill: <name>`; forked → `Skill "<name>" completed (forked execution).\n\nResult:\n<result>` |

### 3.2 Skill loading (intra-CLI)

| Export | File:Line | Role |
|---|---|---|
| `getSkillDirCommands(cwd)` | `loadSkillsDir.ts:638` | Memoized merge: managed → user → project (cwd-up-to-home) → --add-dir → legacy `/commands/`, with realpath-based dedup |
| `clearSkillCaches()` | `loadSkillsDir.ts:806` | Clears the above cache + `loadMarkdownFilesForSubdir` + conditional state |
| `getSkillsPath(source, dir)` | `loadSkillsDir.ts:78` | Derives the canonical disk path for a given settings source + `'skills' \| 'commands'` subdir |
| `discoverSkillDirsForPaths(filePaths, cwd)` | `loadSkillsDir.ts:861` | Walks each path's parents up to (but not including) cwd, collecting newly discovered `.claude/skills/` dirs (caches misses too); skips gitignored containers; sorts deepest-first |
| `addSkillDirectories(dirs)` | `loadSkillsDir.ts:923` | Loads and merges dynamic skills (deeper-path overrides shallower); fires `skillsLoaded` signal |
| `activateConditionalSkillsForPaths(filePaths, cwd)` | `loadSkillsDir.ts:997` | Promotes `paths`-gated skills to dynamic when a file path matches (gitignore-style); idempotent within a session |
| `getDynamicSkills()` | `loadSkillsDir.ts:981` | Snapshot of dynamic registry |
| `getConditionalSkillCount()` | `loadSkillsDir.ts:1063` | Test/debug |
| `clearDynamicSkills()` | `loadSkillsDir.ts:1070` | Test |
| `onDynamicSkillsLoaded(cb)` | `loadSkillsDir.ts:839` | Listener, wraps each cb in try/catch (matches `growthbook.ts` pattern) |
| `parseSkillFrontmatterFields` | `loadSkillsDir.ts:185` | Pure parser shared with MCP-skill builders via `mcpSkillBuilders.ts` |
| `createSkillCommand` | `loadSkillsDir.ts:270` | Pure factory shared with MCP-skill builders via `mcpSkillBuilders.ts` |
| `estimateSkillFrontmatterTokens(skill)` | `loadSkillsDir.ts:100` | name + description + whenToUse only (full SKILL.md is loaded only on invocation) |
| `registerBundledSkill(def)` | `bundledSkills.ts:53` | Mutates module-local `bundledSkills` |
| `getBundledSkills()` / `clearBundledSkills()` | `bundledSkills.ts:106, 113` | |
| `getBundledSkillExtractDir(name)` | `bundledSkills.ts:120` | `<bundledSkillsRoot>/<name>` |
| `initBundledSkills()` | `bundled/index.ts:24` | Boot-time call |
| `getMcpSkillCommands(mcpCommands)` | `commands.ts:547-559` | Filters AppState's `mcp.commands` to prompt-type, not-disable-model-invocation, `loadedFrom === 'mcp'`. Returns `[]` when `MCP_SKILLS` is off. **Used by the listing attachment path only** (`utils/attachments.ts:2677`), NOT by `SkillTool.getAllCommands` (which performs its own equivalent filter inline). |
| `getSkillToolCommands(cwd)` | `commands.ts:563-581` | Memoized; what the **listing attachment** enumerates (`utils/attachments.ts:2676`). Reads only local `getCommands(cwd)` — MCP skills are spliced in by the attachment helper later. |
| `getSlashCommandToolSkills(cwd)` | `commands.ts:586-608` | Memoized; what context-assembly counts as "skills" for the system prompt — strictly tighter than `getSkillToolCommands` (e.g., excludes `commands_DEPRECATED`; requires `hasUserSpecifiedDescription \|\| whenToUse` even for bundled). |
| `recordSkillUsage(name)` | `skillUsageTracking.ts:13` | Called from `SkillTool.ts:619, 1059` |
| `getSkillUsageScore(name)` | `skillUsageTracking.ts:44` | Consumed by `commandSuggestions.ts` |
| `initSkillImprovement()` | `skillImprovement.ts:175` | Conditional registration |
| `applySkillImprovement(name, updates)` | `skillImprovement.ts:188` | Side-channel SKILL.md rewrite |

### 3.3 Internal contract preserved across calls

- After a successful inline invocation, SkillTool **returns** rather than runs anything; the messages it emits via `newMessages` are picked up by the turn pipeline, which re-enters QueryEngine with the expanded skill prompt (see 04, 03).
- `addInvokedSkill` and `registerSkillHooks` are called inside `processPromptSlashCommand` (via `getMessagesForPromptSlashCommand`) — **NOT** by SkillTool. Calling them again here would double-register (`SkillTool.ts:761-764`).
- For remote canonical skills, SkillTool **does** call `addInvokedSkill` itself (no slash-command expansion path) using the *transformed* `finalContent` (header + substitutions) so post-compact restoration is faithful (`SkillTool.ts:1086-1093`).

---

## 4. Data Model & State

### 4.1 `Command` shape produced by `createSkillCommand`

```
type: 'prompt'
name                           // skill key (slugified dir name, possibly namespaced 'sub:dir')
description                    // frontmatter or first-line fallback
hasUserSpecifiedDescription
allowedTools[]                 // parsed from frontmatter['allowed-tools']
argumentHint
argNames[]                     // from frontmatter.arguments
whenToUse                      // from frontmatter.when_to_use
version
model                          // parsed via parseUserSpecifiedModel; 'inherit' → undefined
disableModelInvocation
userInvocable                  // default true; if false → isHidden
context: 'inline' | 'fork'     // from frontmatter.context
agent                          // from frontmatter.agent
effort: EffortValue            // from frontmatter.effort, validated against EFFORT_LEVELS
paths[]                        // gitignore-style; presence => conditional skill
contentLength
isHidden = !userInvocable
progressMessage = 'running'
userFacingName(): string       // displayName (frontmatter.name) || skillName
source: 'bundled'|'plugin'|'project'|'user'|'managed'|'builtin'|...
loadedFrom: 'commands_DEPRECATED'|'skills'|'plugin'|'managed'|'bundled'|'mcp'
hooks: HooksSettings | undefined
skillRoot: baseDir | undefined  // physical directory for ${CLAUDE_SKILL_DIR} resolution
getPromptForCommand(args, ctx): Promise<ContentBlockParam[]>
```

(`loadSkillsDir.ts:317-401`)

### 4.2 In-memory registries

| Registry | Storage | Mutators | Reset |
|---|---|---|---|
| `bundledSkills: Command[]` | `bundledSkills.ts:44` | `registerBundledSkill`, `clearBundledSkills` | test only |
| `dynamicSkillDirs: Set<string>` | `loadSkillsDir.ts:821` | `discoverSkillDirsForPaths` | `clearDynamicSkills` |
| `dynamicSkills: Map<name,Command>` | `loadSkillsDir.ts:822` | `addSkillDirectories`, `activateConditionalSkillsForPaths` | `clearDynamicSkills` |
| `conditionalSkills: Map<name,Command>` | `loadSkillsDir.ts:827` | `getSkillDirCommands` (populates), `activateConditionalSkillsForPaths` (drains) | `clearSkillCaches`, `clearDynamicSkills` |
| `activatedConditionalSkillNames: Set<string>` | `loadSkillsDir.ts:829` | `activateConditionalSkillsForPaths` | `clearDynamicSkills` (NOT `clearSkillCaches` until names race in) |
| `skillsLoaded` signal | `loadSkillsDir.ts:832` | emits in `addSkillDirectories`, `activateConditionalSkillsForPaths` | n/a |
| `lastWriteBySkill: Map<string,number>` | `skillUsageTracking.ts:7` | `recordSkillUsage` (60s debounce) | process exit |
| `skillUsage` (persisted) | global config; written by `saveGlobalConfig` | `recordSkillUsage` | global config rewrites |
| `skillImprovement.suggestion` (AppState) | `skillImprovement.ts:160-166` | the post-sampling hook | applied by user UI flow (29) |

### 4.3 Telemetry-only state inside SkillTool

| Field | Source | Routing |
|---|---|---|
| `command_name` | sanitized: `'custom'` for non-built-in/non-bundled/non-official-marketplace | redacted column |
| `_PROTO_skill_name` | the actual name | privileged BQ column (PII-tagged) |
| `execution_context` | `'inline' \| 'fork' \| 'remote'` | redacted |
| `invocation_trigger` | `queryDepth > 0 ? 'nested-skill' : 'claude-proactive'` | redacted |
| `query_depth`, `parent_agent_id` | from context | redacted |
| `was_discovered` | only when `EXPERIMENTAL_SKILL_SEARCH && isSkillSearchEnabled()`; from `context.discoveredSkillNames` | redacted |
| `is_remote`, `remote_cache_hit`, `remote_load_latency_ms` | only on remote path | redacted |
| `skill_name`, `skill_source`, `skill_loaded_from`, `skill_kind`, `remote_slug` | only when `USER_TYPE === 'ant'` | redacted (ANT-only) |
| `_PROTO_plugin_name`, `_PROTO_marketplace_name` | only when `pluginInfo` present | privileged |

(`SkillTool.ts:152-203, 675-726, 1029-1057`)

---

## 5. Algorithm / Control Flow

### 5.1 Skill listing assembly (per turn)

The SkillTool's own `prompt()` is **fixed instruction text only** (memoized at `prompt.ts:173-196`); it does NOT contain the enumerated skill list. The per-turn enumeration is delivered as `skill_listing` **attachments** by `src/utils/attachments.ts:2661-2751` (owned by **05**, cited here). High-level flow:

1. `getSkillListingAttachments(toolUseContext)` is invoked from the attachment maybe-pipeline (`attachments.ts:875`).
2. Skip if `NODE_ENV === 'test'`, or the agent's tool list does not contain `Skill` (`attachments.ts:2664-2673`).
3. Compose `allCommands = uniqBy([...getSkillToolCommands(cwd), ...getMcpSkillCommands(appState.mcp.commands)], 'name')` (`attachments.ts:2676-2683`).
4. If `EXPERIMENTAL_SKILL_SEARCH` is on AND `skillSearchModules.featureCheck.isSkillSearchEnabled()` is true, replace `allCommands` with `filterToBundledAndMcp(...)` (turn-0 gap: bundled + MCP only; user/project/plugin skills go through skill-discovery instead) (`attachments.ts:2692-2697`).
5. Per-agent suppression: a module-scope `sentSkillNames: Map<agentKey, Set<name>>` tracks which skill names have already been emitted to each agent (`attachments.ts:2607, 2699-2704`). On `--resume` (suppressNext flag), mark all current skills as sent and emit nothing (`attachments.ts:2706-2715`).
6. `newSkills = allCommands.filter(cmd => !sent.has(cmd.name))`. If empty → no attachment. Otherwise `isInitial = sent.size === 0` (`attachments.ts:2717-2725`).
7. Format with `formatCommandsWithinBudget(newSkills, getContextWindowForModel(...))` (the SkillTool-owned renderer at `prompt.ts:31-171`) and emit one attachment of shape `{ type: 'skill_listing', content, skillCount, isInitial }` (`attachments.ts:2737-2750`).

The renderer (`formatCommandsWithinBudget`, owned by this spec) sizes the listing as follows (`prompt.ts:31-171`):

1. `getCharBudget(contextWindowTokens)`
   - If `process.env.SLASH_COMMAND_TOOL_CHAR_BUDGET` is numeric and truthy → use that (escape hatch).
   - Else if `contextWindowTokens` provided → `floor(contextWindowTokens × 4 × 0.01)` (1% of context, 4 chars/token).
   - Else → `8_000` fallback (1% of 200K × 4).
2. Per-entry hard cap: `MAX_LISTING_DESC_CHARS = 250` (frontmatter `whenToUse` is appended as `{description} - {whenToUse}` then truncated with ellipsis).
3. Try full descriptions first (sum of `stringWidth(entry) + (n-1)` newlines).
4. If over budget: partition into **bundled** (`source === 'bundled'`) and **rest**. Bundled entries are *never* truncated — their full width is reserved.
5. Compute `availableForDescs = remainingBudget - restNameOverhead - (n-1)` and `maxDescLen = floor(availableForDescs / restCount)`.
6. If `maxDescLen < MIN_DESC_LENGTH (= 20)` → render rest as `- {name}` only (names-only mode), bundled keep full descriptions.
7. Else → truncate rest descriptions to `maxDescLen`. Truncation count and budget are logged via `tengu_skill_descriptions_truncated` (ANT-only).

### 5.2 Per-invocation pipeline (inline path)

```
SkillTool.call({skill, args}, context, canUseTool, parentMessage, onProgress):
  trimmed = skill.trim(); strip leading '/'; commandName = …

  # Remote canonical path (ANT + EXPERIMENTAL_SKILL_SEARCH only)
  if feature('EXPERIMENTAL_SKILL_SEARCH') and USER_TYPE == 'ant':
      slug = stripCanonicalPrefix(commandName)
      if slug != null:
          return executeRemoteSkill(slug, commandName, parentMessage, context)
          # See §5.4

  commands = getAllCommands(context)                      # SkillTool.ts:81-94
  command  = findCommand(commandName, commands)
  recordSkillUsage(commandName)

  if command.context == 'fork':
      return executeForkedSkill(...)                       # §5.3

  processed = await processPromptSlashCommand(             # 20
      commandName, args || '', commands, context)
  if !processed.shouldQuery: throw 'Command processing failed'

  emit tengu_skill_tool_invocation(execution_context='inline', …)

  toolUseID = getToolUseIDFromParentMessage(parentMessage, 'Skill')
  newMessages = tagMessagesWithToolUseID(
      processed.messages
        .filter(m => m.type != 'progress')
        .filter(m => not (m.type == 'user' and content includes <{COMMAND_MESSAGE_TAG}>)),
      toolUseID)

  return {
    data: { success: true, commandName,
            allowedTools: processed.allowedTools.length>0 ? processed.allowedTools : undefined,
            model: processed.model },
    newMessages,
    contextModifier(ctx):
      modified = ctx
      if processed.allowedTools.length > 0:
        chain previousGetAppState; in the new getAppState merge
          allowedTools into toolPermissionContext.alwaysAllowRules.command (Set-deduped)
      if processed.model:
        modified.options.mainLoopModel = resolveSkillModelOverride(model, ctx.options.mainLoopModel)
        # carries [1m] suffix to avoid downgrading a 1M-window session to 200K
      if command.effort != undefined:
        chain previousGetAppState; new appState.effortValue = command.effort
      return modified
  }
```

(`SkillTool.ts:580-841`)

The `contextModifier` builds *chained* `getAppState` closures so multiple modifications compose correctly. Each layer captures `previousGetAppState` from the layer below it, not from the original `context.getAppState`, so subsequent middleware can re-modify on top (`SkillTool.ts:780-836`).

### 5.3 Forked execution

```
executeForkedSkill(command, name, args, context, canUseTool, parent, onProgress):
  agentId = createAgentId()
  emit tengu_skill_tool_invocation(execution_context='fork', …)

  { modifiedGetAppState, baseAgent, promptMessages, skillContent }
      = prepareForkedCommandContext(command, args || '', context)   # 14

  agentDef = command.effort != undefined
    ? { ...baseAgent, effort: command.effort }
    : baseAgent

  agentMessages: Message[] = []
  try:
    for await msg in runAgent({                                      # 14
        agentDefinition, promptMessages,
        toolUseContext: { ...context, getAppState: modifiedGetAppState },
        canUseTool, isAsync: false, querySource: 'agent:custom',
        model: command.model as ModelAlias?, availableTools: ctx.options.tools,
        override: { agentId } }):
      agentMessages.push(msg)
      if onProgress and msg has tool_use|tool_result:
        for normalized in normalizeMessages([msg]):
          onProgress({
            toolUseID: `skill_${parentMessage.message.id}`,
            data: { message: normalized, type: 'skill_progress',
                    prompt: skillContent, agentId } })
    resultText = extractResultText(agentMessages, 'Skill execution completed')
    agentMessages.length = 0   # release memory
    return { data: { success: true, commandName: name,
                     status: 'forked', agentId, result: resultText } }
  finally:
    clearInvokedSkillsForAgent(agentId)
```

(`SkillTool.ts:122-289`)

### 5.4 Remote canonical execution (ANT-only experimental)

```
executeRemoteSkill(slug, commandName, parentMessage, context):
  meta = getDiscoveredRemoteSkill(slug)
  if !meta: throw 'Remote skill … was not discovered in this session …'

  urlScheme = extractUrlScheme(meta.url)   # 'gs'|'http'|'https'|'s3'; default 'gs'
  try:
    loadResult = await loadRemoteSkill(slug, meta.url)
  catch e:
    logRemoteSkillLoaded({ slug, cacheHit: false, latencyMs: 0, urlScheme, error: errorMessage(e) })
    throw `Failed to load remote skill ${slug}: …`

  { cacheHit, latencyMs, skillPath, content, fileCount, totalBytes, fetchMethod } = loadResult
  logRemoteSkillLoaded({ slug, cacheHit, latencyMs, urlScheme, fileCount, totalBytes, fetchMethod })
  emit tengu_skill_tool_invocation(execution_context='remote', is_remote: true,
                                   was_discovered: true, remote_cache_hit, remote_load_latency_ms, …)
  recordSkillUsage(commandName)

  { content: bodyContent } = parseFrontmatter(content, skillPath)   # strip --- yaml ---
  skillDir = dirname(skillPath)
  normalizedDir = win32 ? skillDir.replace(/\\/g,'/') : skillDir
  finalContent = `Base directory for this skill: ${normalizedDir}\n\n${bodyContent}`
  finalContent = finalContent.replace(/\$\{CLAUDE_SKILL_DIR\}/g, normalizedDir)
                              .replace(/\$\{CLAUDE_SESSION_ID\}/g, getSessionId())

  addInvokedSkill(commandName, skillPath, finalContent, getAgentContext()?.agentId ?? null)

  toolUseID = getToolUseIDFromParentMessage(parentMessage, 'Skill')
  return {
    data: { success: true, commandName, status: 'inline' },
    newMessages: tagMessagesWithToolUseID(
        [createUserMessage({ content: finalContent, isMeta: true })], toolUseID)
  }
```

(`SkillTool.ts:957-1108`)

Note: remote canonical skills **do not** run `processPromptSlashCommand`; they are declarative SKILL.md only — no `!\`...\`` shell blocks, no `$ARGUMENTS` interpolation. Comment at `SkillTool.ts:600-604`.

### 5.5 `validateInput` decision tree

```
trimmed = skill.trim()
if !trimmed: → result:false, errorCode:1, 'Invalid skill format: …'
hasLeadingSlash = trimmed.startsWith('/')
if hasLeadingSlash: emit tengu_skill_tool_slash_prefix
normalizedCommandName = strip-leading-slash

if EXPERIMENTAL_SKILL_SEARCH and USER_TYPE=='ant':
   slug = stripCanonicalPrefix(normalizedCommandName)
   if slug != null:
      meta = getDiscoveredRemoteSkill(slug)
      if !meta: → result:false, errorCode:6, 'Remote skill … was not discovered …'
      else: → result:true                # loading deferred to call()

commands = getAllCommands(context)
found = findCommand(normalizedCommandName, commands)
if !found:                          → result:false, errorCode:2, 'Unknown skill: …'
if found.disableModelInvocation:    → result:false, errorCode:4, '… cannot be used with Skill tool due to disable-model-invocation'
if found.type != 'prompt':          → result:false, errorCode:5, '… is not a prompt-based skill'
return result:true
```

errorCodes: `1` (empty), `2` (unknown), `4` (disabled), `5` (not prompt), `6` (remote not discovered). There is no `3`. (`SkillTool.ts:354-430`)

### 5.6 `checkPermissions` decision tree

```
commandName = strip leading '/'
permissionContext = appState.toolPermissionContext
commandObj = findCommand(commandName, getAllCommands(context))

ruleMatches(rc):
   normalizedRule = strip leading '/' of rc
   if normalizedRule == commandName:                return true    # exact
   if normalizedRule endsWith ':*':                                # prefix
       prefix = normalizedRule.slice(0, -2)
       return commandName.startsWith(prefix)
   return false

denyRules = getRuleByContentsForTool(permCtx, SkillTool, 'deny')
for [rc, rule] in denyRules:
   if ruleMatches(rc): return { behavior:'deny',
                                 message:'Skill execution blocked by permission rules',
                                 decisionReason:{type:'rule', rule} }

if EXPERIMENTAL_SKILL_SEARCH and USER_TYPE=='ant':
   slug = stripCanonicalPrefix(commandName)
   if slug != null: return { behavior:'allow', updatedInput:{skill, args}, decisionReason: undefined }
   # placed AFTER deny so user-configured Skill(_canonical_:*) deny still applies

allowRules = getRuleByContentsForTool(permCtx, SkillTool, 'allow')
for [rc, rule] in allowRules:
   if ruleMatches(rc): return { behavior:'allow', updatedInput, decisionReason:{type:'rule', rule} }

if commandObj?.type == 'prompt' and skillHasOnlySafeProperties(commandObj):
   return { behavior:'allow', updatedInput, decisionReason: undefined }

# ask
suggestions = [
  { type:'addRules', behavior:'allow', destination:'localSettings',
    rules: [{ toolName:'Skill', ruleContent: commandName }] },
  { type:'addRules', behavior:'allow', destination:'localSettings',
    rules: [{ toolName:'Skill', ruleContent: `${commandName}:*` }] }
]
return { behavior:'ask', message: `Execute skill: ${commandName}`,
         decisionReason: undefined, suggestions, updatedInput,
         metadata: commandObj ? { command: commandObj } : undefined }
```

(`SkillTool.ts:432-578`)

`skillHasOnlySafeProperties` iterates the command's keys; any key NOT in the `SAFE_SKILL_PROPERTIES` allowlist with a "meaningful" value (not `undefined`/`null`/empty array/empty object) demotes the command to `behavior:'ask'`. Empty arrays and empty objects are treated as not meaningful (`SkillTool.ts:910-933`). The allowlist intentionally defaults new fields to "requires permission" until explicitly added.

### 5.7 Skill-listing inclusion order — cite

`loadAllCommands` returns sources in the fixed order `bundledSkills, builtinPluginSkills, skillDirCommands, workflowCommands, pluginCommands, pluginSkills, COMMANDS()` (`commands.ts:460-468`; spec **20**). Note: built-in plugin skills (`builtinPluginSkills`) are converted by `skillDefinitionToCommand` with `source: 'bundled', loadedFrom: 'bundled'` (`plugins/builtinPlugins.ts:145-150`); they intentionally masquerade as bundled for listing/telemetry/truncation-exemption purposes — they are NOT `loadedFrom:'plugin'`. After ordering, `getCommands(cwd)` filters by `meetsAvailabilityRequirement` and `isCommandEnabled`, then **inserts dynamic skills** before the first built-in command (or appends if no built-ins) and dedupes against existing names (`commands.ts:476-516`).

Two distinct helpers consume `getCommands` for different purposes; **collisions and inclusion rules differ**:

| Helper | Site | Inclusion rule |
|---|---|---|
| `getSkillToolCommands(cwd)` | SkillTool listing path (via `attachments.ts:2676`) | `type === 'prompt' && !disableModelInvocation && source !== 'builtin' && (loadedFrom ∈ {bundled, skills, commands_DEPRECATED} \|\| hasUserSpecifiedDescription \|\| whenToUse)`. Includes legacy `commands_DEPRECATED` unconditionally. |
| `getSlashCommandToolSkills(cwd)` | 03/05 system-prompt builders | `type === 'prompt' && source !== 'builtin' && (hasUserSpecifiedDescription \|\| whenToUse) && (loadedFrom ∈ {skills, plugin, bundled} \|\| disableModelInvocation)`. **Excludes** `commands_DEPRECATED` and **excludes** `bundled` legacy entries lacking description. |

MCP skills are spliced separately:
- **In the listing attachment**: `getMcpSkillCommands(appState.mcp.commands)` is called from `attachments.ts:2677-2683` and concatenated with the local list, then `uniqBy('name')` dedups. Returns `[]` when `MCP_SKILLS` is off.
- **In SkillTool model-invocation lookup**: `SkillTool.getAllCommands()` filters `appState.mcp.commands` directly for `type==='prompt' && loadedFrom==='mcp'` (NOT calling `getMcpSkillCommands`); locals come first in `uniqBy`, so a local skill with the same name wins over an MCP skill. The `MCP_SKILLS` feature gate is enforced upstream by the MCP client when populating `appState.mcp.commands`, not by `getAllCommands` itself (`SkillTool.ts:81-94`).

With `EXPERIMENTAL_SKILL_SEARCH` ON, the per-turn surface is *replaced/augmented* by skill-discovery attachments emitted by `query.ts:66`'s `skillPrefetch`, owned by 12.

### 5.8 Skill filesystem walk (`getSkillDirCommands`)

```
userSkillsDir   = `<CLAUDE_CONFIG_HOME>/skills`
managedSkillsDir = `<MANAGED_FILE_PATH>/.claude/skills`
projectSkillsDirs = getProjectDirsUpToHome('skills', cwd)

skillsLocked = isRestrictedToPluginOnly('skills')
projectSettingsEnabled = isSettingSourceEnabled('projectSettings') && !skillsLocked
additionalDirs = getAdditionalDirectoriesForClaudeMd()

# --bare mode short-circuit:
if isBareMode():
  if additionalDirs.empty or !projectSettingsEnabled: return []
  load only `<dir>/.claude/skills` for each --add-dir; no dedup, return.

[managed, user, projectN, additionalN, legacy] = await Promise.all([
   CLAUDE_CODE_DISABLE_POLICY_SKILLS truthy ? [] : load(managedSkillsDir, 'policySettings'),
   isSettingSourceEnabled('userSettings') && !skillsLocked
       ? load(userSkillsDir, 'userSettings') : [],
   projectSettingsEnabled ? Promise.all(projectSkillsDirs.map(d => load(d,'projectSettings'))) : [],
   projectSettingsEnabled ? Promise.all(additionalDirs.map(d => load(`${d}/.claude/skills`,'projectSettings'))) : [],
   skillsLocked ? [] : loadSkillsFromCommandsDir(cwd)
])

allWithPaths = [ ...managed, ...user, ...projectN.flat(), ...additionalN.flat(), ...legacy ]
fileIds = Promise.all(allWithPaths.map(({skill, filePath}) =>
            skill.type=='prompt' ? realpath(filePath) : null))
seenFileIds: Map<inode, source>
deduplicated: Command[]
for i, entry in enumerate(allWithPaths):
   if entry?.skill.type != 'prompt': continue
   id = fileIds[i]
   if id == null: deduplicated.push(skill); continue
   if seenFileIds.has(id):
      logForDebugging(`Skipping duplicate skill '${name}' from ${source} (same file already loaded from ${existingSource})`)
      continue
   seenFileIds.set(id, skill.source)
   deduplicated.push(skill)

# Conditional skills with `paths:` are split out unless already activated
unconditional, newConditional = partition(deduplicated, skill =>
   skill.paths?.length>0 && !activatedConditionalSkillNames.has(skill.name))
for s in newConditional: conditionalSkills.set(s.name, s)
return unconditional
```

(`loadSkillsDir.ts:638-804`)

A `/skills/` directory only loads `<entry>/SKILL.md` (single .md files at top level are skipped; `loadSkillsDir.ts:425-428`). Legacy `/commands/` supports both single .md and `<dir>/SKILL.md`; when both are present in the same dir, the SKILL.md wins and the dir name becomes the command name (`loadSkillsDir.ts:493-521`).

### 5.9 Dynamic skill discovery on file ops

After every file-touching tool, the orchestrator calls `discoverSkillDirsForPaths(touchedPaths, cwd)` and then `addSkillDirectories(returnedDirs)` and `activateConditionalSkillsForPaths(touchedPaths, cwd)`. The walk:
1. Starts from `dirname(filePath)` and walks parents while `currentDir.startsWith(resolvedCwd + sep)` (cwd-level skills are loaded at startup, so cwd is excluded; `+ sep` prevents `/project-backup` matching `/project`).
2. For each ancestor `currentDir`, the candidate is `${currentDir}/.claude/skills`. The set `dynamicSkillDirs` records both hits *and* misses to avoid repeated `stat`s on every Read/Write/Edit.
3. If `stat` succeeds and the **container** dir is gitignored (`isPathGitignored`), the candidate is skipped. Outside a git repo, `git check-ignore` exits 128 → returns false → skill loads. Trust dialog at invocation is the actual security boundary (`loadSkillsDir.ts:876-901`).
4. Sort by path-component depth descending so deeper skills override shallower in `addSkillDirectories`.

### 5.10 Conditional-skill activation (`paths:` frontmatter)

```
for [name, skill] in conditionalSkills:
   if skill.paths is missing/empty: continue
   ig = ignore().add(skill.paths)
   for filePath in filePaths:
      relativePath = isAbsolute(filePath) ? relative(cwd, filePath) : filePath
      if !relativePath or relativePath.startsWith('..') or isAbsolute(relativePath): continue
      if ig.ignores(relativePath):
         dynamicSkills.set(name, skill)
         conditionalSkills.delete(name)
         activatedConditionalSkillNames.add(name)
         activated.push(name); break
emit tengu_dynamic_skills_changed(source:'conditional_paths', ...)
emit skillsLoaded
```

`paths` patterns ending in `/**` are stripped to bare path (the `ignore` library matches both the path and its contents already; `loadSkillsDir.ts:165-178`). Patterns equal to `**` (match-all) are *removed*, and if all patterns are `**` the skill is treated as having no paths (i.e., unconditional).

### 5.11 Bundled-skill on-disk extraction

When `BundledSkillDefinition.files` is non-empty, `registerBundledSkill` swaps `getPromptForCommand` with a wrapper that lazily extracts those files into `getBundledSkillExtractDir(name) = <bundledSkillsRoot>/<name>` on first invocation, then `prependBaseDir`-prefixes the result so the model can read them with file tools. The extraction promise is memoized once per process to dedupe concurrent calls (`bundledSkills.ts:53-100, 131-145`).

Security:
- Parent dir created with `0o700`.
- Files written with `O_WRONLY|O_CREAT|O_EXCL|O_NOFOLLOW`, mode `0o600` (POSIX). Windows uses string flags `'wx'` (avoids libuv EINVAL on numeric `O_EXCL`).
- `EEXIST` deliberately **not** retried with unlink — `unlink()` follows intermediate symlinks.
- Path traversal: `resolveSkillFilePath` rejects absolute paths and `..` components in either separator (`bundledSkills.ts:170-205`).
- The defense is layered: `getBundledSkillsRoot()` itself uses a per-process **nonce** so an attacker who pre-creates the predictable parent loses; the explicit `O_NOFOLLOW`/`0o700`/`0o600` at this layer keep the nonced subtree owner-only even on `umask=0`.

### 5.12 `getPromptForCommand` body construction

For a file-loaded skill (`loadSkillsDir.ts:344-399`):
1. Prepend `Base directory for this skill: ${baseDir}\n\n` if `baseDir` is set.
2. `substituteArguments(content, args, true, argumentNames)` (positional `$ARGUMENTS` / named substitution).
3. Replace `${CLAUDE_SKILL_DIR}` with `baseDir` (Windows: replace `\\` → `/` first so the value doesn't act as a shell escape).
4. Replace `${CLAUDE_SESSION_ID}` with `getSessionId()`.
5. **If `loadedFrom !== 'mcp'`** → run `executeShellCommandsInPrompt(finalContent, …, '/' + skillName, shell)` to expand inline `!\`…\`` / fenced `\`\`\`! … \`\`\`` blocks. The `getAppState` passed in is patched so `alwaysAllowRules.command = allowedTools` for the duration of expansion. **MCP skills are remote and untrusted; their bodies never execute shell**.

### 5.13 `getCharBudget` / listing budget

| Constant | Value | Source |
|---|---|---|
| `SKILL_BUDGET_CONTEXT_PERCENT` | `0.01` | `prompt.ts:21` |
| `CHARS_PER_TOKEN` | `4` | `prompt.ts:22` |
| `DEFAULT_CHAR_BUDGET` | `8_000` | `prompt.ts:23` |
| `MAX_LISTING_DESC_CHARS` | `250` | `prompt.ts:29` |
| `MIN_DESC_LENGTH` | `20` | `prompt.ts:68` |
| `process.env.SLASH_COMMAND_TOOL_CHAR_BUDGET` | numeric override | `prompt.ts:32-34` |

### 5.14 Skill ranking algorithm

```
recordSkillUsage(name):
  if (lastWriteBySkill.get(name) ?? -inf) within 60_000 ms of now: return    # debounce
  lastWriteBySkill.set(name, now)
  saveGlobalConfig: skillUsage[name] = {
    usageCount: (existing?.usageCount ?? 0) + 1,
    lastUsedAt:  now }

getSkillUsageScore(name):
  usage = config.skillUsage?.[name]
  if !usage: return 0
  daysSinceUse  = (Date.now() - usage.lastUsedAt) / (1000*60*60*24)
  recencyFactor = 0.5 ** (daysSinceUse / 7)               # 7-day half-life
  return usage.usageCount * max(recencyFactor, 0.1)       # floor 0.1×
```

(`skillUsageTracking.ts:13-55`)

### 5.15 Skill-improvement post-sampling hook (gated)

Registered iff `feature('SKILL_IMPROVEMENT')` AND GrowthBook flag `tengu_copper_panda` is true (`skillImprovement.ts:175-182`).

Hook config (`skillImprovement.ts:68-173`):
- `name: 'skill_improvement'`.
- `shouldRun(ctx)` returns false unless: `ctx.querySource === 'repl_main_thread'`, AND `findProjectSkill()` returns one (an invokedSkill whose `skillPath` starts with `'projectSettings:'`), AND `userCount - lastAnalyzedCount >= TURN_BATCH_SIZE (= 5)`. On true it bumps `lastAnalyzedCount`.
- `buildMessages(ctx)` slices `messages.slice(lastAnalyzedIndex)`, advances `lastAnalyzedIndex`, and asks the small/fast model to emit `<updates>[…]</updates>` JSON describing skill-definition deltas (see §6 prompt).
- `useTools: false`. `getModel: getSmallFastModel`.
- `parseResponse` extracts the `<updates>` tag, JSON-parses; failures yield `[]`.
- `logResult` on non-empty success emits `tengu_skill_improvement_detected({ updateCount, uuid, _PROTO_skill_name })` and writes `appState.skillImprovement.suggestion = { skillName, updates }`.

`applySkillImprovement(name, updates)` reads `<cwd>/.claude/skills/<name>/SKILL.md`, calls `queryModelWithoutStreaming` (small/fast model, thinking disabled, temperature 0, no tools, no MCP, fresh abort controller) to rewrite the file inside `<updated_file>` tags, then writes back. Logs and returns silently on read/write/parse failure (`skillImprovement.ts:188-267`).

---

## 6. Verbatim Assets

### 6.1 SkillTool default prompt (memoized)

```ts
// prompt.ts:173-196
export const getPrompt = memoize(async (_cwd: string): Promise<string> => {
  return `Execute a skill within the main conversation

When users ask you to perform tasks, check if any of the available skills match. Skills provide specialized capabilities and domain knowledge.

When users reference a "slash command" or "/<something>" (e.g., "/commit", "/review-pr"), they are referring to a skill. Use this tool to invoke it.

How to invoke:
- Use this tool with the skill name and optional arguments
- Examples:
  - \`skill: "pdf"\` - invoke the pdf skill
  - \`skill: "commit", args: "-m 'Fix bug'"\` - invoke with arguments
  - \`skill: "review-pr", args: "123"\` - invoke with arguments
  - \`skill: "ms-office-suite:pdf"\` - invoke using fully qualified name

Important:
- Available skills are listed in system-reminder messages in the conversation
- When a skill matches the user's request, this is a BLOCKING REQUIREMENT: invoke the relevant Skill tool BEFORE generating any other response about the task
- NEVER mention a skill without actually calling this tool
- Do not invoke a skill that is already running
- Do not use this tool for built-in CLI commands (like /help, /clear, etc.)
- If you see a <${COMMAND_NAME_TAG}> tag in the current conversation turn, the skill has ALREADY been loaded - follow the instructions directly instead of calling this tool again
`
})
```

This file does NOT contain ANT-only branches; the prompt body is identical for all users. ANT-only deltas in the SkillTool live elsewhere (see §6.2).

### 6.2 ANT-only sites in `SkillTool.ts` (≥6 sites)

All gated by `process.env.USER_TYPE === 'ant'`, listed in source order:

| Line | Surface | Purpose |
|---|---|---|
| 171-184 | telemetry (forked) | Adds unredacted `skill_name`, `skill_source`, optional `skill_loaded_from`, `skill_kind` to the forked-execution event |
| 379 | `validateInput` | Combined with `EXPERIMENTAL_SKILL_SEARCH`: intercept `_canonical_<slug>` before local lookup |
| 494 | `checkPermissions` | Combined with `EXPERIMENTAL_SKILL_SEARCH`: auto-allow remote canonical skills (after deny rules) |
| 607 | `call` | Combined with `EXPERIMENTAL_SKILL_SEARCH`: dispatch to `executeRemoteSkill` |
| 694-709 | telemetry (inline) | Unredacted `skill_name`, optional `skill_source`, `skill_loaded_from`, `skill_kind` |
| 1051-1056 | telemetry (remote) | Unredacted `skill_name`, `remote_slug` |

`prompt.ts:125-136, 149-161` — ANT-only telemetry `tengu_skill_descriptions_truncated` (does NOT modify the prompt itself).

### 6.3 SkillTool input Zod schema (verbatim)

```ts
// SkillTool.ts:291-298
export const inputSchema = lazySchema(() =>
  z.object({
    skill: z
      .string()
      .describe('The skill name. E.g., "commit", "review-pr", or "pdf"'),
    args: z.string().optional().describe('Optional arguments for the skill'),
  }),
)
```

### 6.4 SkillTool output Zod schema (verbatim, union)

```ts
// SkillTool.ts:301-326
export const outputSchema = lazySchema(() => {
  const inlineOutputSchema = z.object({
    success: z.boolean().describe('Whether the skill is valid'),
    commandName: z.string().describe('The name of the skill'),
    allowedTools: z
      .array(z.string())
      .optional()
      .describe('Tools allowed by this skill'),
    model: z.string().optional().describe('Model override if specified'),
    status: z.literal('inline').optional().describe('Execution status'),
  })
  const forkedOutputSchema = z.object({
    success: z.boolean().describe('Whether the skill completed successfully'),
    commandName: z.string().describe('The name of the skill'),
    status: z.literal('forked').describe('Execution status'),
    agentId: z
      .string()
      .describe('The ID of the sub-agent that executed the skill'),
    result: z.string().describe('The result from the forked skill execution'),
  })
  return z.union([inlineOutputSchema, forkedOutputSchema])
})
```

### 6.5 Skill manifest schema (frontmatter, parsed)

The manifest is YAML frontmatter at the top of `SKILL.md`. Parsed by `parseSkillFrontmatterFields` (`loadSkillsDir.ts:185-265`) — there is no Zod schema for the frontmatter itself; the parser hand-coerces each field. Field reference, **verbatim** by name and parsing rule:

| Frontmatter key | Type after parse | Source line | Rule |
|---|---|---|---|
| `name` | `string \| undefined` (→ `displayName`) | 238-239 | `frontmatter.name != null ? String(frontmatter.name) : undefined` |
| `description` | `string` | 208-214 | `coerceDescriptionToString(frontmatter.description, resolvedName) ?? extractDescriptionFromMarkdown(markdownContent, descriptionFallbackLabel)` |
| `whenToUse` (key: `when_to_use`) | `string \| undefined` | 252 | `frontmatter.when_to_use as string \| undefined` |
| `version` | `string \| undefined` | 253 | `frontmatter.version as string \| undefined` |
| `model` | `ReturnType<typeof parseUserSpecifiedModel> \| undefined` | 221-226 | `'inherit'` → `undefined`; otherwise `parseUserSpecifiedModel(...)` |
| `effort` | `EffortValue \| undefined` | 228-235 | `parseEffortValue(...)`; logs warning on invalid; valid set: `EFFORT_LEVELS \| integer` |
| `argumentHint` (key: `argument-hint`) | `string \| undefined` | 245-249 | `String(...)` |
| `arguments` | `string[]` (→ `argNames`) | 249-251 | `parseArgumentNames(frontmatter.arguments as string\|string[]\|undefined)` |
| `allowed-tools` | `string[]` | 242-244 | `parseSlashCommandToolsFromFrontmatter(...)` |
| `disable-model-invocation` | `boolean` | 254-256 | `parseBooleanFrontmatter(...)` |
| `user-invocable` | `boolean` | 216-219 | undefined → `true`; else `parseBooleanFrontmatter(...)` |
| `hooks` | `HooksSettings \| undefined` | 258 | `HooksSchema().safeParse(frontmatter.hooks)`; invalid → `undefined` + debug log (`136-153`) |
| `context` | `'fork' \| undefined` | 260 | `frontmatter.context === 'fork' ? 'fork' : undefined` |
| `agent` | `string \| undefined` | 261 | `frontmatter.agent as string \| undefined` |
| `shell` | `FrontmatterShell \| undefined` | 263 | `parseShellFrontmatter(frontmatter.shell, resolvedName)` |
| `paths` | `string[] \| undefined` | 159-178 (separate parser) | `splitPathInFrontmatter(...)` then strip `/**` suffix; if all `==='**'` or empty → `undefined` |

`hasUserSpecifiedDescription = validatedDescription !== null` (`241`).

**There is no `triggers` frontmatter key.** Some bundled `description` strings include "TRIGGER when …" / "DO NOT TRIGGER when …" prose (e.g. `claude-api` at `bundled/claudeApi.ts:183-186`); the loader treats this as ordinary description text. Trigger language is **model-facing prose only** — it is not a runtime mechanism. Deterministic skill matching only exists when `EXPERIMENTAL_SKILL_SEARCH` is active (owned by **12**).

**Default for `userInvocable`.** `parseSkillFrontmatterFields` defaults `userInvocable` to `true` regardless of source (`loadSkillsDir.ts:216-219`), and bundled `skillDefinitionToCommand` defaults to `true` as well (`plugins/builtinPlugins.ts:143`). The doc-comment in `frontmatterParser.ts:28-32` claims the default depends on source ("`commands/` defaults to true, `skills/` defaults to false") — **this comment is stale relative to the implementation**. The implementation (default true everywhere) is what governs runtime visibility; the prompt and system guidance accordingly frame slash commands as user-invocable skills (`tools/SkillTool/prompt.ts:188-194`, `constants/prompts.ts:382-384`).

**Malformed YAML frontmatter.** `parseFrontmatter` (`utils/frontmatterParser.ts:130-175`) (a) splits the frontmatter block off `content` first, (b) tries `parseYaml(frontmatterText)`, (c) on failure retries with `quoteProblematicValues`, (d) on second failure logs a `warn` (`Failed to parse YAML frontmatter in <path>: …`) and returns `frontmatter: {}` while `content` remains the body **after** the failed frontmatter block. So a skill with malformed YAML still loads — with an empty frontmatter (description falls back to first-line extraction by `extractDescriptionFromMarkdown`), and the body is rendered without the broken YAML block.

### 6.6 `BundledSkillDefinition` (verbatim type)

```ts
// bundledSkills.ts:15-41
export type BundledSkillDefinition = {
  name: string
  description: string
  aliases?: string[]
  whenToUse?: string
  argumentHint?: string
  allowedTools?: string[]
  model?: string
  disableModelInvocation?: boolean
  userInvocable?: boolean
  isEnabled?: () => boolean
  hooks?: HooksSettings
  context?: 'inline' | 'fork'
  agent?: string
  /**
   * Additional reference files to extract to disk on first invocation.
   * Keys are relative paths (forward slashes, no `..`), values are content.
   * When set, the skill prompt is prefixed with a "Base directory for this
   * skill: <dir>" line so the model can Read/Grep these files on demand —
   * same contract as disk-based skills.
   */
  files?: Record<string, string>
  getPromptForCommand: (
    args: string,
    context: ToolUseContext,
  ) => Promise<ContentBlockParam[]>
}
```

### 6.7 `SAFE_SKILL_PROPERTIES` allowlist (verbatim)

```ts
// SkillTool.ts:875-908
const SAFE_SKILL_PROPERTIES = new Set([
  // PromptCommand properties
  'type', 'progressMessage', 'contentLength', 'argNames',
  'model', 'effort', 'source', 'pluginInfo',
  'disableNonInteractive', 'skillRoot', 'context', 'agent',
  'getPromptForCommand', 'frontmatterKeys',
  // CommandBase properties
  'name', 'description', 'hasUserSpecifiedDescription',
  'isEnabled', 'isHidden', 'aliases', 'isMcp',
  'argumentHint', 'whenToUse', 'paths', 'version',
  'disableModelInvocation', 'userInvocable',
  'loadedFrom', 'immediate', 'userFacingName',
])
```

A property "with a meaningful value" means `value !== undefined && value !== null && !(Array.isArray(v) && v.length===0) && !(typeof v==='object' && !Array.isArray(v) && Object.keys(v).length===0)` (`SkillTool.ts:910-933`).

### 6.8 `LoadedFrom` enum (verbatim)

```ts
// loadSkillsDir.ts:67-73
export type LoadedFrom =
  | 'commands_DEPRECATED'
  | 'skills'
  | 'plugin'
  | 'managed'
  | 'bundled'
  | 'mcp'
```

### 6.9 Skill matching algorithm — pseudocode

The "matching" referred to in §1 is **name resolution + filtering**, not semantic matching (semantic matching is the EXPERIMENTAL_SKILL_SEARCH index, owned by 12). The model picks `skill` from the listing rendered in §5.1; the runtime then resolves it as follows:

```
resolve(skillInput, context) -> Command | error:
  trimmed = skillInput.trim()
  if trimmed == '': error errorCode 1
  hasSlash = trimmed.startsWith('/')
  if hasSlash: emit tengu_skill_tool_slash_prefix
  name = hasSlash ? trimmed.slice(1) : trimmed

  # Tier 1: Remote canonical (ANT + EXPERIMENTAL_SKILL_SEARCH)
  if EXPERIMENTAL_SKILL_SEARCH and USER_TYPE=='ant':
     slug = stripCanonicalPrefix(name)
     if slug != null:
        return getDiscoveredRemoteSkill(slug) ? remote(slug) : error errorCode 6

  # Tier 2: AppState's MCP commands (if any) merged with cwd-rooted local commands
  # (uniqBy 'name'; local first wins)
  mcpSkills = AppState.mcp.commands.filter(type=='prompt' && loadedFrom=='mcp')
  commands  = mcpSkills.empty ? getCommands(cwd) : uniqBy([...getCommands(cwd), ...mcpSkills], 'name')

  # Tier 3: findCommand (20) — exact name, namespaced 'sub:dir', or alias
  cmd = findCommand(name, commands)
  if !cmd: error errorCode 2
  if cmd.disableModelInvocation: error errorCode 4
  if cmd.type != 'prompt': error errorCode 5
  return cmd
```

(`SkillTool.ts:81-94, 354-430, 580-616`)

**Collision and alias precedence.** When the same name exists in multiple sources, the first-wins ordering is set by `loadAllCommands` (`commands.ts:460-468`): `bundledSkills, builtinPluginSkills, skillDirCommands, workflowCommands, pluginCommands, pluginSkills, COMMANDS()`. Built-in plugin skills are recorded as `loadedFrom:'bundled'` (`plugins/builtinPlugins.ts:145-150`) so they are indistinguishable from in-tree bundled skills at this stage. Inside `getCommands(cwd)`, dynamic skills are inserted only if their name is absent from the base set (`commands.ts:491-498`). For `SkillTool.getAllCommands()`, `uniqBy([...localCommands, ...mcpSkills], 'name')` (`SkillTool.ts:93`) makes a same-named local command win over an MCP skill. The exact alias-matching policy inside `findCommand` (e.g., when `aliases` overlaps with another command's `name`) is owned by **20** — see `commands.ts` and the per-tool catalog spec **21**.

For *listing* construction (`§5.1`), there is no scoring: the order is the natural source order from `loadAllCommands` after filtering, with `getSkillToolCommands` keeping bundled/skills/legacy entries unconditionally and requiring `hasUserSpecifiedDescription || whenToUse` for plugin/MCP entries (`commands.ts:563-583`). With `EXPERIMENTAL_SKILL_SEARCH` ON, the per-turn surface is *replaced/augmented* by skill-discovery attachments emitted by `query.ts:66`'s `skillPrefetch`, owned by 12.

### 6.10 Skill-improvement classifier prompt (verbatim)

```text
// skillImprovement.ts:103-124
You are analyzing a conversation where a user is executing a skill (a repeatable process).
Your job: identify if the user's recent messages contain preferences, requests, or corrections that should be permanently added to the skill definition for future runs.

<skill_definition>
${projectSkill.content}
</skill_definition>

<recent_messages>
${formatRecentMessages(newMessages)}
</recent_messages>

Look for:
- Requests to add, change, or remove steps: "can you also ask me X", "please do Y too", "don't do Z"
- Preferences about how steps should work: "ask me about energy levels", "note the time", "use a casual tone"
- Corrections: "no, do X instead", "always use Y", "make sure to..."

Ignore:
- Routine conversation that doesn't generalize (one-time answers, chitchat)
- Things the skill already does

Output a JSON array inside <updates> tags. Each item: {"section": "which step/section to modify or 'new step'", "change": "what to add/modify", "reason": "which user message prompted this"}.
Output <updates>[]</updates> if no updates are needed.
```

System prompt (`skillImprovement.ts:129-130`):

```
You detect user preferences and process improvements during skill execution. Flag anything the user asks for that should be remembered for next time.
```

Apply prompt (`skillImprovement.ts:215-230`):

```text
You are editing a skill definition file. Apply the following improvements to the skill.

<current_skill_file>
${currentContent}
</current_skill_file>

<improvements>
${updateList}
</improvements>

Rules:
- Integrate the improvements naturally into the existing structure
- Preserve frontmatter (--- block) exactly as-is
- Preserve the overall format and style
- Do not remove existing content unless an improvement explicitly replaces it
- Output the complete updated file inside <updated_file> tags
```

Apply system prompt (`skillImprovement.ts:233-235`):

```
You edit skill definition files to incorporate user preferences. Output only the updated file content.
```

### 6.11 User-facing strings inside SkillTool

| Source | String |
|---|---|
| `SkillTool.ts:295` | `'The skill name. E.g., "commit", "review-pr", or "pdf"'` |
| `SkillTool.ts:296` | `'Optional arguments for the skill'` |
| `SkillTool.ts:333` | `'invoke a slash-command skill'` |
| `SkillTool.ts:342` | ``Execute skill: ${skill}`` |
| `SkillTool.ts:359` | ``Invalid skill format: ${skill}`` |
| `SkillTool.ts:389` | ``Remote skill ${slug} was not discovered in this session. Use DiscoverSkills to find remote skills first.`` |
| `SkillTool.ts:406` | ``Unknown skill: ${normalizedCommandName}`` |
| `SkillTool.ts:415` | ``Skill ${normalizedCommandName} cannot be used with ${SKILL_TOOL_NAME} tool due to disable-model-invocation`` |
| `SkillTool.ts:423` | ``Skill ${normalizedCommandName} is not a prompt-based skill`` |
| `SkillTool.ts:479` | `'Skill execution blocked by permission rules'` |
| `SkillTool.ts:572` | ``Execute skill: ${commandName}`` |
| `SkillTool.ts:646` | `'Command processing failed'` |
| `SkillTool.ts:852` | ``Skill "${result.commandName}" completed (forked execution).\n\nResult:\n${result.result}`` |
| `SkillTool.ts:860` | ``Launching skill: ${result.commandName}`` |
| `SkillTool.ts:984` | ``Remote skill ${slug} was not discovered in this session. Use DiscoverSkills to find remote skills first.`` |
| `SkillTool.ts:1001` | ``Failed to load remote skill ${slug}: ${msg}`` |
| `SkillTool.ts:1076` | ``Base directory for this skill: ${normalizedDir}\n\n${bodyContent}`` |
| `UI.tsx:19` | ``'Initializing…'`` |
| `UI.tsx:25-26` | Forked done byline `['Done']` |
| `UI.tsx:29` | ``'Successfully loaded skill'`` |
| `UI.tsx:34` | ``${count} ${plural(count,'tool')} allowed`` |
| `UI.tsx:88-89` | ``+${hiddenCount} more tool ${plural(hiddenCount,'use')}`` |
| `UI.tsx:59` | Legacy display: ``/${skill}`` when `loadedFrom === 'commands_DEPRECATED'` |
| `bundledSkills.ts:212` | ``Base directory for this skill: ${baseDir}\n\n`` |
| `bundledSkills.ts:203` | ``bundled skill file path escapes skill dir: ${relPath}`` |
| `loadSkillsDir.ts:346` | ``Base directory for this skill: ${baseDir}\n\n${markdownContent}`` |
| `loadSkillsDir.ts:983` | (debug only) ``Skipping duplicate skill '${skill.name}' from ${skill.source} (same file already loaded from ${existingSource})`` |

### 6.12 Constants table

| Constant | Value | Source |
|---|---|---|
| `SKILL_TOOL_NAME` | `'Skill'` | `tools/SkillTool/constants.ts:1` |
| `SKILL_BUDGET_CONTEXT_PERCENT` | `0.01` | `tools/SkillTool/prompt.ts:21` |
| `CHARS_PER_TOKEN` | `4` | `prompt.ts:22` |
| `DEFAULT_CHAR_BUDGET` | `8_000` | `prompt.ts:23` |
| `MAX_LISTING_DESC_CHARS` | `250` | `prompt.ts:29` |
| `MIN_DESC_LENGTH` | `20` | `prompt.ts:68` |
| `MAX_PROGRESS_MESSAGES_TO_SHOW` | `3` | `tools/SkillTool/UI.tsx:18` |
| `INITIALIZING_TEXT` | `'Initializing…'` | `UI.tsx:19` |
| `tool.maxResultSizeChars` | `100_000` | `SkillTool.ts:334` |
| `TURN_BATCH_SIZE` (skill improvement) | `5` | `utils/hooks/skillImprovement.ts:31` |
| `SKILL_USAGE_DEBOUNCE_MS` | `60_000` | `utils/suggestions/skillUsageTracking.ts:3` |
| Ranking half-life | `7 days` | `skillUsageTracking.ts:51` |
| Ranking floor | `max(recencyFactor, 0.1)` | `skillUsageTracking.ts:54` |
| Bundled-skill parent dir mode | `0o700` | `bundledSkills.ts:163` |
| Bundled-skill file mode | `0o600` | `bundledSkills.ts:187` |
| Bundled-skill open flags (POSIX) | `O_WRONLY \| O_CREAT \| O_EXCL \| O_NOFOLLOW` | `bundledSkills.ts:179-184` |
| Bundled-skill open flags (Win32) | `'wx'` | `bundledSkills.ts:179-180` |
| `SLASH_COMMAND_TOOL_CHAR_BUDGET` env override | numeric | `prompt.ts:32-34` |
| `CLAUDE_CODE_DISABLE_POLICY_SKILLS` env | truthy disables managed skills | `loadSkillsDir.ts:686` |
| `errorCode` 1 | empty input | `SkillTool.ts:362` |
| `errorCode` 2 | unknown skill | `SkillTool.ts:407` |
| `errorCode` 4 | disable-model-invocation | `SkillTool.ts:416` |
| `errorCode` 5 | not prompt-based | `SkillTool.ts:425` |
| `errorCode` 6 | remote skill not discovered | `SkillTool.ts:390` |

### 6.13 Skill listing partition algorithm (pseudocode)

```
formatCommandsWithinBudget(commands, contextWindowTokens):
   if commands.empty: return ''
   budget = getCharBudget(contextWindowTokens)
   fullEntries = commands.map(c => ({c, full: `- ${c.name}: ${getCommandDescription(c)}`}))
   fullTotal = sum(stringWidth(e.full) for e) + (n-1)
   if fullTotal <= budget: return fullEntries.map(e=>e.full).join('\n')

   bundledIndices = { i : commands[i].source=='bundled' }
   restCommands   = [ commands[i] for i not in bundledIndices ]
   bundledChars   = sum(stringWidth(e.full)+1 for i in bundledIndices)
   remainingBudget = budget - bundledChars
   if restCommands.empty: return fullEntries.map(e=>e.full).join('\n')

   restNameOverhead = sum(stringWidth(c.name)+4 for c in restCommands) + (m-1)
   availableForDescs = remainingBudget - restNameOverhead
   maxDescLen = floor(availableForDescs / m)

   if maxDescLen < MIN_DESC_LENGTH (=20):
      # names-only mode for rest, bundled keep desc
      emit (ANT-only) tengu_skill_descriptions_truncated(truncation_mode:'names_only',…)
      return commands.map(c, i =>
         bundledIndices.has(i) ? fullEntries[i].full : `- ${c.name}`
      ).join('\n')

   emit (ANT-only) tengu_skill_descriptions_truncated(truncation_mode:'description_trimmed',…)
   return commands.map(c, i =>
      bundledIndices.has(i) ? fullEntries[i].full
                            : `- ${c.name}: ${truncate(getCommandDescription(c), maxDescLen)}`
   ).join('\n')

getCommandDescription(c):
   desc = c.whenToUse ? `${c.description} - ${c.whenToUse}` : c.description
   return desc.length > MAX_LISTING_DESC_CHARS
     ? desc.slice(0, MAX_LISTING_DESC_CHARS - 1) + '…'
     : desc
```

(`prompt.ts:43-171`)

---

## 7. Side Effects & I/O

| Op | Source | Notes |
|---|---|---|
| `realpath(filePath)` | `loadSkillsDir.ts:118-124` | per-skill identity for dedup; null on failure |
| `readdir(basePath)` | `loadSkillsDir.ts:415-419` | per-skills-root |
| `readFile(SKILL.md)` | `loadSkillsDir.ts:435` | per-skill |
| `stat(.claude/skills)` | `loadSkillsDir.ts:885` | per dynamic-walk candidate |
| `git check-ignore` | `loadSkillsDir.ts:892` (via `isPathGitignored`) | conditional skip |
| `mkdir(parent, {recursive:true, mode:0o700})` | `bundledSkills.ts:163` | bundled extraction |
| `open(p, O_WRONLY\|O_CREAT\|O_EXCL\|O_NOFOLLOW, 0o600)` then `writeFile`/`close` | `bundledSkills.ts:186-193` | bundled extraction |
| `executeShellCommandsInPrompt` | `loadSkillsDir.ts:374-396` (skipped for `loadedFrom==='mcp'`) | inline `!\`…\`` expansion |
| `saveGlobalConfig` (mutates `~/.config/claude/.../config.json`) | `skillUsageTracking.ts:22` | debounced 60s |
| `readFile(<cwd>/.claude/skills/<name>/SKILL.md)` then `writeFile` | `skillImprovement.ts:200-263` | apply path |
| `queryModelWithoutStreaming` | `skillImprovement.ts:212` | small/fast model side-channel |
| Remote fetch (`loadRemoteSkill`) — `gs://`, `s3://`, `https://`, `http://` | `SkillTool.ts:991` | ANT + experimental only |
| `addInvokedSkill(name, path, content, agentId)` (compaction-survival) | `SkillTool.ts:1088`; otherwise via `processPromptSlashCommand` | 29 |
| `clearInvokedSkillsForAgent(agentId)` (forked teardown) | `SkillTool.ts:287` | 29 |
| `logEvent(...)` (multiple events; see §10) | various | analytics |
| `skillsLoaded.emit()` | `loadSkillsDir.ts:974, 1054` | fan-out to listeners |

---

## 8. Feature Flags & Variants

| Flag | Behavior | Sites |
|---|---|---|
| `EXPERIMENTAL_SKILL_SEARCH` | Enables remote-canonical skill resolution (`_canonical_<slug>`); enables `was_discovered` telemetry; gates `discoveredSkillNames` propagation; module-level `require()` of `services/skillSearch/*` modules into `remoteSkillModules` (non-null at every call site by construction); enables additional skill-search prefetch in `query.ts:66` and `constants/prompts.ts` framing | `SkillTool.ts:108-115, 140, 378, 493, 606, 662, 966`; `commands.ts:96`; `query.ts:66`; `constants/prompts.ts:87-95, 335, 778`; `utils/messages.ts:3506`; `utils/attachments.ts:95, 801, 2693`; `services/compact/compact.ts:208, 212`; `services/mcp/useManageMCPConnections.ts:27` |
| `MCP_SKILLS` | Treats MCP prompts as skills via `loadedFrom === 'mcp'`; `getMcpSkillCommands` returns the filtered set, otherwise `[]`; controls MCP-side prompt fetch + resource-roundtrip code paths | `commands.ts:550`; `services/mcp/client.ts:117, 1392, 1670, 2174, 2348`; `services/mcp/useManageMCPConnections.ts:22, 684, 718` |
| `BUILDING_CLAUDE_APPS` | Registers the `claudeApi` bundled skill | `skills/bundled/index.ts:64-69` |
| `RUN_SKILL_GENERATOR` | Registers the `runSkillGenerator` bundled skill | `skills/bundled/index.ts:73-78` |
| `SKILL_IMPROVEMENT` | Registers the post-sampling improvement hook (additionally requires `tengu_copper_panda` GrowthBook flag at runtime) | `utils/hooks/skillImprovement.ts:177-181` |
| `KAIROS_DREAM` (or `KAIROS`) | Registers `dream` bundled skill | `skills/bundled/index.ts:35-40` |
| `REVIEW_ARTIFACT` | Registers `hunter` bundled skill | `skills/bundled/index.ts:41-46` |
| `AGENT_TRIGGERS` | Registers `loop` bundled skill (its `isEnabled` further delegates to `isKairosCronEnabled()`) | `skills/bundled/index.ts:47-55` |
| `AGENT_TRIGGERS_REMOTE` | Registers `scheduleRemoteAgents` bundled skill | `skills/bundled/index.ts:56-63` |
| `shouldAutoEnableClaudeInChrome()` | Registers `claudeInChrome` bundled skill (this is a runtime predicate, not a `feature()` flag) | `skills/bundled/index.ts:70-72` |
| `USER_TYPE === 'ant'` | (1) ANT-only telemetry fields in all three SkillTool invocation events. (2) `tengu_skill_descriptions_truncated` event. (3) Combined with `EXPERIMENTAL_SKILL_SEARCH` enables remote-canonical handling (validate, permission auto-allow, dispatch). | `SkillTool.ts:171, 379, 494, 607, 694, 1051`; `prompt.ts:125, 149` |
| `--bare` (`isBareMode()`) | Skip auto-discovery of managed/user/project skill dirs and legacy commands; load only `--add-dir` paths (still subject to `projectSettingsEnabled`/`skillsLocked`). Bundled skills still register. | `loadSkillsDir.ts:658-675` |
| `CLAUDE_CODE_DISABLE_POLICY_SKILLS` | Truthy → skip managed skill loading | `loadSkillsDir.ts:686` |
| `SLASH_COMMAND_TOOL_CHAR_BUDGET` | Numeric override of listing budget | `prompt.ts:32-34` |
| `isRestrictedToPluginOnly('skills')` (`skillsLocked`) | Disables user/project/legacy-commands skill loading and dynamic-discovery loading | `loadSkillsDir.ts:651-713, 925-927` |

The `feature('EXPERIMENTAL_SKILL_SEARCH')` `require()` block at `SkillTool.ts:108-115` is documented in the source as deliberately conditional: a static import would pull `akiBackend.ts` whose module-level `memoize()`/`lazySchema()` initializers survive tree-shaking. Every site that dereferences `remoteSkillModules!` is itself behind a `feature('EXPERIMENTAL_SKILL_SEARCH')` guard, so the non-null assertion is safe by construction.

---

## 9. Error Handling & Edge Cases

| Edge case | Behavior | Source |
|---|---|---|
| Skill name with leading `/` | Stripped; `tengu_skill_tool_slash_prefix` event emitted (used to measure model adherence to the "no slash" instruction) | `SkillTool.ts:366-372, 440, 597` |
| Skill name empty after trim | `errorCode 1`, `Invalid skill format: …` | `SkillTool.ts:357-363` |
| Skill not found | `errorCode 2`, `Unknown skill: …` | `SkillTool.ts:402-408` |
| Skill with `disable-model-invocation: true` | `errorCode 4` | `SkillTool.ts:412-417` |
| Skill is non-prompt type | `errorCode 5` | `SkillTool.ts:421-426` |
| Remote skill name not in session state | `errorCode 6` | `SkillTool.ts:387-393` |
| `processPromptSlashCommand` returns `shouldQuery === false` | `throw new Error('Command processing failed')` | `SkillTool.ts:645-647` |
| Forked skill error | propagates from `runAgent`; `finally` runs `clearInvokedSkillsForAgent(agentId)` to release skill content from compaction state | `SkillTool.ts:285-289` |
| Remote skill load failure | `logRemoteSkillLoaded({…, error})` with `cacheHit:false, latencyMs:0`; throws `Failed to load remote skill ${slug}: …` | `SkillTool.ts:992-1002` |
| Skill listing budget too small for any non-bundled description | Names-only mode for non-bundled; bundled keep full descriptions; `tengu_skill_descriptions_truncated(truncation_mode:'names_only')` (ANT) | `prompt.ts:123-142` |
| Duplicate skill (same realpath) across sources | Silently skipped; `logForDebugging` records source pair; first wins (managed → user → project → additional → legacy order) | `loadSkillsDir.ts:736-770` |
| Symlink directories under `/skills/` | Treated as directory (entries' `isSymbolicLink()` short-circuits the `isDirectory` check) | `loadSkillsDir.ts:425` |
| `SKILL.md` missing in a skill directory | Entry skipped; non-ENOENT errors (EACCES/EPERM/EIO) logged at `warn` for diagnosability | `loadSkillsDir.ts:434-446` |
| Single `.md` file at top of `/skills/` | NOT supported; ignored (must be `<dir>/SKILL.md`) | `loadSkillsDir.ts:425-428` |
| Multiple SKILL.md files in legacy `/commands/<dir>/` | First wins; `logForDebugging` notes the others | `loadSkillsDir.ts:506-513` |
| Bundled skill file write fails (`EEXIST`, `ENOTSUP`, etc.) | Skill continues to work without `Base directory…` prefix; `logForDebugging` records the failure path | `bundledSkills.ts:131-145` |
| `frontmatter.hooks` invalid by `HooksSchema` | Skill loads with `hooks: undefined`; debug log only | `loadSkillsDir.ts:136-153` |
| Malformed YAML frontmatter (parse fails after retry) | `warn`-level log via `logForDebugging`; returns `frontmatter: {}` and `content` sliced past the failed frontmatter block. Skill still loads with description falling back to `extractDescriptionFromMarkdown` and all metadata fields treated as if frontmatter were absent. | `utils/frontmatterParser.ts:148-175`, `loadSkillsDir.ts:447-469` |
| `frontmatter.effort` invalid | `effort: undefined`; warning log lists valid options | `loadSkillsDir.ts:228-235` |
| `frontmatter.paths` all `**` | `paths: undefined` (treated as unconditional) | `loadSkillsDir.ts:172-175` |
| `paths`-conditional skill activated by file path outside cwd | Skipped (relativePath empty / `..`-prefixed / absolute) — `ignore()` would throw on these | `loadSkillsDir.ts:1014-1027` |
| Dynamic skills directory inside a gitignored container | Skipped; `logForDebugging('Skipped gitignored skills dir: …')` | `loadSkillsDir.ts:892-897` |
| `/skills/` inside `node_modules` (typical) | Caught by gitignore check above (most projects ignore `node_modules`) | same |
| MCP skill body containing `!\`…\`` shell expansion | Never executed — `loadedFrom === 'mcp'` skips `executeShellCommandsInPrompt` | `loadSkillsDir.ts:371-396` |
| Skill with `model: opus` invoked in `opus[1m]` session | `resolveSkillModelOverride` carries `[1m]` suffix forward so the effective context window stays at 1M | `SkillTool.ts:809-820` |
| Skill `getSkillInfo` throws | Returns `{ totalSkills: 0, includedSkills: 0 }`; failure does not break system prompt | `prompt.ts:222-240` |
| `processed.messages` containing `<command-message>…</command-message>` user message | Filtered from `newMessages` (the SkillTool itself renders the invocation) | `SkillTool.ts:736-755` |
| `processed.messages` of type `'progress'` | Filtered | `SkillTool.ts:738-739` |
| `command.effort` set | Overridden in `appState.effortValue` via chained `getAppState`; for forked skills, merged into the agent definition | `SkillTool.ts:209-212, 824-836` |
| `command.allowedTools` set | Merged (Set-deduped) into `alwaysAllowRules.command` for the skill's expanded prompt | `SkillTool.ts:780-806` |

---

## 10. Telemetry & Observability

### 10.1 Events emitted by SkillTool / skill loaders

| Event | Site | Payload (selected) |
|---|---|---|
| `tengu_skill_tool_invocation` | `SkillTool.ts:152, 675, 1029` (one per execution path) | `command_name` (sanitized), `_PROTO_skill_name` (PII), `execution_context` ∈ `inline\|fork\|remote`, `invocation_trigger` ∈ `nested-skill\|claude-proactive`, `query_depth`, `parent_agent_id?`, `was_discovered?` (when `EXPERIMENTAL_SKILL_SEARCH && isSkillSearchEnabled()`), `is_remote?`, `remote_cache_hit?`, `remote_load_latency_ms?`, ANT-only `skill_name/skill_source/skill_loaded_from/skill_kind/remote_slug`, plugin `_PROTO_plugin_name/_PROTO_marketplace_name/plugin_name/plugin_repository` + `buildPluginCommandTelemetryFields(...)` |
| `tengu_skill_tool_slash_prefix` | `SkillTool.ts:368` | (no payload) — measures `/`-prefix usage |
| `tengu_skill_descriptions_truncated` | `prompt.ts:126-136, 150-161` (ANT-only) | `skill_count`, `budget`, `full_total`, `truncation_mode` ∈ `names_only\|description_trimmed`, `max_desc_length`, `bundled_count`, `bundled_chars`, optional `truncated_count` |
| `tengu_dynamic_skills_changed` | `loadSkillsDir.ts:962, 1044` | `source` ∈ `file_operation\|conditional_paths`, `previousCount`, `newCount`, `addedCount`, `directoryCount` |
| `tengu_skill_improvement_detected` | `skillImprovement.ts:151-158` | `updateCount`, `uuid`, `_PROTO_skill_name` |
| `logRemoteSkillLoaded(…)` | `SkillTool.ts:993-1000, 1014-1022` | `slug`, `cacheHit`, `latencyMs`, `urlScheme`, optional `error`, `fileCount`, `totalBytes`, `fetchMethod` |

### 10.2 Debug logs (non-telemetry)

`SkillTool executing forked skill <name> with agent <type>`; `SkillTool forked skill <name> completed in <ms>ms`; `SkillTool returning <n> newMessages for skill <name>`; `SkillTool loaded remote skill <slug> (cacheHit=…, …ms, … chars)` (`SkillTool.ts:217-218, 272-274, 757-759, 1061-1063`).

`Loading skills from: managed=…, user=…, project=[…]`; `Loaded N unique skills (…)`; `[bare] Skipping skill dir discovery (…)`; `[skills] N conditional skills stored (…)`; `[skills] Activated conditional skill '<name>' (matched path: …)`; `[skills] Skipped gitignored skills dir: …`; `[skills] Dynamic skill discovery skipped: projectSettings disabled or plugin-only policy`; `[skills] Dynamically discovered N skills from M directories`; `Skipping duplicate skill '…' from <source> (same file already loaded from <existingSource>)`; `Multiple skill files found in <dir>, using <name>`; `Skill prompt: showing "<name>" (userFacingName="<displayName>")` (`loadSkillsDir.ts` various; `prompt.ts:60-63`).

---

## 11. Reimplementation Checklist

A faithful reimplementation must:

1. Build a `Skill` Tool whose name is exactly `'Skill'`, `searchHint = 'invoke a slash-command skill'`, `maxResultSizeChars = 100_000`, with the input/output Zod schemas in §6.3-6.4.
2. Reproduce the `getPrompt` body in §6.1 verbatim, memoized once per process.
3. Implement listing with the partition algorithm in §6.13, the constants in §6.12, and the env override `SLASH_COMMAND_TOOL_CHAR_BUDGET`.
4. Resolve a skill via the tiered algorithm in §6.9 with errorCodes `1, 2, 4, 5, 6` (no `3`).
5. Strip a leading `/` and emit `tengu_skill_tool_slash_prefix`; do **not** treat the slash as a normalization that affects `findCommand`.
6. Implement `checkPermissions` exactly per §5.6 — deny first, ANT remote auto-allow second, allow rules third, `SAFE_SKILL_PROPERTIES` allowlist fourth, `behavior:'ask'` last with the two `addRules` suggestions for `commandName` and `${commandName}:*` to `localSettings`.
7. The "ask" rendering must include `metadata: { command: commandObj }` only when the command was found.
8. Inline execution must emit `newMessages` and a `contextModifier` that:
   - Set-dedups `processed.allowedTools` into `alwaysAllowRules.command` via a chained `getAppState`.
   - Replaces `mainLoopModel` via `resolveSkillModelOverride(model, ctx.options.mainLoopModel)` (carrying `[1m]` suffix).
   - Sets `appState.effortValue = command.effort` via a *second* chained `getAppState`.
9. Filter out `progress` messages and `<command-message>` user messages before tagging with `toolUseID`.
10. Forked execution must merge `command.effort` into the agent definition, run `runAgent` with `querySource: 'agent:custom'`, report tool-use/tool-result progress with `toolUseID = skill_${parentMessage.message.id}` and `data.type = 'skill_progress'`, drop messages from memory after `extractResultText`, and call `clearInvokedSkillsForAgent(agentId)` in `finally`.
11. Remote canonical execution (when applicable) must NOT route through `processPromptSlashCommand`; instead it must (a) strip frontmatter, (b) prepend `Base directory for this skill: <normalizedDir>\n\n`, (c) substitute `${CLAUDE_SKILL_DIR}` and `${CLAUDE_SESSION_ID}` over the body, (d) call `addInvokedSkill(name, skillPath, finalContent, agentId ?? null)` with the *transformed* content, (e) emit one user meta-message tagged with `toolUseID`. Win32 must replace `\\` with `/` in the directory path.
12. Bundled-skill registration must support `files: Record<string,string>` with the secure extraction recipe in §5.11 (mode `0o700`/`0o600`, `O_NOFOLLOW|O_EXCL` POSIX, `'wx'` Win32, `..` rejection, no unlink-on-EEXIST), and memoize the extraction promise per process.
13. Disk-loaded skill bodies must run shell-block expansion (`!\`…\``, fenced `\`\`\`!`) UNLESS `loadedFrom === 'mcp'`. The `getAppState` passed to `executeShellCommandsInPrompt` must temporarily set `alwaysAllowRules.command = allowedTools`.
14. `getSkillDirCommands` must:
    - Walk managed (`<MANAGED_FILE_PATH>/.claude/skills`), user (`<CLAUDE_CONFIG_HOME>/skills`), project (`getProjectDirsUpToHome('skills', cwd)`), `--add-dir` (`<dir>/.claude/skills`), and legacy `loadSkillsFromCommandsDir(cwd)` in parallel.
    - Honor `CLAUDE_CODE_DISABLE_POLICY_SKILLS`, `isSettingSourceEnabled('userSettings'/'projectSettings')`, `isRestrictedToPluginOnly('skills')`, `isBareMode()` (the bare short-circuit includes `--add-dir` only).
    - Realpath-dedup; first wins in source order.
    - Partition out `paths`-conditional skills into `conditionalSkills`, EXCEPT those whose names appear in `activatedConditionalSkillNames`.
15. `discoverSkillDirsForPaths` must walk parents while `dir.startsWith(resolvedCwd + sep)` — exclude cwd itself; cache **misses** in `dynamicSkillDirs`; skip gitignored containers (`isPathGitignored`); sort returned dirs deepest-first.
16. `addSkillDirectories` must process input array in *reverse* (shallower first) so deeper entries overwrite in `dynamicSkills`. After mutation, fire `skillsLoaded`.
17. `activateConditionalSkillsForPaths` must use the `ignore` library (gitignore semantics), strip `/**` suffixes at *load* time, treat all-`**` as unconditional, skip relative paths that escape cwd, move activated skills from `conditionalSkills` to `dynamicSkills`, record `activatedConditionalSkillNames`, fire `skillsLoaded` and `tengu_dynamic_skills_changed`.
18. `getMcpSkillCommands` must filter to `prompt + loadedFrom==='mcp' + !disableModelInvocation` ONLY when `MCP_SKILLS` is on; else `[]`. This helper is the **listing-side** gate; `SkillTool.getAllCommands()` does NOT call this helper — it filters `appState.mcp.commands` directly for `type==='prompt' && loadedFrom==='mcp'` (`SkillTool.ts:81-94`). The build-time `MCP_SKILLS` gate that determines whether MCP-provided skills exist in `appState.mcp.commands` at all is enforced upstream by the MCP client (spec **23**).
19. Skill-improvement hook must be registered only when `feature('SKILL_IMPROVEMENT') && getFeatureValue_CACHED_MAY_BE_STALE('tengu_copper_panda', false)`. Trigger every 5 user messages, only when `querySource === 'repl_main_thread'` and `findProjectSkill()` matches. Use the small/fast model with `useTools:false`. Parse `<updates>…</updates>`. Use the prompts in §6.10 verbatim.
20. `recordSkillUsage` must debounce 60 s in-process, persist `{usageCount, lastUsedAt}` via `saveGlobalConfig`. `getSkillUsageScore` must use `0.5 ** (days/7)` floored at `0.1`.

---

## 11.5 Bundled skills enumeration (catalog)

**Invariant.** Each *register module* under `src/skills/bundled/` calls `registerBundledSkill(...)` exactly once. **Content / data-only modules** (`claudeApiContent.ts`, `verifyContent.ts`) and the **registry/init module** (`index.ts`) do NOT call `registerBundledSkill` — they exist purely as Bun-text-loader inlining stores or as the entry point that orchestrates registration. `index.ts:initBundledSkills()` (`bundled/index.ts:24-79`) drives the order: unconditional registrations first, then a series of `if (feature(...))` / runtime-gated blocks.

### 11.5.1 Unconditional register modules (always run `registerBundledSkill`)

These call `registerBundledSkill` from their `register*Skill()` function unconditionally; ANT-only ones early-return inside that function on the `USER_TYPE` check before the call.

| File | Registered name | Internal gate (inside register fn) | Purpose |
|---|---|---|---|
| `bundled/updateConfig.ts` | `update-config` | none | Surface the JSON Schema for `SettingsSchema()` and per-scope settings docs. |
| `bundled/keybindings.ts` | `keybindings-help` | runtime: `isEnabled = isKeybindingCustomizationEnabled` | Customize keyboard shortcuts in `~/.claude/keybindings.json`. **Note: registered name is `keybindings-help`, not `keybindings`.** |
| `bundled/verify.ts` | `verify` | **ANT-only**: `if (process.env.USER_TYPE !== 'ant') return` (`verify.ts:13-15`) | Verify a change works by running the app. Description loaded from `verifyContent.ts` SKILL.md frontmatter; passes `SKILL_FILES` through `files:` for on-disk extraction. |
| `bundled/debug.ts` | `debug` | `disableModelInvocation: true` (user-only); description swaps text on `USER_TYPE === 'ant'` but skill is registered for everyone (`debug.ts:14-23`) | Read the session debug log; non-ants get debug-logging-on side effect. |
| `bundled/loremIpsum.ts` | `lorem-ipsum` | **ANT-only**: `if (process.env.USER_TYPE !== 'ant') return` (`loremIpsum.ts:234-237`) | Filler-text generator using a verified single-token vocabulary. |
| `bundled/skillify.ts` | `skillify` | **ANT-only**: `if (process.env.USER_TYPE !== 'ant') return` (`skillify.ts:158-161`); also `disableModelInvocation: true` | Capture session's repeatable process into a skill draft. |
| `bundled/remember.ts` | `remember` | **ANT-only**: `if (process.env.USER_TYPE !== 'ant') return` (`remember.ts:5-7`); also `isEnabled: () => isAutoMemoryEnabled()` | Audit auto-memory and propose relocations to CLAUDE.md/CLAUDE.local.md/team. |
| `bundled/simplify.ts` | `simplify` | none (registered for all users) | Three parallel agents (Reuse/Quality/Efficiency) run over `git diff`. |
| `bundled/batch.ts` | `batch` | none | Orchestrate parallelizable changes via plan mode + Agent worker fan-out (5..30 agents). |
| `bundled/stuck.ts` | `stuck` | **ANT-only**: `if (process.env.USER_TYPE !== 'ant') return` (`stuck.ts:62-64`) | Diagnose frozen Claude Code sessions on this machine and post to `#claude-code-feedback`. |

### 11.5.2 Feature-gated register modules (conditional in `index.ts`)

These are imported via `require()` only when their feature flag is on, so their *register call* itself is gated. The module body is absent from this leak when the flag is off (DCE'd). `loop.ts` and `scheduleRemoteAgents.ts` are present in this tree even though their `index.ts` gates are `AGENT_TRIGGERS` / `AGENT_TRIGGERS_REMOTE`.

| File | Registered name | `index.ts` gate | Inner runtime gate | Purpose |
|---|---|---|---|---|
| `bundled/dream.ts` (absent in leak) | `dream` (per spec 00 Missing-Source Ledger) | `feature('KAIROS') \|\| feature('KAIROS_DREAM')` | (module body not in tree) | Required by call site at `index.ts:35-39`. |
| `bundled/hunter.ts` (absent in leak) | `hunter` | `feature('REVIEW_ARTIFACT')` | (module body not in tree) | Required by call site at `index.ts:41-45`. |
| `bundled/loop.ts` (present) | `loop` | `feature('AGENT_TRIGGERS')` | `isEnabled: isKairosCronEnabled` (`loop.ts:83`) | Run a prompt/slash command on a recurring interval. |
| `bundled/scheduleRemoteAgents.ts` (present) | `schedule` | `feature('AGENT_TRIGGERS_REMOTE')` | `isEnabled: () => getFeatureValue_CACHED_MAY_BE_STALE('tengu_surreal_dali', false) && isPolicyAllowed('allow_remote_sessions')` (`scheduleRemoteAgents.ts:332-334`) | Create/update/list/run scheduled remote agents on a cron schedule. |
| `bundled/claudeApi.ts` (present) | `claude-api` | `feature('BUILDING_CLAUDE_APPS')` | none (registers via `registerClaudeApiSkill` once flag is on) | Build apps with Claude API / Anthropic SDK. Lazy-imports `claudeApiContent.js` and inlines selected docs into the prompt body with `<doc>` tags — does NOT pass a `files:` map. |
| `bundled/claudeInChrome.ts` (present) | `claude-in-chrome` | runtime: `if (shouldAutoEnableClaudeInChrome()) registerClaudeInChromeSkill()` (`index.ts:70-72`) | `isEnabled: () => shouldAutoEnableClaudeInChrome()` | Wraps `BROWSER_TOOLS` from `@ant/claude-for-chrome-mcp` as `mcp__claude-in-chrome__*` allowedTools. |
| `bundled/runSkillGenerator.ts` (absent in leak) | (per call site) | `feature('RUN_SKILL_GENERATOR')` | (module body not in tree) | Required by call site at `index.ts:73-77`. |

(For DCE'd modules only the call-site evidence is verifiable; module bodies are absent from this leak — see 00 §2.5 Missing-Source Ledger.)

### 11.5.3 Data / content modules (NOT skills, do NOT call `registerBundledSkill`)

These are pure Bun-text-loader inlining modules. They export the inlined string content for sibling register modules to consume. They never appear in the bundled-skill registry.

| File | Exports | Consumer | How content reaches the runtime |
|---|---|---|---|
| `bundled/claudeApiContent.ts` (~247 KB after inlining all `.md`) | many named string consts (`pythonClaudeApiReadme`, `csharpClaudeApi`, `goClaudeApi`, etc.) | `bundled/claudeApi.ts` via `await import('./claudeApiContent.js')` (lazy, only when the skill fires; `claudeApi.ts:189-194`) | Embedded into the prompt body as `<doc>`-tagged sections by `buildPrompt`. **No `files:` map.** |
| `bundled/verifyContent.ts` | `SKILL_MD: string`, `SKILL_FILES: Record<string,string>` | `bundled/verify.ts` (eager, top-level) | `SKILL_MD` is parsed for frontmatter then concatenated into the prompt body; `SKILL_FILES` is passed through `registerBundledSkill({ files: SKILL_FILES, ... })` for on-disk extraction by `bundledSkills.ts`. |
| `bundled/index.ts` | `initBundledSkills()` | Boot path | Drives all calls in 11.5.1 + 11.5.2 in fixed order. |

### 11.5.4 Summary by gate kind

| Gate kind | Skills |
|---|---|
| **Always registered** | `update-config`, `keybindings-help`, `debug`, `simplify`, `batch` |
| **ANT-only (`USER_TYPE === 'ant'` early-return inside register fn)** | `verify`, `lorem-ipsum`, `skillify`, `remember`, `stuck` |
| **`isEnabled` runtime predicate (still registered)** | `keybindings-help` (isKeybindingCustomizationEnabled), `remember` (isAutoMemoryEnabled), `loop` (isKairosCronEnabled), `schedule` (`tengu_surreal_dali` GB + `allow_remote_sessions` policy), `claude-in-chrome` (shouldAutoEnableClaudeInChrome) |
| **Build-time `feature()` gate (register call itself gated)** | `dream` (KAIROS\|KAIROS_DREAM), `hunter` (REVIEW_ARTIFACT), `loop` (AGENT_TRIGGERS), `schedule` (AGENT_TRIGGERS_REMOTE), `claude-api` (BUILDING_CLAUDE_APPS), runSkillGenerator (RUN_SKILL_GENERATOR) |
| **Runtime gate at index.ts call site (skip register if false)** | `claude-in-chrome` (shouldAutoEnableClaudeInChrome) |

All registrations target the same `registerBundledSkill` API (see §3 of this spec); `userInvocable`, `isEnabled`, `allowedTools`, `files`, `disableModelInvocation`, and `getPromptForCommand` are the per-skill wiring points. The `verify` skill is the **only** one in this catalog that uses `files:` for `bundledSkills.ts` extraction. `claude-api` lazy-imports its content module and inlines docs directly into the prompt body — these two patterns are *different mechanisms* even though both rely on Bun text-loader inlining of `.md`.

---

## 12. Open Questions / Unknowns

> **Phase 9.7 sweep (2026-05-09)**: items below re-audited.

1. ~~**Remote-canonical loading internals**~~ — **DEFERRED (missing-leaked-source)**: `src/services/skillSearch/` is DCE'd at this build configuration. `loadRemoteSkill`, `getDiscoveredRemoteSkill`, `stripCanonicalPrefix`, `LoadResult` shape, AKI/GCS backend protocol, canonical-prefix string, cache key, and `LoadResult.fetchMethod` value space are all unknown. Recorded in spec 00 §2.5 Missing-Source Ledger and 00 §13 (gated by `EXPERIMENTAL_SKILL_SEARCH` feature flag).
2. ~~**`DiscoverSkills` tool name**~~ — **DEFERRED (missing-leaked-source)**: `tools/DiscoverSkillsTool/prompt.ts` is DCE'd at this build configuration. Same gating as Q1.
3. ~~**`isSkillSearchEnabled()` decision policy**~~ — **DEFERRED**: source for `services/skillSearch/featureCheck.ts` is DCE'd. The runtime predicate composition (over `feature('EXPERIMENTAL_SKILL_SEARCH')`) is not source-derivable from this leak.
4. ~~**`builtinPluginSkills` source and spec-28 cross-spec drift**~~ — **RESOLVED Phase 9.7**: spec 28 (plugins) was updated in Phase 9.6 to match `commands.ts:460-468` source order: `bundledSkills, builtinPluginSkills, skillDirCommands, workflowCommands, pluginCommands, pluginSkills, COMMANDS()`. Built-in plugin skills converted with `source:'bundled'`, `loadedFrom:'bundled'` (`plugins/builtinPlugins.ts:145-150`) confirmed.
5. ~~**`tengu_copper_panda` GrowthBook flag**~~ — **DEFERRED**: GrowthBook server-side artifact (per spec 26 §12). Source-default `false` at `skillImprovement.ts:178` documented; production value not source-derivable.
6. ~~**`recordSkillUsage` invocation completeness**~~ — **RESOLVED Phase 9.7**: confirmed by source-order trace: `SkillTool.ts:619` fires on inline path before any fork branch, `SkillTool.ts:1059` fires on remote path. `executeForkedSkill` correctly relies on prior `:619` call. Documented as load-bearing invariant in §5.
7. ~~**`Command.kind` field**~~ — **RESOLVED Phase 9.7**: spec 23 (service-mcp) §3 confirms MCP skill builders populate `Command.kind`; spec 28 (plugins) likewise for plugin-loaded skills. ANT-only telemetry usage at `SkillTool.ts:180-183, 705-708` is consumer-only.
8. ~~**`isOfficialMarketplaceSkill` predicate**~~ — **RESOLVED Phase 9.7**: spec 28 §3 enumerates `parsePluginIdentifier`/`isOfficialMarketplaceName` in `utils/plugins/pluginIdentifier.ts`; the "official" set is hard-coded in that module.
9. ~~**Conditional skill caching invariant**~~ — **NOTE Phase 9.7**: source-trace confirms `clearSkillCaches` invalidation triggers re-load on next read; conditional skills re-activate on file-op trigger evaluation post-clear. The invariant is preserved by design — clearing forces re-evaluation rather than silent stale state.
10. ~~**`shouldAutoEnableClaudeInChrome()` predicate**~~ — **DEFERRED (out-of-scope)**: predicate body lives in `utils/claudeInChrome/setup.ts` (spec 42 §A territory or future Claude-in-Chrome subsystem spec). The bundled-skill registry's use of the runtime predicate (instead of build-time `feature()`) is intentional per source comment.
