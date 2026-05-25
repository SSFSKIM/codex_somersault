# 06 — Cost & Token Tracking

> Owner: sub-B4 · Status: done · Last updated: 2026-05-08
> Adjacent: 03 (query engine emits usage), 04 (turn pipeline holds turn budget snapshot), 22 (Anthropic SDK client wiring), 26 (analytics pipeline)

---

## §1 Purpose & Boundaries

This spec covers the in-process aggregation of **per-API-call usage emitted by the streaming/non-streaming Anthropic message loop into per-model and total session costs**, the **session-end summary hook**, and the **`TOKEN_BUDGET` per-turn auto-continue mechanism**.

In scope:
- `src/cost-tracker.ts` (323 LOC) — session-cost mutator + formatter + project-config persistence
- `src/costHook.ts` (22 LOC) — React `useEffect`-based `process.on('exit')` summary printer
- `src/services/tokenEstimation.ts` (495 LOC) — token-count *estimation* (rough char/byte heuristic + Haiku-fallback API count + Bedrock CountTokens) used by compaction and pre-flight budget checks
- The cost-related slice of `src/bootstrap/state.ts` (`STATE.totalCostUSD`, `STATE.modelUsage`, `STATE.hasUnknownModelCost`, `STATE.costCounter`, `STATE.tokenCounter`)
- `src/utils/modelCost.ts` — pricing tables and `calculateUSDCost`
- `src/utils/billing.ts` — gate for whether the on-exit summary is printed
- `src/utils/advisor.ts::getAdvisorUsage` — sub-iterations whose tokens get re-attributed
- The two `feature('TOKEN_BUDGET')` gates in `src/query.ts` (lines 280 and 1308) plus `src/query/tokenBudget.ts` and `src/utils/tokenBudget.ts`

Out of scope (cite spec #):
- The streaming SSE event loop that *produces* a `BetaUsage` object → **03**
- Anthropic SDK client construction, beta header logic, retry/backoff → **22**
- The `tengu_*` analytics pipeline (this spec cites emission sites only) → **26**
- The per-turn pipeline fan-out (system reminders, hook fan-out) → **04**
- Compaction triggered when token *thresholds* are crossed → **07** (this spec only documents the per-turn budget continuation gate, not auto-compact)
- Side-query classifier accounting → **04**/**09** (`utils/permissions/permissions.ts:766` explicitly notes "does NOT call addToTotalSessionCost")

---

## §2 Source Coverage Inventory

| Path | Coverage | Notes |
|---|---|---|
| `src/cost-tracker.ts` | fully-read | All 323 lines |
| `src/costHook.ts` | fully-read | All 22 lines |
| `src/services/tokenEstimation.ts` | fully-read | All 495 lines |
| `src/utils/modelCost.ts` | fully-read | All 232 lines |
| `src/utils/billing.ts` | fully-read | All 78 lines |
| `src/utils/advisor.ts` | fully-read | 1–146 |
| `src/query/tokenBudget.ts` | fully-read | All 93 lines |
| `src/utils/tokenBudget.ts` | fully-read | All 73 lines |
| `src/bootstrap/state.ts` | sampled | Cost state slice (50–170, 540–745, 826–916, 968–1015) |
| `src/services/api/claude.ts` | sampled | 2240–2270, 2810–2830 (cost-emission sites) |
| `src/services/vcr.ts` | sampled | 122, 160–173, 382 (VCR re-attribution + token-count VCR) |
| `src/QueryEngine.ts` | grep-inspected | Only imports from `cost-tracker.js`; cost mutations live in claude.ts |
| `src/query.ts` | sampled | 270–330, 1290–1360 (budget gate sites) |
| `src/screens/REPL.tsx` | grep-inspected | 5, 2134, 2893, 2959 (turn-budget snapshot drivers), 3823 (`useCostSummary` mount) |
| `src/utils/permissions/permissions.ts:766` | grep-inspected | Comment confirming classifier exclusion |
| `src/entrypoints/agentSdkTypes.ts` | missing | `ModelUsage` type — referenced but not enumerated here; per HANDOFF gotchas table, locate or record as missing |

---

## §3 Architecture & Data Flow

```
                        ┌─────────────────────────────────────────────────┐
   API stream (claude.ts)│  message_delta → BetaUsage (input_tokens, …)   │
                        └─────────────────────────────────────────────────┘
                                            │ calculateUSDCost(model, usage)
                                            ▼
                    ┌──────────────────────────────────────────────────────┐
                    │ addToTotalSessionCost(cost, usage, model)            │
                    │   ├─ addToTotalModelUsage → STATE.modelUsage[model]  │
                    │   ├─ addToTotalCostState → STATE.totalCostUSD       │
                    │   ├─ getCostCounter().add(cost, attrs)               │
                    │   ├─ getTokenCounter().add(input/output/cache_*)    │
                    │   └─ for advisor in getAdvisorUsage(usage):          │
                    │        logEvent('tengu_advisor_tool_token_usage')   │
                    │        addToTotalSessionCost(…) recursively         │
                    └──────────────────────────────────────────────────────┘
                                            │
                                            ▼
                Read accessors: getTotalCostUSD, getTotalInputTokens, …
                                            │
                       ┌────────────────────┴────────────────────┐
                       ▼                                          ▼
   process.on('exit')  formatTotalCost()              saveCurrentSessionCosts()
   useCostSummary →    printed iff hasConsoleBillingAccess()    → projectConfig
   stdout
```

Three **independent** sinks consume the same `STATE.modelUsage`/`STATE.totalCostUSD`:

1. **Session summary** (`useCostSummary`) prints `formatTotalCost()` to stdout on `process.exit`.
2. **Project-config persistence** (`saveCurrentSessionCosts`) writes a snapshot to project config under `lastCost`, `lastModelUsage`, etc., so a resumed session restores via `restoreCostStateForSession`.
3. **OpenTelemetry counters** (`costCounter` / `tokenCounter` from `setMeter`) re-emit each addition with `{ model, type }` (and `{ speed: 'fast' }` when `isFastModeEnabled() && usage.speed === 'fast'`).

The **token-budget feature** (`TOKEN_BUDGET`) is a **per-turn** decision over `getTurnOutputTokens()` (cumulative output token delta vs `outputTokensAtTurnStart`), not a global gate. Its turn baseline is taken at `snapshotOutputTokensForTurn(budget)` and its accumulator reads through `getTotalOutputTokens()` summed across the whole `STATE.modelUsage` map — so output tokens billed to a sub-agent or advisor count toward the parent turn's budget consumption.

---

## §4 Subsystem Map

| Concern | Where |
|---|---|
| Per-call cost math | `src/utils/modelCost.ts::tokensToUSDCost`, `calculateUSDCost`, `getModelCosts` |
| Pricing constants | `src/utils/modelCost.ts:36–87, 104–126` |
| Session aggregator | `src/cost-tracker.ts::addToTotalSessionCost` (`:278–323`) |
| Per-model accumulator | `src/cost-tracker.ts::addToTotalModelUsage` (`:250–276`) |
| State slice | `src/bootstrap/state.ts` `STATE.totalCostUSD`, `STATE.modelUsage`, `STATE.hasUnknownModelCost`; mutators `addToTotalCostState` (`:557–564`), `setHasUnknownModelCost` (`:745–747`), `resetCostState` (`:864–875`), `setCostStateForRestore` (`:881–916`) |
| Persisted-resume snapshot | `src/cost-tracker.ts::getStoredSessionCosts` (`:87–123`), `restoreCostStateForSession` (`:130–137`), `saveCurrentSessionCosts` (`:143–175`) |
| Session-end print hook | `src/costHook.ts::useCostSummary` |
| Billing-access gate | `src/utils/billing.ts::hasConsoleBillingAccess` |
| Output formatter | `src/cost-tracker.ts::formatCost` (`:177–179`), `formatModelUsage` (`:181–226`), `formatTotalCost` (`:228–244`) |
| Token estimation: rough | `src/services/tokenEstimation.ts::roughTokenCountEstimation` (`:203–208`), `bytesPerTokenForFileType` (`:215–224`), `roughTokenCountEstimationForMessages`/`Block` (`:327–435`) |
| Token estimation: API CountTokens | `src/services/tokenEstimation.ts::countTokensWithAPI` (`:124–137`), `countMessagesTokensWithAPI` (`:140–201`) |
| Token estimation: Haiku-side `messages.create` fallback | `src/services/tokenEstimation.ts::countTokensViaHaikuFallback` (`:251–325`) |
| Token estimation: Bedrock `CountTokens` | `src/services/tokenEstimation.ts::countTokensWithBedrock` (`:437–495`) |
| OTel counters | `src/bootstrap/state.ts::setMeter` (`:968–975`), `getCostCounter` (`:1009`), `getTokenCounter` (`:1013`) |
| Advisor sub-usage extraction | `src/utils/advisor.ts::getAdvisorUsage` (`:115–128`) |
| Per-turn budget snapshot | `src/bootstrap/state.ts::snapshotOutputTokensForTurn` (`:733–737`), `getTurnOutputTokens` (`:726–728`), `getCurrentTurnTokenBudget` (`:729–731`), `getBudgetContinuationCount` / `incrementBudgetContinuationCount` (`:738–743`) |
| `TOKEN_BUDGET` decision | `src/query/tokenBudget.ts::checkTokenBudget` |
| `TOKEN_BUDGET` user-prompt parser | `src/utils/tokenBudget.ts::parseTokenBudget`, `findTokenBudgetPositions`, `getBudgetContinuationMessage` |
| `TOKEN_BUDGET` query-loop integration | `src/query.ts:280` (tracker creation), `src/query.ts:1308–1355` (decision gate) |
| `TOKEN_BUDGET` REPL drivers | `src/screens/REPL.tsx:2134, 2893, 2959` (snapshot calls) |

---

## §5 Algorithms (Pseudocode)

### 5.1 `addToTotalSessionCost(cost, usage, model)` — `src/cost-tracker.ts:278–323`

```
function addToTotalSessionCost(cost: number, usage: BetaUsage, model: string) -> number:
  modelUsage = addToTotalModelUsage(cost, usage, model)
  addToTotalCostState(cost, modelUsage, model)        // STATE.modelUsage[model] = modelUsage
                                                       // STATE.totalCostUSD   += cost

  attrs = (isFastModeEnabled() && usage.speed === 'fast')
            ? { model, speed: 'fast' }
            : { model }

  costCounter?.add(cost, attrs)
  tokenCounter?.add(usage.input_tokens,                            { ...attrs, type: 'input' })
  tokenCounter?.add(usage.output_tokens,                           { ...attrs, type: 'output' })
  tokenCounter?.add(usage.cache_read_input_tokens     ?? 0,        { ...attrs, type: 'cacheRead' })
  tokenCounter?.add(usage.cache_creation_input_tokens ?? 0,        { ...attrs, type: 'cacheCreation' })

  totalCost = cost
  for advisorUsage in getAdvisorUsage(usage):                       // iterations array filtered to type==='advisor_message'
    advisorCost = calculateUSDCost(advisorUsage.model, advisorUsage)
    logEvent('tengu_advisor_tool_token_usage', {
      advisor_model:               advisorUsage.model,
      input_tokens:                advisorUsage.input_tokens,
      output_tokens:               advisorUsage.output_tokens,
      cache_read_input_tokens:     advisorUsage.cache_read_input_tokens     ?? 0,
      cache_creation_input_tokens: advisorUsage.cache_creation_input_tokens ?? 0,
      cost_usd_micros:             round(advisorCost * 1_000_000),
    })
    totalCost += addToTotalSessionCost(advisorCost, advisorUsage, advisorUsage.model)  // RECURSIVE
  return totalCost
```

Notes:
- `addToTotalModelUsage` (`:250–276`) is **mutating**: it reads `getUsageForModel(model)` (or initialises a zero-filled `ModelUsage`), increments each per-model field by the new usage's fields, and **overwrites** `contextWindow = getContextWindowForModel(model, getSdkBetas())` and `maxOutputTokens = getModelMaxOutputTokens(model).default` on every call (i.e. these reflect the *latest* known values, not the value at first observation).
- Advisor usage is added **on top of** the parent-call cost. The advisor iterations are billed *separately* by the server: each `iterations[*]` entry of `type === 'advisor_message'` carries its own `model`, `input_tokens`, `output_tokens`, `cache_*_input_tokens`. The recursion attributes those tokens to the advisor's *own* model entry in `STATE.modelUsage`, not the parent's, so per-model accumulators stay coherent and `STATE.totalCostUSD` correctly reflects parent + advisor (no double-count). This is consistent with §10 invariant 7: `tengu_advisor_tool_token_usage` reports only the advisor iteration cost, and the recursive `addToTotalSessionCost` call is what attributes that cost to its own model. The producer-side contract (whether the parent's `usage.input_tokens` already includes advisor tokens) is owned by spec 22 — but for cost-tracker's purposes, each iteration is treated as an independent billable unit keyed on its own `model` field.
- `usage.server_tool_use?.web_search_requests ?? 0` is added to `modelUsage.webSearchRequests` but **not** emitted to `tokenCounter` (it's a request count, not tokens).

### 5.2 `calculateUSDCost(resolvedModel, usage)` — `src/utils/modelCost.ts:177–180`

```
function calculateUSDCost(resolvedModel, usage) -> number:
  modelCosts = getModelCosts(resolvedModel, usage)
  return tokensToUSDCost(modelCosts, usage)

function tokensToUSDCost(modelCosts, usage) -> number:        // helper at modelCost.ts above this line
  return ( usage.input_tokens                       / 1_000_000) * modelCosts.inputTokens
       + ( usage.output_tokens                      / 1_000_000) * modelCosts.outputTokens
       + ((usage.cache_read_input_tokens     ?? 0)  / 1_000_000) * modelCosts.promptCacheReadTokens
       + ((usage.cache_creation_input_tokens ?? 0)  / 1_000_000) * modelCosts.promptCacheWriteTokens
       + ( usage.server_tool_use?.web_search_requests ?? 0)        * modelCosts.webSearchRequests

function getModelCosts(model, usage) -> ModelCosts:
  shortName = getCanonicalName(model)
  if shortName === canonical(CLAUDE_OPUS_4_6_CONFIG.firstParty):
    return getOpus46CostTier(usage.speed === 'fast')      // see §6 constants
  costs = MODEL_COSTS[shortName]
  if costs === undefined:
    logEvent('tengu_unknown_model_cost', { model, shortName })
    setHasUnknownModelCost()
    return MODEL_COSTS[canonical(getDefaultMainLoopModelSetting())] ?? COST_TIER_5_25  // DEFAULT_UNKNOWN_MODEL_COST
  return costs

function getOpus46CostTier(fastMode: boolean) -> ModelCosts:
  if isFastModeEnabled() && fastMode:                     // BOTH must be true
    return COST_TIER_30_150
  return COST_TIER_5_25
```

The Opus 4.6 fast-mode branch is the *only* place pricing depends on `usage.speed`. For all other models the tier is a static lookup.

### 5.2a `calculateCostFromTokens(model, tokens)` — `src/utils/modelCost.ts:186–202`

```
function calculateCostFromTokens(model, { inputTokens, outputTokens, cacheReadInputTokens, cacheCreationInputTokens }) -> number:
  usage = { input_tokens: inputTokens,
            output_tokens: outputTokens,
            cache_read_input_tokens: cacheReadInputTokens,
            cache_creation_input_tokens: cacheCreationInputTokens } as Usage
  return calculateUSDCost(model, usage)            // synthesises a Usage; web_search_requests defaults to 0
```

This is the **only** caller surface that bypasses `addToTotalSessionCost` — used by side-query/classifier consumers (per §1, §8 "side-query / classifier costs are intentionally excluded"; spec 09 owns the policy). It returns a USD figure but does **not** mutate `STATE.totalCostUSD`, `STATE.modelUsage`, OTel counters, or emit `tengu_advisor_tool_token_usage`. Callers that want session-cost attribution must call `addToTotalSessionCost` separately.

### 5.3 `formatTotalCost()` — `src/cost-tracker.ts:228–244`, `formatModelUsage` `:181–226`

The output is `chalk.dim(...)` over a multi-line string. See §6 verbatim assets for the exact format strings. `formatCost(cost)` (`:177–179`):

```
if cost > 0.5:
  display = '$' + round(cost, 100).toFixed(2)        // round to 2 decimals
else:
  display = '$' + cost.toFixed(maxDecimalPlaces)     // default maxDecimalPlaces = 4
```

`round(n, precision) = Math.round(n * precision) / precision`. With `precision = 100`, `round(n, 100)` rounds to 2 decimals; `.toFixed(2)` then pads to 2 places.

`formatModelUsage` accumulates per-`shortName` (`getCanonicalName(model)`) entries and emits one `{shortName}:` row per short name, padded to width 21 (`${shortName}:`.padStart(21)`). The accumulator initialises `contextWindow` and `maxOutputTokens` to 0 in the rollup but never writes them back; only individual model entries in `STATE.modelUsage` carry meaningful values.

### 5.4 `useCostSummary` (`costHook.ts`)

```
useCostSummary(getFpsMetrics?):
  useEffect(() => {
    f = () => {
      if hasConsoleBillingAccess():
        process.stdout.write('\n' + formatTotalCost() + '\n')
      saveCurrentSessionCosts(getFpsMetrics?.())
    }
    process.on('exit', f)
    return () => process.off('exit', f)
  }, [])      // mount-once
```

Mounted by `src/screens/REPL.tsx:3823` (`useCostSummary(useFpsMetrics())`). Runs synchronously inside Node's `'exit'` event — no async work. The summary is printed only when `hasConsoleBillingAccess()` returns true, but `saveCurrentSessionCosts` runs unconditionally.

### 5.5 `hasConsoleBillingAccess()` — `src/utils/billing.ts:10–44`

```
function hasConsoleBillingAccess() -> boolean:
  if isEnvTruthy(process.env.DISABLE_COST_WARNINGS):  return false
  if isClaudeAISubscriber():                           return false   // hide for Max/Pro/Team users
  if !authSource.hasToken && getAnthropicApiKey() === null: return false   // logged out
  config = getGlobalConfig()
  orgRole       = config.oauthAccount?.organizationRole
  workspaceRole = config.oauthAccount?.workspaceRole
  if !orgRole || !workspaceRole:  return false        // grandfathered users without re-auth
  return ['admin','billing'].includes(orgRole)
      || ['workspace_admin','workspace_billing'].includes(workspaceRole)
```

### 5.6 Token-budget per-turn auto-continue — `src/query/tokenBudget.ts::checkTokenBudget`

```
COMPLETION_THRESHOLD = 0.9
DIMINISHING_THRESHOLD = 500

function checkTokenBudget(tracker, agentId, budget, globalTurnTokens) -> Decision:
  if agentId !== undefined or budget === null or budget <= 0:
    return { action: 'stop', completionEvent: null }                  // disabled in subagents

  turnTokens = globalTurnTokens
  pct        = round( turnTokens / budget * 100 )
  delta      = globalTurnTokens - tracker.lastGlobalTurnTokens

  isDiminishing = tracker.continuationCount >= 3
                && delta                  < 500
                && tracker.lastDeltaTokens < 500

  if not isDiminishing and turnTokens < budget * 0.9:
    tracker.continuationCount  += 1
    tracker.lastDeltaTokens     = delta
    tracker.lastGlobalTurnTokens = globalTurnTokens
    return {
      action: 'continue',
      nudgeMessage: getBudgetContinuationMessage(pct, turnTokens, budget),
      continuationCount: tracker.continuationCount,
      pct, turnTokens, budget,
    }

  if isDiminishing or tracker.continuationCount > 0:
    return { action: 'stop', completionEvent: {
      continuationCount, pct, turnTokens, budget,
      diminishingReturns: isDiminishing,
      durationMs: Date.now() - tracker.startedAt,
    } }

  return { action: 'stop', completionEvent: null }
```

Integration in `src/query.ts:1308–1355`:
- Created at `query.ts:280`: `budgetTracker = feature('TOKEN_BUDGET') ? createBudgetTracker() : null`
- Inside the main loop, after the stop-hook gate and before the "completed" return, the gate runs (`query.ts:1308–1355`):
  - On `'continue'`: appends a `createUserMessage({ content: decision.nudgeMessage, isMeta: true })` to the message list, calls `incrementBudgetContinuationCount()`, sets `transition.reason = 'token_budget_continuation'`, and `continue`s the loop with `hasAttemptedReactiveCompact: false` and `stopHookActive: undefined`.
  - On `'stop'` with a non-null `completionEvent`: `logEvent('tengu_token_budget_completed', { ...decision.completionEvent, queryChainId, queryDepth })`.
  - On `'stop'` with `completionEvent === null` (no continuations were ever issued): silent stop.
- Returns from the query loop with `{ reason: 'completed' }` after the gate decides stop.
- `getCurrentTurnTokenBudget()` is read by `query.ts` (the budget itself) and `getTurnOutputTokens()` provides the accumulator.

The budget value comes from REPL parsing at `src/screens/REPL.tsx:2893–2895`: `parsedBudget = parseTokenBudget(userPrompt)` (regex from `src/utils/tokenBudget.ts`). It's snapshotted via `snapshotOutputTokensForTurn(parsedBudget ?? getCurrentTurnTokenBudget())`. A `null`/`0`/`undefined` budget disables the gate.

### 5.7 Token estimation algorithm

Three modes, fallbacks chained at the call site (not in this file):

1. **Rough char-count** (`roughTokenCountEstimation`): `Math.round(content.length / bytesPerToken)` with `bytesPerToken=4` default; `2` for `json`/`jsonl`/`jsonc` extensions.
2. **API `countTokens`** (`countMessagesTokensWithAPI`): wraps `anthropic.beta.messages.countTokens(...)` via `withTokenCountVCR(messages, tools, fn)`; on Bedrock provider, dispatches to `countTokensWithBedrock` (which dynamically imports `@aws-sdk/client-bedrock-runtime` to defer ~279KB). Vertex client throws on success; Bedrock returns an UnknownOperationException envelope — both treated as `null`.
3. **Haiku-fallback `messages.create`** (`countTokensViaHaikuFallback`): used when an actual `messages.create` request is needed but only as a token sizer. Picks model via:
   - Vertex global region (`getVertexRegionForModel(getSmallFastModel()) === 'global'`) → `getDefaultSonnetModel()`
   - Bedrock + thinking blocks → `getDefaultSonnetModel()`
   - Vertex + thinking blocks → `getDefaultSonnetModel()`
   - Otherwise → `getSmallFastModel()` (Haiku)

Returns `usage.input_tokens + (cache_creation_input_tokens||0) + (cache_read_input_tokens||0)` — the *sum* across all three input variants.

`hasThinkingBlocks` (`tokenEstimation.ts:38–56`) walks assistant messages for `block.type === 'thinking' || 'redacted_thinking'`. When true, `thinking: { type: 'enabled', budget_tokens: 1024 }` is set and `max_tokens = 2048` (constants `TOKEN_COUNT_THINKING_BUDGET = 1024`, `TOKEN_COUNT_MAX_TOKENS = 2048`). When false, `max_tokens = 1` for fallback mode (cheapest possible non-empty response).

`stripToolSearchFieldsFromMessages` (`tokenEstimation.ts:66–122`) removes `caller` from `tool_use` blocks and `tool_reference` blocks from `tool_result.content`; if the filtered content is empty, replaces it with `[{ type: 'text', text: '[tool references]' }]`. Used by `countTokensViaHaikuFallback` only.

Per-block estimation (`roughTokenCountEstimationForBlock`, `:391–435`):
- `text` → `roughTokenCountEstimation(block.text)`
- `image` or `document` → constant `2000` (matches `microCompact`'s `IMAGE_MAX_TOKEN_SIZE`; chosen to **avoid underestimating** so auto-compact triggers in time)
- `tool_result` → recurse on `block.content`
- `tool_use` → `roughTokenCountEstimation(block.name + jsonStringify(block.input ?? {}))`
- `thinking` → `roughTokenCountEstimation(block.thinking)`
- `redacted_thinking` → `roughTokenCountEstimation(block.data)`
- any other block (`server_tool_use`, `web_search_tool_result`, `mcp_tool_use`, …) → `roughTokenCountEstimation(jsonStringify(block))`

### 5.8 Persistence on session switch / resume

```
saveCurrentSessionCosts(fpsMetrics?):
  saveCurrentProjectConfig(current => ({
    ...current,
    lastCost: getTotalCostUSD(),
    lastAPIDuration: getTotalAPIDuration(),
    lastAPIDurationWithoutRetries: getTotalAPIDurationWithoutRetries(),
    lastToolDuration: getTotalToolDuration(),
    lastDuration: getTotalDuration(),
    lastLinesAdded:  getTotalLinesAdded(),
    lastLinesRemoved: getTotalLinesRemoved(),
    lastTotalInputTokens:               getTotalInputTokens(),
    lastTotalOutputTokens:              getTotalOutputTokens(),
    lastTotalCacheCreationInputTokens:  getTotalCacheCreationInputTokens(),
    lastTotalCacheReadInputTokens:      getTotalCacheReadInputTokens(),
    lastTotalWebSearchRequests:         getTotalWebSearchRequests(),
    lastFpsAverage: fpsMetrics?.averageFps,
    lastFpsLow1Pct: fpsMetrics?.low1PctFps,
    lastModelUsage: { [model]: { inputTokens, outputTokens, cacheReadInputTokens,
                                 cacheCreationInputTokens, webSearchRequests, costUSD } ... },
    lastSessionId: getSessionId(),
  }))

restoreCostStateForSession(sessionId) -> bool:
  data = getStoredSessionCosts(sessionId)        // returns undefined if lastSessionId !== sessionId
  if !data: return false
  setCostStateForRestore(data)                    // see state.ts:881–916
  return true
```

`getStoredSessionCosts` (`:87–123`) re-derives `contextWindow` and `maxOutputTokens` from `getContextWindowForModel(model, getSdkBetas())` and `getModelMaxOutputTokens(model).default` — they are NOT persisted, only re-computed at restore time.

`setCostStateForRestore` (`state.ts:881–916`) overwrites `STATE.totalCostUSD`, `totalAPIDuration`, `totalAPIDurationWithoutRetries`, `totalToolDuration`, `totalLinesAdded`, `totalLinesRemoved`, and `STATE.modelUsage` (only if the modelUsage param is truthy). It also adjusts `STATE.startTime = Date.now() - lastDuration` so wall-time keeps accumulating.

---

## §6 Verbatim Assets

### 6.1 Pricing tiers — `src/utils/modelCost.ts:36–87`

```ts
// Standard pricing tier for Sonnet models: $3 input / $15 output per Mtok
export const COST_TIER_3_15 = {
  inputTokens: 3,
  outputTokens: 15,
  promptCacheWriteTokens: 3.75,
  promptCacheReadTokens: 0.3,
  webSearchRequests: 0.01,
} as const satisfies ModelCosts

// Pricing tier for Opus 4/4.1: $15 input / $75 output per Mtok
export const COST_TIER_15_75 = {
  inputTokens: 15,
  outputTokens: 75,
  promptCacheWriteTokens: 18.75,
  promptCacheReadTokens: 1.5,
  webSearchRequests: 0.01,
} as const satisfies ModelCosts

// Pricing tier for Opus 4.5: $5 input / $25 output per Mtok
export const COST_TIER_5_25 = {
  inputTokens: 5,
  outputTokens: 25,
  promptCacheWriteTokens: 6.25,
  promptCacheReadTokens: 0.5,
  webSearchRequests: 0.01,
} as const satisfies ModelCosts

// Fast mode pricing for Opus 4.6: $30 input / $150 output per Mtok
export const COST_TIER_30_150 = {
  inputTokens: 30,
  outputTokens: 150,
  promptCacheWriteTokens: 37.5,
  promptCacheReadTokens: 3,
  webSearchRequests: 0.01,
} as const satisfies ModelCosts

// Pricing for Haiku 3.5: $0.80 input / $4 output per Mtok
export const COST_HAIKU_35 = {
  inputTokens: 0.8,
  outputTokens: 4,
  promptCacheWriteTokens: 1,
  promptCacheReadTokens: 0.08,
  webSearchRequests: 0.01,
} as const satisfies ModelCosts

// Pricing for Haiku 4.5: $1 input / $5 output per Mtok
export const COST_HAIKU_45 = {
  inputTokens: 1,
  outputTokens: 5,
  promptCacheWriteTokens: 1.25,
  promptCacheReadTokens: 0.1,
  webSearchRequests: 0.01,
} as const satisfies ModelCosts

const DEFAULT_UNKNOWN_MODEL_COST = COST_TIER_5_25
```

Units: USD per Mtok for token rates; USD per request for `webSearchRequests`.

### 6.2 MODEL_COSTS dispatch table — `src/utils/modelCost.ts:104–126`

```ts
export const MODEL_COSTS: Record<ModelShortName, ModelCosts> = {
  [firstPartyNameToCanonical(CLAUDE_3_5_HAIKU_CONFIG.firstParty)]: COST_HAIKU_35,
  [firstPartyNameToCanonical(CLAUDE_HAIKU_4_5_CONFIG.firstParty)]: COST_HAIKU_45,
  [firstPartyNameToCanonical(CLAUDE_3_5_V2_SONNET_CONFIG.firstParty)]: COST_TIER_3_15,
  [firstPartyNameToCanonical(CLAUDE_3_7_SONNET_CONFIG.firstParty)]: COST_TIER_3_15,
  [firstPartyNameToCanonical(CLAUDE_SONNET_4_CONFIG.firstParty)]: COST_TIER_3_15,
  [firstPartyNameToCanonical(CLAUDE_SONNET_4_5_CONFIG.firstParty)]: COST_TIER_3_15,
  [firstPartyNameToCanonical(CLAUDE_SONNET_4_6_CONFIG.firstParty)]: COST_TIER_3_15,
  [firstPartyNameToCanonical(CLAUDE_OPUS_4_CONFIG.firstParty)]: COST_TIER_15_75,
  [firstPartyNameToCanonical(CLAUDE_OPUS_4_1_CONFIG.firstParty)]: COST_TIER_15_75,
  [firstPartyNameToCanonical(CLAUDE_OPUS_4_5_CONFIG.firstParty)]: COST_TIER_5_25,
  [firstPartyNameToCanonical(CLAUDE_OPUS_4_6_CONFIG.firstParty)]: COST_TIER_5_25,
}
```

Opus 4.6 is overridden at lookup time by `getModelCosts` to dispatch to `getOpus46CostTier`; the table value is only used when fast-mode is off.

### 6.3 BetaUsage field names consumed (verbatim)

From `cost-tracker.ts:266–274`, `modelCost.ts:131–141`, `tokenEstimation.ts:319–324`:

```
usage.input_tokens
usage.output_tokens
usage.cache_read_input_tokens
usage.cache_creation_input_tokens
usage.server_tool_use.web_search_requests
usage.speed                                 // 'fast' | other (Opus 4.6 only)
usage.iterations                            // Array<{ type: string }> — advisor sub-iterations
```

**Not consumed by cost-tracker** (but present on `BetaUsage` and produced by `services/api/claude.ts:2983, 3013, 3034`): `usage.service_tier` and `usage.inference_geo`. These fields participate in the producer-side `updateUsage` / `accumulateUsage` asymmetry documented in spec 22 (most-recent-wins for `accumulateUsage`, prior-preserved for `updateUsage`), but the cost-tracker reads neither — pricing is determined solely by `model` (with `usage.speed` as a sub-key for Opus 4.6). If pricing ever needs to vary by service tier or inference region, both `getModelCosts` and `MODEL_COSTS` would need to be reworked. Spec 22 owns the producer-side accumulation contract.

The internal aggregated shape (`STATE.modelUsage[model]`):

```
{
  inputTokens: number,
  outputTokens: number,
  cacheReadInputTokens: number,
  cacheCreationInputTokens: number,
  webSearchRequests: number,
  costUSD: number,
  contextWindow: number,
  maxOutputTokens: number,
}
```

Persisted-config keys (verbatim, `cost-tracker.ts:144–174`):

```
lastCost, lastAPIDuration, lastAPIDurationWithoutRetries, lastToolDuration,
lastDuration, lastLinesAdded, lastLinesRemoved,
lastTotalInputTokens, lastTotalOutputTokens,
lastTotalCacheCreationInputTokens, lastTotalCacheReadInputTokens,
lastTotalWebSearchRequests,
lastFpsAverage, lastFpsLow1Pct,
lastModelUsage,
lastSessionId
```

### 6.4 OTel counter identifiers — `src/bootstrap/state.ts:955–987`

```ts
STATE.sessionCounter = createCounter('claude_code.session.count', { description: 'Count of CLI sessions started' })
STATE.locCounter     = createCounter('claude_code.lines_of_code.count', { description: "Count of lines of code modified, with the 'type' attribute indicating whether lines were added or removed" })
STATE.prCounter      = createCounter('claude_code.pull_request.count',  { description: 'Number of pull requests created' })
STATE.commitCounter  = createCounter('claude_code.commit.count',        { description: 'Number of git commits created' })
STATE.costCounter    = createCounter('claude_code.cost.usage',          { description: 'Cost of the Claude Code session', unit: 'USD' })
STATE.tokenCounter   = createCounter('claude_code.token.usage',         { description: 'Number of tokens used',           unit: 'tokens' })
STATE.codeEditToolDecisionCounter = createCounter('claude_code.code_edit_tool.decision', { description: 'Count of code editing tool permission decisions (accept/reject) for Edit, Write, and NotebookEdit tools' })
STATE.activeTimeCounter = createCounter('claude_code.active_time.total', { description: 'Total active time in seconds', unit: 's' })
```

`tokenCounter.add` `type` attribute values (verbatim, `cost-tracker.ts:292–301`): `'input'`, `'output'`, `'cacheRead'`, `'cacheCreation'`.
`costCounter.add` attrs: `{ model }` always, plus `{ speed: 'fast' }` when `isFastModeEnabled() && usage.speed === 'fast'`.

### 6.5 Analytics event identifiers emitted by §6 code

| Event ID | Site | Payload (verbatim keys) |
|---|---|---|
| `tengu_advisor_tool_token_usage` | `src/cost-tracker.ts:306–315` | `advisor_model`, `input_tokens`, `output_tokens`, `cache_read_input_tokens`, `cache_creation_input_tokens`, `cost_usd_micros` |
| `tengu_unknown_model_cost` | `src/utils/modelCost.ts:167–172` | `model`, `shortName` |
| `tengu_token_budget_completed` | `src/query.ts:1349–1353` | spread of `decision.completionEvent` (`continuationCount`, `pct`, `turnTokens`, `budget`, `diminishingReturns`, `durationMs`) plus `queryChainId`, `queryDepth` |

`cost_usd_micros = Math.round(advisorCost * 1_000_000)` — integer micro-USD.

### 6.6 User-facing format strings — `src/cost-tracker.ts`

```
Usage:                 0 input, 0 output, 0 cache read, 0 cache write
```
(line 184 — emitted when `Object.keys(modelUsageMap).length === 0`)

Per-model row (`:213–223`), 21-char left-pad on `${shortName}:`:

```
{shortName}:  {inputTokens} input, {outputTokens} output, {cacheReadInputTokens} cache read, {cacheCreationInputTokens} cache write[, {webSearchRequests} web search] ({formatCost(costUSD)})
```

Rolled-up summary (`formatTotalCost`, `:228–243`), all wrapped in `chalk.dim(...)`:

```
Total cost:            {costDisplay}
Total duration (API):  {formatDuration(getTotalAPIDuration())}
Total duration (wall): {formatDuration(getTotalDuration())}
Total code changes:    {n} {line|lines} added, {n} {line|lines} removed
{modelUsageDisplay}
```

`costDisplay` appends ` (costs may be inaccurate due to usage of unknown models)` iff `hasUnknownModelCost()` (`:230–233`).

`formatCost` (`:177–179`):
```ts
return `$${cost > 0.5 ? round(cost, 100).toFixed(2) : cost.toFixed(maxDecimalPlaces)}`
```
`maxDecimalPlaces = 4` default.

`formatModelPricing` (`modelCost.ts:217–219`): `"$3/$15 per Mtok"`. `formatPrice` integers → `"$3"`; non-integers → `"$0.80"`, `"$22.50"`.

### 6.7 `TOKEN_BUDGET` parser regexes & continuation message — `src/utils/tokenBudget.ts:1–13, 66–73`

```ts
const SHORTHAND_START_RE = /^\s*\+(\d+(?:\.\d+)?)\s*(k|m|b)\b/i
const SHORTHAND_END_RE   = /\s\+(\d+(?:\.\d+)?)\s*(k|m|b)\s*[.!?]?\s*$/i
const VERBOSE_RE         = /\b(?:use|spend)\s+(\d+(?:\.\d+)?)\s*(k|m|b)\s*tokens?\b/i
const VERBOSE_RE_G       = new RegExp(VERBOSE_RE.source, 'gi')

const MULTIPLIERS = { k: 1_000, m: 1_000_000, b: 1_000_000_000 }
```

Precedence in `parseTokenBudget`: SHORTHAND_START → SHORTHAND_END → VERBOSE; first match wins, returns `parseFloat(value) * MULTIPLIERS[suffix.toLowerCase()]`.

Continuation message (verbatim, `:66–73`):

```ts
return `Stopped at ${pct}% of token target (${fmt(turnTokens)} / ${fmt(budget)}). Keep working — do not summarize.`
```

`fmt = (n) => new Intl.NumberFormat('en-US').format(n)` — comma-grouped.

### 6.8 Token-estimation constants — `src/services/tokenEstimation.ts:32–33`, `:411`, `:215–223`

```ts
const TOKEN_COUNT_THINKING_BUDGET = 1024
const TOKEN_COUNT_MAX_TOKENS      = 2048
```

Image/document block estimate: `2000` tokens (matches `microCompact.IMAGE_MAX_TOKEN_SIZE`).

`bytesPerTokenForFileType`: `'json'|'jsonl'|'jsonc' → 2`, default `→ 4`.

Default `roughTokenCountEstimation` `bytesPerToken = 4`.

### 6.9 Token-budget thresholds — `src/query/tokenBudget.ts:3–4`

```ts
const COMPLETION_THRESHOLD  = 0.9
const DIMINISHING_THRESHOLD = 500
```

Diminishing-return condition: `continuationCount >= 3 && deltaSinceLastCheck < 500 && lastDeltaTokens < 500`.

### 6.10 Constants table

| Constant | Value | Source |
|---|---|---|
| `TOKEN_COUNT_THINKING_BUDGET` | `1024` | `tokenEstimation.ts:32` |
| `TOKEN_COUNT_MAX_TOKENS` | `2048` | `tokenEstimation.ts:33` |
| Image/document block tokens | `2000` | `tokenEstimation.ts:411` |
| Default `bytesPerToken` | `4` | `tokenEstimation.ts:206` |
| `bytesPerToken` for json/jsonl/jsonc | `2` | `tokenEstimation.ts:217–222` |
| `formatCost` threshold | `0.5` USD | `cost-tracker.ts:178` |
| `formatCost` precision | `0.01` USD when ≥0.5; `0.0001` USD otherwise | `cost-tracker.ts:178` |
| `formatModelUsage` shortName padding | width 21 | `cost-tracker.ts:223` |
| `COMPLETION_THRESHOLD` (token budget) | `0.9` | `query/tokenBudget.ts:3` |
| `DIMINISHING_THRESHOLD` (token budget) | `500` | `query/tokenBudget.ts:4` |
| Min continuations before diminishing-stop | `3` | `query/tokenBudget.ts:60` |
| Multipliers | `k=1_000, m=1_000_000, b=1_000_000_000` | `utils/tokenBudget.ts:11–15` |
| `cost_usd_micros` scale | `1_000_000` | `cost-tracker.ts:314` |
| Default unknown-model cost tier | `COST_TIER_5_25` | `modelCost.ts:89` |

Pricing table values: see §6.1.

---

## §7 Configuration & Flags

| Flag / env / setting | Effect | Source |
|---|---|---|
| `feature('TOKEN_BUDGET')` | Gates `createBudgetTracker()` and the per-turn continuation gate. When false, `budgetTracker = null` and the gate at `query.ts:1308` is skipped. | `query.ts:280, 1308` |
| `feature('TOKEN_BUDGET')` (other sites) | Also gates: prompt injection (`constants/prompts.ts:538`), attachment hook (`utils/attachments.ts:3829`), REPL UI (`screens/REPL.tsx:2134, 2893, 2959`), spinner (`components/Spinner.tsx:263`), prompt-input regex highlighter (`PromptInput.tsx:534`). Per-spec `04`/`05`/`37`. | grep |
| `process.env.DISABLE_COST_WARNINGS` (truthy) | `hasConsoleBillingAccess` returns false → on-exit summary suppressed. | `billing.ts:12–14` |
| `isClaudeAISubscriber()` | Hides cost summary (Max/Pro users billed differently). | `billing.ts:16–20` |
| Org/workspace role | `['admin','billing']` org or `['workspace_admin','workspace_billing']` workspace required to see summary. | `billing.ts:40–43` |
| `process.env.CLAUDE_CODE_USE_BEDROCK` (truthy) + thinking blocks | Force Sonnet (not Haiku 3.5) for fallback token counting. | `tokenEstimation.ts:263–268` |
| `process.env.CLAUDE_CODE_USE_VERTEX` (truthy) + thinking blocks | Same: Sonnet. | `tokenEstimation.ts:265–267` |
| Vertex global region | Force Sonnet (Haiku not available). | `tokenEstimation.ts:259–261` |
| `getAPIProvider() === 'vertex'` | Filter betas to `VERTEX_COUNT_TOKENS_ALLOWED_BETAS` set. | `tokenEstimation.ts:167–169, 296–299` |
| `getAPIProvider() === 'bedrock'` | Use `countTokensWithBedrock` instead of SDK `countTokens`. | `tokenEstimation.ts:150–158` |
| `isFastModeEnabled() && usage.speed === 'fast'` | (a) Adds `{ speed: 'fast' }` to OTel cost/token attrs. (b) For Opus 4.6, switches pricing to `COST_TIER_30_150`. | `cost-tracker.ts:287–289`, `modelCost.ts:94–98, 148–153` |

Settings precedence (per spec 02) governs all non-env knobs above. No spec-06–owned setting is defined.

---

## §8 Errors & Edge Cases

- **Unknown model**: `getModelCosts` falls back to `MODEL_COSTS[canonical(getDefaultMainLoopModelSetting())] ?? COST_TIER_5_25`, logs `tengu_unknown_model_cost`, sets `STATE.hasUnknownModelCost = true` (one-way until `resetCostState`). `formatTotalCost` appends `" (costs may be inaccurate due to usage of unknown models)"`.
- **Vertex/Bedrock token-count failure**: `countMessagesTokensWithAPI` catches all errors, calls `logError(error)`, returns `null`. Bedrock-specific UnknownOperationException returns succeeds-with-null (no `inputTokens`).
- **Empty content** in `countTokensWithAPI`: returns `0` immediately (API rejects empty messages).
- **Empty messages array** with non-empty `tools`: substitutes `[{ role: 'user', content: 'foo' }]` (`tokenEstimation.ts:177, 465`) for `messages.countTokens`; substitutes `[{ role: 'user', content: 'count' }]` (`:291`) for `messages.create` Haiku fallback.
- **Subagent with budget**: `checkTokenBudget` returns immediate `stop` when `agentId !== undefined`. The token-budget feature is foreground-only.
- **Budget ≤ 0 or null**: stop with no completion event. Silent.
- **Diminishing returns**: requires ≥ 3 prior continuations *and* the last two deltas both < 500 tokens. Avoids burning continuations on a model that's just emitting a few hundred tokens per turn.
- **Stop-hook + budget interplay**: After a stop_hook_blocking transition (`query.ts` ~line 1290), `hasAttemptedReactiveCompact` is preserved to prevent the budget gate from initiating an infinite continue/compact/error loop.
- **Restoring session costs**: `getStoredSessionCosts` returns undefined unless `projectConfig.lastSessionId === sessionId`, so cost from a prior unrelated session never bleeds into a new one.
- **`saveCurrentSessionCosts` runs on every exit** even when `hasConsoleBillingAccess()` is false — so persistence is independent of the print gate.
- **Non-streaming fallback**: `services/api/claude.ts:2820–2830` adds the cost in the `finally` block, **after** stream resource release. Critical because if the consumer aborts mid-stream the streaming `message_delta` handler (`:2251–2256`) may have run first; the fallback path covers the non-streaming case where the entire response arrives via a single push to `newMessages`.
- **VCR replay** (`services/vcr.ts:122`): when a cached transcript is replayed, every cached assistant message has its cost re-attributed via `addCachedCostToTotalSessionCost`, calling `addToTotalSessionCost` from a non-API source.
- **Advisor recursion** (`cost-tracker.ts:316–321`): `addToTotalSessionCost` calls itself for each `iterations[]` entry of type `'advisor_message'`. The advisor's `BetaUsage` fields are added to the per-model accumulator under the advisor's own model name (not the base model's), so a single API call can update multiple `STATE.modelUsage[*]` entries.
- **Counter overflow**: `tokenCounter.add(usage.input_tokens, …)` is called with the raw int. No clamping. OTel exporter is presumed to handle wraparound.

---

## §9 ANT-only / Internal Behavior

No `process.env.USER_TYPE === 'ant'` gates inside the in-scope files. The advisor tool itself has ANT bypasses (`utils/advisor.ts:94, 104`) but the cost-tracking path treats ANT and prod identically.

---

## §10 Observed Invariants & Cross-Cutting Notes

1. **`STATE.totalCostUSD` is monotonic** within a session, except `resetCostState()` (`state.ts:864–875`) and `setCostStateForRestore` (`:881–916`) — both called only by cost-tracker.
2. **Per-model totals are derived from `STATE.modelUsage` map** at read time (`getTotalInputTokens` etc. use `sumBy`). Adding a new per-model field to `ModelUsage` requires no touchpoints in the readers.
3. **`contextWindow` and `maxOutputTokens` on `ModelUsage` are derivative** — overwritten on each `addToTotalModelUsage` call from `getContextWindowForModel` / `getModelMaxOutputTokens` and re-derived on restore from project config.
4. **The Anthropic API's `BetaUsage` shape may include sub-iterations** (`iterations: Array<{ type: string }>`); only `'advisor_message'` is recognised. Future iteration types are silently ignored.
5. **Token-budget continuation injects an `isMeta: true` user message** (`query.ts:1325–1328`). Per spec 04, `isMeta` messages are filtered out of certain UI views.
6. **`saveCurrentSessionCosts` is fire-and-forget** synchronously inside `process.on('exit')`. If `saveCurrentProjectConfig` performs IO, it must complete before the event loop exits.
7. **`tengu_advisor_tool_token_usage` is emitted before** the recursive `addToTotalSessionCost` call, so its `cost_usd_micros` reflects only the advisor's own iteration cost — not the post-recursion total.
8. **Side-query / classifier costs are intentionally excluded** (`utils/permissions/permissions.ts:766` — comment confirms classifier path "does NOT call addToTotalSessionCost"). Spec 09 owns that policy.
9. **VCR token-counting** wraps `countMessagesTokensWithAPI` via `withTokenCountVCR(messages, tools, fn)` — recordings are keyed on the full `(messages, tools)` tuple. Spec 22 owns VCR mechanics.
10. **Image/document `2000`-token estimate is direction-asymmetric.** For images: source comment (`tokenEstimation.ts:401–405`) says max 2000×2000 → `~5333` tokens API-billed, so `2000` is conservative-low for full-resolution images (slight under-estimate, ~2.7×). For documents (`document` block, base64 PDF in `source.data`): the `2000` is a **deliberate upper-bound replacement** for the catch-all `jsonStringify` path that would otherwise count base64 chars (a 1MB PDF → ~1.33M base64 chars / 4 bytes-per-token ≈ ~325k *estimated* tokens vs ~2000 the API actually charges per the source comment at `:407–410`). So the 2000 constant **avoids the catch-all over-estimate** for documents, not under-estimate. The risk surface: large/complex images can mildly under-estimate (auto-compact may fire late on image-heavy turns); document PDFs are well-bounded by the constant. See spec 11 (compaction) for the same constant's role in `microCompact`.
11. **`STATE.hasUnknownModelCost` is a one-way latch within a session, NOT persisted across sessions.** Set by `setHasUnknownModelCost()` (`state.ts:745–747`) on first unknown-model lookup; cleared only by `resetCostState()` and `resetStateForTests()`. The persistence keys written by `saveCurrentSessionCosts` (`cost-tracker.ts:144–174`) do **not** include `hasUnknownModelCost`, so a session resumed via `restoreCostStateForSession` starts with the latch false even if the prior session hit an unknown model — the cost figure restored from `lastCost` may already reflect unknown-model fallback pricing without the resumed session displaying the "(costs may be inaccurate due to usage of unknown models)" suffix until the resumed session itself hits an unknown model.
12. **`STATE.totalCostUSD` has no midnight / day rollover.** Cumulative session cost grows monotonically until `resetCostState()` (production reset paths) or process exit. Cross-day sessions accumulate without recalibration; the `lastCost` persisted snapshot also carries no time component.
13. **`contextWindow` reflects the SDK beta set at the time of the latest API call** for that model. `addToTotalModelUsage` reads `getSdkBetas()` on every call (`cost-tracker.ts:273`), so beta-header changes between calls (e.g. an advisor invocation enabling a 1P-only beta) can alter the displayed window for that model on subsequent observations.

> **Phase 9.7 cost-multiplier callouts (C1 latch, C2 midnight)** — invariants 11 and 12 above are the spec-06 surface for the Phase 9.7 cost-multiplier risk register. Both behaviours are intentional but worth elevated review during regression sweeps: a model rename or pricing-tier flip that lands silently can leave a session reporting stale costs (C1, no resumed-latch) or a long-running session reporting a single growing total without daily checkpoints (C2). Analytics consumers (spec 26) should treat `tengu_unknown_model_cost` as the only signal of the C1 condition and as **per-session-not-resumed**.

---

## §11 Test Hooks / Reset Surface

- `resetCostState()` — public, called from production reset paths.
- `resetStateForTests()` — wraps `getInitialState()` reseed; throws unless `process.env.NODE_ENV === 'test'` (`state.ts:919–930`). Also clears `outputTokensAtTurnStart`, `currentTurnTokenBudget`, `budgetContinuationCount`.
- `resetTotalDurationStateAndCost_FOR_TESTS_ONLY()` — `state.ts:551–555`, narrow scope (cost + API durations only).

---

## §12 Open Questions / Missing Source

1. **`ModelUsage` type definition**: imported from `src/entrypoints/agentSdkTypes.ts`, but that file's full surface was not inventoried. The shape used by `cost-tracker.ts` is fully reconstructible from `:191–202, 255–264`, but other fields may exist. Per HANDOFF §6, this is the same class of "type-source not enumerated" gotcha as `src/types/message.ts` for spec 08; treat as missing-leaked-source.
2. **Where `usage.iterations` is populated by the API** — first-party only (advisor uses a first-party-only beta header per `utils/advisor.ts:64–66`). The advisor server-tool integration into the SDK shape is not in this spec's scope; spec 22 should resolve.
3. **Whether `isFastModeEnabled()` is dynamic or session-static** — `cost-tracker.ts:287` reads it on every emission, suggesting dynamic, but the gating policy for fast-mode is in `utils/fastMode.ts` (out-of-scope here; touch in 22 or 26).
4. **`AttributedCounter` add-time behaviour** — `cost-tracker.ts` calls `getTokenCounter()?.add(0, …)` for absent `cache_read_input_tokens`; whether OTel emits a zero-valued data point or short-circuits is exporter-specific. Spec 26.
5. **Interaction with `BUDGET_TRACKER`-style task budget at `query.ts:282–291`** — that comment refers to a *separate* `task_budget.remaining` tracking (server-side `api/api/sampling/prompt/renderer.py:292`) distinct from `TOKEN_BUDGET`. The `taskBudgetRemaining` local var is **not** wired through any of this spec's code paths; this spec confirms it is decoupled but does not document it.
6. **`getModelMaxOutputTokens` and `getContextWindowForModel`** sources (`utils/context.ts`) — used here only as opaque lookups; spec 22 or a new model-config spec should own them.
7. **`AnalyticsMetadata_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS`** type assertions at `cost-tracker.ts:308`, `modelCost.ts:168, 170` — the type's runtime meaning (presumably a string nominal-type that participates in PII auditing) is owned by spec 26.
