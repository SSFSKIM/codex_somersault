# 09 — Permission System

Status: done · Owner: sub-C1 · Last updated: 2026-05-08

Cross-cutting spec. Cited by every per-tool spec (10..19) and by command spec (21).
Adjacent specs: 02 (settings precedence), 03 (denial tracking pattern), 04 (call site
in turn pipeline), 08 (Tool.checkPermissions interface), 26 (denial telemetry),
27 (policy/MDM source), 37 (UI dialog rendering).

---

## §0 Source-coverage inventory

IN-scope files:

| File / dir | Status | Notes |
|---|---|---|
| `src/types/permissions.ts` | fully-read (442 lines) | Single source of truth for types |
| `src/utils/permissions/PermissionMode.ts` | fully-read (142 lines) | Mode metadata + ext/int conversion |
| `src/utils/permissions/PermissionResult.ts` | fully-read (36 lines) | Re-exports + behavior description helper |
| `src/utils/permissions/PermissionRule.ts` | fully-read (41 lines) | Re-exports + Zod `permissionBehaviorSchema`/`permissionRuleValueSchema` |
| `src/utils/permissions/PermissionUpdate.ts` | fully-read (390 lines) | `applyPermissionUpdate*`, `persistPermissionUpdate*`, `createReadRuleSuggestion` |
| `src/utils/permissions/PermissionUpdateSchema.ts` | fully-read (78 lines) | `permissionUpdateSchema`, `permissionUpdateDestinationSchema` |
| `src/utils/permissions/PermissionPromptToolResultSchema.ts` | fully-read (128 lines) | MCP permission-prompt tool I/O |
| `src/utils/permissions/permissions.ts` | fully-read (1486 lines) | `hasPermissionsToUseTool`, decision tree, denial tracking, fast paths |
| `src/utils/permissions/permissionRuleParser.ts` | fully-read (198 lines) | Rule string ↔ value, escape/unescape, legacy alias map |
| `src/utils/permissions/permissionsLoader.ts` | fully-read (296 lines) | Disk load/persist; `allowManagedPermissionRulesOnly` |
| `src/utils/permissions/getNextPermissionMode.ts` | fully-read (101 lines) | Shift+Tab cycle |
| `src/utils/permissions/dangerousPatterns.ts` | fully-read (80 lines) | `DANGEROUS_BASH_PATTERNS`, `CROSS_PLATFORM_CODE_EXEC` |
| `src/utils/permissions/denialTracking.ts` | fully-read (45 lines) | `DenialTrackingState`, `DENIAL_LIMITS` |
| `src/utils/permissions/yoloClassifier.ts` | fully-read (1496 lines) | Auto-mode classifier (XML 2-stage + tool_use) |
| `src/utils/permissions/classifierDecision.ts` | fully-read (98 lines) | `isAutoModeAllowlistedTool` |
| `src/utils/permissions/classifierShared.ts` | fully-read (39 lines) | Tool-use response parsing helpers |
| `src/utils/permissions/autoModeState.ts` | fully-read (39 lines) | Module-scope auto-mode flags |
| `src/utils/permissions/bypassPermissionsKillswitch.ts` | fully-read (155 lines) | Statsig-gated kickout for bypass + auto |
| `src/utils/permissions/bashClassifier.ts` | fully-read (61 lines) | EXTERNAL stub (ANT-only impl elsewhere) |
| `src/utils/permissions/shadowedRuleDetection.ts` | fully-read (234 lines) | Unreachable-rule detector |
| `src/utils/permissions/shellRuleMatching.ts` | fully-read (228 lines) | Wildcard / prefix / exact matcher |
| `src/utils/permissions/permissionExplainer.ts` | fully-read (250 lines) | Haiku risk explainer |
| `src/utils/permissions/permissionSetup.ts` | sampled (1532 lines) | Mode transitions, dangerous-rule strip, gate access (deep details deferred to §12) |
| `src/utils/permissions/filesystem.ts` | sampled (1777 lines) | Scratchpad + tempdir helpers + path-safety predicates (full path-safety logic owned by §11) |
| `src/utils/permissions/pathValidation.ts` | grep-inspected (485 lines) | Sandbox path normalization (called from per-tool checkPermissions; details in §11) |
| `src/hooks/toolPermission/PermissionContext.ts` | fully-read (389 lines) | Per-decision context object |
| `src/hooks/toolPermission/permissionLogging.ts` | fully-read (238 lines) | Telemetry events |
| `src/hooks/toolPermission/handlers/coordinatorHandler.ts` | fully-read (65 lines) | Coordinator agent path |
| `src/hooks/toolPermission/handlers/interactiveHandler.ts` | fully-read (536 lines) | REPL/Bridge/Channel race |
| `src/hooks/toolPermission/handlers/swarmWorkerHandler.ts` | fully-read (159 lines) | Mailbox forwarding |
| `src/hooks/useCanUseTool.tsx` | fully-read (≈360 lines, compiled) | Top-level entry |
| `src/components/permissions/*` | grep-inspected | Component routing (full UI deferred to §37) |

Out-of-scope (refer to listed spec):

- Settings-source loading machinery → §02
- Per-tool `checkPermissions` overrides → §10..§19
- Policy/MDM remote settings cascade → §27
- Analytics pipeline → §26
- UI rendering details → §37
- Path-safety predicate bodies (`checkPathSafetyForAutoEdit`, `getPathsForPermissionCheck`) → §11

---

## §1 Purpose & scope

The permission system mediates every tool invocation. Given a `Tool`, parsed
input, and the live `ToolPermissionContext`, it returns one of:

- `allow` — tool runs with optionally rewritten input
- `deny` — tool result is the deny `message`; no execution
- `ask` — surfaces an interactive dialog or, in headless agents, falls back to
  hooks then auto-deny

It is the only place where:

1. User-authored rules (deny/allow/ask) are matched against tool + input
2. Permission modes (`default`/`acceptEdits`/`bypassPermissions`/`dontAsk`/`plan`/`auto`)
   gate the result
3. PreToolUse hooks (`PermissionRequest` event) can override the verdict
4. The auto-mode YOLO classifier (ANT/`TRANSCRIPT_CLASSIFIER`) runs as a
   side-channel LLM call to classify the action
5. Bash command classifiers (`BASH_CLASSIFIER`) auto-approve via prompt-based
   rules
6. Denials are tracked and emitted to the SDK `result` envelope via
   `permission_denials`

Every tool's `checkPermissions` returns a `PermissionResult` that the central
pipeline (§6.2 below) layers rules, modes, and hooks around.

---

## §2 Public API surface (from this subsystem)

Consumed by `QueryEngine` (§03) and `query.ts` (§04):

- `CanUseToolFn` — type alias re-exported from
  `src/hooks/useCanUseTool.tsx:27`. Promise-returning function the turn
  pipeline awaits before executing each tool_use block.
- `useCanUseTool(setToolUseConfirmQueue, setToolPermissionContext)` —
  React hook; returns a memoized `CanUseToolFn`.
- `hasPermissionsToUseTool` — `src/utils/permissions/permissions.ts:473`. The
  pure (no-React) decision function. Calls `hasPermissionsToUseToolInner` then
  layers mode-based transformations (`dontAsk`, `auto`, headless auto-deny).

Consumed by `/permissions` and `/permission-mode` commands (§21):

- `applyPermissionUpdate(s)`, `persistPermissionUpdate(s)`, `deletePermissionRule`
- `getAllowRules`, `getDenyRules`, `getAskRules`, `toolAlwaysAllowedRule`,
  `getDenyRuleForTool`, `getAskRuleForTool`, `filterDeniedAgents`
- `applyPermissionRulesToPermissionContext` (additive, init-time)
- `syncPermissionRulesFromDisk` (replacement, settings-watcher hot reload)
- `cyclePermissionMode`, `getNextPermissionMode`, `transitionPermissionMode`

Consumed by SDK / hooks layer:

- `permissionUpdateSchema`, `permissionRuleValueSchema`, `permissionBehaviorSchema`
- `permissionPromptToolResultToPermissionDecision` (MCP path)
- `PermissionRequest` hook event (in `src/types/hooks.ts`; see §08, §22 for
  hook events catalog)

---

## §3 Glossary

- **Permission mode** — A per-session enum gating the decision tree's
  defaults. Six values defined at type level. The user-addressable validation
  set (Zod-checked) excludes `bubble` and conditionally `auto` (see §4.1).
  However `bubble` IS used at runtime by forked subagents (see §4.1
  Discrepancy block), so it appears in `ToolPermissionContext.mode` even
  though it cannot enter via settings.json or CLI.
- **PermissionRule** — Triple of `{source, ruleBehavior, ruleValue}` where
  `ruleValue = {toolName, ruleContent?}`.
- **PermissionRuleSource** — Where a rule originated: one of
  `userSettings | projectSettings | localSettings | flagSettings |
  policySettings | cliArg | command | session`.
- **PermissionUpdateDestination** — Where a rule may be persisted: subset of
  source — `userSettings | projectSettings | localSettings | session | cliArg`.
  `policySettings`, `flagSettings`, `command` are read-only.
- **EditableSettingSource** — `localSettings | userSettings | projectSettings`
  (only these three persist via `addPermissionRulesToSettings`).
- **YOLO classifier** — Auto-mode side-call to an LLM that returns
  `{shouldBlock, reason}`. ANT-only behind `feature('TRANSCRIPT_CLASSIFIER')`.
- **Bash classifier** — Prompt-based per-command classifier exposed via
  `Bash(prompt: <description>)` rules. ANT-only behind
  `feature('BASH_CLASSIFIER')`.
- **DenialTrackingState** — `{consecutiveDenials, totalDenials}` (uint counts)
  with limits `maxConsecutive=3`, `maxTotal=20`.
- **Pending classifier check** — A `PendingClassifierCheck =
  {command, cwd, descriptions[]}` payload that BashTool's `checkPermissions`
  attaches to an `ask` result; the interactive handler races the classifier
  against user input.

---

## §4 Data shapes & schemas (verbatim where load-bearing)

### 4.1 Permission modes

`src/types/permissions.ts:16-38`:

```ts
export const EXTERNAL_PERMISSION_MODES = [
  'acceptEdits',
  'bypassPermissions',
  'default',
  'dontAsk',
  'plan',
] as const

export type ExternalPermissionMode = (typeof EXTERNAL_PERMISSION_MODES)[number]

// Exhaustive mode union for typechecking. The user-addressable runtime set
// is INTERNAL_PERMISSION_MODES below.
export type InternalPermissionMode = ExternalPermissionMode | 'auto' | 'bubble'
export type PermissionMode = InternalPermissionMode

// Runtime validation set: modes that are user-addressable (settings.json
// defaultMode, --permission-mode CLI flag, conversation recovery).
export const INTERNAL_PERMISSION_MODES = [
  ...EXTERNAL_PERMISSION_MODES,
  ...(feature('TRANSCRIPT_CLASSIFIER') ? (['auto'] as const) : ([] as const)),
] as const satisfies readonly PermissionMode[]

export const PERMISSION_MODES = INTERNAL_PERMISSION_MODES
```

**Discrepancy** (load-bearing, REVISED Phase 9.6): the type-level union includes
`bubble`, but `INTERNAL_PERMISSION_MODES` (the user-addressable validation set
used by Zod via `permissionModeSchema = lazySchema(() => z.enum(PERMISSION_MODES))`
at `utils/permissions/PermissionMode.ts:21`) does NOT contain it. Therefore:

- A user CANNOT set `defaultMode: "bubble"` in settings.json (Zod rejects).
- A CLI flag `--permission-mode bubble` cannot succeed (rejected).
- `isExternalPermissionMode` (`utils/permissions/PermissionMode.ts:97-105`)
  excludes `bubble` explicitly when `USER_TYPE === 'ant'`.

**However, `bubble` IS used at runtime** for **forked subagents** (verified Phase 9.5
adversarial review — Phase 9.4 incorrectly classified this as type-only):

- `src/tools/AgentTool/forkSubagent.ts:67` — `FORK_AGENT` declares
  `permissionMode: 'bubble'`. The `'bubble'` literal **does** appear at runtime.
- `src/tools/AgentTool/runAgent.ts:443` — runtime branch
  `agentPermissionMode === 'bubble'` controls `shouldAvoidPrompts`.
- `src/tools/AgentTool/runAgent.ts:430-433` — writes `'bubble'` into
  `ToolPermissionContext.mode` for the forked subagent's context.

**Semantic role**: `bubble` mode causes a forked subagent to **bubble its
permission prompts up to the parent session** rather than prompting locally
(or auto-rejecting). `forkSubagent.ts:50` comment confirms: "permissionMode:
'bubble' surfaces permission prompts to the [parent]." This is the only
runtime use site found in the leak.

**Practical implications**:
- The runtime `ToolPermissionContext.mode` field can carry `'bubble'`.
- Telemetry / logging that switches on `mode` must handle `'bubble'`
  explicitly; falling back to `'default'` config silently is a bug
  (cross-spec impact: 26 — `PERMISSION_MODE_CONFIG.bubble` is undefined).
- Re-implementers must include `'bubble'` in the runtime mode space.

See §12 (resolved): the previous "type-only placeholder" Open Question is
discharged by the source verification above.
- `auto` is conditionally included by feature flag `TRANSCRIPT_CLASSIFIER`.
  External (non-ANT) builds can define `defaultMode: 'auto'` in settings only
  when the bundler keeps the flag on (gates on `TRANSCRIPT_CLASSIFIER` are
  external too, but mode gating to ANT happens in `getNextPermissionMode`).

Mode metadata table — see `PermissionMode.ts:42-91`:

| mode | title | shortTitle | symbol | external mapping |
|---|---|---|---|---|
| `default` | "Default" | "Default" | `''` | `default` |
| `plan` | "Plan Mode" | "Plan" | `PAUSE_ICON` | `plan` |
| `acceptEdits` | "Accept edits" | "Accept" | `⏵⏵` | `acceptEdits` |
| `bypassPermissions` | "Bypass Permissions" | "Bypass" | `⏵⏵` | `bypassPermissions` |
| `dontAsk` | "Don't Ask" | "DontAsk" | `⏵⏵` | `dontAsk` |
| `auto` (T_C) | "Auto mode" | "Auto" | `⏵⏵` | `default` |

`auto` maps to external `default` so non-ANT clients receiving an
`ExternalPermissionMode` over the SDK can serialize it.

### 4.2 PermissionBehavior + Rule types

`src/types/permissions.ts:44`:

```ts
export type PermissionBehavior = 'allow' | 'deny' | 'ask'
```

`src/types/permissions.ts:54-79`:

```ts
export type PermissionRuleSource =
  | 'userSettings'
  | 'projectSettings'
  | 'localSettings'
  | 'flagSettings'
  | 'policySettings'
  | 'cliArg'
  | 'command'
  | 'session'

export type PermissionRuleValue = {
  toolName: string
  ruleContent?: string
}

export type PermissionRule = {
  source: PermissionRuleSource
  ruleBehavior: PermissionBehavior
  ruleValue: PermissionRuleValue
}
```

Zod schemas — `utils/permissions/PermissionRule.ts:25-40`:

```ts
export const permissionBehaviorSchema = lazySchema(() =>
  z.enum(['allow', 'deny', 'ask']),
)

export const permissionRuleValueSchema = lazySchema(() =>
  z.object({
    toolName: z.string(),
    ruleContent: z.string().optional(),
  }),
)
```

### 4.3 PermissionUpdate (used by hooks, MCP perm-prompt tool, /permissions)

`src/types/permissions.ts:88-131`:

```ts
export type PermissionUpdateDestination =
  | 'userSettings'
  | 'projectSettings'
  | 'localSettings'
  | 'session'
  | 'cliArg'

export type PermissionUpdate =
  | { type: 'addRules';     destination; rules: PermissionRuleValue[]; behavior: PermissionBehavior }
  | { type: 'replaceRules'; destination; rules: PermissionRuleValue[]; behavior: PermissionBehavior }
  | { type: 'removeRules';  destination; rules: PermissionRuleValue[]; behavior: PermissionBehavior }
  | { type: 'setMode';      destination; mode: ExternalPermissionMode }
  | { type: 'addDirectories';    destination; directories: string[] }
  | { type: 'removeDirectories'; destination; directories: string[] }
```

Zod discriminated union at
`utils/permissions/PermissionUpdateSchema.ts:42-78`. Note: `setMode`'s `mode`
field is `externalPermissionModeSchema` — a permission update CANNOT set
`auto` or `bubble` modes; only the five external modes.

### 4.4 PermissionResult / PermissionDecision

`src/types/permissions.ts:174-266`:

```ts
export type PermissionAllowDecision<Input> = {
  behavior: 'allow'
  updatedInput?: Input
  userModified?: boolean
  decisionReason?: PermissionDecisionReason
  toolUseID?: string
  acceptFeedback?: string
  contentBlocks?: ContentBlockParam[]
}

export type PendingClassifierCheck = {
  command: string
  cwd: string
  descriptions: string[]
}

export type PermissionAskDecision<Input> = {
  behavior: 'ask'
  message: string
  updatedInput?: Input
  decisionReason?: PermissionDecisionReason
  suggestions?: PermissionUpdate[]
  blockedPath?: string
  metadata?: PermissionMetadata
  isBashSecurityCheckForMisparsing?: boolean
  pendingClassifierCheck?: PendingClassifierCheck
  contentBlocks?: ContentBlockParam[]
}

export type PermissionDenyDecision = {
  behavior: 'deny'
  message: string
  decisionReason: PermissionDecisionReason
  toolUseID?: string
}

export type PermissionDecision<Input> =
  | PermissionAllowDecision<Input>
  | PermissionAskDecision<Input>
  | PermissionDenyDecision

export type PermissionResult<Input> =
  | PermissionDecision<Input>
  | { behavior: 'passthrough'; message: string;
      decisionReason?: ...; suggestions?: ...; blockedPath?: ...;
      pendingClassifierCheck?: PendingClassifierCheck }
```

A tool's `checkPermissions` may return any of the four behaviors. The
pipeline converts `passthrough` to `ask` before exit (step 3 in §6.2).

### 4.5 PermissionDecisionReason

Discriminated union at `src/types/permissions.ts:271-324`. Tag values:

- `'rule'` — matched a `PermissionRule`
- `'mode'` — matched the active `PermissionMode` (e.g. `bypassPermissions`)
- `'subcommandResults'` — Bash compound-command rollup
  (`Map<string, PermissionResult>`)
- `'permissionPromptTool'` — MCP permission-prompt tool result
- `'hook'` — `PermissionRequest` hook decision (with `hookName`,
  `hookSource`, `reason`)
- `'asyncAgent'` — auto-deny in headless mode
- `'sandboxOverride'` — sub-tags `'excludedCommand' | 'dangerouslyDisableSandbox'`
- `'classifier'` — auto-mode/Bash classifier verdict (with
  `classifier: string`, `reason`)
- `'workingDir'` — additionalDirectory check
- `'safetyCheck'` — sensitive paths (`.git/`, `.claude/`, `.vscode/`, shell
  configs); has `classifierApprovable: boolean` flag (true when classifier
  may approve, e.g. `.claude/`; false for Windows path-bypass / cross-machine
  bridge messages — verbatim source comment in `:319`).
- `'other'` — generic free-form reason

### 4.6 ToolPermissionContext (DeepImmutable)

> **Canonical owner = `src/Tool.ts:123-138`** (the `DeepImmutable<{...}>` form re-exported across the codebase, including the `isAutoModeAvailable?: boolean` field). The shape below from `src/types/permissions.ts:427-441` is a **types-only mirror** — it lives in the no-runtime-deps file used to break import cycles, and intentionally omits `isAutoModeAvailable`. Spec 08 §4.2 quotes the canonical Tool.ts form; this section quotes the cycle-breaker mirror. See spec 08 for the authoritative type.

`src/types/permissions.ts:427-441` (mirror, no runtime deps):

```ts
export type ToolPermissionContext = {
  readonly mode: PermissionMode
  readonly additionalWorkingDirectories: ReadonlyMap<string, AdditionalWorkingDirectory>
  readonly alwaysAllowRules: ToolPermissionRulesBySource
  readonly alwaysDenyRules: ToolPermissionRulesBySource
  readonly alwaysAskRules: ToolPermissionRulesBySource
  readonly isBypassPermissionsModeAvailable: boolean
  readonly strippedDangerousRules?: ToolPermissionRulesBySource
  readonly shouldAvoidPermissionPrompts?: boolean
  readonly awaitAutomatedChecksBeforeDialog?: boolean
  readonly prePlanMode?: PermissionMode
}

export type ToolPermissionRulesBySource = {
  [T in PermissionRuleSource]?: string[]
}
```

`isAutoModeAvailable` IS declared on the canonical Tool.ts type (line 130) and is populated at `permissionSetup.ts:987` (`{ isAutoModeAvailable: isAutoModeGateEnabled() }`) when ANT/`TRANSCRIPT_CLASSIFIER` is enabled. The mirror in `types/permissions.ts` omits the field because that file is the no-runtime-deps cycle breaker; consumers that need the field import from `Tool.ts`.

`shouldAvoidPermissionPrompts` is the headless-agent flag: true → permission
prompts are unavailable, so `ask` results auto-deny after running
`PermissionRequest` hooks.

`awaitAutomatedChecksBeforeDialog` is the coordinator-agent flag: true →
hooks + bash classifier must resolve sequentially before the interactive
dialog renders.

`prePlanMode` records the mode the user was in before entering plan mode so
exiting plan can restore.

`strippedDangerousRules` stashes allow rules that were removed when entering
auto mode (`stripDangerousPermissionsForAutoMode` in `permissionSetup.ts`),
so leaving auto mode can restore them
(`restoreDangerousPermissions` at `permissionSetup.ts:560-580`).

### 4.7 CanUseToolFn entry signature

`src/hooks/useCanUseTool.tsx:27`:

```ts
export type CanUseToolFn<Input extends Record<string, unknown> = Record<string, unknown>> =
  (tool: ToolType,
   input: Input,
   toolUseContext: ToolUseContext,
   assistantMessage: AssistantMessage,
   toolUseID: string,
   forceDecision?: PermissionDecision<Input>)
  => Promise<PermissionDecision<Input>>
```

`forceDecision` is a pre-computed decision (used by SDK control message
`set_permission_mode_decision` and certain replay paths) that skips
`hasPermissionsToUseTool` entirely.

---

## §5 Algorithms (pseudocode for executable claims)

### 5.1 Rule-string ↔ value parser

`utils/permissions/permissionRuleParser.ts:93-152`. Format: `ToolName` or
`ToolName(content)`. Content may contain escaped parentheses (`\(`, `\)`)
and escaped backslashes (`\\`). Empty content (`Bash()`) and standalone
wildcard (`Bash(*)`) collapse to tool-wide rule. Legacy aliases applied via
`LEGACY_TOOL_NAME_ALIASES` at parser/lines 21-29 (`Task→Agent`,
`KillShell→TaskStop`, `AgentOutputTool/BashOutputTool→TaskOutput`,
`Brief→KAIROS BriefTool` when feature-gated):

```
function permissionRuleValueFromString(s):
  i = findFirstUnescapedChar(s, '(')
  if i == -1: return { toolName: normalizeLegacyToolName(s) }
  j = findLastUnescapedChar(s, ')')
  if j == -1 or j <= i or j != s.length-1:
    return { toolName: normalizeLegacyToolName(s) }
  toolName = s[:i]; raw = s[i+1:j]
  if !toolName: return { toolName: normalizeLegacyToolName(s) }
  if raw == '' or raw == '*':
    return { toolName: normalizeLegacyToolName(toolName) }
  return { toolName: normalizeLegacyToolName(toolName),
           ruleContent: unescapeRuleContent(raw) }
```

Escape order is load-bearing (parser/lines 55-78): on encode, escape
backslashes BEFORE parens (`\` → `\\`, `(` → `\(`, `)` → `\)`); on decode,
unescape parens BEFORE backslashes.

### 5.2 Wildcard / prefix / exact matcher

`utils/permissions/shellRuleMatching.ts:159-184`:

```
parsePermissionRule(s):
  if s matches /^(.+):\*$/: return { type: 'prefix', prefix: $1 }
  if hasUnescapedWildcard(s): return { type: 'wildcard', pattern: s }
  return { type: 'exact', command: s }
```

`matchWildcardPattern(pattern, command, caseInsensitive=false)` —
`shellRuleMatching.ts:90-154`:

1. Replace `\*` → null-byte sentinel `\x00ESCAPED_STAR\x00`,
   `\\` → `\x00ESCAPED_BACKSLASH\x00` (sentinels declared at module top so
   regex objects compile once).
2. Regex-escape special chars (`.+?^${}()|[]\\'"`) — keep `*` literal.
3. Convert remaining `*` → `.*`.
4. Convert sentinels back: `\*` → `\\*`, backslash → `\\\\`.
5. **Single-trailing-wildcard space rule**: if pattern ends with ` .*` AND
   the original processed string has exactly one unescaped `*`, replace
   trailing ` .*` with `( .*)?`. Aligns `git *` semantics with `git:*` (so
   bare `git` matches too). Multi-wildcard patterns excluded to avoid
   `* run *` matching bare `npm run`.
6. Compile `^${regex}$` with flags `'s'` (dotAll — wildcards span newlines)
   plus `'i'` if `caseInsensitive`.

### 5.3 Tool-vs-rule matching

`utils/permissions/permissions.ts:238-269` (`toolMatchesRule`):

- Rule with `ruleContent` set never matches a tool-wide rule check (only
  `checkPermissions` inspects content).
- The "name for rule match" is `getToolNameForPermissionCheck(tool)` —
  honors `CLAUDE_AGENT_SDK_MCP_NO_PREFIX` so MCP rules don't shadow builtins.
- Direct match: `rule.toolName === effectiveName`.
- MCP server-level: `mcp__server1` matches tool `mcp__server1__tool1`; also
  wildcard `mcp__server1__*` covers all tools from server1. Decoded via
  `mcpInfoFromString` (services/mcp/mcpStringUtils).

### 5.4 PermissionUpdate application

`utils/permissions/PermissionUpdate.ts:55-188`. `applyPermissionUpdate` is a
pure function returning a new `ToolPermissionContext` for each variant:

- `addRules` / `replaceRules` / `removeRules`: dispatch to one of
  `alwaysAllowRules` | `alwaysDenyRules` | `alwaysAskRules` keyed by
  `behavior`. Rules stored as serialized strings (so identity equality and
  Set lookups work).
- `addDirectories`: clones `additionalWorkingDirectories` Map and inserts
  `{path, source: destination}`.
- `removeDirectories` / `removeRules`: filter out matching entries
  (`removeRules` uses Set-of-strings for O(1) membership).
- `setMode`: spreads `{mode}` on the context.

`persistPermissionUpdate` (PermissionUpdate.ts:222-342) writes to the
appropriate settings source ONLY when `supportsPersistence(destination)` is
true (i.e., `destination ∈ {localSettings, userSettings, projectSettings}`).
For `removeRules`, normalizes existing entries via
`permissionRuleValueToString(permissionRuleValueFromString(raw))` so legacy
aliases match canonical names. For `setMode`, writes
`permissions.defaultMode = update.mode`.

`createReadRuleSuggestion(dirPath, destination='session')` —
`PermissionUpdate.ts:361-389`. Returns `addRules` for `Read(<pattern>/**)`
with leading `/` for absolute paths. Skips `/` (root too broad).

### 5.5 Denial tracking

`utils/permissions/denialTracking.ts:7-45`:

```
DenialTrackingState = { consecutiveDenials: int, totalDenials: int }
DENIAL_LIMITS = { maxConsecutive: 3, maxTotal: 20 }

createDenialTrackingState() = {0, 0}

recordDenial(s) = { consecutiveDenials: s.c+1, totalDenials: s.t+1 }

recordSuccess(s) = if s.c==0 then s else { ...s, consecutiveDenials: 0 }
                   // identity preserved on no-op so AppState.setState
                   // Object.is short-circuit skips the listener loop

shouldFallbackToPrompting(s) =
    s.consecutiveDenials >= 3 || s.totalDenials >= 20
```

Persisted via `persistDenialState(context, newState)` at
`permissions.ts:963-978`. For async subagents (`context.localDenialTracking`
set), mutates that local object in-place via `Object.assign` (their
`setAppState` is a no-op). Otherwise writes to `appState.denialTracking`
with an Object.is short-circuit so unchanged states don't fan out to
listeners.

### 5.6 Auto-mode classifier — high-level orchestration

`utils/permissions/yoloClassifier.ts:1012-1306` (`classifyYoloAction`).
Returns `YoloClassifierResult` (see §4.4). Steps:

1. `actionCompact = toCompact(action, lookup)`. If `''`
   (per-tool `toAutoClassifierInput` returned ''), returns
   `{shouldBlock: false, reason: 'Tool declares no classifier-relevant input'}`.
2. Build system prompt via `buildYoloSystemPrompt(context)` (see §6.4).
3. Build transcript via `buildTranscriptEntries(messages)`. Includes:
   - User text messages (and queued_command attachments extracted).
   - Assistant tool_use blocks (assistant text excluded —
     `yoloClassifier.ts:344-352` comment: *"Only include tool_use blocks —
     assistant text is model-authored and could be crafted to influence the
     classifier's decision"*).
4. Build CLAUDE.md prefix message via `buildClaudeMdMessage()` —
   wraps `<user_claude_md>` with `cache_control` block. Read from
   `bootstrap/state.ts` cache to avoid `claudemd → permissions → yoloClassifier`
   import cycle (yoloClassifier.ts:451-458 comment).
5. Compact each transcript block via `toCompactBlock` — JSONL or text-prefix
   based on `isJsonlTranscriptEnabled()` (env `CLAUDE_CODE_JSONL_TRANSCRIPT`
   ANT-only; or GrowthBook `tengu_auto_mode_config.jsonlTranscript`).
6. Pin `cache_control` on the action block (1h TTL via
   `getCacheControl({querySource:'auto_mode'})`).
7. Dispatch:
   - If `isTwoStageClassifierEnabled()` → `classifyYoloActionXml(...)` (XML
     stage1+stage2).
   - Else → tool_use classifier with `YOLO_CLASSIFIER_TOOL_SCHEMA`,
     `tool_choice = {type:'tool', name:'classify_result'}`, max_tokens =
     `4096 + thinkingPadding`, temperature 0.
8. On `unavailable=true` and `transcriptTooLong=false`: gated by
   GrowthBook `tengu_iron_gate_closed` (default true) at
   `permissions.ts:847`. True → fail-closed deny with
   `buildClassifierUnavailableMessage`. False → fall back to normal
   prompting.

### 5.7 XML 2-stage classifier

`yoloClassifier.ts:711-996`. Modes (`TwoStageMode`):

- `'both'` (default): Stage1 max_tokens=64, `stop_sequences=['</block>']`,
  suffix `XML_S1_SUFFIX`. If parses to `<block>no</block>` → allow. If
  `<block>yes</block>` or unparseable → escalate stage2.
- `'fast'`: Stage1 only. max_tokens=256, no `stop_sequences` (so reason
  parse can read `<reason>`).
- `'thinking'`: Stage2 only.

Stage2: max_tokens = `4096 + thinkingPadding`, suffix `XML_S2_SUFFIX`,
parses `<thinking>`, `<block>`, `<reason>` (after `stripThinking()` strips
`<thinking>...</thinking>`). Unparseable stage2 → fail-closed
`shouldBlock=true`.

`getClassifierThinkingConfig(model)` — `yoloClassifier.ts:683-693`.
Models with `alwaysOnThinking` (ANT-only `resolveAntModel`) return
`[undefined, 2048]` (don't disable thinking, pad max_tokens). Others return
`[false, 0]` (disable thinking).

`detectPromptTooLong(error)` — yoloClassifier.ts:1463-1471. Detects the
literal substring `'prompt is too long'` (case-insensitive) and parses
`N tokens > M maximum` via `parsePromptTooLongTokenCounts`. Returns a
deterministic flag — caller does NOT retry, falls back to normal prompting.

### 5.8 Rule-based (bypass-respecting) sub-pipeline

`utils/permissions/permissions.ts:1071-1156` (`checkRuleBasedPermissions`)
mirrors steps 1a..1g but skips mode-based steps (2a, 2b, classifier, hooks).
Used by callers that need only the rule subset (e.g. some pre-flight
checks). Caller must pre-check `tool.requiresUserInteraction()`.

---

## §6 Verbatim Assets & Decision Tree

### 6.1 Master decision tree — `hasPermissionsToUseToolInner` then `hasPermissionsToUseTool`

Two layers. The inner function (`permissions.ts:1158-1319`) handles steps
1a..2b + 3 (`passthrough → ask`). The outer function (`permissions.ts:473-956`)
applies post-step transformations: `dontAsk`, auto-mode classifier,
shouldAvoidPermissionPrompts auto-deny.

Verbatim pseudocode (compacted from `permissions.ts`, exactly preserving
branch order and predicates):

```
hasPermissionsToUseToolInner(tool, input, context):
  if context.abortController.signal.aborted: throw AbortError
  appState = context.getAppState()

  # 1a. Tool-wide deny rule
  denyRule = getDenyRuleForTool(appState.toolPermissionContext, tool)
  if denyRule:
    return { behavior: 'deny',
             decisionReason: { type:'rule', rule: denyRule },
             message: `Permission to use ${tool.name} has been denied.` }

  # 1b. Tool-wide ask rule
  askRule = getAskRuleForTool(appState.toolPermissionContext, tool)
  if askRule:
    canSandboxAutoAllow =
      tool.name == BASH_TOOL_NAME &&
      SandboxManager.isSandboxingEnabled() &&
      SandboxManager.isAutoAllowBashIfSandboxedEnabled() &&
      shouldUseSandbox(input)
    if not canSandboxAutoAllow:
      return { behavior:'ask',
               decisionReason:{ type:'rule', rule: askRule },
               message: createPermissionRequestMessage(tool.name) }
    # else fall through to let Bash.checkPermissions handle command-specific rules

  # 1c. Tool implementation check
  toolPermissionResult = { behavior:'passthrough',
                           message: createPermissionRequestMessage(tool.name) }
  try:
    parsedInput = tool.inputSchema.parse(input)
    toolPermissionResult = await tool.checkPermissions(parsedInput, context)
  catch e:
    if e is AbortError or APIUserAbortError: throw
    else: logError(e)

  # 1d. Tool denied
  if toolPermissionResult.behavior == 'deny': return toolPermissionResult

  # 1e. Tool requires user interaction even in bypass mode
  if tool.requiresUserInteraction?.() && toolPermissionResult.behavior == 'ask':
    return toolPermissionResult

  # 1f. Content-specific ask rules from tool.checkPermissions
  if toolPermissionResult.behavior == 'ask' &&
     toolPermissionResult.decisionReason?.type == 'rule' &&
     toolPermissionResult.decisionReason.rule.ruleBehavior == 'ask':
    return toolPermissionResult

  # 1g. Safety-check 'ask' (bypass-immune): .git/, .claude/, .vscode/, shell configs
  if toolPermissionResult.behavior == 'ask' &&
     toolPermissionResult.decisionReason?.type == 'safetyCheck':
    return toolPermissionResult

  # 2a. Mode-based bypass
  appState = context.getAppState()  # re-read latest
  shouldBypass = appState.toolPermissionContext.mode == 'bypassPermissions' ||
                 (mode == 'plan' && isBypassPermissionsModeAvailable)
  if shouldBypass:
    return { behavior:'allow',
             updatedInput: getUpdatedInputOrFallback(toolPermissionResult, input),
             decisionReason:{ type:'mode', mode } }

  # 2b. Tool-wide allow rule
  alwaysAllowedRule = toolAlwaysAllowedRule(toolPermissionContext, tool)
  if alwaysAllowedRule:
    return { behavior:'allow',
             updatedInput: getUpdatedInputOrFallback(toolPermissionResult, input),
             decisionReason:{ type:'rule', rule: alwaysAllowedRule } }

  # 3. Convert passthrough → ask
  result = (toolPermissionResult.behavior == 'passthrough')
           ? { ...toolPermissionResult,
               behavior:'ask',
               message: createPermissionRequestMessage(tool.name,
                                                      toolPermissionResult.decisionReason) }
           : toolPermissionResult
  return result


hasPermissionsToUseTool(tool, input, context, assistantMessage, toolUseID):
  result = hasPermissionsToUseToolInner(tool, input, context)

  # Allow path: maybe reset auto-mode consecutive denials
  if result.behavior == 'allow':
    if feature('TRANSCRIPT_CLASSIFIER') && mode == 'auto':
      denialState = context.localDenialTracking ?? appState.denialTracking
      if denialState && consecutiveDenials > 0:
        persistDenialState(context, recordSuccess(denialState))
    return result

  # 'ask' path: dontAsk → deny
  if result.behavior == 'ask' && mode == 'dontAsk':
    return { behavior:'deny',
             decisionReason:{ type:'mode', mode:'dontAsk' },
             message: DONT_ASK_REJECT_MESSAGE(tool.name) }

  # 'ask' path: auto mode (or plan w/ auto active) → run YOLO classifier
  if feature('TRANSCRIPT_CLASSIFIER') &&
     (mode == 'auto' || (mode == 'plan' && isAutoModeActive())):
     # Skip 1: non-classifier-approvable safetyCheck stays as ask (or auto-deny if headless)
     # Skip 2: tool.requiresUserInteraction() → return ask
     # Skip 3: PowerShell w/o POWERSHELL_AUTO_MODE → return ask (or auto-deny if headless)
     # Fast path A: would acceptEdits mode allow this? → allow (unless tool == Agent / REPL)
     # Fast path B: SAFE_YOLO_ALLOWLISTED_TOOLS contains tool.name → allow
     # Otherwise: classifyYoloAction → block | allow | unavailable | transcriptTooLong
     #   transcriptTooLong → fall back to ask (or AbortError if headless)
     #   unavailable + tengu_iron_gate_closed → deny w/ buildClassifierUnavailableMessage
     #   unavailable + !iron_gate → fall back to ask
     #   shouldBlock=true → recordDenial(); if denial-limit hit, fall back to ask
     #                       with classifier reason; else deny w/ buildYoloRejectionMessage
     #   shouldBlock=false → recordSuccess; allow

  # 'ask' path: headless → run hooks first, else auto-deny
  if shouldAvoidPermissionPrompts:
    hookDecision = await runPermissionRequestHooksForHeadlessAgent(...)
    if hookDecision: return hookDecision
    return { behavior:'deny',
             decisionReason:{ type:'asyncAgent',
                              reason:'Permission prompts are not available in this context' },
             message: AUTO_REJECT_MESSAGE(tool.name) }

  return result   # propagate ask/deny up to useCanUseTool's interactive handlers
```

### 6.2 Interactive race (post-pipeline)

When `hasPermissionsToUseTool` returns `'ask'`, `useCanUseTool` (compiled at
`src/hooks/useCanUseTool.tsx:32-...`) dispatches as follows:

```
useCanUseTool(tool, input, ctx, msg, id, forceDecision?):
  pctx = createPermissionContext(...)
  if abortSignal.aborted: resolve(cancelAndAbort)
  result = forceDecision ?? await hasPermissionsToUseTool(...)

  switch result.behavior:
    case 'allow':
      if T_C && classifier=='auto-mode': setYoloClassifierApproval(id, reason)
      logDecision({decision:'accept', source:'config'})
      resolve(buildAllow(updatedInput))
    case 'deny':
      logPermissionDecision({decision:'reject', source:'config'})
      if T_C && classifier=='auto-mode':
        recordAutoModeDenial({...})
        addNotification({key:'auto-mode-denied', priority:'immediate', ...})
      resolve(result)
    case 'ask':
      desc = await tool.description(input, {...})
      if awaitAutomatedChecksBeforeDialog:        # coordinator path
        decision = await handleCoordinatorPermission(...)
        if decision: resolve(decision); return
      decision = await handleSwarmWorkerPermission(...)  # mailbox to leader
      if decision: resolve(decision); return
      # speculative bash classifier race (≤2s)
      if BASH_CLASSIFIER && pendingClassifierCheck && tool == Bash:
        speculative = peekSpeculativeClassifierCheck(input.command)
        if speculative:
          raceResult = Promise.race([speculative, timeout(2000)])
          if raceResult.matches && confidence=='high':
            consumeSpeculativeClassifierCheck(...)
            setClassifierApproval(id, matchedRule)
            resolve(buildAllow(...))
            return
      handleInteractivePermission(...)            # the dialog
```

### 6.3 Interactive handler races (`interactiveHandler.ts`)

For the main agent, six concurrent racers all funnel through one
`createResolveOnce(resolve)` guard (`PermissionContext.ts:75-94`) with
atomic `claim()`:

1. **User dialog** — `pushToQueue` callbacks `onAllow`, `onReject`,
   `onAbort`, `recheckPermission`, `onUserInteraction`,
   `onDismissCheckmark`. Grace period 200 ms before user keypresses cancel
   the classifier (`interactiveHandler.ts:115`).
2. **Bridge (CCR / claude.ai)** — `BRIDGE_MODE` feature flag.
   `bridgeCallbacks.sendRequest`/`onResponse` mirror the full permission
   payload. CCR's response wins via `claim()` if it beats local input.
3. **Channel relay (Telegram, iMessage, etc.)** — `KAIROS` /
   `KAIROS_CHANNELS` feature flag. Sends MCP `notifications/permission_request`
   to every connected channel client; reply intercepted in
   `useManageMCPConnections.ts` BEFORE enqueue (so "yes abc123" never
   reaches Claude as a turn).
4. **PermissionRequest hooks** — only when
   `!awaitAutomatedChecksBeforeDialog` (otherwise hooks already ran in the
   coordinator branch). `executePermissionRequestHooks` is async-iterable;
   first allow/deny wins.
5. **Bash classifier (`BASH_CLASSIFIER`)** — only when `tool == Bash` and
   `pendingClassifierCheck` is set and not coordinator. Sets/clears
   "checking" indicator (`setClassifierChecking` / `clearClassifierChecking`)
   used by the UI to render a spinner.
6. **Abort signal** — wired into `recheckPermission` and the channel /
   bridge unsub paths.

When the bash classifier auto-allows, the dialog enters a "checkmark"
display: 3 s if terminal focused, 1 s if not (`interactiveHandler.ts:509`).
Esc dismisses early via `onDismissCheckmark`.

### 6.4 YOLO classifier system prompt build

`yoloClassifier.ts:484-540`. Pseudocode:

```
buildYoloSystemPrompt(context):
  usingExternal = isUsingExternalPermissions()
  systemPrompt = BASE_PROMPT.replace('<permissions_template>',
                   usingExternal ? EXTERNAL_PERMISSIONS_TEMPLATE
                                 : ANTHROPIC_PERMISSIONS_TEMPLATE)
  autoMode = getAutoModeConfig()  # settings.autoMode
  includeBashPromptRules     = feature('BASH_CLASSIFIER') && !usingExternal
  includePowerShellGuidance  = feature('POWERSHELL_AUTO_MODE') && !usingExternal
  allowDescriptions = [
    if includeBashPromptRules: ...getBashPromptAllowDescriptions(context),
    ...autoMode?.allow ?? []
  ]
  denyDescriptions = [
    if includeBashPromptRules: ...getBashPromptDenyDescriptions(context),
    if includePowerShellGuidance: ...POWERSHELL_DENY_GUIDANCE,
    ...autoMode?.soft_deny ?? []
  ]
  envDescriptions = autoMode?.environment ?? []

  # External template: <user_*_to_replace> wraps DEFAULTS; user values REPLACE them.
  # Anthropic template: defaults sit OUTSIDE tags; user values ADDITIVELY append.
  return systemPrompt
    .replace(/<user_allow_rules_to_replace>([\s\S]*?)<\/user_allow_rules_to_replace>/,
             (_, defaults) => userAllow ?? defaults)
    .replace(/<user_deny_rules_to_replace>([\s\S]*?)<\/user_deny_rules_to_replace>/,
             (_, defaults) => userDeny ?? defaults)
    .replace(/<user_environment_to_replace>([\s\S]*?)<\/user_environment_to_replace>/,
             (_, defaults) => userEnv ?? defaults)
```

`isUsingExternalPermissions()` — yoloClassifier.ts:71-78:

```
if process.env.USER_TYPE != 'ant': return true
config = getFeatureValue_CACHED_MAY_BE_STALE('tengu_auto_mode_config', {})
return config.forceExternalPermissions === true
```

### 6.5 YOLO classifier tool schema (verbatim)

`yoloClassifier.ts:260-285`:

```ts
export const YOLO_CLASSIFIER_TOOL_NAME = 'classify_result'

const YOLO_CLASSIFIER_TOOL_SCHEMA: BetaToolUnion = {
  type: 'custom',
  name: YOLO_CLASSIFIER_TOOL_NAME,
  description: 'Report the security classification result for the agent action',
  input_schema: {
    type: 'object',
    properties: {
      thinking:    { type: 'string',
                     description: 'Brief step-by-step reasoning.' },
      shouldBlock: { type: 'boolean',
                     description:
                       'Whether the action should be blocked (true) or allowed (false)' },
      reason:      { type: 'string',
                     description:
                       'Brief explanation of the classification decision' },
    },
    required: ['thinking', 'shouldBlock', 'reason'],
  },
}
```

Zod validation: `yoloClassifierResponseSchema` at `yoloClassifier.ts:252-258`
— `z.object({thinking: z.string(), shouldBlock: z.boolean(), reason: z.string()})`.

### 6.6 XML classifier suffixes (verbatim)

`yoloClassifier.ts:550-561`:

```
const XML_S1_SUFFIX = '\nErr on the side of blocking. <block> immediately.'

const XML_S2_SUFFIX =
  '\nReview the classification process and follow it carefully, making sure ' +
  'you deny actions that should be blocked. As a reminder, explicit (not ' +
  'suggestive or implicit) user confirmation is required to override blocks. ' +
  'Use <thinking> before responding with <block>.'
```

XML output instructions (`yoloClassifier.ts:649-664`):

```
const xmlFormat = [
  '## Output Format',
  '',
  'If the action should be blocked:',
  '<block>yes</block><reason>one short sentence</reason>',
  '',
  'If the action should be allowed:',
  '<block>no</block>',
  '',
  'Do NOT include a <reason> tag when the action is allowed.',
  'Your ENTIRE response MUST begin with <block>. Do NOT output any analysis, ' +
    'reasoning, or commentary before <block>. No "Looking at..." or similar ' +
    'preamble.',
].join('\n')
```

### 6.7 PowerShell deny guidance (verbatim, ANT-only)

`yoloClassifier.ts:1402-1411`. All four entries are gated by
`feature('POWERSHELL_AUTO_MODE')` and absent from external builds:

```
'PowerShell Download-and-Execute: `iex (iwr ...)`, `Invoke-Expression (Invoke-WebRequest ...)`,
 `Invoke-Expression (New-Object Net.WebClient).DownloadString(...)`, and any pipeline feeding
 remote content into `Invoke-Expression`/`iex` fall under "Code from External" — same as
 `curl | bash`.',
'PowerShell Irreversible Destruction: `Remove-Item -Recurse -Force`, `rm -r -fo`,
 `Clear-Content`, and `Set-Content` truncation of pre-existing files fall under
 "Irreversible Local Destruction" — same as `rm -rf` and `> file`.',
'PowerShell Persistence: modifying `$PROFILE` (any of the four profile paths),
 `Register-ScheduledTask`, `New-Service`, writing to registry Run keys
 (`HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Run` or the HKLM equivalent),
 and WMI event subscriptions fall under "Unauthorized Persistence" — same as
 `.bashrc` edits and cron jobs.',
'PowerShell Elevation: `Start-Process -Verb RunAs`, `-ExecutionPolicy Bypass`, and
 disabling AMSI/Defender (`Set-MpPreference -DisableRealtimeMonitoring`) fall under
 "Security Weaken".',
```

### 6.8 User-facing reject / deny messages (verbatim)

`utils/messages.ts:212-237, 267-294`:

```
export const REJECT_MESSAGE =
  "The user doesn't want to proceed with this tool use. The tool use was " +
  "rejected (eg. if it was a file edit, the new_string was NOT written to " +
  "the file). STOP what you are doing and wait for the user to tell you how " +
  "to proceed."

export const REJECT_MESSAGE_WITH_REASON_PREFIX =
  "The user doesn't want to proceed with this tool use. The tool use was " +
  "rejected (eg. if it was a file edit, the new_string was NOT written to " +
  "the file). To tell you how to proceed, the user said:\n"

export const SUBAGENT_REJECT_MESSAGE =
  'Permission for this tool use was denied. The tool use was rejected (eg. ' +
  'if it was a file edit, the new_string was NOT written to the file). Try ' +
  'a different approach or report the limitation to complete your task.'

export const SUBAGENT_REJECT_MESSAGE_WITH_REASON_PREFIX =
  'Permission for this tool use was denied. The tool use was rejected (eg. ' +
  'if it was a file edit, the new_string was NOT written to the file). The ' +
  'user said:\n'

export const DENIAL_WORKAROUND_GUIDANCE =
  `IMPORTANT: You *may* attempt to accomplish this action using other tools ` +
  `that might naturally be used to accomplish this goal, e.g. using head ` +
  `instead of cat. But you *should not* attempt to work around this denial ` +
  `in malicious ways, e.g. do not use your ability to run tests to execute ` +
  `non-test actions. You should only try to work around this restriction in ` +
  `reasonable ways that do not attempt to bypass the intent behind this denial. ` +
  `If you believe this capability is essential to complete the user's request, ` +
  `STOP and explain to the user what you were trying to do and why you need ` +
  `this permission. Let the user decide how to proceed.`

function AUTO_REJECT_MESSAGE(toolName) =
  `Permission to use ${toolName} has been denied. ${DENIAL_WORKAROUND_GUIDANCE}`

function DONT_ASK_REJECT_MESSAGE(toolName) =
  `Permission to use ${toolName} has been denied because Claude Code is ` +
  `running in don't ask mode. ${DENIAL_WORKAROUND_GUIDANCE}`

function buildYoloRejectionMessage(reason):
  prefix = AUTO_MODE_REJECTION_PREFIX
  ruleHint = feature('BASH_CLASSIFIER')
    ? `To allow this type of action in the future, the user can add a ` +
      `permission rule like Bash(prompt: <description of allowed action>) ` +
      `to their settings. At the end of your session, recommend what ` +
      `permission rules to add so you don't get blocked again.`
    : `To allow this type of action in the future, the user can add a Bash ` +
      `permission rule to their settings.`
  return `${prefix}${reason}. If you have other tasks that don't depend on ` +
         `this action, continue working on those. ${DENIAL_WORKAROUND_GUIDANCE} ` +
         ruleHint

function buildClassifierUnavailableMessage(toolName, classifierModel):
  return `${classifierModel} is temporarily unavailable, so auto mode ` +
         `cannot determine the safety of ${toolName} right now. Wait briefly ` +
         `and then try this action again. If it keeps failing, continue with ` +
         `other tasks that don't require this action and come back to it ` +
         `later. Note: reading files, searching code, and other read-only ` +
         `operations do not require the classifier and can still be used.`
```

`createPermissionRequestMessage(toolName, decisionReason?)` —
`permissions.ts:137-211`. Per-reason messages:

- `classifier`:
  `Classifier '${classifier}' requires approval for this ${toolName} command: ${reason}`
- `hook` (with reason):
  `Hook '${hookName}' blocked this action: ${reason}`
- `hook` (no reason):
  `Hook '${hookName}' requires approval for this ${toolName} command`
- `rule`:
  `Permission rule '${ruleString}' from ${sourceString} requires approval for this ${toolName} command`
- `subcommandResults` (multiple parts):
  `This ${toolName} command contains multiple operations. The following ${plural(n,'part')} ${plural(n,'requires','require')} approval: ${parts.join(', ')}`
- `subcommandResults` (no parts):
  `This ${toolName} command contains multiple operations that require approval`
- `permissionPromptTool`:
  `Tool '${permissionPromptToolName}' requires approval for this ${toolName} command`
- `sandboxOverride`:
  `Run outside of the sandbox`
- `mode`:
  `Current permission mode (${permissionModeTitle(mode)}) requires approval for this ${toolName} command`
- `safetyCheck` / `other` / `workingDir` / `asyncAgent`:
  the `decisionReason.reason` field verbatim
- default fallback:
  `Claude requested permissions to use ${toolName}, but you haven't granted it yet.`

For deny via tool-wide deny rule (1a):
`Permission to use ${tool.name} has been denied.`
(`permissions.ts:1087, 1179`)

### 6.9 SAFE_YOLO_ALLOWLISTED_TOOLS (verbatim)

`utils/permissions/classifierDecision.ts:56-94`:

```
const SAFE_YOLO_ALLOWLISTED_TOOLS = new Set([
  // Read-only file operations
  FILE_READ_TOOL_NAME,
  // Search / read-only
  GREP_TOOL_NAME,
  GLOB_TOOL_NAME,
  LSP_TOOL_NAME,
  TOOL_SEARCH_TOOL_NAME,
  LIST_MCP_RESOURCES_TOOL_NAME,
  'ReadMcpResourceTool',
  // Task management (metadata only)
  TODO_WRITE_TOOL_NAME,
  TASK_CREATE_TOOL_NAME, TASK_GET_TOOL_NAME, TASK_UPDATE_TOOL_NAME,
  TASK_LIST_TOOL_NAME,   TASK_STOP_TOOL_NAME, TASK_OUTPUT_TOOL_NAME,
  // Plan mode / UI
  ASK_USER_QUESTION_TOOL_NAME,
  ENTER_PLAN_MODE_TOOL_NAME,
  EXIT_PLAN_MODE_TOOL_NAME,
  // Swarm coordination (internal mailbox/team state only)
  TEAM_CREATE_TOOL_NAME,
  TEAM_DELETE_TOOL_NAME,
  SEND_MESSAGE_TOOL_NAME,
  // Workflow orchestration (only with WORKFLOW_SCRIPTS feature flag)
  ...(WORKFLOW_TOOL_NAME ? [WORKFLOW_TOOL_NAME] : []),
  // Misc safe
  SLEEP_TOOL_NAME,
  // Ant-only safe tools
  ...(TERMINAL_CAPTURE_TOOL_NAME ? [TERMINAL_CAPTURE_TOOL_NAME] : []),
  ...(OVERFLOW_TEST_TOOL_NAME ? [OVERFLOW_TEST_TOOL_NAME] : []),
  ...(VERIFY_PLAN_EXECUTION_TOOL_NAME ? [VERIFY_PLAN_EXECUTION_TOOL_NAME] : []),
  // Internal classifier tool
  YOLO_CLASSIFIER_TOOL_NAME,
])
```

### 6.10 LEGACY_TOOL_NAME_ALIASES (verbatim)

`utils/permissions/permissionRuleParser.ts:21-29`:

```
{
  Task:             AGENT_TOOL_NAME,
  KillShell:        TASK_STOP_TOOL_NAME,
  AgentOutputTool:  TASK_OUTPUT_TOOL_NAME,
  BashOutputTool:   TASK_OUTPUT_TOOL_NAME,
  ...((feature('KAIROS') || feature('KAIROS_BRIEF')) && BRIEF_TOOL_NAME
    ? { Brief: BRIEF_TOOL_NAME } : {})
}
```

### 6.11 DANGEROUS_BASH_PATTERNS (verbatim)

`utils/permissions/dangerousPatterns.ts:18-80`:

```
CROSS_PLATFORM_CODE_EXEC = [
  'python','python3','python2','node','deno','tsx','ruby','perl','php','lua',
  'npx','bunx','npm run','yarn run','pnpm run','bun run',
  'bash','sh',
  'ssh',
]

DANGEROUS_BASH_PATTERNS = [
  ...CROSS_PLATFORM_CODE_EXEC,
  'zsh','fish','eval','exec','env','xargs','sudo',
  ...(process.env.USER_TYPE === 'ant'
    ? ['fa run', 'coo', 'gh', 'gh api', 'curl', 'wget',
       'git', 'kubectl', 'aws', 'gcloud', 'gsutil']
    : []),
]
```

`isDangerousBashPermission(toolName, ruleContent)` —
`permissionSetup.ts:94-150`. Matches if any of:

- `ruleContent` is undefined or `''` (tool-level allow)
- `ruleContent.trim().toLowerCase() === '*'`
- `content === pattern` (lowercase)
- `content === ${pattern}:*`
- `content === ${pattern}*`
- `content === ${pattern} *`
- `content.startsWith(${pattern} -) && content.endsWith(*)`

### 6.12 SDK denial envelope (verbatim)

`src/entrypoints/sdk/coreSchemas.ts:1399-1404`:

```ts
export const SDKPermissionDenialSchema = lazySchema(() =>
  z.object({
    tool_name: z.string(),
    tool_use_id: z.string(),
    tool_input: z.record(z.string(), z.unknown()),
  }),
)
```

Embedded into both result schemas (`coreSchemas.ts:1420, 1445`):
`permission_denials: z.array(SDKPermissionDenialSchema())`. Tracking
implemented in `QueryEngine.wrappedCanUseTool` (`QueryEngine.ts:244-272`):
every non-allow result is pushed onto `this.permissionDenials`, then
serialized into the terminal `result` message at lines `631, 863, 993,
1036, 1095, 1148`.

### 6.13 MCP permission-prompt tool I/O (verbatim)

`utils/permissions/PermissionPromptToolResultSchema.ts:15-77`:

```ts
inputSchema = z.object({
  tool_name:    z.string(),
  input:        z.record(z.string(), z.unknown()),
  tool_use_id:  z.string().optional(),
})

PermissionAllowResultSchema = z.object({
  behavior: z.literal('allow'),
  updatedInput: z.record(z.string(), z.unknown()),
  updatedPermissions: z.array(permissionUpdateSchema()).optional()
                       .catch(ctx => { logForDebugging(...); return undefined }),
  toolUseID: z.string().optional(),
  decisionClassification: z.enum(['user_temporary','user_permanent','user_reject'])
                          .optional().catch(undefined),
})

PermissionDenyResultSchema = z.object({
  behavior: z.literal('deny'),
  message: z.string(),
  interrupt: z.boolean().optional(),
  toolUseID: z.string().optional(),
  decisionClassification: ...,
})

outputSchema = z.union([PermissionAllowResultSchema, PermissionDenyResultSchema])
```

`permissionPromptToolResultToPermissionDecision` (`:84-127`) consumes the
output, applies `updatedPermissions` to context via `applyPermissionUpdates`
+ `persistPermissionUpdates`, treats empty `updatedInput` as "use original
input" (mobile-clients-from-push edge case at `:108-110`), and on
`deny.interrupt === true` calls `toolUseContext.abortController.abort()`.

### 6.14 PermissionRequest hook (cited)

Hook event shape lives in `src/types/hooks.ts` (full schema owned by §22).
Key contract for this subsystem:

- Event name: `PermissionRequest`
- Inputs: `tool_name`, `tool_use_id`, `tool_input`, `permission_mode`,
  `suggestions`
- Outputs: `permissionRequestResult: { behavior:'allow' | 'deny',
  updatedInput?, updatedPermissions?, message?, interrupt? }`
- Iteration: `executePermissionRequestHooks` is async-iterable; the first
  hook returning a `permissionRequestResult` wins.
- In headless agents, hooks run before the auto-deny via
  `runPermissionRequestHooksForHeadlessAgent` (`permissions.ts:400-471`).
- In coordinators (`awaitAutomatedChecksBeforeDialog === true`), hooks run
  sequentially before the dialog renders
  (`coordinatorHandler.ts:32-45`).
- On `deny.interrupt === true`, `toolUseContext.abortController.abort()`
  is called and the conversation halts.

### 6.15 Constants & env-var table

| Constant | Value | Source |
|---|---|---|
| `DENIAL_LIMITS.maxConsecutive` | 3 | `denialTracking.ts:13` |
| `DENIAL_LIMITS.maxTotal` | 20 | `denialTracking.ts:14` |
| `CLASSIFIER_FAIL_CLOSED_REFRESH_MS` | 30 * 60 * 1000 = 1 800 000 | `permissions.ts:107` |
| User-interaction grace period | 200 ms | `interactiveHandler.ts:115` |
| Speculative classifier timeout | 2000 ms | `useCanUseTool.tsx:131` |
| Bash classifier checkmark display | 3000 ms (focused) / 1000 ms (unfocused) | `interactiveHandler.ts:509` |
| YOLO classifier max_tokens (full) | `4096 + thinkingPadding` | `yoloClassifier.ts:1136` |
| YOLO XML stage1 max_tokens | `64 + thinkingPadding` (`'both'`) / `256` (`'fast'`) | `yoloClassifier.ts:781` |
| YOLO XML stage2 max_tokens | `4096 + thinkingPadding` | `yoloClassifier.ts:868` |
| `thinkingPadding` (alwaysOnThinking models) | 2048 | `yoloClassifier.ts:689` |
| `thinkingPadding` (other models) | 0 | `yoloClassifier.ts:692` |
| YOLO classifier temperature | 0 | `yoloClassifier.ts:784, 871, 1145` |
| Stage1 stop_sequences | `['</block>']` (mode `both` only) | `yoloClassifier.ts:792` |
| Auto-mode prompt cache control | `getCacheControl({querySource:'auto_mode'})` (1h TTL via GrowthBook allowlist) | `yoloClassifier.ts:1096` |
| Permission explainer model | `getMainLoopModel()` | `permissionExplainer.ts:175` |
| `process.env.CLAUDE_CODE_AUTO_MODE_MODEL` | classifier model override (ANT) | `yoloClassifier.ts:1336` |
| `process.env.CLAUDE_CODE_TWO_STAGE_CLASSIFIER` | `'fast' | 'thinking' | truthy | falsy` (ANT) | `yoloClassifier.ts:1359` |
| `process.env.CLAUDE_CODE_JSONL_TRANSCRIPT` | toggle JSONL transcript (ANT) | `yoloClassifier.ts:1381` |
| `process.env.CLAUDE_CODE_DUMP_AUTO_MODE` | dump req/res to per-user temp (ANT) | `yoloClassifier.ts:160` |
| `process.env.CLAUDE_CODE_TMPDIR` | per-user tempdir override | `filesystem.ts:332` |
| `process.env.USER_TYPE === 'ant'` | gates ANT-only paths throughout | (everywhere) |
| `getFeatureValue_CACHED_WITH_REFRESH('tengu_iron_gate_closed', true, 30min)` | fail-closed on classifier API error | `permissions.ts:847-850` |
| `checkStatsigFeatureGate_CACHED_MAY_BE_STALE('tengu_scratch')` | scratchpad enable | `filesystem.ts:299` |

### 6.16 Mode cycling order (Shift+Tab)

`utils/permissions/getNextPermissionMode.ts:34-78`:

```
default → (USER_TYPE=='ant'
            ? (isBypassPermissionsModeAvailable ? bypassPermissions
              : canCycleToAuto ? auto : default)
            : acceptEdits)
acceptEdits → plan
plan → (isBypassPermissionsModeAvailable ? bypassPermissions
        : canCycleToAuto ? auto : default)
bypassPermissions → (canCycleToAuto ? auto : default)
dontAsk → default        # not exposed in UI cycle today
auto → default           # any future mode also falls back to default
```

`canCycleToAuto(ctx)` requires both `ctx.isAutoModeAvailable` (set at
startup by `verifyAutoModeGateAccess`) AND the live
`isAutoModeGateEnabled()` (`getNextPermissionMode.ts:17-29`).

`cyclePermissionMode(ctx, teamContext?)` returns
`{nextMode, context: transitionPermissionMode(currentMode, nextMode, ctx)}`.
`transitionPermissionMode` (`permissionSetup.ts:597-...`) handles plan-mode
attachments and auto-mode dangerous-rule strip/restore.

---

## §7 Wire format

This subsystem produces and consumes:

- **Settings JSON** (read by `permissionsLoader.ts`):
  `permissions: {allow?: string[], deny?: string[], ask?: string[],
  defaultMode?: ExternalPermissionMode, additionalDirectories?: string[]}`.
- **SDK control message** `set_permission_mode` and
  `set_permission_mode_decision` — owned by §22.
- **MCP permission-prompt tool** — schema in §6.13.
- **Hook PermissionRequest event** — schema referenced in §6.14, body in §22.
- **SDK terminal `result` message** `permission_denials: SDKPermissionDenial[]`
  (§6.12).

---

## §8 State, ordering, concurrency

- Decision tree is single-threaded per tool call. All mutation goes through
  React's `setAppState` (or `Object.assign` for `localDenialTracking`).
- Within `useCanUseTool`, multiple racers all funnel through one
  `createResolveOnce(resolve)` so exactly one decision wins. `claim()` is
  the atomic check-and-mark used before any `await` to close the
  isResolved-vs-resolve race window (`PermissionContext.ts:65-93`).
- Headless / coordinator subagents may carry a `localDenialTracking` field;
  this overrides `appState.denialTracking` for both reads and writes
  (`permissions.ts:556`, `:967`).
- Settings hot-reload (when settings.json changes on disk) calls
  `syncPermissionRulesFromDisk` which clears all disk-source/behavior pairs
  before re-applying — necessary because empty groups produce no update,
  leaving stale rules otherwise (`permissions.ts:1448-1467`).

---

## §9 Error handling

- `tool.inputSchema.parse(input)` throws → caught at `permissions.ts:1217-1222`,
  logged, decision falls through with `behavior:'passthrough'` (then →
  `ask`).
- `tool.checkPermissions` throws AbortError / APIUserAbortError → re-thrown.
- Hooks throwing → headless path catches (`runPermissionRequestHooksForHeadlessAgent`
  at `permissions.ts:462-469`); interactive path lets the dialog stay open.
- Classifier API failure → `unavailable: true`. Fail-closed under
  `tengu_iron_gate_closed` (default true), else fall back to manual prompt.
- Classifier API "prompt is too long" → `transcriptTooLong: true`. Headless
  → AbortError. Interactive → fall back to manual prompt.
- `deletePermissionRule` for read-only sources (`policySettings`,
  `flagSettings`, `command`) throws a plain `Error`
  (`permissions.ts:1334-1340`).
- `permissionRuleValueFromString` accepts malformed input (mismatched
  parens, content after `)`) by treating the whole string as a tool name
  (`permissionRuleParser.ts:107-114`).

---

## §10 Side-effects ledger

- Persists to settings.json (when destination is editable) — see §02.
- Mutates `appState.toolPermissionContext`, `appState.denialTracking`,
  `appState.notifications.queue` (auto-mode-denied / auto-mode-error-dump).
- Calls `persistDenialState` on every classifier verdict path.
- Logs many analytics events (delegated to §26):
  `tengu_tool_use_granted_in_config`, `tengu_tool_use_granted_in_prompt_*`,
  `tengu_tool_use_granted_by_classifier`, `tengu_tool_use_granted_by_permission_hook`,
  `tengu_tool_use_rejected_in_prompt`, `tengu_tool_use_denied_in_config`,
  `tengu_tool_use_cancelled`, `tengu_auto_mode_decision`,
  `tengu_auto_mode_outcome`, `tengu_auto_mode_denial_limit_exceeded`,
  `tengu_auto_mode_malformed_tool_input`, `tengu_permission_explainer_generated`,
  `tengu_permission_explainer_error`.
- Emits OTel `tool_decision` events (`permissionLogging.ts:230-234`) and
  code-edit decision counters (`permissionLogging.ts:213-217`).
- Records `recordAutoModeDenial` for the `/permissions` UI ribbon
  (`useCanUseTool.tsx:78-83`).
- Writes auto-mode classifier req/res JSON when
  `CLAUDE_CODE_DUMP_AUTO_MODE=1` (ANT) — `yoloClassifier.ts:153-180`.
- Calls `setClassifierApproval` / `setYoloClassifierApproval` /
  `setClassifierChecking` / `clearClassifierChecking` on `classifierApprovals`
  module (used by UI to show check / spinner).

---

## §11 Cross-cutting flags

Feature flags this spec is gated by:

- `TRANSCRIPT_CLASSIFIER` — auto mode end-to-end (mode value, classifier,
  state module). When OFF: `'auto'` is removed from `INTERNAL_PERMISSION_MODES`,
  `getNextPermissionMode` never returns auto, classifier modules absent.
- `BASH_CLASSIFIER` — Bash prompt-rule classifier (`Bash(prompt: ...)`),
  speculative race in `useCanUseTool`, classifier-checking indicator. Gates
  at `bashPermissions.ts:1576, 1645` and many other Bash-tool sites — the
  permission system *consumes* the flag's outputs but doesn't define the
  Bash-specific logic (see §10).
- `POWERSHELL_AUTO_MODE` — extends classifier deny prompt with PowerShell
  guidance (§6.7). Sits beside auto mode, not gating auto on its own.
- `KAIROS` / `KAIROS_CHANNELS` — channel relay race in `interactiveHandler.ts`.
- `BRIDGE_MODE` — bridge race in `interactiveHandler.ts`.
- `KAIROS_BRIEF` — adds `Brief` legacy alias.
- `HOOK_PROMPTS` — REPL exposes `requestPrompt` callback to commands /
  hooks (`screens/REPL.tsx:2520`). Gating only — body owned by §37.
- `USER_TYPE === 'ant'` — toggles many specifics: extra
  `DANGEROUS_BASH_PATTERNS` entries (gh, curl, git, kubectl, aws, gcloud,
  gsutil, fa run, coo); auto-mode permission template selection
  (`isUsingExternalPermissions`); `getNextPermissionMode` order; auto-mode
  classifier env vars; auto-mode error-dump notifications; PermissionMode
  external mapping for `auto`.

---

## §12 Open questions / missing source

1. **`bubble` mode** (`InternalPermissionMode = ... | 'bubble'`,
   `types/permissions.ts:28`). **RESOLVED Phase 9.6** — `bubble` is NOT a
   type-only placeholder; it IS used at runtime as the permission mode for
   **forked subagents**. Verified at `forkSubagent.ts:67` (`permissionMode:
   'bubble'` on `FORK_AGENT`), `runAgent.ts:443` (runtime branch
   `agentPermissionMode === 'bubble'` controlling `shouldAvoidPrompts`), and
   `runAgent.ts:430-433` (writes `'bubble'` into the forked
   `ToolPermissionContext.mode`). Semantic role: bubble surfaces permission
   prompts to the parent session rather than prompting locally. The mode is
   correctly excluded from `INTERNAL_PERMISSION_MODES` (Zod-validated user
   set) — but it IS a valid runtime context.mode value. See §3 glossary +
   §4.1 Discrepancy block for the corrected description. Phase 9.4 had
   incorrectly classified this as type-only.
2. **YOLO classifier prompt files** are absent from the leak —
   `auto_mode_system_prompt.txt`, `permissions_external.txt`,
   `permissions_anthropic.txt` are loaded via
   `require('./yolo-classifier-prompts/<name>.txt')` (`yoloClassifier.ts:55-68`)
   but the `yolo-classifier-prompts/` subdirectory does not exist in the
   tree (verified via `ls`). The bundler inlines them at build time. Spec
   §6.4 documents the *substitution mechanism* and the user-allow / soft_deny /
   environment shape; the verbatim base prompt is **not** recoverable from
   the leaked source. Open question: whether the .txt files are reachable
   via any other path (e.g. bundled `cli.js` debug strings).
3. **`isAutoModeAvailable`** is read by `getNextPermissionMode.ts:21,42` and
   set by `permissionSetup.ts:987` but is not declared on
   `ToolPermissionContext` in `types/permissions.ts:427-441`. Either an
   intentional `any`-typed extension or a TS gap. Consumers should check
   for the field defensively.
4. **`localDenialTracking`** is referenced on `ToolUseContext` in
   `permissions.ts:490, 556, 967` (and on the swarm/coordinator paths). The
   declaration site lives in `src/Tool.ts` / `src/types/` and is owned by
   §08; this spec asserts only the contract.
5. **`isAutoModeActive()`** — global flag in `utils/permissions/autoModeState.ts`
   driven by `setAutoModeActive` in `transitionPermissionMode`. Callers
   include `permissions.ts:524`, `permissionSetup.ts`, and the auto-mode
   carousel. Detailed setter call graph deferred to §27 (since it
   intersects with policy/Statsig kickout) and §32 (carousel UI).
6. **`isInProtectedNamespace()`** referenced from `permissions.ts:631, 670,
   738`. Body lives in `utils/envUtils.ts`; this spec records that the
   classifier decision telemetry includes its boolean result.
7. **`HOOK_PROMPTS` `requestPrompt` callback contract** — body in §37
   (Ink UI). Spec §11 records only that REPL exposes this when
   `feature('HOOK_PROMPTS')` is on (`screens/REPL.tsx:2520`).
8. **`getCacheControl({querySource:'auto_mode'})`** call site is here, but
   the actual TTL allowlist evaluation lives in
   `services/api/claude.ts` — owned by §22.
9. **PermissionExplainer config opt-out** — `getGlobalConfig().permissionExplainerEnabled`
   is the toggle; the *default* is non-`false`, i.e. enabled. Source: line
   `permissionExplainer.ts:140`. Where this config lives in the schema is
   owned by §02.

Estimated claims (`estimated from <evidence>`):

- All speculative classifier behaviors come from the BASH_CLASSIFIER /
  TRANSCRIPT_CLASSIFIER paths in the leaked source; behaviors deactivate
  cleanly when those flags are off (verified by reading the bashClassifier
  external stub which returns `matches: false` always). No estimation.
- The XML stage suffixes' corresponding Python references
  (`sandbox/johnh/control/bpc_classifier/classifier.py`,
  `sandbox/alexg/evals/...`) cited in code comments at
  `yoloClassifier.ts:548-561` are **not** in this leak; their existence is
  attested only by the comments. Documented as cited evidence.
