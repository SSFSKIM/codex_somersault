# PHASE 9.6 B-full RETRY — Spec 17 Fix Log

Source: `docs/specs/PHASE9-ADVERSARIAL-17.md` (3 high, 6 medium, 3 low — total 12 findings).

## Coverage

All 3 high and all 6 medium findings applied. Low findings folded in opportunistically.

## Findings applied

### High

- **Finding 1 — §11.5 incomplete & misstates invariant.** Replaced §11.5 with three buckets: 11.5.1 unconditional register modules, 11.5.2 feature-gated register modules, 11.5.3 data/content modules. Restated the invariant: "register modules call `registerBundledSkill`; content/registry/init files do not." Added `debug`, `keybindings-help`, `skillify`, `loop`, `schedule`, plus the absent `dream`/`hunter`/`runSkillGenerator` entries. Verified each in source.
- **Finding 2 — §11.5 misclassifies ANT-only skills.** Verified directly:
  - `loremIpsum.ts:234-237` `if (process.env.USER_TYPE !== 'ant') return` → ANT-only.
  - `skillify.ts:158-161` same → ANT-only.
  - `stuck.ts:62-64` same → ANT-only.
  - `verify.ts:13-15` same → ANT-only (already noted).
  - `remember.ts:5-7` same → ANT-only (already noted).
  - `claudeApi.ts` is `BUILDING_CLAUDE_APPS`-gated at `index.ts:64-69` (build-time `feature()`), not a USER_TYPE gate. Updated.
  - `claudeInChrome.ts` runtime-gated by `shouldAutoEnableClaudeInChrome()` at `index.ts:70-72`. Updated.
  - Added a §11.5.4 summary table by gate kind.
- **Finding 3 — listing as system-prompt vs `skill_listing` attachment.** Fixed in §1 purpose, glossary entry "Skill listing", §1 IN-scope list, and §5.1 ("SkillTool's own `prompt()` is fixed instruction text only"). The full attachment delivery flow (`getSkillListingAttachments`, `sentSkillNames`, `suppressNext`, MCP splice via `getMcpSkillCommands`, `formatCommandsWithinBudget` formatting, `EXPERIMENTAL_SKILL_SEARCH` filter) is now described and pointed at `utils/attachments.ts:2607-2751`, with ownership delegated to spec **05**.

### Medium

- **Finding 4 — MCP skill conflation between SkillTool lookup and listing.** Fixed in §3.2 entries for `getMcpSkillCommands`/`getSkillToolCommands` and again in §5.7 with an explicit two-channel split. Listing path: `getSkillToolCommands(cwd)` ∪ `getMcpSkillCommands(appState.mcp.commands)`. SkillTool model-invocation path: inline filter on `appState.mcp.commands` for `prompt + loadedFrom==='mcp'`, NOT calling `getMcpSkillCommands` (verified at `SkillTool.ts:81-94`). MCP_SKILLS feature gate clarified as enforced upstream by MCP client.
- **Finding 5 — `getSlashCommandToolSkills` filtering loose.** Added a side-by-side inclusion-rule table to §5.7 distinguishing the two helpers; updated glossary with the "two slightly different surfaces" subitem citing `commands.ts:563-608`.
- **Finding 6 — Malformed YAML frontmatter edge case.** Added an explicit row to §9 and a paragraph in §6.5 describing the retry-then-warn-then-empty-frontmatter behavior (`utils/frontmatterParser.ts:148-175`), confirming the body is sliced past the failed block.
- **Finding 7 — `user-invocable` default doc-comment drift.** Added a paragraph in §6.5 that the default in `parseSkillFrontmatterFields` and `skillDefinitionToCommand` is `true` regardless of source, and that the `frontmatterParser.ts:28-32` doc comment claiming "skills/ defaults to false" is **stale**. Verified at `loadSkillsDir.ts:216-219` and `plugins/builtinPlugins.ts:143`.
- **Finding 8 — Trigger language is prompt prose only.** Added a paragraph in §6.5: "There is no `triggers` frontmatter key" and `claude-api`'s "TRIGGER when …" prose is treated as ordinary description text. Cited `bundled/claudeApi.ts:183-186` and `loadSkillsDir.ts:185-265`.
- **Finding 9 — Plugin/bundled precedence drift with spec 28.** Added a note in §5.7 and an expanded note in §12 (Open Questions) flagging that spec 28 line 638-647 lists `pluginSkills` before `builtinPluginSkills` but source `commands.ts:460-468` is the opposite. Also documented that built-in plugin skills are `source:'bundled', loadedFrom:'bundled'` (verified at `plugins/builtinPlugins.ts:145-150`).

### Low

- **Finding 10 — `claudeApiContent` lazy-load wording.** Fixed in §11.5.3 ("`claude-api` lazy-imports content and embeds docs into the prompt; `verify` passes `SKILL_FILES` through `files` for extraction"). Phrased so the two patterns are visibly different even though both rely on text-loader inlining.
- **Finding 11 — Source-map authority for absent DCE'd modules.** §11.5.2 now marks DCE'd modules with "(absent in leak)" and notes only call-site evidence is verifiable; deferred to 00 §2.5 ledger.
- **Finding 12 — Multiple matching skills / alias precedence.** Added a "Collision and alias precedence" subsection at the end of §6.9 documenting first-wins ordering, dynamic-skill insertion rule (only if name absent), and `uniqBy` local-vs-MCP behavior. Exact `findCommand` alias logic delegated to spec **20**.

## Top 5 fixes with src evidence

1. **§11.5 rewrite**: bucketed catalog + corrected gates. Evidence: `bundled/index.ts:24-79` (init order + gates), `bundled/loremIpsum.ts:234-237`, `bundled/skillify.ts:158-161`, `bundled/stuck.ts:61-64` (ANT early-returns), `bundled/keybindings.ts:292-299` (registered name `keybindings-help`), `bundled/loop.ts:74-83` and `bundled/scheduleRemoteAgents.ts:324-334` (present in tree, `isEnabled` gates), `bundled/claudeApiContent.ts:1-3` and `bundled/verifyContent.ts:1-13` (data-only, no `registerBundledSkill`).
2. **Skill listing → attachment delivery**. Evidence: `utils/attachments.ts:2661-2751` is the owner; `tools/SkillTool/prompt.ts:173-196` is fixed instruction text only. `sentSkillNames` per-agent suppression at `attachments.ts:2607, 2699-2715`. Cross-ref to spec 05.
3. **MCP skill dual channel**. Evidence: `SkillTool.ts:81-94` filters `mcp.commands` inline (no helper call); `attachments.ts:2677-2683` calls `getMcpSkillCommands`; `commands.ts:547-559` shows the helper itself is `MCP_SKILLS`-gated.
4. **`getSkillToolCommands` vs `getSlashCommandToolSkills`**. Evidence: `commands.ts:563-581` (former includes `commands_DEPRECATED` unconditionally) vs `commands.ts:586-608` (latter excludes it; requires description even for bundled). Now reflected in glossary, §3.2, §5.7.
5. **Built-in plugin skills masquerade as bundled**. Evidence: `plugins/builtinPlugins.ts:132-158` `skillDefinitionToCommand` returns `source:'bundled', loadedFrom:'bundled'`. Cross-spec note added flagging spec 28 ordering drift.

## Classification corrections in §11.5

| Skill | Old | New (verified) |
|---|---|---|
| `lorem-ipsum` | (no gate noted) | **ANT-only** (`loremIpsum.ts:234-237`) |
| `skillify` | (not in catalog) | added; **ANT-only** + `disableModelInvocation: true` |
| `stuck` | (no ANT gate noted) | **ANT-only** (`stuck.ts:62-64`) |
| `keybindings-help` | (not in catalog) | added; unconditional + `isEnabled: isKeybindingCustomizationEnabled` |
| `debug` | (not in catalog) | added; unconditional, `disableModelInvocation: true`, description swaps on USER_TYPE |
| `loop` | (only mentioned as DCE'd) | present in tree; `feature('AGENT_TRIGGERS')` gate + `isKairosCronEnabled` runtime gate |
| `schedule` | (only mentioned as DCE'd) | present in tree; `feature('AGENT_TRIGGERS_REMOTE')` gate + `tengu_surreal_dali` GB + `allow_remote_sessions` policy |
| `claude-api` | implied always-bundled | `feature('BUILDING_CLAUDE_APPS')`-gated at register call site |
| `claude-in-chrome` | implied always-bundled | runtime-gated at index.ts call site by `shouldAutoEnableClaudeInChrome()` |
| `claudeApiContent.ts` | catalogued as data | reaffirmed: data-only; lazy-imported by `claude-api`; **does NOT use `files:` map** |
| `verifyContent.ts` | catalogued as data | reaffirmed: data-only; eager-imported by `verify`; **does pass `SKILL_FILES` through `files:`** |

## Skipped / not-applied

None. All findings actioned.

## Verify-before-edit notes

- All ANT-only claims for `lorem-ipsum`/`skillify`/`stuck` were verified by reading the `register*Skill` body in source.
- `loop.ts` and `scheduleRemoteAgents.ts` are present in this tree (contradicts spec's prior implication that they are DCE'd). Their `index.ts` call sites are still feature-gated; only the modules themselves ship in this leak.
- The frontmatterParser doc-comment is genuinely stale relative to its consumer — flagged as such rather than rewritten, since this spec doesn't own that file.

## Post-fix verdict

**Major revision applied. Spec is now consistent with source on §11.5 catalog, listing-delivery layer, MCP/local helper split, and `getSkillToolCommands` vs `getSlashCommandToolSkills` semantics.** Spec 28 still needs an independent fix to its plugin ordering excerpt (flagged in §12 Open Questions for cross-spec follow-up). Remaining hardest-to-verify items (alias precedence inside `findCommand`, remote-skill backend internals) are explicitly delegated to specs 20 and to 00's Missing-Source Ledger.
