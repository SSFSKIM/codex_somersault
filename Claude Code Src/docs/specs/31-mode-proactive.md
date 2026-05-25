# 31 — Mode: Proactive (`PROACTIVE` flag)

> **Mode-delta spec.** Documents only what changes when `feature('PROACTIVE')`
> evaluates true (the same gate is shared with `feature('KAIROS')`; this spec
> covers behavior triggered by either flag because every callsite uses the
> disjunction `feature('PROACTIVE') || feature('KAIROS')`). Host code already
> documented elsewhere is not redocumented here.
>
> **Out of scope (refer by spec).**
> - `SleepTool` surface (schema, prompt, runtime) → 19.
> - Kairos overlay (`KAIROS`, `KAIROS_BRIEF`, `KAIROS_PUSH_NOTIFICATION`,
>   `KAIROS_GITHUB_WEBHOOKS`, `KAIROS_CHANNELS`, BriefTool, assistant mode,
>   crons) → 32.
> - Coordinator multi-agent run loop → 30.
> - Session persistence (resume picker, log enrichment) → 41.

---

## §0. Source-coverage inventory

The implementation module `src/proactive/index.js` is **referenced but absent
from the leaked tree** (every consumer wraps the import in
`feature('PROACTIVE') || feature('KAIROS')` and `require('../proactive/index.js')`).
This spec documents only the caller-side wiring; the proactive module's
internal state machine (timer scheduler, listener set, `getNextTickAt` math,
context-block flag storage) is recorded as a Phase-0 gap in §12.

| Surface | Citation | Source present? |
|---|---|---|
| `SleepTool` registry inclusion | `src/tools.ts:25-28`, `:234` | partial (prompt only — see 19) |
| `proactive` slash command registry | `src/commands.ts:62-65` | **missing** (`./commands/proactive.js`) |
| Proactive prompt-fragment append (REPL path) | `src/main.tsx:2194-2205` | yes |
| `--proactive` CLI option registration | `src/main.tsx:3832-3834` | yes |
| `maybeActivateProactive(options)` (headless path) | `src/main.tsx:4611-4621` | yes |
| Pre-`getTools()` activation in headless | `src/main.tsx:1864-1867` | yes |
| Headless tick scheduler (`scheduleProactiveTick`) | `src/cli/print.ts:1831-1856` | yes |
| Headless tick re-arm after run loop | `src/cli/print.ts:2475-2485` | yes |
| Headless `set_proactive` control RPC | `src/cli/print.ts:3875-3891` | yes |
| Headless fallback activation (env-only path) | `src/cli/print.ts:534-545` | yes |
| REPL `useSyncExternalStore(isProactiveActive)` | `src/screens/REPL.tsx:686-696` | yes |
| Pause on cancel (Esc) | `src/screens/REPL.tsx:2113-2117` | yes |
| Resume on submit | `src/screens/REPL.tsx:3153-3156` | yes |
| Context-block clear on compact / `/clear` | `src/screens/REPL.tsx:2604-2607,2738-2740,4966-4970`; `src/commands/clear/conversation.ts:111-117` | yes |
| Context-block set on API error | `src/screens/REPL.tsx:2631-2640` | yes |
| `terminalFocus` user-context injection when unfocused | `src/screens/REPL.tsx:2776-2779` | yes |
| Hide spinner while only `Sleep` is in flight | `src/screens/REPL.tsx:1654-1660,1682` | yes |
| `nextTickAt` countdown footer | `src/components/PromptInput/PromptInputFooterLeftSide.tsx:74-126,264` | yes |
| Disable terminal progress bars while active | `src/components/Messages.tsx:80,603` | yes |
| `sleep_progress` ephemeral type registration | `src/utils/sessionStorage.ts:186-196` | yes |
| Synthetic resume-list label `'Proactive session'` | `src/utils/sessionStorage.ts:4889-4912` | yes |
| Tick-content emission constant | `src/constants/xml.ts:25` (`TICK_TAG = 'tick'`) | yes |
| Autonomous-agent system-prompt path (simple) | `src/constants/prompts.ts:466-489` | yes |
| `getProactiveSection()` system-prompt addendum | `src/constants/prompts.ts:860-913` | yes |
| Brief-section suppression while proactive | `src/constants/prompts.ts:843-858` | yes |
| Build-effective-prompt: append (not replace) when active | `src/utils/systemPrompt.ts:14-122` | yes |
| Force agents async while active | `src/tools/AgentTool/AgentTool.tsx:59,567` | yes |
| Drain priority upgrade after sleep ran | `src/query.ts:91,1566,1571` | yes |
| Queue `priority` semantics (wakes Sleep) | `src/types/textInputTypes.ts:275-294` | yes |
| Interruptible-tool abort path on submit | `src/utils/handlePromptSubmit.ts:319-332` | yes |
| Sleep tool name in classifier-decision allow | `src/utils/permissions/classifierDecision.ts:11,87` | yes |
| Channel notification: poll-driven Sleep wake | `src/services/mcp/channelNotification.ts:9` (comment) | yes |
| `minSleepDurationMs` / `maxSleepDurationMs` settings keys (`PROACTIVE \|\| KAIROS` gate) | `src/utils/settings/types.ts:841-863` | yes (owned by spec 19 — see §6.11) |

---

## §1. Purpose & Scope

Proactive mode keeps the model alive between user turns by injecting periodic
synthetic user messages (`<tick>HH:MM:SS</tick>`) so the agent can self-pace,
self-direct, and call `SleepTool` between actions. When the flag is off
nothing in this spec runs; when on, every callsite above gains the listed
behavioral delta. The mode is shared verbatim with Kairos (32) — every gate
is `feature('PROACTIVE') || feature('KAIROS')`.

Activation requires both the flag AND user opt-in via either:
- `--proactive` CLI option (registered only when the flag is set), OR
- `CLAUDE_CODE_PROACTIVE` env truthy, OR
- the `set_proactive` control-protocol RPC (SDK transport).

Coordinator mode short-circuits the prompt-append path (`src/main.tsx:2194-2199`):
when `coordinatorModeModule?.isCoordinatorMode()` returns true, the proactive
system-prompt fragment is suppressed even if `--proactive` is set. The
boot-time activation (`maybeActivateProactive`) is NOT short-circuited by
coordinator mode at `src/main.tsx:4611-4621`.

**Wake mechanism — no callback wakeup exists in the leak.** Ticks are
synthetic user prompts enqueued by a `setTimeout(0)` scheduler
(`scheduleProactiveTick` at `src/cli/print.ts:1831-1856`); there is no
typed wakeup callback, no Zod schema for a wake payload, and no
caller-visible auto-wake API. The Sleep tool's wake-from-queue mechanism
lives inside the absent `src/proactive/index.js` and `SleepTool` (spec 19);
the only on-disk hint is the comment at
`src/services/mcp/channelNotification.ts:9` ("SleepTool polls
hasCommandsInQueue() and wakes within 1s"). See §12 for the Phase-0 gap.

---

## §2. Source Map

Owned-by-31 deltas (host code stays under its existing spec):

| File | Line(s) | Delta |
|---|---|---|
| `src/tools.ts` | 25-28, 234 | Spread `[SleepTool]` into the base tool list (gate `PROACTIVE \|\| KAIROS`). |
| `src/commands.ts` | 62-65 | Register the `proactive` slash command (source absent). |
| `src/main.tsx` | 1864-1867 | Headless: `maybeActivateProactive(options)` runs before `getTools()` so `SleepTool.isEnabled()` (gates on `isProactiveActive()`) passes. |
| `src/main.tsx` | 2194-2205 | REPL path: append the Proactive system-prompt fragment to `appendSystemPrompt`, suppressed under coordinator mode. |
| `src/main.tsx` | 3832-3834 | Register `--proactive` CLI option. |
| `src/main.tsx` | 4611-4621 | `maybeActivateProactive(options)` → `proactiveModule.activateProactive('command')` if not already active. |
| `src/cli/print.ts` | 361-364 | Top-level conditional `require('../proactive/index.js')`. |
| `src/cli/print.ts` | 534-545 | Late env-only fallback activation (covers SDK transport injecting `CLAUDE_CODE_PROACTIVE` after argv parse). |
| `src/cli/print.ts` | 1831-1856 | `scheduleProactiveTick` closure (definition). |
| `src/cli/print.ts` | 2475-2485 | Re-arm tick after `run()` finishes if queue is empty and not paused/closed. |
| `src/cli/print.ts` | 3875-3891 | Control-protocol `set_proactive` handler. |
| `src/screens/REPL.tsx` | 194 | Top-level conditional require. |
| `src/screens/REPL.tsx` | 686-696 | Subscribe `proactiveActive` so `localTools` recomputes when it flips. |
| `src/screens/REPL.tsx` | 1654-1660, 1682 | `onlySleepToolActive` hides spinner. |
| `src/screens/REPL.tsx` | 2113-2117 | `onCancel` → `pauseProactive()`. |
| `src/screens/REPL.tsx` | 2604-2607, 2631-2640, 2738-2740, 4966-4970 | `setContextBlocked(true/false)` on API error / compact / partial-compact. |
| `src/screens/REPL.tsx` | 2776-2779 | Add `terminalFocus` user-context line when unfocused. |
| `src/screens/REPL.tsx` | 3153-3156 | `onSubmit` → `resumeProactive()`. |
| `src/components/PromptInput/PromptInputFooterLeftSide.tsx` | 47, 74-126, 264 | `ProactiveCountdown` component + footer subscription. |
| `src/components/Messages.tsx` | 80, 603 | Disable `terminalProgressBarEnabled` while active. |
| `src/utils/systemPrompt.ts` | 14-26, 99-122 | When proactive is active and an agent definition is set, **append** agent prompt to defaults rather than replace. |
| `src/constants/prompts.ts` | 72-75, 466-489, 843-913 | Simple-mode autonomous prompt path; `getProactiveSection()`; brief-section suppression. |
| `src/utils/sessionStorage.ts` | 186-196, 4889-4912 | `sleep_progress` is ephemeral; tick-only sessions get title `'Proactive session'`. |
| `src/commands/clear/conversation.ts` | 111-117 | `/clear` calls `setContextBlocked(false)`. |
| `src/tools/AgentTool/AgentTool.tsx` | 59, 567 | Force agent spawns async while active. |
| `src/query.ts` | 91, 1566, 1571 | Drain `'later'`-priority queued commands too when the just-finished turn ran `Sleep` (otherwise drain at `'next'`). |

**Imports from**: `bun:bundle`'s `feature`; `proactive/index.js` (absent —
referenced shape declared by every site below); `tools/SleepTool/prompt.ts`
(`SLEEP_TOOL_NAME`); `constants/xml.ts` (`TICK_TAG`).

**Imported by**: `tools.ts`, `commands.ts`, `main.tsx`, `cli/print.ts`,
`screens/REPL.tsx`, `components/Messages.tsx`,
`components/PromptInput/PromptInputFooterLeftSide.tsx`,
`tools/AgentTool/AgentTool.tsx`, `constants/prompts.ts`,
`utils/systemPrompt.ts`, `commands/clear/conversation.ts`.

---

## §3. Public Interface (Contract)

The proactive module exposes the following symbols (declared by usage —
source absent). Every consumer optional-chains except where the gate is
already proven, so all members are at minimum nullable references.

| Symbol | Used at | Inferred signature |
|---|---|---|
| `isProactiveActive` | `tools.ts` (via `SleepTool.isEnabled`), `main.tsx:4617`, `cli/print.ts:541,1839,2478,3884`, `screens/REPL.tsx:687`, `components/Messages.tsx:603`, `components/PromptInput/PromptInputFooterLeftSide.tsx`, `constants/prompts.ts:468,854,862`, `utils/systemPrompt.ts:25`, `tools/AgentTool/AgentTool.tsx:567` | `() => boolean` |
| `isProactivePaused` | `cli/print.ts:1840,2479` | `() => boolean` |
| `activateProactive` | `main.tsx:4618`, `cli/print.ts:544,3885` | `(source: 'command' \| string) => void` (only `'command'` observed) |
| `deactivateProactive` | `cli/print.ts:3889` | `() => void` |
| `pauseProactive` | `screens/REPL.tsx:2116` | `() => void` |
| `resumeProactive` | `screens/REPL.tsx:3155` | `() => void` |
| `setContextBlocked` | `screens/REPL.tsx:2606,2636,2638,2739,4969`; `commands/clear/conversation.ts:116` | `(blocked: boolean) => void` |
| `subscribeToProactiveChanges` | `screens/REPL.tsx:687`; `components/PromptInput/PromptInputFooterLeftSide.tsx:76,264` | `(cb: () => void) => () => void` (React `useSyncExternalStore` subscribe contract) |
| `getNextTickAt` | `components/PromptInput/PromptInputFooterLeftSide.tsx:76,264` | `() => number \| null` (epoch-ms; UI converts via `Math.ceil((nextTickAt - Date.now()) / 1000)`) |

The slash command (`commands.ts:62-65`) exports `default` (Command shape per
spec 20). Source absent.

The control-protocol RPC accepts:

```ts
// src/cli/print.ts:3879-3882 (verbatim)
const req = message.request as unknown as {
  subtype: string
  enabled: boolean
}
```

When `req.enabled` is `true`, the harness calls
`proactiveModule!.activateProactive('command')` then
`scheduleProactiveTick!()`; when `false`, `deactivateProactive()`.

---

## §4. Data Model & State

The proactive module's state is opaque (source absent). Caller-visible state:

| Predicate | Meaning (inferred from call sites) |
|---|---|
| `isProactiveActive()` | Mode is on. Drives tool-pool inclusion of `SleepTool`, system-prompt branches, footer countdown, `terminalFocus` user-context, AgentTool async forcing, `Messages.tsx` progress-bar suppression. |
| `isProactivePaused()` | Mode is on but ticks must not fire (Esc was pressed). Re-armed by `resumeProactive()` from `onSubmit`. Combined with `isProactiveActive` for tick gating in `cli/print.ts:1838-1842,2476-2480`. |
| Context-blocked (no public reader) | Set true on assistant API error (`isApiErrorMessage`); cleared on next non-error assistant message, on compact boundary, on partial-compact, and by `/clear`. Caller sites only call `setContextBlocked`; the proactive module presumably consults this internally to suppress ticks during error storms (per the comment at `screens/REPL.tsx:2631-2633`: "Block ticks on API errors to prevent tick → error → tick runaway loops"). |
| `getNextTickAt()` | Epoch-ms or `null`. UI countdown only; not a timer the consumer drives. |

Headless run-loop locals interacting with the module
(`src/cli/print.ts:1831-1856`, `2475-2485`):

- `running: boolean` — re-entrancy guard for `run()`.
- `inputClosed: boolean` — stdin EOF; suppresses tick re-arm.
- `peek(isMainThread)` — returns the next queued command for the main agent.
- `enqueue({mode:'prompt', value, uuid, priority:'later', isMeta:true})`
  appends the tick onto the shared command queue.

---

## §5. Algorithm / Control Flow

### 5.1 Activation (boot)

```
# Headless (cli/print.ts via main.tsx:1864-1867)
on argv parsed:
  if (PROACTIVE || KAIROS) && (--proactive || env CLAUDE_CODE_PROACTIVE):
    if !isProactiveActive(): activateProactive('command')
  tools = getTools(toolPermissionContext)         # SleepTool.isEnabled() now true
# Late fallback (cli/print.ts:534-545):
  if (PROACTIVE || KAIROS) && !isProactiveActive() && env CLAUDE_CODE_PROACTIVE:
    activateProactive('command')

# REPL prompt-append (main.tsx:2194-2205):
  if (PROACTIVE || KAIROS) && (options.proactive || env CLAUDE_CODE_PROACTIVE)
     && !coordinatorModule?.isCoordinatorMode():
    appendSystemPrompt += "\n\n" + proactivePrompt        # see §6
```

### 5.2 Tick scheduling (headless)

```
scheduleProactiveTick = (PROACTIVE || KAIROS) ? function() {
  setTimeout(() => {
    if !isProactiveActive() || isProactivePaused() || inputClosed: return
    enqueue({
      mode: 'prompt',
      value: `<tick>${new Date().toLocaleTimeString()}</tick>`,
      uuid: randomUUID(),
      priority: 'later',
      isMeta: true,
    })
    void run()
  }, 0)
} : undefined
```

After every `run()` settles (`cli/print.ts:2475-2485`):

```
if (PROACTIVE || KAIROS) && isProactiveActive() && !isProactivePaused():
  if peek(isMainThread) === undefined && !inputClosed:
    scheduleProactiveTick()
    return
```

The `setTimeout(0)` yields to the event loop so any pending stdin (interrupts,
user messages) is processed before the tick fires (`cli/print.ts:1832-1834`).

### 5.3 Pause / resume (REPL)

```
onCancel():                                 # Esc handler (REPL.tsx:2106-2118)
  if (PROACTIVE || KAIROS): pauseProactive()
  ...                                       # rest of cancel logic continues
onSubmit(input, ...):                       # REPL.tsx:3142-3156
  repinScroll()
  if (PROACTIVE || KAIROS): resumeProactive()
  ...                                       # then route immediate vs queued
```

### 5.4 Context-block lifecycle

```
on assistant message arrived (REPL.tsx:2634-2640):
  if newMessage.isApiErrorMessage: setContextBlocked(true)
  else if newMessage.type==='assistant': setContextBlocked(false)
on compact-boundary (handleMessageFromStream): setContextBlocked(false)
on manual /compact (newMessages contains compact-boundary): setContextBlocked(false)
on partial compact: setContextBlocked(false)
on /clear: setContextBlocked(false)
```

### 5.5 System-prompt selection

`buildEffectiveSystemPrompt` (`src/utils/systemPrompt.ts:99-113`):

```
if agentSystemPrompt && (PROACTIVE || KAIROS) && isProactiveActive():
  return [
    ...defaultSystemPrompt,
    "\n# Custom Agent Instructions\n" + agentSystemPrompt,
    ...(appendSystemPrompt ? [appendSystemPrompt] : []),
  ]
# else normal "agent prompt REPLACES default" path
```

`getSystemPrompt` simple branch (`constants/prompts.ts:466-489`): when
proactive is active, it returns a lean prompt set ending with
`getProactiveSection()` (see §6).

### 5.6 Agent (sub-agent) spawn forcing

`AgentTool.tsx:567` (verbatim conjuncts):

```
shouldRunAsync =
  (run_in_background === true
   || selectedAgent.background === true
   || isCoordinator
   || forceAsync                       // FORK_SUBAGENT
   || assistantForceAsync              // KAIROS+kairosEnabled
   || (proactiveModule?.isProactiveActive() ?? false))
  && !isBackgroundTasksDisabled
```

### 5.7 Drain-priority upgrade after sleep (`src/query.ts:1566,1571`)

```
sleepRan = toolUseBlocks.some(b => b.name === SLEEP_TOOL_NAME)
queuedCommandsSnapshot =
  getCommandsByMaxPriority(sleepRan ? 'later' : 'next').filter(...)
```

The mid-turn drain normally only takes `'next'` priority items; if `Sleep`
ran in the just-completed tool-use round, the drain threshold is raised to
also take `'later'` items so a queued user message reaches the model in
the same turn rather than after the next API round-trip.

### 5.8 UI subscription wiring

```
# REPL.tsx:686-696
proactiveActive = useSyncExternalStore(
  proactiveModule?.subscribeToProactiveChanges ?? PROACTIVE_NO_OP_SUBSCRIBE,
  proactiveModule?.isProactiveActive            ?? PROACTIVE_FALSE)
localTools = useMemo(() => getTools(ctx),
  [ctx, proactiveActive, isBriefOnly])         # re-runs on activate/deactivate
```

`PromptInputFooterLeftSide.tsx:76,264` subscribes the same change-source for
`getNextTickAt` to drive `ProactiveCountdown` (1-second `setInterval` while
`nextTickAt !== null`).

---

## §6. Verbatim Assets

### 6.1 REPL system-prompt fragment (`src/main.tsx:2201-2204`, verbatim including escapes)

```ts
const briefVisibility = feature('KAIROS') || feature('KAIROS_BRIEF') ? (require('./tools/BriefTool/BriefTool.js') as typeof import('./tools/BriefTool/BriefTool.js')).isBriefEnabled() ? 'Call SendUserMessage at checkpoints to mark where things stand.' : 'The user will see any text you output.' : 'The user will see any text you output.';
const proactivePrompt = `\n# Proactive Mode\n\nYou are in proactive mode. Take initiative — explore, act, and make progress without waiting for instructions.\n\nStart by briefly greeting the user.\n\nYou will receive periodic <tick> prompts. These are check-ins. Do whatever seems most useful, or call Sleep if there's nothing to do. ${briefVisibility}`;
```

### 6.2 Simple-mode autonomous prompt (`src/constants/prompts.ts:472-474`, verbatim)

```
\nYou are an autonomous agent. Use the available tools to do useful work.

${CYBER_RISK_INSTRUCTION}
```

### 6.3 `getProactiveSection()` addendum (`src/constants/prompts.ts:864-913`, verbatim)

```
# Autonomous work

You are running autonomously. You will receive `<${TICK_TAG}>` prompts that keep you alive between turns — just treat them as "you're awake, what now?" The time in each `<${TICK_TAG}>` is the user's current local time. Use it to judge the time of day — timestamps from external tools (Slack, GitHub, etc.) may be in a different timezone.

Multiple ticks may be batched into a single message. This is normal — just process the latest one. Never echo or repeat tick content in your response.

## Pacing

Use the ${SLEEP_TOOL_NAME} tool to control how long you wait between actions. Sleep longer when waiting for slow processes, shorter when actively iterating. Each wake-up costs an API call, but the prompt cache expires after 5 minutes of inactivity — balance accordingly.

**If you have nothing useful to do on a tick, you MUST call ${SLEEP_TOOL_NAME}.** Never respond with only a status message like "still waiting" or "nothing to do" — that wastes a turn and burns tokens for no reason.

## First wake-up

On your very first tick in a new session, greet the user briefly and ask what they'd like to work on. Do not start exploring the codebase or making changes unprompted — wait for direction.

## What to do on subsequent wake-ups

Look for useful work. A good colleague faced with ambiguity doesn't just stop — they investigate, reduce risk, and build understanding. Ask yourself: what don't I know yet? What could go wrong? What would I want to verify before calling this done?

Do not spam the user. If you already asked something and they haven't responded, do not ask again. Do not narrate what you're about to do — just do it.

If a tick arrives and you have no useful action to take (no files to read, no commands to run, no decisions to make), call ${SLEEP_TOOL_NAME} immediately. Do not output text narrating that you're idle — the user doesn't need "still waiting" messages.

## Staying responsive

When the user is actively engaging with you, check for and respond to their messages frequently. Treat real-time conversations like pairing — keep the feedback loop tight. If you sense the user is waiting on you (e.g., they just sent a message, the terminal is focused), prioritize responding over continuing background work.

## Bias toward action

Act on your best judgment rather than asking for confirmation.

- Read files, search code, explore the project, run tests, check types, run linters — all without asking.
- Make code changes. Commit when you reach a good stopping point.
- If you're unsure between two reasonable approaches, pick one and go. You can always course-correct.

## Be concise

Keep your text output brief and high-level. The user does not need a play-by-play of your thought process or implementation details — they can see your tool calls. Focus text output on:
- Decisions that need the user's input
- High-level status updates at natural milestones (e.g., "PR created", "tests passing")
- Errors or blockers that change the plan

Do not narrate each step, list every file you read, or explain routine actions. If you can say it in one sentence, don't use three.

## Terminal focus

The user context may include a `terminalFocus` field indicating whether the user's terminal is focused or unfocused. Use this to calibrate how autonomous you are:
- **Unfocused**: The user is away. Lean heavily into autonomous action — make decisions, explore, commit, push. Only pause for genuinely irreversible or high-risk actions.
- **Focused**: The user is watching. Be more collaborative — surface choices, ask before committing to large changes, and keep your output concise so it's easy to follow in real time.${BRIEF_PROACTIVE_SECTION && briefToolModule?.isBriefEnabled() ? `\n\n${BRIEF_PROACTIVE_SECTION}` : ''}
```

Interpolated slots: `${TICK_TAG}` = `'tick'` (`src/constants/xml.ts:25`);
`${SLEEP_TOOL_NAME}` = `'Sleep'` (`src/tools/SleepTool/prompt.ts:3`);
`${BRIEF_PROACTIVE_SECTION}` is appended only when both Kairos brief flag is
on AND `briefToolModule.isBriefEnabled()` (Kairos territory — see 32).

### 6.4 `terminalFocus` user-context line (`src/screens/REPL.tsx:2776-2778`, verbatim — note the `—` em-dash)

```ts
...((feature('PROACTIVE') || feature('KAIROS')) && proactiveModule?.isProactiveActive() && !terminalFocusRef.current ? {
  terminalFocus: 'The terminal is unfocused — the user is not actively watching.'
} : {})
```

### 6.5 Tick content format (`src/cli/print.ts:1845`, verbatim)

```ts
const tickContent = `<${TICK_TAG}>${new Date().toLocaleTimeString()}</${TICK_TAG}>`
```

The enqueued message uses `priority: 'later'`, `isMeta: true`, `mode: 'prompt'`
(`src/cli/print.ts:1846-1852`). `isMeta: true` keeps it out of the
session-title heuristic (`screens/REPL.tsx:2685`).

### 6.6 Ephemeral-progress registration (`src/utils/sessionStorage.ts:186-193`, verbatim)

```ts
const EPHEMERAL_PROGRESS_TYPES = new Set([
  'bash_progress',
  'powershell_progress',
  'mcp_progress',
  ...(feature('PROACTIVE') || feature('KAIROS')
    ? (['sleep_progress'] as const)
    : []),
])
```

### 6.7 Resume-list synthetic title (`src/utils/sessionStorage.ts:4911-4912`, verbatim)

```ts
if ((feature('PROACTIVE') || feature('KAIROS')) && hasTickMessages)
  return 'Proactive session'
```

### 6.8 Footer countdown UI (`src/components/PromptInput/PromptInputFooterLeftSide.tsx:118-125`, verbatim)

```tsx
t4 = <Text dimColor={true}>waiting{" "}{t3}</Text>;
```

`t3` is `formatDuration(remainingSeconds * 1000, { mostSignificantOnly: true })`.

### 6.9 CLI option (`src/main.tsx:3833`, verbatim)

```ts
program.addOption(new Option('--proactive', 'Start in proactive autonomous mode'));
```

### 6.10 Wakeup callback Zod schema — N/A. The proactive module's source is
absent, and the tick is delivered as a synthetic user prompt rather than via a
typed callback. The `set_proactive` control-protocol payload is plain
`{subtype: string; enabled: boolean}` (no Zod schema declared at the call
site — see `src/cli/print.ts:3879-3882`).

### 6.11 Constants table

| Constant | Value | Citation |
|---|---|---|
| `TICK_TAG` | `'tick'` | `src/constants/xml.ts:25` |
| `SLEEP_TOOL_NAME` | `'Sleep'` | `src/tools/SleepTool/prompt.ts:3` |
| Tick `priority` | `'later'` | `src/cli/print.ts:1850` |
| Tick `isMeta` | `true` | `src/cli/print.ts:1851` |
| Tick `mode` | `'prompt'` | `src/cli/print.ts:1847` |
| Tick scheduler delay | `0` ms (`setTimeout(0)`) | `src/cli/print.ts:1837,1854` |
| Footer countdown re-render interval | `1000` ms | `src/components/PromptInput/PromptInputFooterLeftSide.tsx:91` |
| `activateProactive` source string (only value seen) | `'command'` | `src/main.tsx:4618`; `src/cli/print.ts:544,3885` |
| `minSleepDurationMs` settings key | optional `z.number().nonnegative().int()` (no default) | `src/utils/settings/types.ts:843-851` (gate `PROACTIVE \|\| KAIROS`; semantics owned by spec 19) |
| `maxSleepDurationMs` settings key | optional `z.number().int().min(-1)`; `-1` = indefinite (waits for user input) | `src/utils/settings/types.ts:852-861` (gate `PROACTIVE \|\| KAIROS`; semantics owned by spec 19) |
| Default sleep duration / max chain length | unknown — proactive module source absent | (§12) |

### 6.12 User-facing strings (verbatim)

- `'Start in proactive autonomous mode'` — `--proactive` help (§6.9).
- `'The terminal is unfocused — the user is not actively watching.'` — user-context terminalFocus line (§6.4).
- `'Proactive session'` — synthetic session title for tick-only sessions (§6.7).
- `'waiting {t3}'` — footer countdown (§6.8).
- `'You are in proactive mode. Take initiative — …'` and `'You are an autonomous agent. …'` — system prompt fragments (§6.1, §6.2).
- `'You are running autonomously. …'` — `getProactiveSection()` (§6.3).
- No `"I'm taking a break"` string is present in source; reference search returns no match.

---

## §7. Side Effects & I/O

- Inside the headless run loop only: enqueues a synthetic prompt onto the
  shared command queue (`src/cli/print.ts:1846-1852`) and re-enters `run()`.
  No filesystem, network, or process spawn from this spec's deltas.
- Reads `process.env.CLAUDE_CODE_PROACTIVE` (`src/main.tsx:2199,4614`,
  `src/cli/print.ts:542`) — only as `isEnvTruthy(...)`.
- The REPL footer schedules a 1-second `setInterval` while `nextTickAt` is
  non-null (`PromptInputFooterLeftSide.tsx:91`). Cleared in the effect
  cleanup.
- Trust boundary: the synthetic tick is `isMeta: true`, mode `'prompt'`, and
  is delivered into the same queue as user input. Permission prompts and
  hooks treat it like any other prompt — there is no proactive-specific
  permission bypass.

---

## §8. Feature Flags & Variants

| Flag / gate | On → off delta |
|---|---|
| `feature('PROACTIVE') \|\| feature('KAIROS')` | This entire spec is gated. Off → none of the deltas fire; `SleepTool` is excluded from the tool pool; the `proactive` slash command, `--proactive` CLI option, autonomous prompt path, footer countdown, `terminalFocus` injection, async-agent forcing, drain-priority upgrade, and ephemeral `sleep_progress` registration are all absent. |
| `--proactive` / `CLAUDE_CODE_PROACTIVE` | User opt-in. Without one of these (or the `set_proactive` RPC) the module is never activated even when the flag is on. |
| `coordinatorModule?.isCoordinatorMode()` (gate `feature('COORDINATOR_MODE')`) | When true and proactive is requested, the REPL system-prompt fragment is **not** appended (`src/main.tsx:2194-2199`). The asymmetry is intentional: prompt-append is suppressed, but `maybeActivateProactive` (boot-time activation at `src/main.tsx:4611-4621`) and the slash-command/tool-registry wiring are **not** suppressed by coordinator mode. The rationale is the inline comment at `src/main.tsx:2194-2196` (verbatim): `// Coordinator mode has its own system prompt and filters out Sleep, so / // the generic proactive prompt would tell it to call a tool it can't / // access and conflict with delegation instructions.` The coordinator-side Sleep-filter mechanism itself lives in spec 30. |
| `feature('KAIROS') \|\| feature('KAIROS_BRIEF')` | Augments the proactive prompt with Brief instructions: changes `briefVisibility` text in §6.1 and appends `BRIEF_PROACTIVE_SECTION` in §6.3. Kairos-specific overlays — see 32. |
| `feature('FORK_SUBAGENT')` (`forceAsync`) and KAIROS `assistantForceAsync` | Independent OR'd conjuncts in `AgentTool.tsx:567`. Proactive is one of several reasons agents are forced async. |

`USER_TYPE === 'ant'`: this spec's deltas do not gate on USER_TYPE. The
`proactive` slash command itself may further restrict — its source is absent.

---

## §9. Error Handling & Edge Cases

- **API error storms**: the REPL sets `setContextBlocked(true)` on assistant
  messages with `isApiErrorMessage` (`src/screens/REPL.tsx:2634-2637`); the
  proactive module is presumed to suppress tick scheduling while blocked
  (per the comment "Block ticks on API errors to prevent tick → error → tick
  runaway loops … Cleared on compact boundary or successful response below").
  The exact suppression mechanism lives in the absent `proactive/index.js`.
- **User Esc mid-Sleep**: `Sleep` has `interruptBehavior: 'cancel'` (per
  `src/utils/handlePromptSubmit.ts:319-320` comment). On submit-while-loading
  with `hasInterruptibleToolInProgress`, the harness aborts the current
  turn and enqueues the user input.
- **Session resume with tick-only history**: `getResumeListTitle` returns
  `'Proactive session'` so the entry isn't filtered out by enrichLogs
  (`src/utils/sessionStorage.ts:4906-4912`).
- **Coordinator overlay**: silently skips the proactive prompt-append step
  to avoid telling the model to call a tool the coordinator filters out.
  The activation call itself is not skipped, so the slash command and tool
  registry behave normally.
- **SDK transport injecting env after argv parse**: caught by the
  `cli/print.ts:534-545` fallback that re-checks `CLAUDE_CODE_PROACTIVE` and
  calls `activateProactive('command')` if not already active.
- **`onlySleepToolActive`**: spinner is hidden when every in-progress
  tool-use on the latest assistant message is `Sleep`
  (`src/screens/REPL.tsx:1654-1660,1682`).
- **`set_proactive` payload is not Zod-validated**: the control-protocol
  handler at `src/cli/print.ts:3879-3882` casts
  `message.request as unknown as { subtype: string; enabled: boolean }`
  with no Zod parser at the call site (other control-protocol subtypes in
  the same file do use Zod-parsed payloads). A non-boolean `enabled` value
  is truthy-coerced into the activate branch. Candidate for
  `BUGS-IN-SOURCE.md`.

No verbatim user-facing error strings are emitted from this spec's deltas.

---

## §10. Telemetry & Observability

- No analytics events are emitted from the proactive module's call-sites in
  the leak. The activation call accepts a `source` string (only `'command'`
  observed) which the absent module presumably logs internally.
- `tengu_cancel` (`src/utils/handlePromptSubmit.ts:325-330`) is logged when
  an interruptible tool (e.g. Sleep) is aborted by user submit, with
  `source: 'interrupt_on_submit'`. This is owned by spec 04 — not specific
  to proactive — but it is the path Sleep aborts flow through.
- **Co-enabled `KAIROS` / `KAIROS_BRIEF`**: when proactive runs alongside
  the brief subsystem (32), Brief's `tengu_brief_send`,
  `tengu_brief_mode_enabled`, and `tengu_brief_mode_toggled` events become
  observable in Datadog (allow-listed at
  `src/services/analytics/datadog.ts:29-31`). These are emitted from the
  Brief path, not from any proactive call site — see specs 32 and 26.

---

## §11. Reimplementation Checklist

A reimplementer of proactive mode must preserve:

- [ ] All call sites are gated by `feature('PROACTIVE') || feature('KAIROS')` —
      not just `feature('PROACTIVE')`.
- [ ] Activation requires opt-in (CLI flag, env, or RPC) on top of the build flag.
- [ ] Headless tick uses `setTimeout(0)`, `mode: 'prompt'`, `priority: 'later'`,
      `isMeta: true`, `value = '<tick>${new Date().toLocaleTimeString()}</tick>'`.
- [ ] Tick re-arming gates on `peek(isMainThread) === undefined` AND not paused
      AND not closed.
- [ ] `pauseProactive()` on Esc; `resumeProactive()` on submit.
- [ ] `setContextBlocked(true)` on assistant `isApiErrorMessage`; clear on next
      successful assistant message, on any compact-boundary path
      (handleMessageFromStream, manual `/compact`, partial-compact), and on
      `/clear`.
- [ ] When proactive is active and `mainThreadAgentDefinition` is set, agent
      prompt is **appended** (under `# Custom Agent Instructions`) to defaults
      rather than replacing them.
- [ ] When proactive is active in simple/headless prompt mode, return the
      autonomous-agent system prompt set ending with `getProactiveSection()`.
- [ ] Coordinator mode suppresses the REPL prompt-append (but does not
      suppress activation).
- [ ] AgentTool spawns with `proactiveModule?.isProactiveActive()` true must
      run async (so the main loop can keep ticking).
- [ ] `query.ts` drain raises threshold to `'later'` exactly when the
      just-finished tool-use round contained a `Sleep` invocation.
- [ ] `sleep_progress` is in the ephemeral-progress set.
- [ ] Resume-list title for tick-only sessions is `'Proactive session'`.
- [ ] Footer subscribes `subscribeToProactiveChanges` for `nextTickAt` and
      renders `waiting <duration>` updated every 1s.
- [ ] `terminalProgressBarEnabled` is suppressed while active.
- [ ] User-context gains `terminalFocus: 'The terminal is unfocused — the user is not actively watching.'` (em-dash literal, `—`) when active and terminal unfocused.
- [ ] `--proactive` CLI option exists only when the flag is on.
- [ ] Spinner is hidden when only `Sleep` tools are in flight.
- [ ] The proactive system-prompt fragments in §6.1, §6.2, §6.3 are
      bit-exact (including Unicode em-dashes, escaped newlines, and the
      `${briefVisibility}` interpolation that swaps `'Call SendUserMessage…'`
      vs. `'The user will see any text you output.'`).

---

## §12. Open Questions / Unknowns

- **`src/proactive/index.js` is absent from the leaked tree.** Inferred
  exports are documented in §3 from caller usage. Internal mechanics not
  resolvable from source: tick-interval default (defaults sleep duration),
  pause-state semantics, max chain length, listener fan-out for
  `subscribeToProactiveChanges`, `getNextTickAt` math, context-block
  consumer logic. These should be re-cited if the module is recovered or
  reconstructed from the bundled `main.tsx`.
- **`src/commands/proactive.js` is absent.** The `/proactive` slash command's
  trigger keyword, args, display category, and command body are unknown.
  `processSlashCommand.tsx:589` lists `/proactive` alongside `/usage` and
  `/rename` as `display:'system'` in a comment; the command file itself is
  not in the leak.
- The `activateProactive(source)` parameter has only one observed value
  (`'command'`). Whether the absent module uses it (e.g. for telemetry
  source tagging) is not visible from callers.
- Whether `setContextBlocked` directly suppresses ticks, or only prevents
  re-arming, is inferred from comments at `screens/REPL.tsx:2631-2633` and
  not confirmed against a reader call.
- `proactive` MCP channel notification interaction: a comment at
  `src/services/mcp/channelNotification.ts:9` mentions "SleepTool polls
  hasCommandsInQueue() and wakes within 1s" — this poll cadence and the
  exact wake mechanism live in `SleepTool` (spec 19) and the absent
  `proactive/index.js`.
- Wakeup-callback Zod schema: not declared at any caller; any schema, if
  present, lives in the absent module.
- Whether `--proactive` and `CLAUDE_CODE_PROACTIVE` differ at all beyond
  surface (e.g. whether one persists across `/clear`) is not visible from
  callers.
- Bundled `src/main.tsx` may contain additional proactive references not
  reachable by the grep patterns in §0.
