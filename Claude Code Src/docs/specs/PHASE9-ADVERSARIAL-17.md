# PHASE9 Adversarial Review - Spec 17 SkillTool & Skill Loading

## 1. Severity counts

- Critical: 0
- High: 3
- Medium: 6
- Low: 3

## 2. Findings list

### Finding 1 - High - §11.5 bundled skills enumeration is materially incomplete and misstates the file/register invariant

- Spec location: `docs/specs/17-tool-skill.md:1137-1155`
- Evidence paths:
  - `src/skills/bundled/index.ts:3-13`, `src/skills/bundled/index.ts:24-78`
  - `src/skills/bundled/debug.ts:12-19`
  - `src/skills/bundled/keybindings.ts:292-297`
  - `src/skills/bundled/skillify.ts:158-167`
  - `src/skills/bundled/loop.ts:74-79`
  - `src/skills/bundled/scheduleRemoteAgents.ts:324-335`
  - `src/skills/bundled/claudeApiContent.ts:1-3`
  - `src/skills/bundled/verifyContent.ts:1-10`
- Description: The spec says "Each file under `src/skills/bundled/` calls `registerBundledSkill(...)` exactly once" and catalogs 11 in-tree bundled skills. That is false. `index.ts`, `claudeApiContent.ts`, and `verifyContent.ts` do not register skills; they are registry/data files. The actual unconditional registry imports and calls `debug`, `keybindings`, `skillify`, and several ANT-only files omitted by §11.5. The actual in-tree files also include feature-gated `loop` and `scheduleRemoteAgents`, which the catalog omits even though they exist in this source tree.
- Suggested fix: Replace the invariant with "register modules call `registerBundledSkill`; content and registry files do not." Expand §11.5 into three buckets: unconditionally imported register modules, feature-gated modules, and data-only content modules. Include `debug`, `keybindings-help`, `skillify`, `loop`, and `schedule` with their real names and gates.

### Finding 2 - High - §11.5 misclassifies ANT-only bundled skills and omits real gates

- Spec location: `docs/specs/17-tool-skill.md:1147-1155`
- Evidence paths:
  - `src/skills/bundled/loremIpsum.ts:234-240`
  - `src/skills/bundled/skillify.ts:158-167`
  - `src/skills/bundled/stuck.ts:61-68`
  - `src/skills/bundled/verify.ts:12-21`
  - `src/skills/bundled/remember.ts:4-65`
  - `src/skills/bundled/claudeApi.ts:180-190`
  - `src/skills/bundled/index.ts:64-72`
- Description: The spec says only `remember` and `verify` are ANT-only. Source shows `lorem-ipsum`, `skillify`, and `stuck` also early-return unless `process.env.USER_TYPE === 'ant'`. Conversely, `claude-api` is feature-gated by `BUILDING_CLAUDE_APPS`, and `claude-in-chrome` is runtime-gated by `shouldAutoEnableClaudeInChrome()`.
- Suggested fix: Add a gate column to §11.5 and mark `lorem-ipsum`, `skillify`, `stuck`, `remember`, and `verify` as ANT-only. Mark `claude-api`, `loop`, `schedule`, `claude-in-chrome`, and missing DCE-only entries by their actual feature/runtime gates.

### Finding 3 - High - Skill listing delivery is described as system-prompt emission, but source emits `skill_listing` attachments

- Spec location: `docs/specs/17-tool-skill.md:11`, `docs/specs/17-tool-skill.md:241-252`, `docs/specs/17-tool-skill.md:831`
- Evidence paths:
  - `src/utils/attachments.ts:2661-2683`, `src/utils/attachments.ts:2692-2749`
  - `src/constants/prompts.ts:352-388`, `src/constants/prompts.ts:456-493`
  - `src/tools/SkillTool/prompt.ts:173-195`
- Description: The spec says the listing is rendered "inside the Skill-tool prompt" or "inside the system prompt block." The source actually builds the listing in `getSkillListingAttachments()` and emits an attachment of type `skill_listing`; the system prompt only includes guidance about `/skill-name` and DiscoverSkills, not the enumerated skill list. This matters because attachment resend/suppression, per-agent `sentSkillNames`, and experimental filtering control visibility.
- Suggested fix: Move the listing algorithm description under an attachment-delivery subsection. State that `SkillTool.prompt()` is fixed text, while `src/utils/attachments.ts` sends incremental `skill_listing` attachments using `formatCommandsWithinBudget`.

### Finding 4 - Medium - MCP skill inclusion is conflated between listing helpers and SkillTool lookup

- Spec location: `docs/specs/17-tool-skill.md:155-157`, `docs/specs/17-tool-skill.md:467`, `docs/specs/17-tool-skill.md:816-831`
- Evidence paths:
  - `src/tools/SkillTool/SkillTool.ts:81-94`
  - `src/commands.ts:547-559`, `src/commands.ts:563-580`
  - `src/utils/attachments.ts:2675-2683`
- Description: The spec says `getMcpSkillCommands` is called by SkillTool via `getAllCommands`, and implies `getSkillToolCommands` lists MCP entries. In source, `SkillTool.getAllCommands()` directly filters `context.getAppState().mcp.commands` for `type === 'prompt' && loadedFrom === 'mcp'` and does not call `getMcpSkillCommands`. The `MCP_SKILLS` gate is applied by the MCP client and by `getMcpSkillCommands()` in attachment listing, not by `SkillTool.getAllCommands()`. `getSkillToolCommands()` itself only reads local `getCommands(cwd)`; MCP skills are spliced in later by `attachments.ts`.
- Suggested fix: Split "model invocation resolution" from "skill listing attachment assembly." Document that `SkillTool` performs a local-first `uniqBy([...localCommands, ...mcpSkills], 'name')`, while listing uses `getSkillToolCommands(cwd)` plus `getMcpSkillCommands(appState.mcp.commands)`.

### Finding 5 - Medium - `getSlashCommandToolSkills` filtering is described too loosely and hides legacy/plugin differences

- Spec location: `docs/specs/17-tool-skill.md:37-39`, `docs/specs/17-tool-skill.md:157`, `docs/specs/17-tool-skill.md:831`
- Evidence paths:
  - `src/commands.ts:563-580`
  - `src/commands.ts:586-599`
- Description: The glossary says a skill is distinguished by `loadedFrom` including `commands_DEPRECATED`, but `getSlashCommandToolSkills()` excludes `commands_DEPRECATED` unless the command is included through the `disableModelInvocation` fallback. Meanwhile `getSkillToolCommands()` includes `commands_DEPRECATED` unconditionally if prompt/model-invocable. This is a subtle but real difference between "SkillTool listing" and "context-assembly skill counts."
- Suggested fix: Make the glossary explicitly distinguish "SkillTool-invocable prompt commands" from "slash-command-tool skills counted by `getSlashCommandToolSkills`." Add a small table showing legacy command inclusion differs between the two helpers.

### Finding 6 - Medium - Frontmatter parse failure edge case is missing

- Spec location: `docs/specs/17-tool-skill.md:706-729`, `docs/specs/17-tool-skill.md:1046-1079`
- Evidence paths:
  - `src/utils/frontmatterParser.ts:123-175`
  - `src/skills/loadSkillsDir.ts:447-469`
- Description: The spec covers invalid hooks and invalid effort but not malformed YAML frontmatter. Source strips the frontmatter block from `content` before parsing, retries with quoted problematic scalar values, and on final failure logs a warning and returns `frontmatter: {}` with `content` still sliced past the failed frontmatter block. That means the skill can still load with fallback description while the malformed frontmatter text is omitted from the prompt body.
- Suggested fix: Add an edge-case row: "Malformed YAML frontmatter logs a warning, drops parsed metadata, and still returns the body after the frontmatter block; fields fall back as if frontmatter were absent."

### Finding 7 - Medium - `user-invocable` default comment drift is not reconciled

- Spec location: `docs/specs/17-tool-skill.md:187`, `docs/specs/17-tool-skill.md:721-722`, `docs/specs/17-tool-skill.md:628-641`
- Evidence paths:
  - `src/skills/loadSkillsDir.ts:216-219`
  - `src/utils/frontmatterParser.ts:28-32`
  - `src/tools/SkillTool/prompt.ts:188-194`
  - `src/constants/prompts.ts:382-384`
- Description: The implementation defaults `userInvocable` to true for skills, while `frontmatterParser.ts` still comments that the default depends on source and says `skills/` defaults to false. The spec follows the implementation but does not flag this source-level drift. The prompt and system guidance also repeatedly frame slash commands as "skills," so the default controls a user-visible command surface.
- Suggested fix: Add a note that source comments in `frontmatterParser.ts` are stale relative to `parseSkillFrontmatterFields()`, or verify whether the implementation or comment is intended and align specs 17/20.

### Finding 8 - Medium - Skill triggering claims are mostly prompt-policy, not enforceable runtime behavior

- Spec location: `docs/specs/17-tool-skill.md:624-642`, `docs/specs/17-tool-skill.md:798-831`, `docs/specs/17-tool-skill.md:1139-1155`
- Evidence paths:
  - `src/tools/SkillTool/prompt.ts:173-195`
  - `src/skills/bundled/claudeApi.ts:183-186`
  - `src/types/command.ts:25-57`, `src/types/command.ts:175-203`
  - `src/skills/loadSkillsDir.ts:185-265`
- Description: Runtime matching is name resolution plus filtering; there is no parsed `triggers` frontmatter field and no deterministic semantic matcher in the visible non-experimental source. Bundled skills sometimes encode "TRIGGER when" text inside `description`, e.g. `claude-api`, but the loader treats that as ordinary description text. The spec partially admits this in §6.9, but the catalog and prompt excerpts can still read as if triggers are a runtime mechanism.
- Suggested fix: State plainly that "trigger" language is model-facing prose only unless `EXPERIMENTAL_SKILL_SEARCH` is active. Add "there is no `triggers` frontmatter key" to §6.5 or §9.

### Finding 9 - Medium - Plugin/bundled precedence cross-spec drift with spec 28

- Spec location: `docs/specs/17-tool-skill.md:467`, `docs/specs/17-tool-skill.md:1164`; cross-spec `docs/specs/28-service-plugins.md:638-647`
- Evidence paths:
  - `src/commands.ts:353-385`, `src/commands.ts:449-469`
  - `src/plugins/builtinPlugins.ts:132-158`
  - `docs/specs/28-service-plugins.md:638-647`
- Description: Spec 17 matches current `commands.ts` order: `bundledSkills`, `builtinPluginSkills`, `skillDirCommands`, workflows, plugin commands, plugin skills, built-ins. Spec 28's excerpt lists `pluginSkills` before `builtinPluginSkills`, which is not current source and conflicts with 17. Built-in plugin skills are also converted with `source:'bundled'`, `loadedFrom:'bundled'`, so they get bundled treatment in listing and telemetry.
- Suggested fix: Update spec 28 to match `src/commands.ts:460-467`, and in spec 17 clarify that built-in plugin skills are not `loadedFrom:'plugin'`; they intentionally masquerade as bundled for listing/telemetry.

### Finding 10 - Low - `claudeApiContent.ts` inlining is broadly correct but the catalog overstates lazy/file-map behavior

- Spec location: `docs/specs/17-tool-skill.md:1144-1145`, `docs/specs/17-tool-skill.md:1155`
- Evidence paths:
  - `src/skills/bundled/claudeApi.ts:5-7`, `src/skills/bundled/claudeApi.ts:55-93`, `src/skills/bundled/claudeApi.ts:180-194`
  - `src/skills/bundled/claudeApiContent.ts:1-3`, `src/skills/bundled/claudeApiContent.ts:47-75`
  - `src/skills/bundled/verify.ts:17-28`
- Description: The Phase 10d claim is partly verified: `claudeApiContent.ts` inlines Markdown via Bun text-loader imports and `claudeApi.ts` lazy-imports it on invocation. But unlike `verify`, `claude-api` does not pass a `files` map to `registerBundledSkill`; it inlines selected docs into the prompt body with `<doc>` tags. The §11.5 wording "lazy-loaded or fed via the `files` map" is easy to misread as both content modules using the bundled extraction path.
- Suggested fix: Say: "`claude-api` lazy-imports text-loader content and embeds docs into the prompt; `verify` passes `SKILL_FILES` through `files` for extraction."

### Finding 11 - Low - Source map lists line counts and flags for absent DCE paths too authoritatively

- Spec location: `docs/specs/17-tool-skill.md:58-63`, `docs/specs/17-tool-skill.md:79-97`, `docs/specs/17-tool-skill.md:1028-1035`
- Evidence paths:
  - `src/skills/bundled/index.ts:35-78`
  - `find src/skills/bundled -maxdepth 1 -type f` observed no `dream.ts`, `hunter.ts`, or `runSkillGenerator.ts` in this tree
- Description: The spec correctly notes several DCE'd paths are absent, but still treats their registration behavior and file identities as if fully verified. This is acceptable if labeled as compile-branch evidence from conditional `require()` call sites, but not as source-verified module contents.
- Suggested fix: Mark absent feature-gated modules as "required by call site, module body absent/hardest-to-verify" rather than cataloging them as normal bundled skills.

### Finding 12 - Low - Multiple matching skills and alias precedence is under-specified

- Spec location: `docs/specs/17-tool-skill.md:798-831`, `docs/specs/17-tool-skill.md:1046-1079`
- Evidence paths:
  - `src/tools/SkillTool/SkillTool.ts:81-94`, `src/tools/SkillTool/SkillTool.ts:398-403`
  - `src/commands.ts:348-350`, `src/commands.ts:460-467`, `src/commands.ts:491-516`
- Description: The spec says `findCommand` handles exact names, namespaced names, or aliases, but it does not adversarially describe collision behavior: local commands are placed before MCP skills in the SkillTool lookup `uniqBy`, dynamic skills are only added if their name is absent from base commands, and source order decides first-wins among many loaded sources. I did not open `findCommand` under the file budget, so exact alias precedence is hardest-to-verify within this pass.
- Suggested fix: Add a "collision and alias precedence" edge-case subsection and either cite `findCommand` directly or mark alias precedence as delegated to spec 20.

## 3. Cross-spec impact

- Spec 03 QueryEngine: Spec 17 should align its QueryEngine interaction to turn-scoped `discoveredSkillNames` and `skillPrefetch` as described in `docs/specs/03-query-engine.md:278`, `docs/specs/03-query-engine.md:1067`, and source `src/QueryEngine.ts:192-238`. The static skill listing itself is not a QueryEngine system prompt concern; it is emitted through attachments.
- Spec 05 Context assembly / attachments: The skill listing delivery belongs partly to attachment assembly. Source `src/utils/attachments.ts:2661-2749` is the concrete owner of initial/dynamic `skill_listing` attachments, sent-name suppression, MCP splice-in, and experimental filtering.
- Spec 21 Commands: Spec 17 correctly points to spec 20/21 for `findCommand`, but it still needs to name the `getSkillToolCommands` vs `getSlashCommandToolSkills` split. Otherwise command catalog readers will assume all "skills" surfaces have the same inclusion rules.
- Spec 28 Plugins: Spec 28 appears stale on plugin skill ordering. Source `src/commands.ts:460-467` places `builtinPluginSkills` before `skillDirCommands` and before marketplace `pluginSkills`; spec 28 says `pluginSkills` before `builtinPluginSkills`. Built-in plugin skills are also represented as `source:'bundled'`, `loadedFrom:'bundled'` in `src/plugins/builtinPlugins.ts:145-150`, which affects the truncation exemption and telemetry classification documented in spec 17.
- Spec 23 MCP: Spec 17 should not imply `SkillTool.getAllCommands()` enforces the `MCP_SKILLS` feature gate; the MCP client populates `loadedFrom:'mcp'` skills behind that gate, while listing uses `getMcpSkillCommands()`.

## 4. Hardest-to-verify claim

The hardest-to-verify claim within the requested file budget is the exact alias precedence inside `findCommand` when multiple commands/skills share names or aliases across bundled, built-in-plugin, disk, plugin, dynamic, and MCP sources. I verified source ordering and local-vs-MCP `uniqBy` behavior, but did not open the full `findCommand` implementation or plugin loader internals beyond the direct ordering evidence.

Remote canonical skill internals are also hardest-to-verify because `src/services/skillSearch/*` is absent/DCE'd; only the conditional call sites and expected return shape in `SkillTool.ts` can be verified from this tree.

## 5. Verdict

Spec needs major revision.

The core `SkillTool` invocation, frontmatter parser, permission allowlist, and bundled extraction mechanics are mostly well grounded. The major revision is needed because §11.5's bundled catalog is wrong/incomplete, the skill-listing delivery path is assigned to the wrong layer, and MCP/plugin/bundled precedence is blurred enough to mislead reimplementations and adjacent specs.
