# 21c — Command Catalog: Feature-Flag Gated

> Per-command spec for every entry in `COMMANDS()` (and `INTERNAL_ONLY_COMMANDS`) registered via a `feature(...)` gate. Read 20-command-system.md and 21-command-catalog.md first.
>
> The `feature(...)` gate is build-time DCE (00 Appendix A): when the flag is off, Bun's bundler strips both the `require(...)` and the registration site. Reimplementers must reproduce the conditional `require` pattern (NOT a runtime check) for the bundle to behave identically.

---

## 1. Purpose & Scope

Every command in this file is registered behind one or more `feature(...)` flags at the top of `commands.ts:60-122` (and a small spread inside `INTERNAL_ONLY_COMMANDS`). Per-flag table:

| Flag | Command | Source path | Source present? |
|---|---|---|---|
| `PROACTIVE` ∨ `KAIROS` | `/proactive` | `src/commands/proactive.ts` | ❌ missing |
| `KAIROS` ∨ `KAIROS_BRIEF` | `/brief` | `src/commands/brief.ts` | ✅ |
| `KAIROS` | `/assistant` | `src/commands/assistant/index.ts` | ❌ missing |
| `BRIDGE_MODE` | `/remote-control` (alias `/rc`) | `src/commands/bridge/index.ts` | ✅ |
| `DAEMON` ∧ `BRIDGE_MODE` | `/remoteControlServer` | `src/commands/remoteControlServer/index.ts` | ❌ missing |
| `VOICE_MODE` | `/voice` | `src/commands/voice/index.ts` | ✅ |
| `HISTORY_SNIP` | `/force-snip` (in INTERNAL_ONLY) | `src/commands/force-snip.ts` | ❌ missing |
| `WORKFLOW_SCRIPTS` | `/workflows` | `src/commands/workflows/index.ts` | ❌ missing |
| `CCR_REMOTE_SETUP` | `/web-setup` | `src/commands/remote-setup/index.ts` | ✅ |
| `EXPERIMENTAL_SKILL_SEARCH` | (no command — wires `clearSkillIndexCache`) | n/a | n/a |
| `KAIROS_GITHUB_WEBHOOKS` | `/subscribe-pr` (in INTERNAL_ONLY) | `src/commands/subscribe-pr.ts` | ❌ missing |
| `ULTRAPLAN` | `/ultraplan` (in INTERNAL_ONLY) | `src/commands/ultraplan.tsx` | ✅ (compiled) |
| `TORCH` | `/torch` | `src/commands/torch.ts` | ❌ missing |
| `UDS_INBOX` | `/peers` | `src/commands/peers/index.ts` | ❌ missing |
| `FORK_SUBAGENT` | `/fork` | `src/commands/fork/index.ts` | ❌ missing |
| `BUDDY` | `/buddy` | `src/commands/buddy/index.ts` | ❌ missing |
| `MCP_SKILLS` | (no command — gates `getMcpSkillCommands`) | n/a | n/a |

Out of 17 flag/command sites, **6 sources are present** in this leak; the rest are documented at registry-citation level.

### IN scope
- All 17 entries above. Sourced ones get full §3/§5/§6 treatment; sourceless ones get registry citations only.

### OUT of scope
- Mode runtime mechanics (PROACTIVE/KAIROS/BRIDGE_MODE/DAEMON/VOICE_MODE/UDS_INBOX) → specs 31..36.
- Per-flag DCE mechanism → 00 Appendix A.

---

## 2. Source Map

| Path | Lines | Read |
|---|---|---|
| `src/commands.ts:60-122` | 63 | ✅ (read in 20-command-system) |
| `src/commands/brief.ts` | 131 | ✅ |
| `src/commands/bridge/index.ts` | 25 | ✅ |
| `src/commands/voice/index.ts` | 18 | ✅ |
| `src/commands/remote-setup/index.ts` | 21 | ✅ |
| `src/commands/ultraplan.tsx` | unknown — compiled | ❌ (>53K tokens; not read fully) |

`bridge/bridge.tsx`, `voice/voice.ts`, `remote-setup/remote-setup.tsx`, `remote-setup/api.ts` are sub-implementation files — not read here. Documented at registry/index level.

---

## 3. Public Interface (Contract)

### 3.1 `/proactive` — Proactive mode (PROACTIVE ∨ KAIROS)

- Path: `src/commands/proactive.ts` (source missing)
- Registration: `commands.ts:62-65` — `feature('PROACTIVE') || feature('KAIROS') ? require('./commands/proactive.js').default : null`
- Spread at `commands.ts:323` — `...(proactive ? [proactive] : [])`
- Documentation: defer to spec 31 (mode-proactive) for runtime semantics. Command surface: registered at top-level (NOT in `INTERNAL_ONLY_COMMANDS`); appears for all users when either flag is on.

### 3.2 `/brief` — Toggle brief-only mode (KAIROS ∨ KAIROS_BRIEF)

- Path: `src/commands/brief.ts:47-128`
- Kind: `local-jsx`
- Description: `Toggle brief-only mode`
- `immediate: true`
- `isEnabled` (verbatim, brief.ts:51-56):
  ```typescript
  isEnabled: () => {
    if (feature('KAIROS') || feature('KAIROS_BRIEF')) {
      return getBriefConfig().enable_slash_command
    }
    return false
  }
  ```
- `getBriefConfig` reads `tengu_kairos_brief_config` GrowthBook value with Zod parse (brief.ts:38-45). Default: `{ enable_slash_command: false }`. Schema: `z.object({ enable_slash_command: z.boolean() })`.
- Behavior (verbatim from brief.ts:60-126):
  - Toggles `context.getAppState().isBriefOnly`.
  - On enable, checks `isBriefEntitled()` (`tools/BriefTool/BriefTool.js`); if false, emits `tengu_brief_mode_toggled` with `{enabled: false, gated: true, source: 'slash_command'}` and returns `'Brief tool is not enabled for your account'` with `display: 'system'`.
  - On allowed toggle: `setUserMsgOptIn(newState)`, `context.setAppState(prev => ({...prev, isBriefOnly: newState}))`, emits `tengu_brief_mode_toggled` with `{enabled: newState, gated: false, source: 'slash_command'}`.
  - Injects metaMessage (verbatim, brief.ts:111-119) — skipped when `getKairosActive()`:
    ```
    <system-reminder>
    Brief mode is now enabled. Use the ${BRIEF_TOOL_NAME} tool for all user-facing output — plain text outside it is hidden from the user's view.
    </system-reminder>
    ```
    or for disable:
    ```
    <system-reminder>
    Brief mode is now disabled. The ${BRIEF_TOOL_NAME} tool is no longer available — reply with plain text.
    </system-reminder>
    ```
  - Final user message: `'Brief-only mode enabled'` or `'Brief-only mode disabled'`, `display: 'system'`, with metaMessages.
- Side effects: app state mutation, GB cache observation, analytics, optional system-reminder metaMessage injection.

### 3.3 `/assistant` — KAIROS-only

- Path: `src/commands/assistant/index.ts` (source missing)
- Registration: `commands.ts:70-72` — `feature('KAIROS') ? require('./commands/assistant/index.js').default : null`
- Spread at `commands.ts:325`. Defer to spec 32 (mode-kairos).

### 3.4 `/remote-control`, `/rc` — BRIDGE_MODE

- Path: `src/commands/bridge/index.ts:11-25`
- Registration: `commands.ts:73-75` — `feature('BRIDGE_MODE') ? require('./commands/bridge/index.js').default : null`
- Spread at `commands.ts:326`.
- Kind: `local-jsx`
- name: `'remote-control'`
- aliases: `['rc']`
- argumentHint: `[name]`
- `immediate: true`
- `isEnabled` (verbatim, bridge/index.ts:4-9):
  ```typescript
  function isEnabled(): boolean {
    if (!feature('BRIDGE_MODE')) {
      return false
    }
    return isBridgeEnabled()
  }
  ```
- `isHidden` mirrors via getter.
- Description: `Connect this terminal for remote-control sessions`
- Implementation `bridge/bridge.tsx` — Ink dialog (not read).
- Defer to spec 34 (mode-bridge) for `isBridgeEnabled` and the runtime mechanics.

### 3.5 `/remoteControlServer` — DAEMON ∧ BRIDGE_MODE

- Path: `src/commands/remoteControlServer/index.ts` (source missing)
- Registration: `commands.ts:76-79` — `feature('DAEMON') && feature('BRIDGE_MODE') ? require('./commands/remoteControlServer/index.js').default : null`
- Spread at `commands.ts:327`.
- Defer to spec 33 (mode-daemon) and 34 (mode-bridge).

### 3.6 `/voice` — VOICE_MODE

- Path: `src/commands/voice/index.ts:5-17`
- Registration: `commands.ts:80-82`. Spread at `commands.ts:328`.
- Kind: `local`
- availability: `['claude-ai']`
- `isEnabled: () => isVoiceGrowthBookEnabled()`
- `isHidden`: `!isVoiceModeEnabled()` (computed via getter)
- `supportsNonInteractive: false`
- Description: `Toggle voice mode`
- Implementation `voice/voice.ts` (not read). Defer to spec 36 (mode-voice).

### 3.7 `/force-snip` — HISTORY_SNIP (ANT-only INTERNAL spread)

- Path: `src/commands/force-snip.ts` (source missing)
- Registration: `commands.ts:83-85`; spread inside `INTERNAL_ONLY_COMMANDS` at `commands.ts:235` — only present when ANT && !IS_DEMO && HISTORY_SNIP.
- Defer to spec 07 (compaction) for snip semantics.

### 3.8 `/workflows` — WORKFLOW_SCRIPTS

- Path: `src/commands/workflows/index.ts` (source missing)
- Registration: `commands.ts:86-90`. Spread at `commands.ts:341`.
- Note: `WORKFLOW_SCRIPTS` ALSO conditionally enables `getWorkflowCommands` at `commands.ts:401-405`, which loads dynamic workflow commands from disk via `tools/WorkflowTool/createWorkflowCommand.js` (spec 19). The `/workflows` command itself is the management UI; the dynamic commands are user-defined.

### 3.9 `/web-setup` — CCR_REMOTE_SETUP

- Path: `src/commands/remote-setup/index.ts:5-21`
- Registration: `commands.ts:91-95`. Spread at `commands.ts:320`.
- Kind: `local-jsx`
- name: `'web-setup'`
- availability: `['claude-ai']`
- `isEnabled` (verbatim, remote-setup/index.ts:9-12):
  ```typescript
  isEnabled: () =>
    getFeatureValue_CACHED_MAY_BE_STALE('tengu_cobalt_lantern', false) &&
    isPolicyAllowed('allow_remote_sessions'),
  ```
- `isHidden`: `!isPolicyAllowed('allow_remote_sessions')` (getter).
- Description (verbatim): `Setup Claude Code on the web (requires connecting your GitHub account)`
- Sub-files: `remote-setup/api.ts`, `remote-setup/remote-setup.tsx`. Defer to spec 35 (mode-remote-server).

### 3.10 `/subscribe-pr` — KAIROS_GITHUB_WEBHOOKS (ANT-only INTERNAL spread)

- Path: `src/commands/subscribe-pr.ts` (source missing)
- Registration: `commands.ts:101-103`; spread inside `INTERNAL_ONLY_COMMANDS` at `commands.ts:240`.
- Defer to spec 32 (mode-kairos) for KAIROS GitHub webhooks integration.

### 3.11 `/ultraplan` — ULTRAPLAN (ANT-only INTERNAL spread)

- Path: `src/commands/ultraplan.tsx` (compiled file present, not read in detail due to size)
- Registration: `commands.ts:104-106`; spread at `commands.ts:239`.
- Surface defer to detail; the compiled file is too large for full inline. Registry citation only at this level.
- Deferred to a follow-up spec (potentially 21d) if reverse-spec discipline demands the full corpus.

### 3.12 `/torch` — TORCH

- Path: `src/commands/torch.ts` (source missing)
- Registration: `commands.ts:107`. Spread at `commands.ts:342`. Note: this is at top-level (NOT inside INTERNAL_ONLY_COMMANDS), so when TORCH is on it's visible to non-ANT users too.

### 3.13 `/peers` — UDS_INBOX

- Path: `src/commands/peers/index.ts` (source missing)
- Registration: `commands.ts:108-112`. Spread at `commands.ts:339`.

### 3.14 `/fork` — FORK_SUBAGENT

- Path: `src/commands/fork/index.ts` (source missing)
- Registration: `commands.ts:113-117`. Spread at `commands.ts:321`.
- When this command exists, `/branch` removes its `'fork'` alias (`branch/index.ts:8`).

### 3.15 `/buddy` — BUDDY

- Path: `src/commands/buddy/index.ts` (source missing)
- Registration: `commands.ts:118-122`. Spread at `commands.ts:322`.

### 3.16 `EXPERIMENTAL_SKILL_SEARCH` (no command, gate-only)

- `commands.ts:96-100` — gates the `clearSkillIndexCache` import from `services/skillSearch/localSearch.js`.
- Effect: when on, `clearCommandMemoizationCaches()` (`commands.ts:531`) calls the cache clear.
- Documented for completeness; no `/foo` surface.

### 3.17 `MCP_SKILLS` (no command, gate-only)

- `commands.ts:550` — gates `getMcpSkillCommands(mcpCommands)` to filter MCP-provided prompt commands as model-invocable skills. When off, returns `[]`.
- No `/foo` surface.

---

## 4. Data Model & State

For sourced commands:
- `/brief` mutates `context.getAppState().isBriefOnly` and calls `setUserMsgOptIn(boolean)`. Reads from GrowthBook (`tengu_kairos_brief_config` for visibility, plus `isBriefEntitled()` and `getKairosActive()` from session bootstrap state).

Sourceless commands: unknown.

---

## 5. Algorithm / Control Flow

For each gated command:
1. At module load (DCE-time): `feature(FLAG)` evaluated; require either resolves or is stripped.
2. At `COMMANDS()` build: spread `...(cmd ? [cmd] : [])` — when require is stripped, `cmd` is `null` and spread emits nothing.
3. At `getCommands(cwd)` call: `meetsAvailabilityRequirement(cmd) && isCommandEnabled(cmd)` filter — for KAIROS_BRIEF that includes the GrowthBook check; for BRIDGE_MODE the `isBridgeEnabled()` runtime check.

For `/brief` toggle algorithm see §3.2 above.

---

## 6. Verbatim Assets

### 6.1 Feature-gated `require` block (verbatim, `commands.ts:60-122`)

```typescript
import { feature } from 'bun:bundle'
// Dead code elimination: conditional imports
/* eslint-disable @typescript-eslint/no-require-imports */
const proactive =
  feature('PROACTIVE') || feature('KAIROS')
    ? require('./commands/proactive.js').default
    : null
const briefCommand =
  feature('KAIROS') || feature('KAIROS_BRIEF')
    ? require('./commands/brief.js').default
    : null
const assistantCommand = feature('KAIROS')
  ? require('./commands/assistant/index.js').default
  : null
const bridge = feature('BRIDGE_MODE')
  ? require('./commands/bridge/index.js').default
  : null
const remoteControlServerCommand =
  feature('DAEMON') && feature('BRIDGE_MODE')
    ? require('./commands/remoteControlServer/index.js').default
    : null
const voiceCommand = feature('VOICE_MODE')
  ? require('./commands/voice/index.js').default
  : null
const forceSnip = feature('HISTORY_SNIP')
  ? require('./commands/force-snip.js').default
  : null
const workflowsCmd = feature('WORKFLOW_SCRIPTS')
  ? (
      require('./commands/workflows/index.js') as typeof import('./commands/workflows/index.js')
    ).default
  : null
const webCmd = feature('CCR_REMOTE_SETUP')
  ? (
      require('./commands/remote-setup/index.js') as typeof import('./commands/remote-setup/index.js')
    ).default
  : null
const clearSkillIndexCache = feature('EXPERIMENTAL_SKILL_SEARCH')
  ? (
      require('./services/skillSearch/localSearch.js') as typeof import('./services/skillSearch/localSearch.js')
    ).clearSkillIndexCache
  : null
const subscribePr = feature('KAIROS_GITHUB_WEBHOOKS')
  ? require('./commands/subscribe-pr.js').default
  : null
const ultraplan = feature('ULTRAPLAN')
  ? require('./commands/ultraplan.js').default
  : null
const torch = feature('TORCH') ? require('./commands/torch.js').default : null
const peersCmd = feature('UDS_INBOX')
  ? (
      require('./commands/peers/index.js') as typeof import('./commands/peers/index.js')
    ).default
  : null
const forkCmd = feature('FORK_SUBAGENT')
  ? (
      require('./commands/fork/index.js') as typeof import('./commands/fork/index.js')
    ).default
  : null
const buddy = feature('BUDDY')
  ? (
      require('./commands/buddy/index.js') as typeof import('./commands/buddy/index.js')
    ).default
  : null
/* eslint-enable @typescript-eslint/no-require-imports */
```

### 6.2 `getWorkflowCommands` gate (verbatim, `commands.ts:400-406`)

```typescript
/* eslint-disable @typescript-eslint/no-require-imports */
const getWorkflowCommands = feature('WORKFLOW_SCRIPTS')
  ? (
      require('./tools/WorkflowTool/createWorkflowCommand.js') as typeof import('./tools/WorkflowTool/createWorkflowCommand.js')
    ).getWorkflowCommands
  : null
/* eslint-enable @typescript-eslint/no-require-imports */
```

### 6.3 `/brief` registration (verbatim, `brief.ts:47-128`)

(Reproduced inline in §3.2.)

### 6.4 `/remote-control` (verbatim, `bridge/index.ts`)

```typescript
import { feature } from 'bun:bundle'
import { isBridgeEnabled } from '../../bridge/bridgeEnabled.js'
import type { Command } from '../../commands.js'

function isEnabled(): boolean {
  if (!feature('BRIDGE_MODE')) {
    return false
  }
  return isBridgeEnabled()
}

const bridge = {
  type: 'local-jsx',
  name: 'remote-control',
  aliases: ['rc'],
  description: 'Connect this terminal for remote-control sessions',
  argumentHint: '[name]',
  isEnabled,
  get isHidden() {
    return !isEnabled()
  },
  immediate: true,
  load: () => import('./bridge.js'),
} satisfies Command

export default bridge
```

### 6.5 `/voice` (verbatim, `voice/index.ts`)

```typescript
import type { Command } from '../../commands.js'
import {
  isVoiceGrowthBookEnabled,
  isVoiceModeEnabled,
} from '../../voice/voiceModeEnabled.js'

const voice = {
  type: 'local',
  name: 'voice',
  description: 'Toggle voice mode',
  availability: ['claude-ai'],
  isEnabled: () => isVoiceGrowthBookEnabled(),
  get isHidden() {
    return !isVoiceModeEnabled()
  },
  supportsNonInteractive: false,
  load: () => import('./voice.js'),
} satisfies Command

export default voice
```

### 6.6 `/web-setup` (verbatim, `remote-setup/index.ts`)

```typescript
import type { Command } from '../../commands.js'
import { getFeatureValue_CACHED_MAY_BE_STALE } from '../../services/analytics/growthbook.js'
import { isPolicyAllowed } from '../../services/policyLimits/index.js'

const web = {
  type: 'local-jsx',
  name: 'web-setup',
  description:
    'Setup Claude Code on the web (requires connecting your GitHub account)',
  availability: ['claude-ai'],
  isEnabled: () =>
    getFeatureValue_CACHED_MAY_BE_STALE('tengu_cobalt_lantern', false) &&
    isPolicyAllowed('allow_remote_sessions'),
  get isHidden() {
    return !isPolicyAllowed('allow_remote_sessions')
  },
  load: () => import('./remote-setup.js'),
} satisfies Command

export default web
```

### 6.7 `/brief` Zod schema (verbatim, `brief.ts:22-31`)

```typescript
const briefConfigSchema = lazySchema(() =>
  z.object({
    enable_slash_command: z.boolean(),
  }),
)
type BriefConfig = z.infer<ReturnType<typeof briefConfigSchema>>

const DEFAULT_BRIEF_CONFIG: BriefConfig = {
  enable_slash_command: false,
}
```

### 6.8 `/brief` system-reminder strings (verbatim, `brief.ts:114-118`)

```
Brief mode is now enabled. Use the ${BRIEF_TOOL_NAME} tool for all user-facing output — plain text outside it is hidden from the user's view.
```

```
Brief mode is now disabled. The ${BRIEF_TOOL_NAME} tool is no longer available — reply with plain text.
```

---

## 7. Side Effects & I/O

| Command | Side effect |
|---|---|
| `/brief` | App state mutation (`isBriefOnly`); `setUserMsgOptIn`; analytics; conditional metaMessage injection |
| `/remote-control` | Renders Ink dialog; defers to spec 34 |
| `/voice` | Toggles voice mode (spec 36) |
| `/web-setup` | Connects GitHub account; spec 35 |
| Sourceless | Unknown |

---

## 8. Feature Flags & Variants

(See §1 table.) DCE strips both the `require()` and the spread for inactive flags. The on-state of each flag changes the count of universally-visible commands by exactly 1 (or 2 for DAEMON+BRIDGE_MODE which adds `/remoteControlServer` AND requires BRIDGE_MODE which adds `/remote-control`).

---

## 9. Error Handling & Edge Cases

- `/brief` Zod parse failure → falls back to `DEFAULT_BRIEF_CONFIG = { enable_slash_command: false }`. Brief is silently hidden, not crashed.
- `/brief` entitlement check (`isBriefEntitled()`) only gates the on-transition; off is always allowed (so a user whose GB gate flipped mid-session isn't stuck).
- `/remote-control` `isEnabled` gates BOTH the build-time DCE AND the runtime `isBridgeEnabled()` check — the latter handles the case where BRIDGE_MODE is built in but disabled at runtime.

---

## 10. Telemetry & Observability

- `/brief` emits `tengu_brief_mode_toggled` with `{enabled, gated, source: 'slash_command' as AnalyticsMetadata_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS}`.
- Other commands' telemetry: deferred to mode specs.

---

## 11. Reimplementation Checklist

- [ ] All feature-gated commands MUST use `require(...)` (not `import`) inside the ternary so Bun's DCE can strip them. Top-level `import` would prevent stripping.
- [ ] `/brief` `isEnabled` gate hierarchy: `feature('KAIROS') || feature('KAIROS_BRIEF')` AND `getBriefConfig().enable_slash_command`. Both must hold.
- [ ] `/brief` toggles `setUserMsgOptIn(boolean)` to keep the BriefTool tool list in sync — this invalidates the prompt cache; document the trade-off.
- [ ] `/brief` skips metaMessage injection when `getKairosActive()` is true; reuses the system-reminder wrap inline (NOT via `wrapInSystemReminder` from `utils/messages.ts` — that pulls `constants/xml.ts` into the bridge SDK bundle).
- [ ] `/remote-control` checks BOTH build-time `feature('BRIDGE_MODE')` AND runtime `isBridgeEnabled()`.
- [ ] `/branch`'s alias list checks `feature('FORK_SUBAGENT')` to drop `'fork'` when the standalone command is built in (cross-flag reference).
- [ ] `/web-setup` requires BOTH GrowthBook gate (`tengu_cobalt_lantern`) AND policy gate (`allow_remote_sessions`). Both at `isEnabled` and `isHidden`.
- [ ] `EXPERIMENTAL_SKILL_SEARCH` ungated: `clearSkillIndexCache` is `null` and the `?.()` invocation is a no-op — preserve the optional-chain.
- [ ] `MCP_SKILLS` ungated: `getMcpSkillCommands` returns `[]` — preserve the explicit empty return.
- [ ] `/torch` is registered at the top level (not inside `INTERNAL_ONLY_COMMANDS`); when TORCH flag is on, all users see it, NOT just ANTs.
- [ ] `/force-snip`, `/subscribe-pr`, `/ultraplan` are spread inside `INTERNAL_ONLY_COMMANDS` — they require BOTH the flag AND `USER_TYPE === 'ant' && !IS_DEMO`.

---

## 12. Open Questions / Unknowns

1. **Sourceless commands** (`proactive`, `assistant`, `remoteControlServer`, `force-snip`, `workflows`, `subscribe-pr`, `torch`, `peers`, `fork`, `buddy`) — none reverse-engineerable from the leak. Reimplementer needs alternative source.
2. **`ultraplan.tsx`** — large compiled file; not inlined here. Could be the basis of a future 21d if needed.
3. **`isBridgeEnabled` semantics** — defer to spec 34 (mode-bridge).
4. **`isVoiceGrowthBookEnabled` vs `isVoiceModeEnabled`** — split between enablement and visibility; spec 36 to enumerate.
5. **`tengu_cobalt_lantern`** — GrowthBook flag name for `/web-setup` enablement; spec 26 to enumerate the full flag set.
6. **`getKairosActive`** — bootstrap state read in `/brief`; spec 32 to enumerate.
7. **`BRIEF_TOOL_NAME`** — imported from `tools/BriefTool/prompt.js`; spec 19 to enumerate the tool surface.
8. **`isBriefEntitled`** — entitlement check; likely tied to subscription tier or org policy. Spec 27 (policy).
9. **Comments mention removed `wrapInSystemReminder` import** (brief.ts:108-110): the inline wrap is intentional to avoid pulling `constants/xml.ts` into the bridge SDK bundle — spec 34 to confirm bridge-bundle exclusion list.
