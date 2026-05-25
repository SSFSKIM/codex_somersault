# 38 — Output Styles Specification

> Spec 38 of 42. Owner sub-H8. Last updated 2026-05-09.
> Adjacent: 05 (system-prompt assembly), 37 (Ink UI shell).

---

## 1. Purpose & Scope

The output-style subsystem selects a named "response shape" preset and injects its prompt fragment into the assistant's system prompt. A style governs how Claude communicates (concise default, educational `Explanatory`, hands-on `Learning`, or arbitrary user/plugin-defined fragments) and can optionally suppress the default coding-instructions block. Selection is persisted to settings as the string `outputStyle`; the picker UI lives in `OutputStylePicker.tsx` and is reachable through `/config` (the legacy `/output-style` slash command is deprecated).

**IN scope:**

- `src/outputStyles/loadOutputStylesDir.ts` — filesystem discovery of `.md` styles.
- `src/constants/outputStyles.ts` — built-in registry, type schema, aggregation, selection.
- `src/utils/plugins/loadPluginOutputStyles.ts` — plugin-sourced styles.
- `src/commands/output-style/` — deprecated slash command shell.
- `src/components/OutputStylePicker.tsx` — selection dialog.
- The single output-style fragment slot in system-prompt assembly (cite to 05).
- Persistence: `outputStyle` in settings (cite to 02), `tipsHistory` config (for tips assignment, see below).
- `src/services/tips/`, `src/services/MagicDocs/`, `src/services/PromptSuggestion/` — assigned **HERE** per overview §2.3 recommendation. These three are textual-augmentation services that overlay the assistant's surface output (spinner tips, magic-doc rewrite agent, predicted-prompt suggestions). See §12 for boundary notes.

**OUT of scope:** System-prompt assembly mechanics (→ 05), Ink shell rendering and `Dialog`/`Select` primitives (→ 37), settings storage/precedence (→ 02), plugin loader internals (→ 28).

---

## 2. Source Map

### 2.1 Source-coverage inventory

| Path | Status | Lines | Notes |
|---|---|---|---|
| `src/outputStyles/loadOutputStylesDir.ts` | read fully | 99 | Sole file in the dir. |
| `src/constants/outputStyles.ts` | read fully | 217 | Built-in registry, prompts. |
| `src/utils/plugins/loadPluginOutputStyles.ts` | read fully | 179 | Plugin discovery. |
| `src/commands/output-style/index.ts` | read fully | 11 | Deprecated stub. |
| `src/commands/output-style/output-style.tsx` | read fully | (compiled) | Deprecation message. |
| `src/components/OutputStylePicker.tsx` | read fully | (compiled) | Picker dialog. |
| `src/services/tips/tipRegistry.ts` | read fully | 686 | All built-in tip definitions. |
| `src/services/tips/tipScheduler.ts` | read fully | 58 | Selection. |
| `src/services/tips/tipHistory.ts` | read fully | 17 | Cooldown bookkeeping. |
| `src/services/tips/types.ts` | **missing-leaked-source** | n/a | Imported by `tipScheduler.ts:8` and `tipRegistry.ts:57` but not present in tree. See §12. |
| `src/services/MagicDocs/magicDocs.ts` | read fully | 254 | Hook + agent dispatch. |
| `src/services/MagicDocs/prompts.ts` | read fully | 127 | Update prompt template. |
| `src/services/PromptSuggestion/promptSuggestion.ts` | sampled (entrypoint+enable gate) | 17065 B | Detail deferred (§12). |
| `src/services/PromptSuggestion/speculation.ts` | sampled | 30680 B | Speculative pre-call. |
| Cross-cutters | grepped | — | `prompts.ts` 28/99/151-157/176/180/349/457/506/562-565; `attachments.ts` 440/556/952/1597-1611; `messages.ts` 26/3798-3808; `messages/systemInit.ts` 4/55/75; `StatusLine.tsx` 8/44/81; `Settings/Config.tsx` 10/28/40/84/103/718-722/1121/1300/1320/1532-1541; `cli/print.ts` 161/4430/4467; `services/api/withRetry.ts` 60-67; `services/compact/microCompact.ts` 240-251/520-525; `utils/promptCategory.ts` 5/38/47-48; `utils/settings/types.ts` 639-642/664-686; `utils/plugins/schemas.ts` 507-525/891; `utils/plugins/pluginLoader.ts` 1370-1410/1586-1609/2658-2700/2882-2900; `types/plugin.ts` 64-65; `utils/plugins/cacheUtils.ts` 4. |

### 2.2 Feature-flag and ANT guards

- **No `feature(...)` gate** on the core output-style code. Always compiled.
- ANT-only carve-outs:
  - `tipRegistry.ts:635-653` — `internalOnlyTips` (`important-claudemd`, `skillify`) loaded only when `process.env.USER_TYPE === 'ant'`.
  - `tipRegistry.ts:112` — `plan-mode-for-complex-tasks.isRelevant` returns `false` for ANT.
  - `tipRegistry.ts:404-406` — `shift-tab` content variant differs for ANT.
  - `tipRegistry.ts:478-487` — `opusplan-mode-reminder` returns `false` for ANT.
  - `tipRegistry.ts:626-632` — `feedback-command` returns `false` for ANT.
  - `magicDocs.ts:243-252` — `initMagicDocs()` registers listeners ONLY when `USER_TYPE === 'ant'`.
- GrowthBook A/B variants drive copy for `effort-high-nudge`, `subagent-fanout-nudge`, `loop-command-nudge` (`tengu_tide_elm`, `tengu_tern_alloy`, `tengu_timber_lark`).

### 2.3 Imports / Imported by

`outputStyles/loadOutputStylesDir.ts` imports `lodash-es/memoize`, `path.basename`, `constants/outputStyles.js`, `utils/debug.js`, `utils/frontmatterParser.js`, `utils/log.js`, `utils/markdownConfigLoader.js`, `utils/plugins/loadPluginOutputStyles.js` (`loadOutputStylesDir.ts:1-11`).

`constants/outputStyles.ts` is imported by `OutputStylePicker.tsx`, `Settings/Config.tsx`, `StatusLine.tsx`, `messages/systemInit.ts`, `cli/print.ts`, `constants/prompts.ts`, `utils/messages.ts`, `utils/promptCategory.ts`, `utils/plugins/cacheUtils.ts`, `outputStyles/loadOutputStylesDir.ts`.

---

## 3. Public Interface (Contract)

```ts
// src/constants/outputStyles.ts:11-23
export type OutputStyleConfig = {
  name: string
  description: string
  prompt: string
  source: SettingSource | 'built-in' | 'plugin'
  keepCodingInstructions?: boolean
  forceForPlugin?: boolean
}

// src/constants/outputStyles.ts:25-27
export type OutputStyles = {
  readonly [K in OutputStyle]: OutputStyleConfig | null
}

// src/utils/config.ts:181
export type OutputStyle = string
```

Functions exposed:

- `getAllOutputStyles(cwd: string): Promise<{ [styleName: string]: OutputStyleConfig | null }>` — memoized aggregator (`constants/outputStyles.ts:137-175`).
- `clearAllOutputStylesCache(): void` (`constants/outputStyles.ts:177-179`).
- `getOutputStyleConfig(): Promise<OutputStyleConfig | null>` — applies forced-plugin override then settings lookup (`constants/outputStyles.ts:181-211`).
- `hasCustomOutputStyle(): boolean` (`constants/outputStyles.ts:213-216`).
- `DEFAULT_OUTPUT_STYLE_NAME = 'default'` (`constants/outputStyles.ts:39`).
- `getOutputStyleDirStyles(cwd): Promise<OutputStyleConfig[]>` — memoized filesystem loader (`outputStyles/loadOutputStylesDir.ts:26-92`).
- `clearOutputStyleCaches(): void` (`outputStyles/loadOutputStylesDir.ts:94-98`).
- `loadPluginOutputStyles(): Promise<OutputStyleConfig[]>` (`utils/plugins/loadPluginOutputStyles.ts:87-174`).
- `clearPluginOutputStyleCache(): void` (`utils/plugins/loadPluginOutputStyles.ts:176-178`).

Settings field: `outputStyle: z.string().optional()` (`utils/settings/types.ts:639-642`).

Plugin manifest field: `outputStyles` accepting a single `RelativePath` or an array of `RelativePath` (`utils/plugins/schemas.ts:507-525`); merged into the manifest via `PluginManifestSchema` (`schemas.ts:891`).

Plugin runtime fields:

```ts
// src/types/plugin.ts:64-65
outputStylesPath?: string
outputStylesPaths?: string[] // Additional output style paths from manifest
```

The deprecated slash command (`commands/output-style/index.ts:1-11`) is registered with `type: 'local-jsx'`, `name: 'output-style'`, `description: 'Deprecated: use /config to change output style'`, `isHidden: true`.

---

## 4. Data Model & State

- **In-memory state**: three module-level memoized caches (lodash `memoize`):
  1. `getOutputStyleDirStyles(cwd)` — keyed by cwd (`loadOutputStylesDir.ts:26`).
  2. `getAllOutputStyles(cwd)` — keyed by cwd (`outputStyles.ts:137`).
  3. `loadPluginOutputStyles()` — no-arg memo (`loadPluginOutputStyles.ts:87`).
- **Cache invalidation**: `clearOutputStyleCaches()` clears both filesystem and plugin caches plus the upstream `loadMarkdownFilesForSubdir.cache` (`loadOutputStylesDir.ts:94-98`); `clearAllOutputStylesCache()` clears the aggregator. `cacheUtils.ts:4` imports `clearAllOutputStylesCache`, indicating plugin-reload paths invalidate output styles together with other plugin caches.
- **Persistence**: the chosen style is the string `outputStyle` in settings (`utils/settings/types.ts:639-642`); read from settings via `getSettings_DEPRECATED().outputStyle` and falls back to `'default'` everywhere it is consumed (`outputStyles.ts:206-208`, `messages/systemInit.ts:55`, `attachments.ts:1599`, `StatusLine.tsx:44`, `cli/print.ts:4430`).
- **Tips persistence**: per-tip last-shown session number lives in `getGlobalConfig().tipsHistory: Record<string, number>` (`tipHistory.ts:3-9`; config typed in `utils/config.ts:265`, default `{}` at `:606`, persisted via `saveGlobalConfig`).
- **Magic Docs in-memory state**: `trackedMagicDocs: Map<string, MagicDocInfo>` keyed by absolute file path (`magicDocs.ts:42`); cleared via `clearTrackedMagicDocs()` (`:44-46`).

State machine (output-style resolution):

```
read settings.outputStyle (string|undefined)
  ↓
getAllOutputStyles(cwd)
  ↓
filter forced-plugin styles → first-wins (warn if >1)
  ↓
if forced: return forced
else: return allStyles[settings.outputStyle ?? 'default'] ?? null
```

---

## 5. Algorithm / Control Flow

### 5.1 Filesystem discovery (`getOutputStyleDirStyles`)

Per `loadOutputStylesDir.ts:26-92`:

```
for each markdown file under <cwd-walk>/.claude/output-styles/*.md
                            and ~/.claude/output-styles/*.md
                            (project entries override user; loader provides `source` field):
  styleName = basename(filePath).replace(/\.md$/, '')
  name        = frontmatter.name ?? styleName
  description = coerceDescriptionToString(frontmatter.description, styleName)
                ?? extractDescriptionFromMarkdown(content,
                    `Custom ${styleName} output style`)
  keepCodingInstructionsRaw = frontmatter['keep-coding-instructions']
  keepCodingInstructions =
        keepCodingInstructionsRaw === true || keepCodingInstructionsRaw === 'true'  → true
        keepCodingInstructionsRaw === false || keepCodingInstructionsRaw === 'false' → false
        otherwise                                                                    → undefined
  if frontmatter['force-for-plugin'] !== undefined: warn-debug (ignored on non-plugin styles)
  return { name, description, prompt: content.trim(), source, keepCodingInstructions }
errors per file → logError; final filter drops null entries; outer try/catch returns []
```

Discovery uses `loadMarkdownFilesForSubdir('output-styles', cwd)` whose own algorithm is owned by the markdown-config-loader (out of scope). The `source` field is set by that upstream loader and is one of `'projectSettings'`, `'userSettings'`, `'policySettings'`, etc.

### 5.2 Plugin discovery (`loadPluginOutputStyles`)

`loadPluginOutputStyles.ts:87-174`:

```
{ enabled, errors } = loadAllPluginsCacheOnly()
log errors (debug)
for each plugin in enabled:
  loadedPaths = new Set<string>()
  if plugin.outputStylesPath:
    walkPluginMarkdown(plugin.outputStylesPath, fullPath →
      loadOutputStyleFromFile(fullPath, plugin.name, loadedPaths))
  for stylePath in plugin.outputStylesPaths ?? []:
    stat = fs.stat(stylePath)
    if directory: walk it (same as above)
    elif file && endsWith('.md'): loadOutputStyleFromFile
log total
```

Per-file (`loadPluginOutputStyles.ts:36-85`):

```
if isDuplicatePath(fs, filePath, loadedPaths): return null
content = fs.readFile(filePath, utf-8)
{ frontmatter, content: markdownContent } = parseFrontmatter(content, filePath)
fileName = basename(filePath, '.md')
baseStyleName = (frontmatter.name as string) || fileName
name = `${pluginName}:${baseStyleName}`     // namespaced (consistent with commands/agents)
description = coerceDescriptionToString(frontmatter.description, name)
              ?? extractDescriptionFromMarkdown(markdownContent,
                   `Output style from ${pluginName} plugin`)
forceRaw = frontmatter['force-for-plugin']
forceForPlugin =
   forceRaw === true || forceRaw === 'true'   → true
   forceRaw === false || forceRaw === 'false' → false
   otherwise                                  → undefined
return { name, description, prompt: markdownContent.trim(),
         source: 'plugin', forceForPlugin }
```

### 5.3 Aggregation (`getAllOutputStyles`)

`outputStyles.ts:137-175`:

```
customStyles = await getOutputStyleDirStyles(cwd)
pluginStyles = await loadPluginOutputStyles()
allStyles = { ...OUTPUT_STYLE_CONFIG }      // default:null, Explanatory, Learning

managedStyles = customStyles where source==='policySettings'
userStyles    = customStyles where source==='userSettings'
projectStyles = customStyles where source==='projectSettings'

// priority lowest → highest:
for styles in [pluginStyles, userStyles, projectStyles, managedStyles]:
  for style in styles:
    allStyles[style.name] = { name, description, prompt, source,
                              keepCodingInstructions, forceForPlugin }
return allStyles
```

Effective precedence (later writers override earlier): built-in < plugin < user < project < managed. Note the local file scope for `customStyles` only filters three sources; `customStyles` whose `source` is anything else (e.g. `'localSettings'`, `'flagSettings'`) are silently dropped from aggregation (`outputStyles.ts:148-159`).

> **Source-comment bug.** The inline comment at `outputStyles.ts:158` reads `built-in, plugin, managed, user, project` — but the array literal at `:159` is `[pluginStyles, userStyles, projectStyles, managedStyles]`, so the actual last-writer-wins order is **managed**, not project. The spec transcribes the array correctly above; the comment in source is wrong. See **BUGS-IN-SOURCE.md §8** ("`getAllOutputStyles` priority-order comment contradicts the array literal", cosmetic).

### 5.4 Resolution (`getOutputStyleConfig`)

`outputStyles.ts:181-211`:

```
allStyles = getAllOutputStyles(getCwd())
forcedStyles = values(allStyles) where source==='plugin' && forceForPlugin===true
if forcedStyles[0]:
  if forcedStyles.length > 1: log-warn `Multiple plugins have forced output styles: ...`
  log-debug `Using forced plugin output style: <name>`
  return forcedStyles[0]
settings = getSettings_DEPRECATED()
outputStyle = settings?.outputStyle || 'default'
return allStyles[outputStyle] ?? null
```

`'default'` maps to `null` in `OUTPUT_STYLE_CONFIG`, which is the signal further downstream that no fragment should be added.

### 5.5 System-prompt assembly (cross-cuts to spec 05)

The output-style fragment is rendered by `getOutputStyleSection` and inserted as a **dynamic** section (cacheable across turns but distinct from the static intro):

```
// constants/prompts.ts:151-158
function getOutputStyleSection(cfg: OutputStyleConfig | null): string | null {
  if (cfg === null) return null
  return `# Output Style: ${cfg.name}\n${cfg.prompt}`
}
```

Assembly path (`constants/prompts.ts:457-578`):

```
[skillToolCommands, outputStyleConfig, envInfo] = await Promise.all([…, getOutputStyleConfig(), …])
…
dynamicSections = [
  systemPromptSection('session_guidance', …),
  systemPromptSection('memory', …),
  systemPromptSection('ant_model_override', …),
  systemPromptSection('env_info_simple', …),
  systemPromptSection('language', …),
  systemPromptSection('output_style', () => getOutputStyleSection(outputStyleConfig)),
  DANGEROUS_uncachedSystemPromptSection('mcp_instructions', …),
  // NOTE: `mcp_instructions` is a static slot; live MCP server instructions
  // routed through `isMcpInstructionsDeltaEnabled()` are emitted as
  // `mcp_instructions_delta` *attachments* (attachments.ts) rather than as a
  // dynamic-prompt section. See `prompts.ts:480-481, 509, 516` and
  // attachments.ts wiring. Cross-cut: spec 16 (MCP) + spec 05 (assembly) are
  // joint owners; spec 38 only documents the slot's *position* in the list.
  systemPromptSection('scratchpad', …),
  systemPromptSection('frc', …),
  systemPromptSection('summarize_tool_results', …),
  …(USER_TYPE==='ant' ? [numeric_length_anchors] : []),
  …(feature('TOKEN_BUDGET') ? [token_budget] : []),
  …(feature('KAIROS') || feature('KAIROS_BRIEF') ? [brief] : []),
]
return [
  getSimpleIntroSection(outputStyleConfig),     // static, identity framing
  getSimpleSystemSection(),                     // static
  outputStyleConfig === null
   || outputStyleConfig.keepCodingInstructions === true
       ? getSimpleDoingTasksSection()
       : null,                                  // ↞ key branch
  getActionsSection(),
  getUsingYourToolsSection(enabledTools),
  getSimpleToneAndStyleSection(),
  getOutputEfficiencySection(),
  …(shouldUseGlobalCacheScope() ? [SYSTEM_PROMPT_DYNAMIC_BOUNDARY] : []),
  …resolvedDynamicSections,
].filter(s => s !== null)
```

Two integration points worth highlighting:

1. The **intro section** changes wording when a non-null style is active (`prompts.ts:178-184`): static intro reads `"You are an interactive agent that helps users according to your \"Output Style\" below, which describes how you should respond to user queries."` versus `"... helps users with software engineering tasks."` for default. The intro is intentionally NOT moved to the dynamic registry pending eval (`prompts.ts:347-350` comment).
2. The **doing-tasks coding-instructions block** is suppressed when a style declares `keepCodingInstructions: false` or omits it (built-in `Explanatory` and `Learning` set `true` so the block stays).

### 5.6 Mid-turn injection (attachments)

Each conversation-side attachment append also injects a `<system-reminder>` reinforcing the active style (`attachments.ts:945-953,1597-1611`):

```
// attachments.ts:1597-1611
function getOutputStyleAttachment(): Attachment[] {
  const settings = getSettings_DEPRECATED()
  const outputStyle = settings?.outputStyle || 'default'
  if (outputStyle === 'default') return []
  return [{ type: 'output_style', style: outputStyle }]
}
```

The attachment is rendered into a meta user message wrapped in `<system-reminder>` (`messages.ts:3796-3810`):

```
case 'output_style': {
  const outputStyle = OUTPUT_STYLE_CONFIG[attachment.style as keyof typeof OUTPUT_STYLE_CONFIG]
  if (!outputStyle) return []
  return wrapMessagesInSystemReminder([
    createUserMessage({
      content: `${outputStyle.name} output style is active. Remember to follow the specific guidelines for this style.`,
      isMeta: true,
    }),
  ])
}
```

Note the lookup is `OUTPUT_STYLE_CONFIG[attachment.style]` only — custom or plugin styles return `[]` (no reminder).

### 5.7 Picker UI

`OutputStylePicker.tsx`:

- On mount: `getAllOutputStyles(getCwd())` → `mapConfigsToOptions(allStyles)`; on rejection, falls back to `mapConfigsToOptions(OUTPUT_STYLE_CONFIG)`.
- `mapConfigsToOptions`: `Object.entries(styles).map(([style, config]) => ({ label: config?.name ?? 'Default', value: style, description: config?.description ?? 'Claude completes coding tasks efficiently and provides concise responses' }))`.
- Renders `<Dialog title="Preferred output style">` with optional dim-color text `"This changes how Claude Code communicates with you"` and a `<Select>` of up to 10 visible options.
- `defaultValue={initialStyle}`; on change calls `onComplete(style as OutputStyle)`.

The picker is invoked via `Settings/Config.tsx` submenu `'OutputStyle'` (id `outputStyle`, label/value/description pulled from local state). On confirmation it formats the change as `"Set output style to <bold>${currentOutputStyle}</bold>"` (`Config.tsx:1121-1122`) and writes the global setting via the settings save path (`Config.tsx:1533-1541`).

### 5.8 Slash-command deprecation

`/output-style` is registered (`commands/output-style/index.ts`) with `isHidden: true` and on invocation calls `onDone('/output-style has been deprecated. Use /config to change your output style, or set it in your settings file. Changes take effect on the next session.', { display: 'system' })` (`commands/output-style/output-style.tsx`). No arg parsing.

### 5.9 Spinner Tips

`tipScheduler.ts:32-46`:

```
if getSettings_DEPRECATED().spinnerTipsEnabled === false: return undefined
tips = await getRelevantTips(context)
if tips.length === 0: return undefined
return selectTipWithLongestTimeSinceShown(tips)
```

`selectTipWithLongestTimeSinceShown(tips)` sorts by `getSessionsSinceLastShown(tip.id)` desc and returns the head (`tipScheduler.ts:10-30`).

`getRelevantTips(context)` (`tipRegistry.ts:668-686`):

```
settings = getInitialSettings()
override = settings.spinnerTipsOverride
customTips = getCustomTips()  // map override.tips → cooldown 0, isRelevant true
if override?.excludeDefault && customTips.length > 0: return customTips
tips = [...externalTips, ...internalOnlyTips]    // internalOnlyTips empty if !ANT
isRelevant = await Promise.all(tips.map(_.isRelevant(context)))
filtered = tips
  .filter((_, i) => isRelevant[i])
  .filter(_ => getSessionsSinceLastShown(_.id) >= _.cooldownSessions)
return [...filtered, ...customTips]
```

`recordShownTip(tip)` (`tipScheduler.ts:48-58`): updates `tipsHistory` and emits `logEvent('tengu_tip_shown', { tipIdLength: tip.id, cooldownSessions: tip.cooldownSessions })`. The metric key is named `tipIdLength` but receives the tip id verbatim — a name discrepancy retained as-is.

`tipHistory.ts:12-17`: `getSessionsSinceLastShown(id)` returns `Infinity` if absent, otherwise `numStartups - lastShown`.

### 5.10 Magic Docs

`magicDocs.ts:1-254`:

```
initMagicDocs():
  if USER_TYPE !== 'ant': return
  registerFileReadListener((filePath, content) =>
    if detectMagicDocHeader(content): registerMagicDoc(filePath))
  registerPostSamplingHook(updateMagicDocs)

detectMagicDocHeader(content):
  match = content.match(/^#\s*MAGIC\s+DOC:\s*(.+)$/im)
  if !match: return null
  title = match[1].trim()
  afterHeader = content.slice(match.index + match[0].length)
  nextLineMatch = afterHeader.match(/^\s*\n(?:\s*\n)?(.+?)(?:\n|$)/)
  if italicsPattern (^[_*](.+?)[_*]\s*$) matches nextLine: return { title, instructions }
  return { title }

updateMagicDocs(context):
  if context.querySource !== 'repl_main_thread': return
  if hasToolCallsInLastAssistantTurn(context.messages): return
  if trackedMagicDocs.size === 0: return
  for each tracked doc:
    cloned readFileState; clonedReadFileState.delete(doc.path)
    read file via FileReadTool (ENOENT/EACCES → untrack and continue)
    re-run detectMagicDocHeader; if no longer a magic doc → untrack
    userPrompt = await buildMagicDocsUpdatePrompt(content, path, title, instructions?)
    canUseTool = (tool, input) →
       allow only FILE_EDIT_TOOL_NAME with input.file_path === doc.path
       else deny with reason `only FileEdit is allowed for ${doc.path}`
    runAgent(getMagicDocsAgent(), promptMessages=[userMessage(userPrompt)],
             toolUseContext, canUseTool, isAsync=true,
             forkContextMessages=context.messages,
             querySource='magic_docs',
             override={ systemPrompt, userContext, systemContext })
```

The agent definition (`magicDocs.ts:99-109`) declares `agentType: 'magic-docs'`, `whenToUse: 'Update Magic Docs'`, `tools: [FILE_EDIT_TOOL_NAME]`, `model: 'sonnet'`, `source: 'built-in'`, `baseDir: 'built-in'`, `getSystemPrompt: () => ''` (the actual prompt is fed via `override.systemPrompt`).

### 5.11 Prompt Suggestion (sampled)

`promptSuggestion.ts:30-50` exports `PromptVariant = 'user_intent' | 'stated_intent'` (currently hard-pinned to `'user_intent'` via `getPromptVariant`). `shouldEnablePromptSuggestion()` checks `process.env.CLAUDE_CODE_ENABLE_PROMPT_SUGGESTION` — falsy disables, truthy enables, with `tengu_prompt_suggestion_init` analytics. Detail and the speculation engine (`speculation.ts`, ~30KB) are deferred to a follow-up; see §12.

---

## 6. Verbatim Assets

### 6.1 Constants table

| Constant | Value | Citation |
|---|---|---|
| `DEFAULT_OUTPUT_STYLE_NAME` | `'default'` | `outputStyles.ts:39` |
| Picker fallback label | `'Default'` | `OutputStylePicker.tsx:11` |
| Picker fallback description | `'Claude completes coding tasks efficiently and provides concise responses'` | `OutputStylePicker.tsx:12` |
| Picker title | `'Preferred output style'` | `OutputStylePicker.tsx:101` |
| Picker subhead | `'This changes how Claude Code communicates with you'` | `OutputStylePicker.tsx:83` |
| Picker loading text | `'Loading output styles…'` | `OutputStylePicker.tsx:90` |
| Picker `visibleOptionCount` | `10` | `OutputStylePicker.tsx:90` |
| Output-style settings key | `outputStyle` (string, optional) | `utils/settings/types.ts:639-642` |
| System-prompt section id | `'output_style'` (dynamic-cacheable) | `prompts.ts:506` |
| Mid-turn reminder type | `'output_style'` attachment | `attachments.ts:556` |
| Plugin namespacing | `${pluginName}:${baseStyleName}` | `loadPluginOutputStyles.ts:55` |
| Plugin auto-discovery dir | `output-styles/` under plugin root | `pluginLoader.ts:1586-1589` |
| User dir | `~/.claude/output-styles/` | `loadOutputStylesDir.ts:14-22` (doc) |
| Project dir | `.claude/output-styles/` walked from cwd | `loadOutputStylesDir.ts:14-22` (doc) |
| Custom magic-docs prompt path | `<configHome>/magic-docs/prompt.md` | `MagicDocs/prompts.ts:67-68` |
| Tip cooldown unit | sessions (`numStartups`) | `tipHistory.ts:3-9` |
| Spinner-tips disable key | `spinnerTipsEnabled === false` | `tipScheduler.ts:36`; schema `settings/types.ts:664-668` |
| Spinner-tips override key | `spinnerTipsOverride: { excludeDefault?, tips: string[] }` | `settings/types.ts:677-686` |

### 6.2 Output-style metadata schema (verbatim Zod and TypeScript)

```ts
// src/utils/settings/types.ts:639-642
outputStyle: z
  .string()
  .optional()
  .describe('Controls the output style for assistant responses'),
```

```ts
// src/utils/plugins/schemas.ts:507-525
const PluginManifestOutputStylesSchema = lazySchema(() =>
  z.object({
    outputStyles: z.union([
      RelativePath().describe(
        'Path to additional output styles directory or file (in addition to those in the output-styles/ directory, if it exists), relative to the plugin root',
      ),
      z
        .array(
          RelativePath().describe(
            'Path to additional output styles directory or file (in addition to those in the output-styles/ directory, if it exists), relative to the plugin root',
          ),
        )
        .describe(
          'List of paths to additional output styles directories or files',
        ),
    ]),
  }),
)
```

```ts
// src/constants/outputStyles.ts:11-27
export type OutputStyleConfig = {
  name: string
  description: string
  prompt: string
  source: SettingSource | 'built-in' | 'plugin'
  keepCodingInstructions?: boolean
  /**
   * If true, this output style will be automatically applied when the plugin is enabled.
   * Only applicable to plugin output styles.
   * When multiple plugins have forced output styles, only one is chosen (logged via debug).
   */
  forceForPlugin?: boolean
}

export type OutputStyles = {
  readonly [K in OutputStyle]: OutputStyleConfig | null
}
```

Markdown-frontmatter fields recognised on user/project styles (`loadOutputStylesDir.ts:35-78`): `name` (string), `description` (string-coercible), `keep-coding-instructions` (boolean or `'true'`/`'false'` string; otherwise `undefined`), `force-for-plugin` (logged-and-ignored on non-plugin styles). For plugin styles (`loadPluginOutputStyles.ts:53-78`): `name`, `description`, `force-for-plugin` (boolean or string).

### 6.3 Built-in style fragments — verbatim

#### 6.3.1 `default`

`OUTPUT_STYLE_CONFIG['default'] = null` (`outputStyles.ts:42`). When the default style is selected the picker substitutes a label/description (`OutputStylePicker.tsx:11-12`):

> Default
>
> Claude completes coding tasks efficiently and provides concise responses

…and the prompt assembly emits no `# Output Style:` section (`prompts.ts:151-158`).

#### 6.3.2 `EXPLANATORY_FEATURE_PROMPT` (shared by Explanatory and Learning)

`outputStyles.ts:30-37`:

```
## Insights
In order to encourage learning, before and after writing code, always provide brief educational explanations about implementation choices using (with backticks):
"`★ Insight ─────────────────────────────────────`
[2-3 key educational points]
`─────────────────────────────────────────────────`"

These insights should be included in the conversation, not in the codebase. You should generally focus on interesting insights that are specific to the codebase or the code you just wrote, rather than general programming concepts.
```

The `★` glyph is `figures.star`; line 32 templates `${figures.star}` literally.

#### 6.3.3 `Explanatory`

`outputStyles.ts:43-55`:

- `name`: `'Explanatory'`
- `source`: `'built-in'`
- `description`: `'Claude explains its implementation choices and codebase patterns'`
- `keepCodingInstructions`: `true`
- `prompt`:

```
You are an interactive CLI tool that helps users with software engineering tasks. In addition to software engineering tasks, you should provide educational insights about the codebase along the way.

You should be clear and educational, providing helpful explanations while remaining focused on the task. Balance educational content with task completion. When providing insights, you may exceed typical length constraints, but remain focused and relevant.

# Explanatory Style Active
<EXPLANATORY_FEATURE_PROMPT verbatim>
```

#### 6.3.4 `Learning`

`outputStyles.ts:56-134`:

- `name`: `'Learning'`
- `source`: `'built-in'`
- `description`: `'Claude pauses and asks you to write small pieces of code for hands-on practice'`
- `keepCodingInstructions`: `true`
- `prompt`:

```
You are an interactive CLI tool that helps users with software engineering tasks. In addition to software engineering tasks, you should help users learn more about the codebase through hands-on practice and educational insights.

You should be collaborative and encouraging. Balance task completion with learning by requesting user input for meaningful design decisions while handling routine implementation yourself.   

# Learning Style Active
## Requesting Human Contributions
In order to encourage learning, ask the human to contribute 2-10 line code pieces when generating 20+ lines involving:
- Design decisions (error handling, data structures)
- Business logic with multiple valid approaches  
- Key algorithms or interface definitions

**TodoList Integration**: If using a TodoList for the overall task, include a specific todo item like "Request human input on [specific decision]" when planning to request human input. This ensures proper task tracking. Note: TodoList is not required for all tasks.

Example TodoList flow:
   ✓ "Set up component structure with placeholder for logic"
   ✓ "Request human collaboration on decision logic implementation"
   ✓ "Integrate contribution and complete feature"

### Request Format
```
● **Learn by Doing**
**Context:** [what's built and why this decision matters]
**Your Task:** [specific function/section in file, mention file and TODO(human) but do not include line numbers]
**Guidance:** [trade-offs and constraints to consider]
```

### Key Guidelines
- Frame contributions as valuable design decisions, not busy work
- You must first add a TODO(human) section into the codebase with your editing tools before making the Learn by Doing request      
- Make sure there is one and only one TODO(human) section in the code
- Don't take any action or output anything after the Learn by Doing request. Wait for human implementation before proceeding.

### Example Requests

**Whole Function Example:**
```
● **Learn by Doing**

**Context:** I've set up the hint feature UI with a button that triggers the hint system. The infrastructure is ready: when clicked, it calls selectHintCell() to determine which cell to hint, then highlights that cell with a yellow background and shows possible values. The hint system needs to decide which empty cell would be most helpful to reveal to the user.

**Your Task:** In sudoku.js, implement the selectHintCell(board) function. Look for TODO(human). This function should analyze the board and return {row, col} for the best cell to hint, or null if the puzzle is complete.

**Guidance:** Consider multiple strategies: prioritize cells with only one possible value (naked singles), or cells that appear in rows/columns/boxes with many filled cells. You could also consider a balanced approach that helps without making it too easy. The board parameter is a 9x9 array where 0 represents empty cells.
```

**Partial Function Example:**
```
● **Learn by Doing**

**Context:** I've built a file upload component that validates files before accepting them. The main validation logic is complete, but it needs specific handling for different file type categories in the switch statement.

**Your Task:** In upload.js, inside the validateFile() function's switch statement, implement the 'case "document":' branch. Look for TODO(human). This should validate document files (pdf, doc, docx).

**Guidance:** Consider checking file size limits (maybe 10MB for documents?), validating the file extension matches the MIME type, and returning {valid: boolean, error?: string}. The file object has properties: name, size, type.
```

**Debugging Example:**
```
● **Learn by Doing**

**Context:** The user reported that number inputs aren't working correctly in the calculator. I've identified the handleInput() function as the likely source, but need to understand what values are being processed.

**Your Task:** In calculator.js, inside the handleInput() function, add 2-3 console.log statements after the TODO(human) comment to help debug why number inputs fail.

**Guidance:** Consider logging: the raw input value, the parsed result, and any validation state. This will help us understand where the conversion breaks.
```

### After Contributions
Share one insight connecting their code to broader patterns or system effects. Avoid praise or repetition.

## Insights
<EXPLANATORY_FEATURE_PROMPT verbatim>
```

The `●` glyphs are `${figures.bullet}` runtime substitutions.

### 6.4 Section template assembling output styles

`prompts.ts:151-158`:

```ts
function getOutputStyleSection(
  outputStyleConfig: OutputStyleConfig | null,
): string | null {
  if (outputStyleConfig === null) return null

  return `# Output Style: ${outputStyleConfig.name}
${outputStyleConfig.prompt}`
}
```

`prompts.ts:175-184` — intro section (binds identity to style):

```ts
function getSimpleIntroSection(
  outputStyleConfig: OutputStyleConfig | null,
): string {
  return `
You are an interactive agent that helps users ${outputStyleConfig !== null ? 'according to your "Output Style" below, which describes how you should respond to user queries.' : 'with software engineering tasks.'} Use the instructions below and the tools available to you to assist the user.

${CYBER_RISK_INSTRUCTION}
IMPORTANT: You must NEVER generate or guess URLs for the user unless you are confident that the URLs are for helping the user with programming. You may use URLs provided by the user in their messages or local files.`
}
```

`prompts.ts:562-565` — coding-instructions suppression:

```ts
outputStyleConfig === null ||
outputStyleConfig.keepCodingInstructions === true
  ? getSimpleDoingTasksSection()
  : null,
```

### 6.5 Mid-turn reminder text

`messages.ts:3796-3810` — when an `output_style` attachment is materialised:

```
${outputStyle.name} output style is active. Remember to follow the specific guidelines for this style.
```

(wrapped via `wrapMessagesInSystemReminder` and emitted as a meta user message). Custom and plugin styles whose name is not in `OUTPUT_STYLE_CONFIG` are dropped here.

### 6.6 Deprecated slash command text

`commands/output-style/output-style.tsx`:

```
/output-style has been deprecated. Use /config to change your output style, or set it in your settings file. Changes take effect on the next session.
```

Display category `'system'` (`output-style.tsx:5`).

### 6.7 SDK / status surfaces

`messages/systemInit.ts:55,75` emits `output_style: <name>` on the `system/init` SDKMessage.

`StatusLine.tsx:44,81` emits `output_style: { name: <name> }` on the status-line hook payload.

`cli/print.ts:4430,4467-4468` emits `output_style: <name>` plus `available_output_styles: Object.keys(allStyles)` in `SDKControlInitializeResponse`.

### 6.8 Tip-history persistence shape

`tipHistory.ts:3-9` — the on-disk `tipsHistory` map writes only when changed:

```ts
export function recordTipShown(tipId: string): void {
  const numStartups = getGlobalConfig().numStartups
  saveGlobalConfig(c => {
    const history = c.tipsHistory ?? {}
    if (history[tipId] === numStartups) return c
    return { ...c, tipsHistory: { ...history, [tipId]: numStartups } }
  })
}
```

### 6.9 Magic Docs prompt template (default, verbatim)

`MagicDocs/prompts.ts:8-59`:

```
IMPORTANT: This message and these instructions are NOT part of the actual user conversation. Do NOT include any references to "documentation updates", "magic docs", or these update instructions in the document content.

Based on the user conversation above (EXCLUDING this documentation update instruction message), update the Magic Doc file to incorporate any NEW learnings, insights, or information that would be valuable to preserve.

The file {{docPath}} has already been read for you. Here are its current contents:
<current_doc_content>
{{docContents}}
</current_doc_content>

Document title: {{docTitle}}
{{customInstructions}}

Your ONLY task is to use the Edit tool to update the documentation file if there is substantial new information to add, then stop. You can make multiple edits (update multiple sections as needed) - make all Edit tool calls in parallel in a single message. If there's nothing substantial to add, simply respond with a brief explanation and do not call any tools.

CRITICAL RULES FOR EDITING:
- Preserve the Magic Doc header exactly as-is: # MAGIC DOC: {{docTitle}}
- If there's an italicized line immediately after the header, preserve it exactly as-is
- Keep the document CURRENT with the latest state of the codebase - this is NOT a changelog or history
- Update information IN-PLACE to reflect the current state - do NOT append historical notes or track changes over time
- Remove or replace outdated information rather than adding "Previously..." or "Updated to..." notes
- Clean up or DELETE sections that are no longer relevant or don't align with the document's purpose
- Fix obvious errors: typos, grammar mistakes, broken formatting, incorrect information, or confusing statements
- Keep the document well organized: use clear headings, logical section order, consistent formatting, and proper nesting

DOCUMENTATION PHILOSOPHY - READ CAREFULLY:
- BE TERSE. High signal only. No filler words or unnecessary elaboration.
- Documentation is for OVERVIEWS, ARCHITECTURE, and ENTRY POINTS - not detailed code walkthroughs
- Do NOT duplicate information that's already obvious from reading the source code
- Do NOT document every function, parameter, or line number reference
- Focus on: WHY things exist, HOW components connect, WHERE to start reading, WHAT patterns are used
- Skip: detailed implementation steps, exhaustive API docs, play-by-play narratives

What TO document:
- High-level architecture and system design
- Non-obvious patterns, conventions, or gotchas
- Key entry points and where to start reading code
- Important design decisions and their rationale
- Critical dependencies or integration points
- References to related files, docs, or code (like a wiki) - help readers navigate to relevant context

What NOT to document:
- Anything obvious from reading the code itself
- Exhaustive lists of files, functions, or parameters
- Step-by-step implementation details
- Low-level code mechanics
- Information already in CLAUDE.md or other project docs

Use the Edit tool with file_path: {{docPath}}

REMEMBER: Only update if there is substantial new information. The Magic Doc header (# MAGIC DOC: {{docTitle}}) must remain unchanged.
```

`MagicDocs/prompts.ts:107-115` — the `customInstructions` slot is non-empty only when the magic doc supplies an italicised next-line instruction:

```
DOCUMENT-SPECIFIC UPDATE INSTRUCTIONS:
The document author has provided specific instructions for how this file should be updated. Pay extra attention to these instructions and follow them carefully:

"<instructions>"

These instructions take priority over the general rules below. Make sure your updates align with these specific guidelines.
```

### 6.10 Critical regexes

| Purpose | Regex | Citation |
|---|---|---|
| Magic-doc header detect | `/^#\s*MAGIC\s+DOC:\s*(.+)$/im` | `magicDocs.ts:33` |
| Italicised instruction line | `/^[_*](.+?)[_*]\s*$/m` | `magicDocs.ts:35` |
| Next-content line after header | `/^\s*\n(?:\s*\n)?(.+?)(?:\n|$)/` | `magicDocs.ts:66` |
| Variable substitution | `/\{\{(\w+)\}\}/g` | `MagicDocs/prompts.ts:88` |

### 6.11 QuerySource literals carrying outputStyle context

`utils/promptCategory.ts:36-49`:

```
if (style === 'default')                  → 'repl_main_thread'
elif style ∈ OUTPUT_STYLE_CONFIG          → `repl_main_thread:outputStyle:${style}`
else                                       → 'repl_main_thread:outputStyle:custom'
```

Foreground 529-retry whitelist (`services/api/withRetry.ts:60-67`):

```
'repl_main_thread'
'repl_main_thread:outputStyle:custom'
'repl_main_thread:outputStyle:Explanatory'
'repl_main_thread:outputStyle:Learning'
'sdk', 'agent:custom', 'agent:default', 'agent:builtin', 'compact',
'hook_agent', 'hook_prompt', 'verification_agent', 'side_question'
```

`microCompact.ts:244-251` consumes this prefix via `startsWith('repl_main_thread')` to ensure non-default-style sessions are NOT silently excluded from microcompact.

---

## 7. Side Effects & I/O

- **Filesystem reads (output-styles)**: `.claude/output-styles/*.md` walked from `cwd` upward, plus `~/.claude/output-styles/*.md`, plus per-plugin `<plugin>/output-styles/` and any extra paths from `manifest.outputStyles`. All reads UTF-8 (`loadPluginOutputStyles.ts:46`); duplicates suppressed via `isDuplicatePath` and `walkPluginMarkdown` (`loadPluginOutputStyles.ts:42-44`).
- **Filesystem reads (magic docs)**: Optional custom prompt at `<configHome>/magic-docs/prompt.md` (`MagicDocs/prompts.ts:67-71`); silently falls back on read failure. Magic-doc files re-read on every update via `FileReadTool`.
- **Filesystem writes**: None directly from the output-style code; settings persistence is via `Settings/Config.tsx` save path and `globalConfig.tipsHistory` is mutated via `saveGlobalConfig`.
- **Process / network**: None directly. Magic Docs spins a sub-conversation through `runAgent`; that path issues Anthropic API calls and may invoke `FileEditTool`.
- **Environment variables**: `USER_TYPE` (ANT-only gating across tip registry and Magic Docs); `CLAUDE_CODE_ENABLE_PROMPT_SUGGESTION` (`promptSuggestion.ts:39-50`); GrowthBook flags (`tengu_tide_elm`, `tengu_tern_alloy`, `tengu_timber_lark`); `COLORTERM` (tip).
- **Trust boundaries**: Magic Docs forks a subagent with a deny-by-default `canUseTool` allowing only `FILE_EDIT_TOOL_NAME` writes to the doc's exact `file_path` (`magicDocs.ts:172-191`).

---

## 8. Feature Flags & Variants

- **No `feature(...)` flag** governs the output-style core. The subsystem is always built.
- **ANT (`USER_TYPE === 'ant'`)**: enables Magic Docs (`magicDocs.ts:243-252`); enables ANT-only tips (`tipRegistry.ts:635-653`); flips per-tip relevance for plan-mode/feedback/opusplan/shift-tab content as enumerated in §2.2.
- **GrowthBook variants**: `tengu_tide_elm`, `tengu_tern_alloy`, `tengu_timber_lark` each take values `'off' | 'copy_a' | 'copy_b'`; `'off'` excludes the tip, `'copy_a'`/`'copy_b'` swap copy.
- **Settings overrides**: `spinnerTipsEnabled === false` disables tips entirely; `spinnerTipsOverride.excludeDefault: true` with non-empty `tips` returns ONLY custom tips; otherwise custom tips are appended after filtered built-ins (`tipRegistry.ts:655-686`).

---

## 9. Error Handling & Edge Cases

- File-load errors at the per-file level call `logError` and contribute `null` to the styles array, which is filtered out (`loadOutputStylesDir.ts:79-84`); a top-level catch returns `[]` so a single bad file never breaks the registry.
- Plugin per-file failures call `logForDebugging('Failed to load output style from <path>: <error>', { level: 'error' })` and skip the file (`loadPluginOutputStyles.ts:79-84`). Per-plugin/per-extra-path failures are similarly logged and continue.
- `force-for-plugin` set on a non-plugin (user/project) markdown style logs `Output style "<name>" has force-for-plugin set, but this option only applies to plugin output styles. Ignoring.` and is dropped (`loadOutputStylesDir.ts:64-70`).
- Multiple plugins with `forceForPlugin: true`: first wins, all are listed in a debug-warn (`outputStyles.ts:194-199`).
- Selected style not present (e.g. settings reference a deleted style): `getOutputStyleConfig` returns `null` (`outputStyles.ts:210`), and assembly falls back to default-shaped prompt.
- Picker load failure: falls back to `OUTPUT_STYLE_CONFIG` only (`OutputStylePicker.tsx:53-57`).
- Mid-turn reminder lookup miss for a custom/plugin style: returns `[]` (no reminder), see §6.5. **Asymmetry**: `attachments.ts:1597-1611` queues an `output_style` attachment for **any** non-`'default'` settings value, but `messages.ts:3796-3810` only resolves names present in `OUTPUT_STYLE_CONFIG` (i.e. `Explanatory`, `Learning`). Custom user/project/plugin-namespaced styles therefore receive the attachment chip rendered in the UI but produce **no `<system-reminder>` text** in the conversation. Documented behaviour, not an unhandled error: the attachment array is empty rather than throwing. Reimplementers reproducing this must keep both halves consistent — drop the attachment at the queue stage, OR widen the lookup to `getAllOutputStyles()`. The shipped behaviour silently widens neither.
- Magic Docs file deleted/inaccessible mid-update: untracks (`magicDocs.ts:142-152`).
- Magic Docs header removed from a previously-tracked file: untracks (`magicDocs.ts:155-160`).
- `tipsHistory` write is a no-op when `numStartups` is unchanged (`tipHistory.ts:5-8`), avoiding gratuitous disk writes per tick.

---

## 10. Telemetry & Observability

- `logEvent('tengu_tip_shown', { tipIdLength, cooldownSessions })` — emitted from `recordShownTip` (`tipScheduler.ts:53-57`). Field name `tipIdLength` is misleading (carries the id literal); preserved as-is.
- `logEvent('tengu_prompt_suggestion_init', { enabled, source })` — `promptSuggestion.ts:42-50`.
- Debug logs (via `logForDebugging`):
  - `Output style "<name>" has force-for-plugin set, but this option only applies to plugin output styles. Ignoring.` (`loadOutputStylesDir.ts:66-69`).
  - `Plugin loading errors: <list>` (`loadPluginOutputStyles.ts:93-96`).
  - `Loaded N output styles from plugin <name> default directory` (`:113-117`).
  - `Loaded N output styles from plugin <name> custom path: <p>` (`:142-145`).
  - `Loaded output style from plugin <name> custom file: <p>` (`:155-158`).
  - `Failed to load output styles from plugin <name> default directory: <e>` (`:118-122`).
  - `Failed to load output styles from plugin <name> custom path <p>: <e>` (`:161-165`).
  - `Failed to load output style from <path>: <e>` (`:79-83`).
  - `Total plugin output styles loaded: N` (`:171`).
  - `Multiple plugins have forced output styles: <list>. Using: <chosen>` (`outputStyles.ts:195-198`).
  - `Using forced plugin output style: <name>` (`outputStyles.ts:200-202`).
- `output_style` field on `system/init` SDK message (`messages/systemInit.ts:75`), on `SDKControlInitializeResponse` (`cli/print.ts:4467-4468`), and on the status-line hook payload (`StatusLine.tsx:81`).
- QuerySource analytics carry the active style (`promptCategory.ts:38-48`); downstream retry/compact logic keys off the `repl_main_thread:outputStyle:` prefix (`api/withRetry.ts:60-67`, `microCompact.ts:244-251`).

---

## 11. Reimplementation Checklist

- [ ] Built-in registry shape: `default → null`, `Explanatory`, `Learning` only; preserve verbatim prompt fragments (§6.3). **Trailing-whitespace caveat**: the `Learning` prompt at `outputStyles.ts:64`, `:71`, and `:90` ends three lines with literal trailing spaces (collaborative-tone line, "Business logic with multiple valid approaches  ", and the FILE_EDIT instruction). These are intentional in the shipped fragment but are stripped by most editors / lint pipelines (e.g. `trailing-whitespace` pre-commit). Reimplementers must disable that lint for this file or the prompt will silently differ from the leak by three bytes per line.
- [ ] `OutputStyleConfig` field set including `keepCodingInstructions` and `forceForPlugin` (§6.2); type union `SettingSource | 'built-in' | 'plugin'`.
- [ ] Filesystem discovery from project `.claude/output-styles/*.md` walked upward and `~/.claude/output-styles/*.md`; merge with `loadMarkdownFilesForSubdir` semantics; sources tagged `policySettings`/`userSettings`/`projectSettings`.
- [ ] Plugin discovery from `output-styles/` auto-detect and from manifest `outputStyles` field (string or array); namespace as `${pluginName}:${baseStyleName}`; `forceForPlugin` parsing of boolean OR string `'true'`/`'false'`.
- [ ] Aggregation precedence (lowest → highest): built-in < plugin < user < project < managed.
- [ ] Selection: forced-plugin first-wins (with multi-warn), then `settings.outputStyle ?? 'default'`.
- [ ] System-prompt assembly: `# Output Style: <name>\n<prompt>` inserted under the `output_style` dynamic section id; intro string flips for non-null configs; doing-tasks section suppressed when `keepCodingInstructions` is falsy.
- [ ] Mid-turn `output_style` attachment emitted unless style === `'default'`; reminder text exact (§6.5); custom names produce no reminder.
- [ ] Picker dialog title/subhead/loading text/visibleOptionCount = 10; fallback Default label/description (§6.1).
- [ ] `/output-style` slash command: hidden, deprecated, prints exact stub message (§6.6).
- [ ] Tips: `getRelevantTips` filter chain (relevance ∧ cooldown), `selectTipWithLongestTimeSinceShown`, `tipsHistory: Record<string, number>` keyed by `numStartups`, no-op write when unchanged. ANT-only `internalOnlyTips` block. Custom tips honoured via `spinnerTipsOverride`.
- [ ] Magic Docs (ANT-only): header regex (§6.10), italicised instructions parsing, sub-agent definition (`agentType: 'magic-docs'`, sonnet, FileEdit-only), update prompt template verbatim (§6.9), `repl_main_thread`-only run-condition, idle-only run-condition (no tool calls in last assistant turn), exact deny-reason string.
- [ ] Cache-clear path: `clearOutputStyleCaches()` triple-clears (dir, plugin, markdown loader); `clearAllOutputStylesCache()` clears the aggregator; both are wired to plugin-reload (`utils/plugins/cacheUtils.ts:4`).

---

## 12. Open Questions / Unknowns

1. **Missing-leaked-source: `src/services/tips/types.ts`.** Imported by `tipScheduler.ts:8` and `tipRegistry.ts:57` (`Tip`, `TipContext`); the file is absent from the leaked tree. The validation-tips path (`utils/settings/validationTips.ts:11`) defines a *different* `TipContext`. Without the file, `Tip` shape is reconstructed only from call sites: `{ id: string; content: (ctx) => Promise<string>; cooldownSessions: number; isRelevant: (ctx?: TipContext) => Promise<boolean> }` and `TipContext = { theme; bashTools?: Set<string>; readFileState? }`. Recorded as a §2.5 missing-leaked-source ledger entry; downstream reimplementer must verify against the absent module.
2. **`PromptSuggestion/` (~47KB)**: only the entrypoint and gate were read; the speculation engine (`speculation.ts`, ~30KB) and prompt-suggestion lifecycle are deferred. Borderline assignment — could escalate to spec 42 if a deeper §6 expansion is needed; currently kept in 38 because its surface effect (overlaying assistant output / next-prompt suggestions) is symmetric with output-style framing. Open call: split off into 42 if that spec's owner wants the residual.
3. **Custom-style attachment reminder asymmetry**: `attachments.ts:1597-1611` injects an attachment for any non-default style, but `messages.ts:3796-3810` looks up `OUTPUT_STYLE_CONFIG` only — so custom and plugin styles get no mid-turn reminder. Documented behaviour, not a bug, but downstream reimplementers should be alert.
4. **`source` filtering in aggregation**: `getAllOutputStyles` only re-keys styles whose source ∈ `{policySettings, userSettings, projectSettings}` (`outputStyles.ts:148-159`). Styles whose `source` is e.g. `localSettings`/`flagSettings` reach `customStyles` but are not aggregated. Whether this is intentional is unverified; preserved verbatim.
5. **`magicDocs.ts` interaction with Tool registry**: the Magic Docs sub-agent is dispatched with `availableTools: clonedToolUseContext.options.tools` (`:208`), but `canUseTool` constrains usage to FileEdit. The wider tool list is exposed for system-prompt cache stability; reimplementers must follow the same contract.
6. **Not owned here (→ spec 05): `getSessionStartDate()`.** Imported at `prompts.ts:8` and used only by the *static* intro line at `prompts.ts:452` (`getSimpleSystemSection()`: `"You are Claude Code, Anthropic's official CLI for Claude.\n\nCWD: ${getCwd()}\nDate: ${getSessionStartDate()}"`). It does **NOT** participate in the output-style fragment, the `getOutputStyleSection` template, or any dynamic section keyed by output style. Spec 38 mentions it only to disclaim ownership; the system-prompt-assembly spec (05) is canonical for when and how the session-start date is rendered.
7. **Boundary call (per task instructions)**: tips/MagicDocs/PromptSuggestion are owned **here** in 38. Justification: they are response-shape concerns that overlay how the assistant communicates rather than fitting any single service spec; co-locating with output styles preserves the pattern that "presentation" lives in this spec. If a future Phase-9 audit prefers them in 42, all three blocks are self-contained and migrate cleanly.
