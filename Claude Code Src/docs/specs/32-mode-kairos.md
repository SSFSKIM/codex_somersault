# 32 — Mode: Kairos (KAIROS family flags) Specification

> **Mode-delta spec.** Documents what activating the `KAIROS*` and `AGENT_TRIGGERS*` family of `feature()` flags changes in Claude Code: the assistant-mode REPL latch, Brief tool surface, channel inbound/outbound MCP transport, scheduled-trigger system, GitHub-webhook PR subscriptions, push notifications, dream-cycle skill, and the system-prompt addendums layered on top of the default loop. Tool surfaces and slash-command surfaces are documented in 19/21; this file documents *callsites and behavioral deltas* only.
> Adjacent specs: **19**, **21**, **23**, **26**, **30**, **31**, **35**.

---

## 1. Purpose & Scope

### 1.1 What "Kairos mode" is

Kairos is the **scheduled-trigger / autonomous-assistant** mode of Claude Code. Its build-time gate is the `KAIROS` flag plus seven independently-shippable sub-flags. When the build includes them and the runtime gate (`tengu_kairos`, `tengu_kairos_brief`, `tengu_harbor`, `tengu_kairos_cron`, …) is on, the CLI gains:

1. **Assistant mode latch** — `claude assistant [sessionId]` subcommand + `--assistant` daemon flag, latching `kairosActive=true` and forcing `--brief on`. (`src/main.tsx:80-81,1048-1089,3259-3341,4334-4340`)
2. **Brief output channel** — the `SendUserMessage` (legacy `Brief`) tool replaces plain text as the user-visible output surface. (`src/tools/BriefTool/BriefTool.ts:1-204`)
3. **Channels** — bidirectional MCP-over-channel surface (Telegram/iMessage/Discord/Slack-style servers) for inbound notifications and permission relay. (`src/services/mcp/channelNotification.ts:1-317`, `src/services/mcp/channelPermissions.ts:1-241`, `src/services/mcp/channelAllowlist.ts:1-78`)
4. **Cron triggers** — `CronCreate` / `CronDelete` / `CronList` tools + the `/loop` skill, persisted to `.claude/scheduled_tasks.json`. (`src/utils/cronTasks.ts:1-459`, `src/tools/ScheduleCronTool/prompt.ts:1-136`)
5. **PR subscription** — `SubscribePR` tool + `/subscribe-pr` command on the `KAIROS_GITHUB_WEBHOOKS` flag. (`src/tools.ts:50-52`, `src/commands.ts:101-103`)
6. **Push notifications** — `PushNotificationTool` on the `KAIROS_PUSH_NOTIFICATION` flag. (`src/tools.ts:45-49`)
7. **Send-user-file** — `SendUserFileTool` on the `KAIROS` flag (file delivery to the user's channel). (`src/tools.ts:42-44`)
8. **Dream skill** — `dream` bundled skill on the `KAIROS_DREAM` flag (also auto-enabled under bare `KAIROS`). (`src/skills/bundled/index.ts:35-40`)
9. **Remote triggers** — `RemoteTriggerTool` on the `AGENT_TRIGGERS_REMOTE` flag (separately shippable from local cron). (`src/tools.ts:36-38`)

### 1.2 In scope

- All KAIROS-family flag callsites and behavioral deltas (broad mode activation, Brief surface delta, webhook intake + SubscribePR, PushNotificationTool delta, channel UI surface, Dream skill loading, cron tools registration, RemoteTriggerTool)
- `src/assistant/` directory (only `sessionHistory.ts` is present — the bulk of the assistant engine is missing-source; see §2.4)
- Channel state machine: parsing of `--channels` / `--dangerously-load-development-channels` CLI args, the `gateChannelServer` decision tree, the inbound `notifications/claude/channel` notification handler, and the outbound permission-relay protocol
- Cron triggers wiring: `isKairosCronEnabled()` runtime gate, jitter config, scheduled-trigger fire flow, fire-then-delete (one-shot) vs. recompute-and-stamp (recurring) semantics, missed-task surfacing
- GitHub webhooks intake + PR subscription state machine — registry-level only (source absent)
- Push notifications transport — registry-level only (source absent)
- Brief generation prompt + flow (verbatim where present)
- Channels: Telegram/iMessage adapters — registry-level only (source absent in leak; channels are MCP servers per `channelNotification.ts:2-7`)
- Kairos system prompt delta vs default
- Cross-cite to spec 17 for `KAIROS_DREAM` skill loading
- Cross-cite to spec 34 (replbridge): assistant-mode latch ALSO sets `workerType='claude_code_assistant'` on the bridge worker (`src/bridge/initReplBridge.ts:476-484`); union `BridgeWorkerType` at `src/bridge/types.ts:79`

### 1.3 Out of scope

| Concern | Spec |
|---|---|
| Brief / PushNotification / SubscribePR / RemoteTrigger / Cron / SendUserFile *tool surfaces* (input schema, render, permissions, prompts) | 19 |
| `/brief`, `/subscribe-pr`, `/proactive`, `/assistant` *slash-command surfaces* | 21 |
| Coordinator (multi-agent spawn) | 30 |
| `PROACTIVE` flag (auto-wakeup, SleepTool) — Kairos shares `SleepTool` import gate but the proactive subsystem is its own mode | 31 |
| Remote / CCR (`CLAUDE_CODE_REMOTE`, `claude remote-setup`) | 35 |
| Skills — generic loading | 17 |
| MCP transport plumbing (stdio/SSE/HTTP/WebSocket, capabilities negotiation) | 23 |

### 1.4 Source-coverage inventory

Per-flag present/missing column for the seven KAIROS-family flags + the two AGENT_TRIGGERS* flags:

| Flag | Primary owned source | Status |
|---|---|---|
| `KAIROS` (umbrella) | `src/main.tsx:78-81,559,685,1034-1089,1642,1728,2184-2206,2518,2640-2648,2915-3035,3259-3341,3832-3844,4334-4340,4612,4623`; `src/commands.ts:62-72,101-103`; `src/tools.ts:25-52`; `src/assistant/sessionHistory.ts` | **partial** — assistant engine missing except `sessionHistory.ts` |
| `KAIROS_BRIEF` | `src/tools/BriefTool/BriefTool.ts`; `src/tools/BriefTool/prompt.ts`; `src/commands/brief.ts`; `src/constants/prompts.ts:73-83,552,844-913` | **present** (BriefTool fully present) |
| `KAIROS_GITHUB_WEBHOOKS` | `src/tools.ts:50-52` (`SubscribePRTool` import); `src/commands.ts:101-103` (`subscribePr`) | **missing-source** — `src/tools/SubscribePRTool/`, `src/commands/subscribe-pr.ts` absent |
| `KAIROS_PUSH_NOTIFICATION` | `src/tools.ts:45-49`; `src/tools/ConfigTool/supportedSettings.ts:164`; `src/components/Settings/Config.tsx:658,672` | **missing-source** — `src/tools/PushNotificationTool/` absent |
| `KAIROS_CHANNELS` | `src/main.tsx:1642-…`; `src/interactiveHelpers.tsx:241`; `src/services/mcp/channelNotification.ts`; `src/services/mcp/channelPermissions.ts`; `src/services/mcp/channelAllowlist.ts`; `src/components/LogoV2/ChannelsNotice.tsx`; `src/tools/AskUserQuestionTool/AskUserQuestionTool.tsx:141`; `src/tools/EnterPlanModeTool/EnterPlanModeTool.ts:61`; `src/tools/ExitPlanModeTool/ExitPlanModeV2Tool.ts:172`; `src/utils/messageQueueManager.ts:370`; `src/utils/messages.ts:4669`; `src/cli/print.ts:1673,4674,4789`; `src/hooks/toolPermission/handlers/interactiveHandler.ts`; `src/services/mcp/useManageMCPConnections.ts` | **present** (channel infra; concrete adapter MCP servers are external plugins) |
| `KAIROS_DREAM` | `src/skills/bundled/index.ts:35-40` (`require('./dream.js')`) | **missing-source** — `src/skills/bundled/dream.ts` absent |
| `AGENT_TRIGGERS` | `src/tools.ts:29-35`; `src/skills/bundled/index.ts:47-55`; `src/skills/bundled/loop.ts`; `src/utils/cronTasks.ts`; `src/utils/cronJitterConfig.ts`; `src/utils/cronScheduler.ts`; `src/tools/ScheduleCronTool/{prompt,CronCreateTool,CronDeleteTool,CronListTool,UI}.ts(x)` | **present** |
| `AGENT_TRIGGERS_REMOTE` | `src/tools.ts:36-38`; `src/skills/bundled/index.ts:56-63`; `src/skills/bundled/scheduleRemoteAgents.ts`; `src/tools/RemoteTriggerTool/{RemoteTriggerTool.ts,UI.tsx,prompt.ts}` | **present** (Phase 9.6c: tool source verified present — was phantom missing-source row) |

---

## 2. Source Map

### 2.1 Owned files (present)

| Path | Role |
|---|---|
| `src/assistant/sessionHistory.ts` | Paginated event-fetch over `${BASE_API_URL}/v1/sessions/{sid}/events`, used when a local REPL re-attaches to a remote bridge session via `claude assistant [sessionId]` |
| `src/services/mcp/channelNotification.ts` | Inbound channel transport: `ChannelMessageNotificationSchema`, `wrapChannelMessage`, `gateChannelServer`, `findChannelEntry`, `getEffectiveChannelAllowlist`, outbound permission-request method `notifications/claude/channel/permission_request` |
| `src/services/mcp/channelPermissions.ts` | Outbound permission relay: `PERMISSION_REPLY_RE`, `shortRequestId`, `truncateForPreview`, `filterPermissionRelayClients`, `createChannelPermissionCallbacks` |
| `src/services/mcp/channelAllowlist.ts` | Channel allowlist: `tengu_harbor` (overall on/off), `tengu_harbor_ledger` (approved plugins), `getChannelAllowlist`, `isChannelAllowlisted`, `isChannelsEnabled` |
| `src/components/LogoV2/ChannelsNotice.tsx` | Boot-time channels-status notice (disabled / no-auth / policy-blocked / listening) |
| `src/tools/BriefTool/BriefTool.ts` | `SendUserMessage` (`Brief`) tool — entitlement and activation gates |
| `src/tools/BriefTool/prompt.ts` | `BRIEF_TOOL_NAME`, `LEGACY_BRIEF_TOOL_NAME`, `BRIEF_TOOL_PROMPT`, `BRIEF_PROACTIVE_SECTION` |
| `src/commands/brief.ts` | `/brief` slash command — toggles `isBriefOnly` and re-emits a system-reminder |
| `src/services/autoDream/autoDream.ts` | Auto-dream forked-agent runner — note this is the *non-Kairos* dream gate (`getKairosActive()` returns false in `isGateOpen`); KAIROS uses the `KAIROS_DREAM` bundled skill (missing-source) instead |
| `src/services/autoDream/config.ts` | `isAutoDreamEnabled` (`tengu_onyx_plover` GB) |
| `src/skills/bundled/index.ts` | `KAIROS_DREAM` and `AGENT_TRIGGERS` flag-gated `require()` of skill registrars |
| `src/skills/bundled/loop.ts` | `/loop` skill (gated via `isKairosCronEnabled()` per `bundled/index.ts:51`) |
| `src/utils/cronTasks.ts` | Persistence + scheduling primitives: `CronTask` type, `.claude/scheduled_tasks.json` shape, `addCronTask` / `removeCronTasks` / `markCronTasksFired`, `nextCronRunMs`, `jitteredNextCronRunMs`, `oneShotJitteredNextCronRunMs`, `findMissedTasks`, `DEFAULT_CRON_JITTER_CONFIG` |
| `src/utils/cronJitterConfig.ts` | GrowthBook-driven jitter config refresh (`tengu_kairos_cron_config`) |
| `src/utils/cronScheduler.ts` | Tick loop, per-process lock, kill-switch poll |
| `src/tools/ScheduleCronTool/prompt.ts` | `isKairosCronEnabled()`, `isDurableCronEnabled()`, tool names, prompt builders |
| `src/tools/ScheduleCronTool/{CronCreateTool,CronDeleteTool,CronListTool}.ts` | Tool surfaces (covered by spec 19) |
| `src/bootstrap/state.ts:213,1085-1090,1294-1308,1676-1684` | `kairosActive` latch, `userMsgOptIn` (Brief), `allowedChannels`, `hasDevChannels`, `SessionCronTask` storage |
| `src/constants/prompts.ts:73-83,552,844-913` | `BRIEF_PROACTIVE_SECTION`, `getBriefSection`, brief-only-mode prompt addendum, proactive system prompt section |

### 2.2 Conditional registry sites

- `src/main.tsx:80-81` — top-level conditional `require()` of `./assistant/index.js` (engine — missing-source) and `./assistant/gate.js` (gate — missing-source) under `feature('KAIROS')`
- `src/main.tsx:559,685` — `_pendingAssistantChat` slot for a pending session-attach
- `src/main.tsx:1034-1089` — assistant-mode activation: `--assistant` daemon-flag handling; `isAssistantMode()`/`isAssistantForced()` resolution; trust-dialog precondition; force-`--brief on`; `setKairosActive(true)`; `initializeAssistantTeam()`
- `src/main.tsx:1642-…` — `--channels` / `--dangerously-load-development-channels` parsing
- `src/main.tsx:1728` — Brief tool injection into `--tools` listing
- `src/main.tsx:2184-2201` — first-run defaultView prompt + brief visibility addendum
- `src/main.tsx:2206-2208` — `appendSystemPrompt += assistantModule.getAssistantSystemPromptAddendum()`
- `src/main.tsx:2518` — `assistantActivationPath` analytics tag
- `src/main.tsx:2640-2648,2915-2916,2962,3035` — kairosEnabled propagation through REPL state, `fullRemoteControl`, `teamContext`
- `src/main.tsx:3259-3341` — `claude assistant [sessionId]` viewer attachment (uses `./assistant/sessionDiscovery.js` — missing-source)
- `src/main.tsx:3832-3844` — Commander option registration: `--brief`, `--assistant`, `--channels`, `--dangerously-load-development-channels`
- `src/main.tsx:4334-4340` — `program.command('assistant [sessionId]')` subcommand stub
- `src/main.tsx:4612-4623` — proactive/kairos chat-mode handoff and brief auto-activation
- `src/commands.ts:62-72,101-103` — conditional `proactive` (`PROACTIVE || KAIROS`), `briefCommand` (`KAIROS || KAIROS_BRIEF`), `assistantCommand` (`KAIROS`), `subscribePr` (`KAIROS_GITHUB_WEBHOOKS`)
- `src/commands.ts:324-325` — `briefCommand` push (`:324`) and `assistantCommand` push (`:325`) into the registry array
- `src/tools.ts:25-52` — conditional `SleepTool` (`PROACTIVE || KAIROS`), `cronTools` array (`AGENT_TRIGGERS`), `RemoteTriggerTool` (`AGENT_TRIGGERS_REMOTE`), `SendUserFileTool` (`KAIROS`), `PushNotificationTool` (`KAIROS || KAIROS_PUSH_NOTIFICATION`), `SubscribePRTool` (`KAIROS_GITHUB_WEBHOOKS`)
- `src/skills/bundled/index.ts:35-63` — `KAIROS || KAIROS_DREAM` → `registerDreamSkill()`; `AGENT_TRIGGERS` → `registerLoopSkill()`; `AGENT_TRIGGERS_REMOTE` → `registerScheduleRemoteAgentsSkill()`
- `src/interactiveHelpers.tsx:241` — channel-aware interactive helper branch under `KAIROS || KAIROS_CHANNELS`
- `src/tools/BriefTool/BriefTool.ts:91-133` — `isBriefEntitled()` / `isBriefEnabled()` build-time DCE-load-bearing positive-ternary

### 2.3 Imports from

- `bun:bundle` (`feature` everywhere)
- `src/services/analytics/growthbook.js` (`getFeatureValue_CACHED_*` for `tengu_kairos*`, `tengu_harbor*`, `tengu_onyx_plover`)
- `src/bootstrap/state.js` (`getKairosActive`, `setKairosActive`, `getUserMsgOptIn`, `setUserMsgOptIn`, `getAllowedChannels`, `setAllowedChannels`, `getHasDevChannels`, `getSessionCronTasks`, `addSessionCronTask`, `removeSessionCronTasks`, `getProjectRoot`, `getOriginalCwd`, `getIsRemoteMode`, `getSessionId`)
- `@modelcontextprotocol/sdk/types.js` (server capabilities introspection)
- `zod/v4` (channel and config schemas)
- `axios` (in `assistant/sessionHistory.ts` for the events API)
- `src/utils/teleport/api.js` (`getOAuthHeaders`, `prepareApiRequest`)
- `src/constants/oauth.js` (`getOauthConfig` → `BASE_API_URL`)

### 2.4 Imported by / downstream surfaces

- `QueryEngine.ts` and `query.ts` (turn pipeline) — through Brief deferral, channel content tags
- `tools/AskUserQuestionTool/AskUserQuestionTool.tsx:141`, `tools/EnterPlanModeTool/EnterPlanModeTool.ts:61`, `tools/ExitPlanModeTool/ExitPlanModeV2Tool.ts:172` — channel-aware delivery branches
- `utils/messages.ts:4669`, `utils/messageQueueManager.ts:370`, `cli/print.ts:1673,4674,4789` — message-formatting branches under channel mode
- `screens/REPL.tsx`, `state/AppStateStore.ts` — `kairosActive` state read

### 2.5 Missing-source ledger

Recorded here, also enumerated in §12 below:

| Symbol | Citation | Gate |
|---|---|---|
| `src/assistant/index.js` (`isAssistantMode`, `isAssistantForced`, `markAssistantForced`, `getAssistantActivationPath`, `getAssistantSystemPromptAddendum`, `initializeAssistantTeam`) | `main.tsx:80,1056,1058,1075,1086,2206-2208,2518` | `feature('KAIROS')` |
| `src/assistant/gate.js` (`isKairosEnabled`) | `main.tsx:81,1066,1075` | `feature('KAIROS')` |
| `src/assistant/sessionDiscovery.js` (`buildSessionDiscoveryPrompt` and friends) | `main.tsx:3265-3266` | `feature('KAIROS')` |
| `src/assistant/install.ts` (`writeIfMissing` for assistant-mode permanent crons) | comment in `cronTasks.ts:50-56` | `feature('KAIROS')` |
| `src/commands/assistant/index.ts` (`/assistant`) | `commands.ts:70-72` | `feature('KAIROS')` |
| `src/commands/subscribe-pr.ts` (`/subscribe-pr`) | `commands.ts:101-103` | `feature('KAIROS_GITHUB_WEBHOOKS')` |
| `src/tools/SubscribePRTool/SubscribePRTool.js` | `tools.ts:50-52` | `feature('KAIROS_GITHUB_WEBHOOKS')` |
| `src/tools/PushNotificationTool/PushNotificationTool.js` | `tools.ts:45-49` | `feature('KAIROS') \|\| feature('KAIROS_PUSH_NOTIFICATION')` |
| `src/tools/SendUserFileTool/SendUserFileTool.js` | `tools.ts:42-44` | `feature('KAIROS')` |
| `src/skills/bundled/dream.js` | `skills/bundled/index.ts:36-39` | `feature('KAIROS') \|\| feature('KAIROS_DREAM')` |

---

## 3. Public Interface (Contract)

### 3.1 CLI surface added by KAIROS family

- `claude assistant [sessionId]` (subcommand, `main.tsx:4334-4340`) — REPL attaches as a viewer client to a running bridge session; if `sessionId` is omitted, discovery via API is attempted (uses missing-source `assistant/sessionDiscovery.js`)
- `--assistant` (Option, `main.tsx:3842`, hideHelp) — Force assistant mode (Agent SDK daemon use). Sets `markAssistantForced()` so the GB gate is skipped.
- `--brief` / `--no-brief` (Option, `main.tsx:3838-3839` under `KAIROS || KAIROS_BRIEF`) — Force brief-only on/off
- `--channels` (Option, `main.tsx:3844` under `KAIROS || KAIROS_CHANNELS`) — Comma list of `plugin:name@marketplace` or `server:name` entries that may push channel notifications this session
- `--dangerously-load-development-channels` — Same as `--channels` but bypasses the `tengu_harbor_ledger` allowlist (per-entry `dev: true`)

### 3.2 Programmatic interface (assistant module — missing-source)

```ts
// reconstructed from main.tsx call sites:
isAssistantMode(): boolean                                  // main.tsx:1058
isAssistantForced(): boolean                                // main.tsx:1075
markAssistantForced(): void                                 // main.tsx:1056
getAssistantActivationPath(): string | undefined            // main.tsx:2518
getAssistantSystemPromptAddendum(): string                  // main.tsx:2206-2208
initializeAssistantTeam(): Promise<TeamContext>             // main.tsx:1086,3035
// gate.js:
isKairosEnabled(): Promise<boolean>                         // main.tsx:1075
```

### 3.3 Channel notification protocol (verbatim schemas, `src/services/mcp/channelNotification.ts:37-72,85-95`)

```ts
export const ChannelMessageNotificationSchema = lazySchema(() =>
  z.object({
    method: z.literal('notifications/claude/channel'),
    params: z.object({
      content: z.string(),
      meta: z.record(z.string(), z.string()).optional(),
    }),
  }),
)

export const CHANNEL_PERMISSION_METHOD =
  'notifications/claude/channel/permission'
export const ChannelPermissionNotificationSchema = lazySchema(() =>
  z.object({
    method: z.literal(CHANNEL_PERMISSION_METHOD),
    params: z.object({
      request_id: z.string(),
      behavior: z.enum(['allow', 'deny']),
    }),
  }),
)

export const CHANNEL_PERMISSION_REQUEST_METHOD =
  'notifications/claude/channel/permission_request'
export type ChannelPermissionRequestParams = {
  request_id: string
  tool_name: string
  description: string  // built by interactiveHandler at runtime; see hooks/toolPermission/handlers/interactiveHandler.ts:250,337
  input_preview: string  // JSON-stringified input, <=200 chars + "…"
}
```

Outbound callsite: `src/hooks/toolPermission/handlers/interactiveHandler.ts:345` (`client.notification({ method: CHANNEL_PERMISSION_REQUEST_METHOD, params: { tool_name, description, input_preview } })`); `description` and `input_preview` are computed at `:250,337,338`.

Reply regex (`channelPermissions.ts:75`): `/^\s*(y|yes|n|no)\s+([a-km-z]{5})\s*$/i`

### 3.4 Cron task disk schema (verbatim, `src/utils/cronTasks.ts:30-69, 74-83, 91-140`)

```ts
type CronTask = {
  id: string                      // 8 hex chars from randomUUID().slice(0,8)
  cron: string                    // 5-field cron string, local time
  prompt: string
  createdAt: number               // epoch ms
  lastFiredAt?: number
  recurring?: boolean
  permanent?: boolean             // installer-only; not settable via CronCreateTool
  durable?: boolean               // runtime-only; stripped on write
  agentId?: string                // runtime-only; teammate-scoped tasks
}

const CRON_FILE_REL = join('.claude', 'scheduled_tasks.json')
// File shape on disk: { "tasks": CronTask[] }   (durable + agentId fields stripped)
```

### 3.5 Session-attach events API (`src/assistant/sessionHistory.ts:1-87`)

- `GET ${BASE_API_URL}/v1/sessions/{sessionId}/events`
- Headers: OAuth `Authorization: Bearer <accessToken>` + `anthropic-beta: ccr-byoc-2025-07-29` + `x-organization-uuid: <orgUUID>`
- Page size: `HISTORY_PAGE_SIZE = 100`, `timeout: 15000`, `validateStatus: () => true`
- Pagination cursor: `before_id` (oldest-first within a page); `anchor_to_latest=true` for newest page

---

## 4. Data Model & State

### 4.1 Bootstrap-state additions (`src/bootstrap/state.ts`)

- `kairosActive: boolean` (`:72,301,1085-1090`) — latch set in `main.tsx:1081`. Read by Brief (`isBriefEntitled`/`isBriefEnabled`), AutoDream gate (`autoDream.ts:96`), and the spinner / proactive-prompt gating
- `userMsgOptIn: boolean` (`:1104-1108`) — `--brief`, `defaultView: 'chat'`, `/brief` toggle, `CLAUDE_CODE_BRIEF` env all set this. `kairosActive=true` short-circuits; the tool stays available regardless
- `allowedChannels: ChannelEntry[]` (`:213,1676-1684`) — typed `--channels` parse output
  - `ChannelEntry = { kind: 'plugin', name, marketplace, dev?: boolean } | { kind: 'server', name, dev?: boolean }`
- `hasDevChannels: boolean` (`:1684`) — was `--dangerously-load-development-channels` used
- `SessionCronTask[]` in-memory store (`:1294-1308`) — non-durable cron tasks live here, never written to disk

### 4.2 Channel state machine (Kairos-owned)

```
                     +-------------------------+
                     | MCP server connects     |
                     +-----------+-------------+
                                 |
                                 v
                     +-------------------------+
                     | gateChannelServer()     |   channelNotification.ts:191
                     +-----------+-------------+
                                 |
            +--------------------+----------------------+
            | capability missing → skip 'capability'    |
            | tengu_harbor=false → skip 'disabled'      |
            | no claude.ai OAuth → skip 'auth'          |
            | managed && !channelsEnabled → skip 'policy'|
            | not in --channels → skip 'session'        |
            | plugin marketplace mismatch → skip 'marketplace' |
            | not in tengu_harbor_ledger → skip 'allowlist' |
            +--------------------+----------------------+
                                 |
                                 v action='register'
                +-----------------+----------------+
                | subscribe to                     |
                | notifications/claude/channel     |
                +-----------------+----------------+
                                  |
              inbound notif       v
            +---------------------+----+
            | wrapChannelMessage()     |  channelNotification.ts:106-116
            | <channel source="X" k=v> |
            |   <content>              |
            | </channel>               |
            +---------------------+----+
                                  |
                                  v
                        enqueue user message
                                  |
                                  v
                       SleepTool/REPL polling
```

### 4.3 Permission relay state machine (`channelPermissions.ts:209-240`)

```
PreToolUse hook fires permission dialog
            │
            ▼
shortRequestId(toolUseID) → 5-letter id from a-z\{l}
            │
            ▼
filterPermissionRelayClients(clients, isInAllowlist)  ← all 3 conditions
            │
            ▼
Send notifications/claude/channel/permission_request{request_id, tool_name, description, input_preview}
            │
            ▼
onResponse(requestId, handler) registers in pending: Map<string, handler>
            │
            ▼ first wins (race against local UI / bridge / hooks / classifier)
Server emits notifications/claude/channel/permission{request_id, behavior}
            │
            ▼
resolve(requestId, behavior, fromServer):
  delete BEFORE call (re-entrancy / dup-event safe)
  resolver({behavior, fromServer})
```

### 4.4 Cron task state machine (`utils/cronTasks.ts`, `utils/cronScheduler.ts`)

- **One-shot** (`recurring` falsy): scheduler fires at next match, tool dispatch enqueues prompt, scheduler calls `removeCronTasks([id])`. Backward jitter via `oneShotJitteredNextCronRunMs` only when minute landed on `oneShotMinuteMod` (default 30 → :00 / :30).
- **Recurring** (`recurring: true`): scheduler fires at jittered match (`jitteredNextCronRunMs`); after fire, calls `markCronTasksFired([id], firedAt)`; recomputes `nextFireAt` from `lastFiredAt`. Auto-expires after `recurringMaxAgeMs` (7 days default) unless `permanent: true`.
- **Session-only** (`durable: false`): held in `bootstrap/state.SessionCronTask[]`, stripped on write, dies with the process.
- **Permanent** (`permanent: true`): set only by `src/assistant/install.ts` (missing-source) for built-in assistant tasks (catch-up / morning-checkin / dream); not settable via `CronCreateTool`; exempt from `recurringMaxAgeMs`.
- **Missed-task** (`findMissedTasks`, `:453-458`): a task whose `nextCronRunMs(cron, createdAt) < now` is surfaced at startup.

### 4.5 PR subscription state machine (KAIROS_GITHUB_WEBHOOKS, **missing-source**)

The tool surface and command file are absent. Inferred from registry citations alone:

- `SubscribePRTool` (`tools.ts:50-52`) — model-driven subscription
- `/subscribe-pr` (`commands.ts:101-103`) — user-driven subscription
- The webhook *intake* server is presumed to live behind the same family of bridge / remote endpoints used by `assistant/sessionHistory.ts`; concrete payload schemas, retry policy, and unsubscribe semantics cannot be specified bit-exact from the leaked tree.

### 4.6 Push notifications transport (KAIROS_PUSH_NOTIFICATION, **missing-source**)

`PushNotificationTool` source absent. Indirect hints:

- Settings additions under `KAIROS || KAIROS_PUSH_NOTIFICATION`: `src/tools/ConfigTool/supportedSettings.ts:164` and `src/components/Settings/Config.tsx:658,672` (label `'Local notifications'` vs. plain `'Notifications'`).
- The Brief tool's `status: 'proactive'` enum (`BriefTool.ts:31-35`) is the upstream signal that classifies a message as eligible for push delivery; downstream routing is not in the leaked source.

---

## 5. Algorithm / Control Flow

### 5.1 Boot-time activation (`main.tsx:1034-1089`)

```
1. parseArgs() → options
2. if feature('KAIROS') && options.assistant && assistantModule:
       assistantModule.markAssistantForced()
3. if feature('KAIROS') && assistantModule.isAssistantMode() && !options.agentId && kairosGate:
       a. checkHasTrustDialogAccepted()
          → false: log "Assistant mode disabled: directory is not trusted." and stop
       b. kairosEnabled = isAssistantForced() || await kairosGate.isKairosEnabled()
       c. if kairosEnabled:
            options.brief = true
            setKairosActive(true)
            assistantTeamContext = await assistantModule.initializeAssistantTeam()

NOTE: The same `feature('KAIROS') && isAssistantMode()` predicate ALSO mutates the bridge `workerType` from `'claude_code'` to `'claude_code_assistant'` at `src/bridge/initReplBridge.ts:476-484` (union: `BridgeWorkerType` at `src/bridge/types.ts:79`). This is the bridge-side observable of the assistant-mode latch; cross-spec to **34** (replbridge) for full bridge worker-type metadata flow.
4. Continue with normal bootstrap
5. main.tsx:2206-2208:
       if feature('KAIROS') && kairosEnabled && assistantModule:
           appendSystemPrompt += assistantModule.getAssistantSystemPromptAddendum()
6. main.tsx:3035:
       teamContext = feature('KAIROS') ? assistantTeamContext ?? computeInitialTeamContext?.() : computeInitialTeamContext?.()
```

### 5.2 Channel inbound flow

```
For each connected MCP server:
  result = gateChannelServer(serverName, capabilities, pluginSource)
  if result.action === 'register':
     subscribe(notifications/claude/channel, (params) => {
        wrapped = wrapChannelMessage(serverName, params.content, params.meta)
        enqueue(wrapped)             // <channel source="X" key="v"> ... </channel>
     })
     subscribe(notifications/claude/channel/permission, (params) =>
        callbacks.resolve(params.request_id, params.behavior, serverName)
     )
```

### 5.3 Outbound permission relay (PreToolUse hook + permission dialog)

```
function relayPermissionPrompt(tool, toolUseID, input):
  if !(feature('KAIROS') || feature('KAIROS_CHANNELS')) return
  if !isChannelPermissionRelayEnabled() return        // tengu_harbor_permissions
  clients = filterPermissionRelayClients(connectedClients, isInAllowlist)
  request_id = shortRequestId(toolUseID)
  for c in clients:
    c.send(CHANNEL_PERMISSION_REQUEST_METHOD, {
      request_id, tool_name: tool.name,
      description: tool.description(...),
      input_preview: truncateForPreview(input)        // <=200 + "…"
    })
  return Promise.race([
    localUIDecision,
    bridgeDecision,
    hookDecision,
    classifierDecision,
    new Promise(resolve => onResponse(request_id, resolve))
  ])
```

### 5.4 Cron scheduler tick (re-stating pseudocode anchored in `cronTasks.ts`)

```
loop forever:
  if !isKairosCronEnabled(): sleep + continue
  tasks = listAllCronTasks(getProjectRoot())     // file + session
  now = Date.now()
  fires = []
  for t in tasks:
    anchor = t.lastFiredAt ?? t.createdAt
    nextAt = (t.recurring
              ? jitteredNextCronRunMs(t.cron, anchor, t.id)
              : oneShotJitteredNextCronRunMs(t.cron, anchor, t.id))
    if nextAt !== null and nextAt <= now and replIsIdle:
      fires.push(t)
  if fires not empty:
    enqueue prompts (session prompt routing for `t.agentId`, else REPL queue)
    markCronTasksFired(fires.filter(recurring), now)
    removeCronTasks(fires.filter(!recurring).map(t=>t.id))
  // expiry sweep
  for t in tasks where t.recurring && !t.permanent:
    if (now - t.createdAt) > recurringMaxAgeMs: removeCronTasks([t.id])
  sleep(tickInterval)
```

### 5.5 Brief activation gate (`tools/BriefTool/BriefTool.ts:88-134`)

```
isBriefEntitled():
  return feature('KAIROS') || feature('KAIROS_BRIEF')
    ? kairosActive || env CLAUDE_CODE_BRIEF || GB tengu_kairos_brief (5 min refresh)
    : false

isBriefEnabled():
  return feature('KAIROS') || feature('KAIROS_BRIEF')
    ? (kairosActive || userMsgOptIn) && isBriefEntitled()
    : false
```

### 5.6 Dream skill loading (`skills/bundled/index.ts:35-40`)

```
if (feature('KAIROS') || feature('KAIROS_DREAM')) {
  const { registerDreamSkill } = require('./dream.js')   // missing-source
  registerDreamSkill()
}
```

The non-Kairos `services/autoDream/autoDream.ts` short-circuits when `getKairosActive()` is true (`:96`) — the Kairos build runs the disk-backed Dream skill instead. Skill loading is owned by spec 17; this spec only documents the gate.

---

## 6. Verbatim Assets

### 6.1 Kairos system prompt deltas

The Kairos system-prompt addendum itself lives in `src/assistant/index.js` (`getAssistantSystemPromptAddendum`) which is **missing-source**. The default-loop hooks that *layer in* Brief/proactive sections under the KAIROS family are present:

#### 6.1.1 Brief proactive section (`src/tools/BriefTool/prompt.ts:12-22`, verbatim)

```text
## Talking to the user

SendUserMessage is where your replies go. Text outside it is visible if the user expands the detail view, but most won't — assume unread. Anything you want them to actually see goes through SendUserMessage. The failure mode: the real answer lives in plain text while SendUserMessage just says "done!" — they see "done!" and miss everything.

So: every time the user says something, the reply they actually read comes through SendUserMessage. Even for "hi". Even for "thanks".

If you can answer right away, send the answer. If you need to go look — run a command, read files, check something — ack first in one line ("On it — checking the test output"), then work, then send the result. Without the ack they're staring at a spinner.

For longer work: ack → work → result. Between those, send a checkpoint when something useful happened — a decision you made, a surprise you hit, a phase boundary. Skip the filler ("running tests...") — a checkpoint earns its place by carrying information.

Keep messages tight — the decision, the file:line, the PR number. Second person always ("your config"), never third.
```

#### 6.1.2 Brief tool prompt (`src/tools/BriefTool/prompt.ts:6-10`, verbatim)

```text
Send a message the user will read. Text outside this tool is visible in the detail view, but most won't open it — the answer lives here.

`message` supports markdown. `attachments` takes file paths (absolute or cwd-relative) for images, diffs, logs.

`status` labels intent: 'normal' when replying to what they just asked; 'proactive' when you're initiating — a scheduled task finished, a blocker surfaced during background work, you need input on something they haven't asked about. Set it honestly; downstream routing uses it.
```

#### 6.1.3 Proactive system-prompt anchor (`src/constants/prompts.ts:912-913`, verbatim within KAIROS/PROACTIVE branch)

```text
- **Unfocused**: The user is away. Lean heavily into autonomous action — make decisions, explore, commit, push. Only pause for genuinely irreversible or high-risk actions.
- **Focused**: The user is watching. Be more collaborative — surface choices, ask before committing to large changes, and keep your output concise so it's easy to follow in real time.
```

(`prompts.ts:878` opens this section with the verbatim "On your very first tick…" first-tick instruction, immediately preceding the focused/unfocused text.)

### 6.2 Brief generation prompt

The "brief generation prompt" (system-level) is the proactive section above. There is no separate "summarizer" prompt for Brief — Brief is a tool the model invokes, not a summarization pass. The `/brief` toggle injects an inline `<system-reminder>` (`commands/brief.ts:111-119`):

```text
<system-reminder>
Brief mode is now enabled. Use the SendUserMessage tool for all user-facing output — plain text outside it is hidden from the user's view.
</system-reminder>
```

(symmetric "now disabled" string when toggled off)

### 6.3 Channel envelope schemas (verbatim)

See §3.3 above — the three schemas are inlined verbatim from `channelNotification.ts`.

The wire envelope of an inbound channel message after `wrapChannelMessage` (`channelNotification.ts:106-116`):

```text
<channel source="<server>"<sanitized k="v" attrs>>
<content>
</channel>
```

`SAFE_META_KEY = /^[a-zA-Z_][a-zA-Z0-9_]*$/` (`channelNotification.ts:104`) restricts attribute names. Values are XML-attribute-escaped via `escapeXmlAttr`.

### 6.4 Webhook payload schemas

**Missing-source.** `KAIROS_GITHUB_WEBHOOKS` registers `SubscribePRTool` and `/subscribe-pr` but neither file is present in the leaked tree. Inferred surface only — see §12.

### 6.5 Constants table

| Name | Value | Source |
|---|---|---|
| `KAIROS_BRIEF_REFRESH_MS` (Brief GB-cache TTL) | `5 * 60 * 1000` ms | `tools/BriefTool/BriefTool.ts:67` |
| `KAIROS_CRON_REFRESH_MS` (cron gate TTL) | `5 * 60 * 1000` ms | `tools/ScheduleCronTool/prompt.ts:6` |
| `recurringFrac` | `0.1` | `utils/cronTasks.ts:349` |
| `recurringCapMs` | `15 * 60 * 1000` (15 min) | `:350` |
| `oneShotMaxMs` | `90 * 1000` (90 s) | `:351` |
| `oneShotFloorMs` | `0` | `:352` |
| `oneShotMinuteMod` | `30` | `:353` |
| `recurringMaxAgeMs` (auto-expiry) | `7 * 24 * 60 * 60 * 1000` (7 d) | `:354` |
| `HISTORY_PAGE_SIZE` (session-events fetch) | `100` | `assistant/sessionHistory.ts:7` |
| sessionEvents request `timeout` | `15000` ms | `assistant/sessionHistory.ts:54` |
| sessionEvents `anthropic-beta` header | `'ccr-byoc-2025-07-29'` | `assistant/sessionHistory.ts:39` |
| `SESSION_SCAN_INTERVAL_MS` (auto-dream throttle, non-Kairos) | `10 * 60 * 1000` ms | `services/autoDream/autoDream.ts:56` |
| `DEFAULTS.minHours` (auto-dream) | `24` | `:64` |
| `DEFAULTS.minSessions` (auto-dream) | `5` | `:65` |
| GB key — KAIROS umbrella | `tengu_kairos` | `main.tsx:1034` (comment) |
| GB key — Brief tool kill-switch | `tengu_kairos_brief` | `BriefTool.ts:95` |
| GB key — Brief slash-command visibility | `tengu_kairos_brief_config` | `commands/brief.ts:40` |
| GB key — Channels overall | `tengu_harbor` | `channelAllowlist.ts:52` |
| GB key — Channels plugin allowlist | `tengu_harbor_ledger` | `channelAllowlist.ts:39` |
| GB key — Channels permission relay | `tengu_harbor_permissions` | `channelPermissions.ts:37` |
| GB key — Cron umbrella | `tengu_kairos_cron` | `tools/ScheduleCronTool/prompt.ts:39-43` |
| GB key — Cron jitter config | `tengu_kairos_cron_config` | `utils/cronJitterConfig.ts:69` |
| GB key — Durable cron | `tengu_kairos_cron_durable` | `tools/ScheduleCronTool/prompt.ts:58` |
| GB key — Auto-dream config | `tengu_onyx_plover` | `services/autoDream/{autoDream,config}.ts` |
| Env override — disable cron | `CLAUDE_CODE_DISABLE_CRON` | `tools/ScheduleCronTool/prompt.ts:38` |
| Env override — force Brief | `CLAUDE_CODE_BRIEF` | `tools/BriefTool/BriefTool.ts:93` |

### 6.6 Permission-reply alphabet & blocklist (`channelPermissions.ts:78-110`, verbatim)

```ts
const ID_ALPHABET = 'abcdefghijkmnopqrstuvwxyz'   // 25 letters, no 'l'
const ID_AVOID_SUBSTRINGS = [
  'fuck','shit','cunt','cock','dick','twat','piss','crap','bitch','whore',
  'ass','tit','cum','fag','dyke','nig','kike','rape','nazi','damn',
  'poo','pee','wank','anus',
]
const PERMISSION_REPLY_RE = /^\s*(y|yes|n|no)\s+([a-km-z]{5})\s*$/i
```

Hash function: FNV-1a 32-bit → base-25 of `ID_ALPHABET`, 5 letters; re-salt up to 10 times if blocklisted (`:112-152`).

---

## 7. Side Effects & I/O

- **Filesystem**:
  - `<projectRoot>/.claude/scheduled_tasks.json` — read on startup + on chokidar change; written on add/delete/fire-stamp; pretty-printed JSON, trailing newline (`utils/cronTasks.ts:165-182`).
  - `<projectRoot>/.claude/agents/assistant.md` — appended to system prompt before trust dialog (`main.tsx:1043-1047` comment).
- **Network**:
  - Anthropic events API: `${BASE_API_URL}/v1/sessions/{sid}/events` (assistant viewer mode).
  - GrowthBook: `tengu_kairos`, `tengu_kairos_brief`, `tengu_kairos_brief_config`, `tengu_kairos_cron`, `tengu_kairos_cron_config`, `tengu_kairos_cron_durable`, `tengu_harbor`, `tengu_harbor_ledger`, `tengu_harbor_permissions`, `tengu_onyx_plover`.
  - MCP servers acting as channels (any transport per spec 23).
  - GitHub webhooks intake — **missing-source**.
- **Env vars**: `CLAUDE_CODE_BRIEF` (entitlement bypass), `CLAUDE_CODE_DISABLE_CRON` (kill-switch), `USER_TYPE` (irrelevant — KAIROS family is not ANT-gated).
- **Trust boundaries**:
  - Assistant-mode activation **requires** `checkHasTrustDialogAccepted()` (`main.tsx:1067-1069`) — `.claude/settings.json` is treated as attacker-controllable until trust is granted. The `.claude/agents/assistant.md` content has already been appended to the system prompt by then; refusal is the safe path.
  - Channel allowlist (`tengu_harbor_ledger`) plus per-session `--channels` opt-in plus org policy gate.
  - Permission relay never accepts *text* approvals — only the structured `notifications/claude/channel/permission` event with explicit server-declared `claude/channel/permission` capability.

---

## 8. Feature Flags & Variants

| Flag | Effect on/off | Notes |
|---|---|---|
| `KAIROS` (umbrella) | OFF: assistant module never imported, `setKairosActive` never fires, all KAIROS-family deltas inert. ON: imports `assistant/index.js` + `assistant/gate.js`, registers `--assistant` / `assistant [sessionId]`, gates channel + brief + dream + send-user-file + push-notification + sleep tools. | `main.tsx:80-81,1058,1642,2184,2206`; `commands.ts:62,67,70`; `tools.ts:42,46`; pulls in `KAIROS_DREAM` skill registration via OR. |
| `KAIROS_BRIEF` | OFF: Brief surface gone (when `KAIROS` also off). ON without `KAIROS`: Brief ships independently — entitlement still gated on `tengu_kairos_brief`. | `BriefTool.ts:91,131`; `commands/brief.ts:52`. |
| `KAIROS_GITHUB_WEBHOOKS` | OFF: `SubscribePRTool` import = null; `/subscribe-pr` command not registered. ON: tool + slash command available (sources missing). | `tools.ts:50`; `commands.ts:101`. |
| `KAIROS_PUSH_NOTIFICATION` | OFF: `PushNotificationTool` = null and `Local notifications` settings label degrades to plain `Notifications`. ON: tool registered (source missing) + settings additions enabled. | `tools.ts:46`; `Settings/Config.tsx:658,672`; `ConfigTool/supportedSettings.ts:164`. |
| `KAIROS_CHANNELS` | OFF without `KAIROS`: `--channels` parser branch dead, `ChannelsNotice` not loaded, channel-aware branches in interactiveHelpers/AskUserQuestion/EnterPlanMode/ExitPlanMode/messageQueueManager/messages/cli print all skipped. ON: full channel surface. | `main.tsx:1642,3844`; `interactiveHelpers.tsx:241`; etc. |
| `KAIROS_DREAM` | OFF without `KAIROS`: `dream` skill not registered. ON: `registerDreamSkill()` (source missing) loaded. | `skills/bundled/index.ts:35-40`. |
| `AGENT_TRIGGERS` | OFF: `cronTools = []` (3 tool registrations skipped); `/loop` skill not registered. ON: tools + skill registered; `isKairosCronEnabled()` then defers to `tengu_kairos_cron` runtime gate. | `tools.ts:29-35`; `skills/bundled/index.ts:47-55`; `tools/ScheduleCronTool/prompt.ts:36`. |
| `AGENT_TRIGGERS_REMOTE` | OFF: `RemoteTriggerTool = null`; `scheduleRemoteAgents` skill not registered. | `tools.ts:36-38`; `skills/bundled/index.ts:56-63`. |

ANT-only paths inside this family: none. KAIROS family is explicitly NOT `USER_TYPE === 'ant'`-gated; the comment in `tools.ts:1` (`ANT-ONLY import markers must not be reordered`) refers only to the few ANT-only requires (`REPLTool`, `SuggestBackgroundPRTool`, `TungstenTool`).

---

## 9. Error Handling & Edge Cases

- **Trust dialog not accepted before assistant-mode activation** → `main.tsx:1068-1069` logs (verbatim) `"Assistant mode disabled: directory is not trusted. Accept the trust dialog and restart."` and continues bootstrap with `kairosEnabled=false`.
- **Channel server lacks `claude/channel` capability** → silent skip with `{ kind: 'capability', reason: 'server did not declare claude/channel capability' }` (`channelNotification.ts:200-206`). Not surfaced unless user used `--channels`.
- **`tengu_harbor=false`** mid-session → channels stay registered for the session; the gate is only checked at `useManageMCPConnections` mount (`channelPermissions.ts:33-34`).
- **`tengu_kairos_cron=false`** mid-session → the scheduler injects `() => !isKairosCronEnabled()` as its kill-poll; running schedulers stop on the next tick (`utils/cronScheduler.ts:115-116`).
- **Malformed `.claude/scheduled_tasks.json`** → returns `[]` and logs at debug; per-task malformations skipped individually (`utils/cronTasks.ts:96-126`).
- **Permanent task deleted accidentally** → `assistant/install.ts` (missing-source) uses `writeIfMissing` so re-install is a no-op; the comment at `cronTasks.ts:50-56` warns that permanent tasks cannot be re-created via the user-facing tool.
- **PR-subscription / push-notification errors** → cannot be enumerated; sources missing.
- **Channel allowlist cold cache** → `getChannelAllowlist()` returns `[]` when the GB cache is cold (`channelAllowlist.ts:36-43`); UI's `ChannelsNotice` accepts the false-warn tradeoff (`ChannelsNotice.tsx:223-226` comment).
- **Permission-relay duplicate event** → `resolve()` deletes the pending entry before invoking; second emission is a silent no-op (`channelPermissions.ts:228-238`).
- **Channel meta key with unsafe characters** → silently filtered out by `SAFE_META_KEY` regex (`channelNotification.ts:104,113`); attribute injection prevented.

---

## 10. Telemetry & Observability

- `tengu_brief_send` (`tools/BriefTool/BriefTool.ts:188-191`) — `{ proactive: status === 'proactive', attachment_count }`
- `tengu_brief_mode_toggled` (`commands/brief.ts:70-99`) — `{ enabled, gated, source: 'slash_command' }`
- `tengu_auto_dream_fired` / `tengu_auto_dream_completed` / `tengu_auto_dream_failed` (`services/autoDream/autoDream.ts:195,252,267`) — non-Kairos auto-dream only
- `assistantActivationPath` — analytics tag passed through REPL (`main.tsx:2518,4536-4595`); value computed by missing-source `getAssistantActivationPath()`
- KAIROS-mode metadata `kairosEnabled` propagated through `main.tsx:2647-2648,2962`
- Logs:
  - `[ScheduledTasks] skipping malformed task: …` (cronTasks.ts:117)
  - `[autoDream] firing — …` etc. (autoDream.ts:192-…)
  - `[ChannelsNotice]` is rendered as Ink output, not log

No dedicated OpenTelemetry spans for KAIROS family in the leaked source; spans are inherited from the per-tool / per-turn fabric in specs 22/26.

---

## 11. Reimplementation Checklist

A reimplementer must preserve the following invariants:

- **DCE-load-bearing `feature()` ternaries.** Both `isBriefEntitled()` and `isBriefEnabled()` (`BriefTool.ts:88-134`) wrap the entire body inside `feature('KAIROS') || feature('KAIROS_BRIEF') ? … : false`; refactoring to a negative early return defeats Bun's constant-fold and leaks the GB key strings into external builds.
- **`tools.ts` import order.** The KAIROS family imports are interleaved with ANT-only imports and the `biome-ignore-all` comment on `:1` is load-bearing — the bundler must see them in that physical order.
- **Trust-dialog precondition.** Assistant-mode activation must check `checkHasTrustDialogAccepted()` and refuse to set `kairosActive` otherwise.
- **`--brief` is forced** when assistant mode activates (`main.tsx:1080`).
- **`fullRemoteControl = remoteControl || getRemoteControlAtStartup() || kairosEnabled`** (`main.tsx:2916`) — Kairos implies full remote control.
- **`teamContext` fallback** — Kairos sessions prefer `assistantTeamContext` over `computeInitialTeamContext()`; non-Kairos sessions only have the latter.
- **Channel inbound envelope** must be `<channel source="X" k="v">…content…</channel>` exactly, with `SAFE_META_KEY` filtering and `escapeXmlAttr` on values.
- **Channel permission protocol** must use `notifications/claude/channel/permission_request` outbound (request) and `notifications/claude/channel/permission` inbound (response); approval text in the general channel must NOT count as approval.
- **Cron task on-disk shape** must strip `durable` and `agentId` runtime-only fields; preserve `permanent` flag for assistant-installed tasks.
- **One-shot vs. recurring jitter** must use `oneShotJitteredNextCronRunMs` vs. `jitteredNextCronRunMs` respectively; one-shot jitter is *backward* and only applies when the minute matches `oneShotMinuteMod` (default 30).
- **`recurringMaxAgeMs` exemption** for `permanent: true` tasks; `0` means unlimited.
- **`KAIROS_DREAM`** is implied by bare `KAIROS` (the OR at `skills/bundled/index.ts:35`); shipping `KAIROS` without `KAIROS_DREAM` still loads the Dream skill.
- **`SleepTool` shared with PROACTIVE.** Both flags share the import gate (`tools.ts:25-28`); KAIROS without PROACTIVE still bundles SleepTool.
- **Brief tool name** — emit as `SendUserMessage` with alias `Brief` (`tools/BriefTool/prompt.ts:1-2`); the legacy alias must be preserved for resumed sessions.
- **`/loop` skill registration** (`skills/bundled/index.ts:47-55`) is unconditional under `AGENT_TRIGGERS`; the skill's own `isEnabled` callback delegates to `isKairosCronEnabled()`.

Spec is complete when all of `src/services/mcp/{channelNotification,channelPermissions,channelAllowlist}.ts`, `src/tools/BriefTool/{BriefTool,prompt,UI,attachments,upload}.ts`, `src/utils/{cronTasks,cronJitterConfig,cronScheduler}.ts`, `src/tools/ScheduleCronTool/{prompt,Cron*Tool,UI}.ts(x)`, `src/skills/bundled/{index,loop,scheduleRemoteAgents}.ts`, `src/services/autoDream/*.ts`, `src/assistant/sessionHistory.ts` are reproducible bit-exact, AND the missing-source ledger in §12 is honored as such (no fabrication).

---

## 12. Open Questions / Unknowns

The KAIROS family has the largest missing-source surface in the leaked tree. **Every missing source is enumerated here** — reimplementers cannot satisfy bit-exactness without the original of these files:

1. **`src/assistant/index.js`** (the assistant engine — entrypoint for `feature('KAIROS')` mode). Required exports per `main.tsx`: `isAssistantMode()`, `isAssistantForced()`, `markAssistantForced()`, `getAssistantActivationPath()`, `getAssistantSystemPromptAddendum()`, `initializeAssistantTeam()`. **The verbatim Kairos system prompt addendum lives here and is NOT reconstructible from the leaked tree.**
2. **`src/assistant/gate.js`** — `isKairosEnabled()`. Likely consults the `tengu_kairos` GB gate plus the assistant directory's contents. Returns `Promise<boolean>`.
3. **`src/assistant/sessionDiscovery.js`** — discovery of running bridge sessions to attach to (`main.tsx:3265-3266`).
4. **`src/assistant/install.ts`** — installer for assistant-mode permanent crons (catch-up / morning-checkin / dream); uses `writeIfMissing` semantics (per comment in `utils/cronTasks.ts:50-56`).
5. **`src/commands/assistant/index.ts`** — `/assistant` slash command implementation (`commands.ts:70-72`).
6. **`src/commands/subscribe-pr.ts`** — `/subscribe-pr` slash command (`commands.ts:101-103`).
7. **`src/tools/SubscribePRTool/`** — model-driven PR subscription tool (`tools.ts:50-52`). Webhook intake server, payload schemas, retry policy, unsubscribe semantics — all unknown.
8. **`src/tools/PushNotificationTool/`** — push notifications transport (`tools.ts:45-49`). Whether this targets APNS/FCM/local OS notifications, payload format, retry policy — all unknown.
9. **`src/tools/SendUserFileTool/`** — file-to-channel delivery (`tools.ts:42-44`). Storage backend, signed-URL semantics — unknown.
10. ~~`src/tools/RemoteTriggerTool/`~~ — **RESOLVED Phase 9.6c**: source verified PRESENT (`src/tools/RemoteTriggerTool/{RemoteTriggerTool.ts, UI.tsx, prompt.ts}`). Was phantom missing-source row (Pattern A2 — same as spec 19 ScheduleCronTool / spec 00 §2.5). Tool surface owned by spec 19; relationship to local cron and bridge sessions documented there.
11. **`src/skills/bundled/dream.ts`** — Dream skill content (`skills/bundled/index.ts:36-39`). The non-Kairos `services/autoDream` is present but the disk-backed Dream skill body is the canonical Kairos consolidation; not in the leak.
12. **Channel adapter MCP servers** (Telegram, iMessage, Discord). These ship as MCP plugins, NOT inside the CLI tree. The leak's channel surface is the *generic* MCP-channel transport; concrete adapter source is out of repo by design.
13. **Webhook-intake endpoint and authentication scheme** for `KAIROS_GITHUB_WEBHOOKS`.
14. **Session-events API server-side schema** for `${BASE_API_URL}/v1/sessions/{sid}/events`. The client (`assistant/sessionHistory.ts`) is present and reads `data: SDKMessage[]`, `has_more`, `first_id`, `last_id`; the server-side schema is not in this repo.
15. **`assistantActivationPath` enum values** — populated by missing-source `getAssistantActivationPath()`; analytics tag string set unknown.

Items resolvable from the leak are inlined in §3–§6; items above are estimates marked as such. None of the missing-source items can be inferred bit-exact from registry citations alone.
