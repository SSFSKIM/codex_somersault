# 02 — LLM Hook Handlers (prompt + agent) for the Codex hooks engine

Status: proposal / implementation-ready spec
Audience: a Rust engineer implementing this in `codex-rs`
Scope owner: hooks subsystem

---

## 1. Summary & motivation

Codex's hooks engine (`codex-hooks`) today implements exactly one handler type — `command` (a shell process fed JSON on stdin). The config layer and protocol already declare two more handler types, `prompt` and `agent`, but discovery skips them with a warning (`codex-rs/hooks/src/engine/discovery.rs:516-523`), and `HookHandlerConfig::Prompt {}` / `HookHandlerConfig::Agent {}` are empty structs carrying no fields (`codex-rs/config/src/hook_config.rs:153-155`).

This spec ports the two **LLM-backed** handler types from the upstream Claude Code reference harness:

- **`prompt`** — a single-shot, non-interactive LLM call. The hook's `prompt` (with `$ARGUMENTS` interpolated to the hook-input JSON) is sent to a small/fast model with JSON-schema-enforced output `{ok: bool, reason?: string}`. `ok:false` becomes a blocking decision.
- **`agent`** — a forked, multi-turn sub-agent run that uses tools to verify a condition, then emits the same `{ok, reason?}` contract via a synthetic structured-output tool. `ok:false` blocks.

**What it buys us.** Hook authors get policy/verification logic that does not require shipping a script: "block this `Bash` call if the LLM judges the command destructive" (prompt), or "after the agent stops, spin up a verifier sub-agent that reads the transcript and confirms tests actually passed" (agent). It is the single largest parity gap between Codex hooks and the reference harness, and the wire protocol (`HookHandlerType::{Command,Prompt,Agent}`) is already shipped to clients (`codex-rs/protocol/src/protocol.rs:1370-1374`), so the TUI/IDE can already render these runs.

**Why now.** The protocol, the `HookRunSummary.handler_type` field, the v2 app-server mapping (`codex-rs/app-server-protocol/src/protocol/v2/hook.rs:24-28`), and the PreToolUse/PermissionRequest gating path are all in place. The only missing pieces are (a) config fields, (b) a discovery path that doesn't warn-and-skip, and (c) an LLM-invocation seam. This is a contained, forward-compatible addition.

**v1 (Phase 1) target.** Ship the **`prompt`** handler end-to-end (config → discovery → dispatch → PreToolUse/PermissionRequest gating) and lay the trait seam. Ship **`agent`** in Phase 3 reusing Codex's in-process sub-agent machinery. The `http` (webhook) handler is out of scope (Section 12).

---

## 2. Source-of-truth behavior (Claude Code)

All paths below are under `/Users/new/Documents/GitHub/codex_somersault/Claude Code Src`.

### Config shape (`src/schemas/hooks.ts`)

`HookCommandSchema` is a `discriminatedUnion("type", [...])` over `command | prompt | agent | http` (`src/schemas/hooks.ts:183-189`).

- `PromptHook` (`src/schemas/hooks.ts:67-95`): `{ type:"prompt", prompt:string, if?:string, timeout?:number(sec), model?:string, statusMessage?:string, once?:bool }`.
- `AgentHook` (`src/schemas/hooks.ts:128-163`): identical fields keyed by `prompt`. The schema carries an explicit warning **never to add `.transform()`** because settings round-trip through `JSON.stringify` and a function value would be silently dropped (`src/schemas/hooks.ts:130-137`).

### Shared response contract (`src/utils/hooks/hookHelpers.ts`)

`hookResponseSchema = { ok: boolean, reason?: string }` (`src/utils/hooks/hookHelpers.ts:16-24`). `$ARGUMENTS` substitution is `addArgumentsToPrompt` → `substituteArguments` (`hookHelpers.ts:30-35`), supporting `$ARGUMENTS`, indexed `$ARGUMENTS[N]`, and shorthand `$N`.

### Prompt handler algorithm (`src/utils/hooks/execPromptHook.ts`)

1. `processedPrompt = addArgumentsToPrompt(hook.prompt, jsonInput)` (line 35).
2. Build a user message **directly** via `createUserMessage`, NOT `processUserInput`, to avoid re-triggering `UserPromptSubmit` hooks (recursion guard, lines 40-42).
3. `queryModelWithoutStreaming` with:
   - system prompt: `"You are evaluating a hook in Claude Code. Your response must be a JSON object … {ok:true} … {ok:false, reason}"` (lines 64-70).
   - `thinkingConfig: { type:'disabled' }`, `tools: <parent tools>`, `model: hook.model ?? getSmallFastModel()`, `isNonInteractiveSession:true`, `querySource:'hook_prompt'` (lines 71-86).
   - `outputFormat: { type:'json_schema', schema:{ ok:boolean, reason?:string, required:['ok'], additionalProperties:false } }` (lines 87-98).
4. Timeout = `hook.timeout*1000 || 30000` (line 55), enforced via `createCombinedAbortSignal`.
5. Parse the response text; on JSON-parse failure or schema mismatch → `outcome:'non_blocking_error'` (lines 113-151).
6. `ok:false` → `outcome:'blocking'`, `blockingError.blockingError = "Prompt hook condition was not met: ${reason}"`, `preventContinuation:true`, `stopReason:reason` (lines 154-167).
7. `ok:true` → `outcome:'success'` (lines 170-182).
8. Abort/timeout → `outcome:'cancelled'` (lines 186-191).

### Agent handler algorithm (`src/utils/hooks/execAgentHook.ts`)

1. Same `$ARGUMENTS` substitution + direct `createUserMessage` recursion guard (lines 60-67).
2. Timeout = `hook.timeout*1000 || 60000` (line 75). `MAX_AGENT_TURNS = 50` (line 119).
3. Mint `hookAgentId = "hook-agent-" + randomUUID()` (line 122).
4. Build a cloned `ToolUseContext`: `mode:'dontAsk'`, inject a session rule `Read(/${transcriptPath})` so the verifier can read the transcript (lines 137-153), `thinkingConfig:'disabled'`, `isNonInteractiveSession:true`.
5. Tool set = parent tools minus `ALL_AGENT_DISALLOWED_TOOLS`, minus any pre-existing `SyntheticOutputTool` (avoids schema collision with `--json-schema`), **plus** a fresh `createStructuredOutputTool()` configured with `hookResponseSchema` (lines 89-105).
6. Register a session-scoped `Stop` enforcement hook keyed on `hookAgentId` (`registerStructuredOutputEnforcement`, `hookHelpers.ts:70-83`) that re-prompts "You MUST call the SyntheticOutput tool" if the agent stops without calling it. Timeout 5000ms (`hookHelpers.ts:82`).
7. Drive `query({…, querySource:'hook_agent'})` in an `async for` loop; count `assistant` turns; abort at `MAX_AGENT_TURNS` (lines 167-209).
8. Watch for `attachment.type === 'structured_output'`; parse `{ok, reason?}`; on success abort + break (lines 211-227).
9. After loop: `clearSessionHooks(setAppState, hookAgentId)` to avoid leaking the Stop gate into the parent (lines 232-233).
10. No structured output (max turns OR agent stopped without calling the tool) → `outcome:'cancelled'` (silent; lines 236-268).
11. `ok:false` → `outcome:'blocking'`, `blockingError = "Agent hook condition was not met: ${reason}"`. **Note the asymmetry:** unlike the prompt hook, the agent hook does NOT set `preventContinuation` (lines 271-283 vs. prompt 165).
12. `ok:true` → `outcome:'success'` (lines 285-303).

### Constants summary

| Constant | Value | Source |
|---|---|---|
| Prompt default timeout | 30 s | `execPromptHook.ts:55` |
| Agent default timeout | 60 s | `execAgentHook.ts:75` |
| Max agent turns | 50 | `execAgentHook.ts:119` |
| Stop-enforcement hook timeout | 5 s | `hookHelpers.ts:82` |
| Response contract | `{ok:bool, reason?:string}` | `hookHelpers.ts:16-24` |
| Default model | small/fast (Haiku-class) | `execPromptHook.ts:79`, `execAgentHook.ts:118` |

---

## 3. Target placement in Codex

### New crate: `codex-hook-llm-runner` (dir `codex-rs/hook-llm-runner/`)

The LLM-invocation capability is injected into the hooks engine via a `dyn` trait so that `codex-hooks` stays model-free. The implementation of that trait lives in the new crate.

**Why a new crate, not `codex-core`.** The fork prime directive (root `CLAUDE.md`, `core/CLAUDE.md`) is to resist growing `codex-core` (already ~44k LoC, the largest crate). The LLM-invocation glue does not need core internals beyond a `ModelClient`.

**Why not put model-calling code in `codex-hooks`.** `codex-hooks/Cargo.toml` depends only on `codex-config`, `codex-plugin`, `codex-protocol`, the absolute-path/output-truncation utils, `futures`, `serde`, `tokio`, `regex` — it has **no** dependency on `codex-api` or `codex-core`. Adding `ModelClient` there would invert the layering (core depends on hooks, never the reverse) and drag core's dep tree into hooks. So `codex-hooks` defines/imports only a **trait** and dispatches to it; the trait impl lives elsewhere.

### Important correction to the research pack's seam

The integration map proposed implementing the prompt call against `codex-api`'s `ResponsesClient`, "the same client surface `ModelClient::summarize_memories` uses." Verified against source, this is inaccurate in two ways:

1. `ResponsesClient` is **streaming-only** — its public methods are `stream` / `stream_request` returning a `ResponseStream` (`codex-rs/codex-api/src/endpoint/responses.rs:70-152`). There is no non-streaming completion method to "fire and collect."
2. `ModelClient::summarize_memories` does **not** use `ResponsesClient`; it uses a dedicated `ApiMemoriesClient` against the `/memories/summarize` endpoint (`codex-rs/core/src/client.rs:600-617`).

Re-deriving a `ResponsesClient` from `auth_manager` + provider inside the new crate would mean re-implementing the auth/header/telemetry plumbing that `ModelClient` already owns (`current_client_setup`, `build_subagent_headers`, attestation, retries). That duplicates a lot of `core`-private machinery and is fragile across upstream merges.

**Recommended seam (decided in Section 10, Decision A):** The new crate `codex-hook-llm-runner` defines the trait `LlmHookInvoker` plus the request/result data types and the `$ARGUMENTS`/JSON-schema/output-parsing helpers (all model-transport-agnostic). The **concrete impl is a thin adapter constructed in `codex-core`** (in `session/mod.rs`, next to `build_hooks_for_config`) that captures the session's `ModelClient` and performs a single non-streaming-style turn by calling `ModelClient::stream(...)` and collecting the final text. `codex-core` already depends on both `codex-hooks` and `codex-hook-llm-runner`; this is the single narrow wiring point. This keeps all auth/header/retry logic where it already lives, keeps `codex-hooks` model-free, and adds no model code to a new transport surface.

```
codex-config      codex-protocol
      \                /
       \              /
        codex-hooks  ──────────────►  codex-hook-llm-runner
         (imports the LlmHookInvoker      (trait + LlmHookRequest/Result +
          trait; never the impl)            $ARGUMENTS / schema helpers;
                                            NO codex-core/api dep)
                         ▲
                         │ (Arc<dyn LlmHookInvoker>)
                         │
                     codex-core
            (constructs the concrete impl from its
             ModelClient and injects it via HooksConfig;
             owns the agent-hook spawn path in hook_runtime.rs)
```

---

## 4. Architecture

### 4.1 New config types (`codex-rs/config/src/hook_config.rs`)

Grow the two empty variants. Keep field names matching the TS schema (`prompt`, `model`, `timeout`, `statusMessage`).

```rust
// hook_config.rs — replace the empty `Prompt {}` / `Agent {}` arms
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum HookHandlerConfig {
    #[serde(rename = "command")]
    Command { /* unchanged */ },

    #[serde(rename = "prompt")]
    Prompt {
        prompt: String,
        #[serde(default)]
        model: Option<String>,
        #[serde(default, rename = "timeout")]
        timeout_sec: Option<u64>,
        #[serde(default, rename = "statusMessage")]
        status_message: Option<String>,
    },

    #[serde(rename = "agent")]
    Agent {
        prompt: String,
        #[serde(default)]
        model: Option<String>,
        #[serde(default, rename = "timeout")]
        timeout_sec: Option<u64>,
        #[serde(default, rename = "statusMessage")]
        status_message: Option<String>,
    },
}
```

Note: the `if` / `once` fields from the TS schema are deferred (Section 12). Adding fields to a `#[derive(JsonSchema)]` enum requires a `just write-hooks-schema` regeneration (Section 6).

### 4.2 The runner trait + data types (`codex-rs/hook-llm-runner/src/lib.rs`)

Decision B (Section 10) rejects the RPITIT `LlmHookInvoker` trait as unnecessarily complex for a deliberately object-safe seam. Only the `DynLlmHookInvoker` with a `BoxFuture` method is shipped; `CoreLlmHookInvoker` implements it directly.

```rust
use std::time::Duration;
use codex_protocol::protocol::{HookEventName, HookHandlerType};

/// One LLM-backed hook invocation, transport-agnostic.
#[derive(Debug, Clone)]
pub struct LlmHookRequest {
    pub handler_type: HookHandlerType,     // Prompt | Agent (never Command)
    pub event_name: HookEventName,
    /// Already `$ARGUMENTS`-substituted prompt text.
    pub prompt: String,
    /// Override model slug; None => session's own model_info (see Section 10, Decision D).
    pub model: Option<String>,
    /// Raw hook-input JSON (the same blob a command hook gets on stdin).
    pub input_json: String,
    /// Transcript path for agent hooks to read; None for prompt hooks.
    pub transcript_path: Option<std::path::PathBuf>,
    pub timeout: Duration,
}

/// The `{ok, reason?}` decision plus a coarse status for reporting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmHookResult {
    /// ok:true
    Ok,
    /// ok:false
    Blocked { reason: Option<String> },
    /// JSON parse / schema mismatch — surfaces as a Failed run, non-blocking.
    NonBlockingError { detail: String },
    /// Timeout / abort / (agent) no structured output — silent, non-blocking.
    Cancelled,
}
```

```rust
// hook-llm-runner/src/dyn_invoker.rs
use futures::future::BoxFuture;

/// Object-safe seam stored behind `Arc<dyn …>` by the hooks engine.
/// Mirrors the existing `HookFn = Arc<dyn Fn … -> BoxFuture>` pattern in
/// `hooks/src/types.rs`.  `CoreLlmHookInvoker` (in codex-core) implements
/// this directly — no intermediate RPITIT layer.
pub trait DynLlmHookInvoker: Send + Sync {
    fn invoke_boxed(&self, request: LlmHookRequest) -> BoxFuture<'_, LlmHookResult>;
}
```

The `codex-hooks` engine stores `Option<Arc<dyn DynLlmHookInvoker>>`.

Shared helpers also live in this crate (no model dep): `substitute_arguments(prompt, input_json) -> String` (port of `substituteArguments`), `parse_decision(text) -> LlmHookResult` (port of `hookResponseSchema().safeParse`), and `hook_output_json_schema() -> serde_json::Value` (the `{ok, reason?}` schema for `outputFormat`).

### 4.3 Config-side handler representation in `codex-hooks`

`ConfiguredHandler` (`codex-rs/hooks/src/engine/mod.rs:41-52`) is command-only today. Add a `kind` discriminant rather than rewriting the struct, to minimize churn in the many sites that read `handler.command`, `handler.timeout_sec`, `handler.matcher`, etc.

```rust
// engine/mod.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HandlerKind {
    Command,
    Prompt { prompt: String, model: Option<String> },
    Agent  { prompt: String, model: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfiguredHandler {
    pub event_name: HookEventName,
    pub matcher: Option<String>,
    /// For Command this is the shell command; for Prompt/Agent it holds the
    /// (un-substituted) prompt text so existing `run_id`/reporting is unchanged.
    pub command: String,
    pub timeout_sec: u64,
    pub status_message: Option<String>,
    pub source_path: AbsolutePathBuf,
    pub source: HookSource,
    pub display_order: i64,
    pub env: HashMap<String, String>,
    pub kind: HandlerKind,   // NEW
}

impl ConfiguredHandler {
    pub(crate) fn handler_type(&self) -> HookHandlerType {
        match self.kind {
            HandlerKind::Command   => HookHandlerType::Command,
            HandlerKind::Prompt{..} => HookHandlerType::Prompt,
            HandlerKind::Agent{..}  => HookHandlerType::Agent,
        }
    }
}
```

**Compile-time migration note.** `ConfiguredHandler` does not derive `Default` (`mod.rs:41` — only `Debug, Clone, PartialEq, Eq`). Adding the `kind` field will break every struct-literal constructor. The fix is to add `#[derive(Default)]` to `HandlerKind` (returning `Command`) and add `Default` to `ConfiguredHandler`; test helpers can then use `..Default::default()` to fill `kind` without touching every literal site. Phase 1 step 3 must update all construction sites in `dispatcher.rs` tests (e.g. lines 172-184) and `discovery.rs` tests.

`dispatcher::running_summary` (`dispatcher.rs:70-87`) and `completed_summary` (`dispatcher.rs:118-140`) currently hardcode `HookHandlerType::Command`; switch both to `handler.handler_type()`. This satisfies the exhaustive-match convention and makes the wire `handler_type` correct for the TUI.

### 4.4 Dispatch path

`dispatcher::execute_handlers` (`dispatcher.rs:89-116`) is hardwired to `run_command` and the `parse: fn(&ConfiguredHandler, CommandRunResult, …)` callback — it cannot host the LLM path as-is (the research pack's gotcha is correct). Introduce a sibling that splits handlers by kind and runs LLM leaves through the injected invoker:

```rust
// engine/llm_dispatcher.rs (NEW, <300 LoC)
pub(crate) struct LlmRunResult {
    pub started_at: i64,
    pub completed_at: i64,
    pub duration_ms: i64,
    pub result: codex_hook_llm_runner::LlmHookResult,
}

/// Runs the Prompt/Agent leaves of an already-selected handler set.
/// Prompt hooks may run concurrently; Agent hooks are serialized (see gotcha).
/// `turn_id` is threaded into parse() to populate HookCompletedEvent.turn_id,
/// mirroring the existing execute_handlers pattern (dispatcher.rs:94-95).
pub(crate) async fn execute_llm_handlers<T>(
    invoker: Option<&Arc<dyn DynLlmHookInvoker>>,
    handlers: Vec<ConfiguredHandler>,   // only Prompt/Agent kinds
    input_json: &str,
    transcript_path: Option<&Path>,
    turn_id: Option<String>,
    parse: fn(&ConfiguredHandler, LlmRunResult, Option<String>) -> ParsedHandler<T>,
) -> Vec<ParsedHandler<T>> { /* … */ }
```

Each event's `run()` (e.g. `pre_tool_use::run`) keeps calling `execute_handlers` for command leaves and additionally calls `execute_llm_handlers` for prompt/agent leaves, then merges the two `Vec<ParsedHandler<T>>` by `display_order` before computing `should_block` / `block_reason` exactly as today.

### 4.5 Mapping `LlmHookResult` into the existing PreToolUse gating

The existing `parse_completed` for PreToolUse (`pre_tool_use.rs:188-303`) maps a `CommandRunResult` to `PreToolUseHandlerData { should_block, block_reason, … }`. We add a parallel mapping from `LlmHookResult`:

```rust
// pre_tool_use.rs — new sibling of parse_completed
fn parse_completed_llm(
    handler: &ConfiguredHandler,
    run: LlmRunResult,
    turn_id: Option<String>,
) -> dispatcher::ParsedHandler<PreToolUseHandlerData> {
    use codex_hook_llm_runner::LlmHookResult::*;
    let (status, should_block, block_reason, entries) = match run.result {
        Ok => (HookRunStatus::Completed, false, None, vec![]),
        Blocked { reason } => {
            let reason = reason.unwrap_or_else(|| "blocked by hook".to_string());
            (HookRunStatus::Blocked, true, Some(reason.clone()),
             vec![HookOutputEntry { kind: Feedback, text: reason }])
        }
        NonBlockingError { detail } =>
            (HookRunStatus::Failed, false, None,
             vec![HookOutputEntry { kind: Error, text: detail }]),
        Cancelled => (HookRunStatus::Stopped, false, None, vec![]),
    };
    // … wrap in HookCompletedEvent via a small completed_summary_llm helper
}
```

This reuses `PreToolUseOutcome.should_block` / `block_reason`, so the downstream gating in `hook_runtime.rs:185-217` (`PreToolUseHookResult::Blocked(String)` → system reminder fed to the model) works unchanged. The `preventContinuation` asymmetry between prompt and agent (Section 2) is irrelevant to PreToolUse — both map to `should_block:true` — but is preserved as a flag on `LlmHookResult::Blocked` if/when Stop hooks consume it (Section 12).

### 4.6 Agent-hook execution lives in `codex-core`, not `codex-hooks`

A real sub-agent needs `SessionServices.agent_control` (the spawn machinery in `core/src/tools/handlers/multi_agents_v2/spawn.rs` and `core/src/agent/control.rs`), which `codex-hooks` cannot reach without inverting the layering. Therefore the **agent** variant of `LlmHookInvoker::invoke` is implemented in core's adapter (Section 4.7) where `Arc<Session>` is in scope. The `codex-hooks` engine merely forwards an `LlmHookRequest { handler_type: Agent, … }` through the same injected trait; it does not know whether the invoker forks a sub-agent or makes a flat call.

This keeps the hooks crate uniform (one trait, two handler types) while honoring the constraint that agent spawning happens in core.

### 4.7 The concrete invoker (in `codex-core`)

```rust
// core/src/hook_llm_invoker.rs (NEW)
pub(crate) struct CoreLlmHookInvoker {
    model_client: ModelClient,           // ModelClient is Clone; holds the session's client
    model_info: ModelInfo,               // session's own model; used when req.model is None
    session_telemetry: SessionTelemetry,
    // For agent hooks (Phase 3): a handle able to spawn a verifier sub-agent.
    agent_spawn: Option<AgentHookSpawner>,
}

impl codex_hook_llm_runner::DynLlmHookInvoker for CoreLlmHookInvoker {
    fn invoke_boxed(&self, req: LlmHookRequest) -> BoxFuture<'_, LlmHookResult> {
        Box::pin(async move {
            match req.handler_type {
                HookHandlerType::Prompt => self.invoke_prompt(req).await,
                HookHandlerType::Agent  => self.invoke_agent(req).await,  // Phase 3
                HookHandlerType::Command => LlmHookResult::NonBlockingError {
                    detail: "command hooks are not LLM hooks".into(),
                },
            }
        })
    }
}
```

`invoke_prompt` creates a fresh `ModelClientSession` per invocation via `self.model_client.new_session()` (verified: `ModelClient::new_session(&self)` at `core/src/client.rs:378`; `stream` is on `ModelClientSession`, `client.rs:1587`). It then builds a one-message turn (developer/system text = the fixed evaluator prompt from `execPromptHook.ts:64-70`; user text = `req.prompt`), sets the Responses `text.format` to the `{ok, reason?}` JSON schema via the existing `output_schema` plumbing (`codex-rs/codex-api/src/common.rs:278`), and calls:

```rust
session.stream(
    &prompt,
    &model_info,          // req.model resolved via models_manager, or self.model_info
    &self.session_telemetry,
    None,                 // no reasoning for hook invocations
    ReasoningSummaryConfig::default(),
    service_tier,
    None,                 // no turn_metadata_header
    &InferenceTraceContext::disabled(),
).await
```

It collects the final assistant text (pattern: `compact.rs:547-560`), calls `codex_hook_llm_runner::parse_decision`, and wraps everything in `tokio::time::timeout(req.timeout, …)` → `Cancelled` on elapse (mirrors `command_runner.rs:71-99`).

`invoke_agent` (Phase 3) forks a `dontAsk` sub-agent via `agent_spawn`, injects a synthetic structured-output tool + a Stop enforcement gate keyed to the sub-agent's id, caps turns at 50, and watches for the structured-output payload — the Rust analogue of `execAgentHook.ts`.

### 4.8 Data-flow diagram

```
tool call about to run
        │
        ▼
core/hook_runtime.rs::run_pre_tool_use_hooks
        │  builds PreToolUseRequest (input JSON, transcript_path, model, …)
        ▼
codex-hooks  ClaudeHooksEngine::run_pre_tool_use
        │
        ▼
events/pre_tool_use::run
        ├── select handlers (matcher) ──────────────────────────────┐
        │                                                            │
        ├─ command leaves ─► dispatcher::execute_handlers            │
        │                     (FuturesUnordered → run_command)       │
        │                                                            │
        └─ prompt/agent leaves ─► llm_dispatcher::execute_llm_handlers
                                   │  Arc<dyn DynLlmHookInvoker>
                                   ▼
                          core CoreLlmHookInvoker::invoke_boxed
                             ├─ Prompt: model_client.new_session().stream(...) (json_schema) → {ok,reason}
                             └─ Agent : fork dontAsk sub-agent (Phase 3) → {ok,reason}
        │                                                            │
        ▼  merge by display_order                                   │
   PreToolUseOutcome { should_block, block_reason, … } ◄────────────┘
        │
        ▼
core/hook_runtime.rs → PreToolUseHookResult::Blocked(reason) → system reminder to model
```

---

## 5. Integration points (exact files in `codex-rs`)

| File:line | Change |
|---|---|
| `config/src/hook_config.rs:153-155` | Grow `Prompt {}` / `Agent {}` with `prompt`/`model`/`timeout_sec`/`status_message` (Section 4.1). |
| `hooks/src/engine/mod.rs:41-52` | Add `kind: HandlerKind` to `ConfiguredHandler` + `handler_type()` helper (Section 4.3). |
| `hooks/src/engine/mod.rs:100-105` (struct) and `107-138` (`new()`) | Add `llm_invoker: Option<Arc<dyn DynLlmHookInvoker>>` field to `ClaudeHooksEngine` struct definition; add the parameter and assignment in `new()`. |
| `hooks/src/engine/discovery.rs:516-523` | Replace the two warn-and-skip arms with real handler construction (trust hash, enabled/disabled, push `ConfiguredHandler { kind: Prompt/Agent, … }`). |
| `hooks/src/engine/discovery.rs:431-514` | Factor the trust/state/enabled/`hook_entries.push` block so prompt/agent reuse it (currently command-only inside the `match`; `Command` arm begins at line 431, not 438 — line 438 is inside the arm body). |
| `hooks/src/engine/discovery.rs:531-555` | Generalize `command_hook_hash` → `hook_hash` over the normalized `HookHandlerConfig` (already takes `normalized_handler`; just pass the Prompt/Agent variant). |
| `hooks/src/engine/dispatcher.rs:70-140` | `running_summary` / `completed_summary` use `handler.handler_type()` instead of hardcoded `Command`. |
| `hooks/src/engine/llm_dispatcher.rs` (NEW) | `execute_llm_handlers`, `LlmRunResult`, LLM `completed_summary` helper (Section 4.4). |
| `hooks/src/events/pre_tool_use.rs:71-142` | Split selected handlers by kind; call `execute_llm_handlers` for prompt/agent; merge results before `should_block`. Add `parse_completed_llm` (Section 4.5). |
| `hooks/src/events/permission_request.rs` | Same split (maps `Blocked` → `PermissionRequestDecision::Deny`). |
| `hooks/src/events/{post_tool_use,user_prompt_submit,stop,session_start,compact}.rs` | Same split per event; prompt/agent map to that event's existing outcome fields. (Phase 2+; PreToolUse + PermissionRequest are the v1 gate.) |
| `hooks/src/registry.rs:29-39` | Add `llm_invoker: Option<Arc<dyn DynLlmHookInvoker>>` to `HooksConfig`; pass to `ClaudeHooksEngine::new` (line 67-77). |
| `hooks/Cargo.toml:14-30` | Add `codex-hook-llm-runner = { workspace = true }` (trait + helpers only; no model deps, so no feature gate needed). |
| `core/src/hook_llm_invoker.rs` (NEW) | `CoreLlmHookInvoker` implementing `DynLlmHookInvoker` directly via `invoke_boxed` (Section 4.7). Extracting the construction here keeps `session/mod.rs` (3316 lines, a hot upstream file) as purely wiring. |
| `core/src/session/mod.rs:3291-3313` | In `build_hooks_for_config`, construct `CoreLlmHookInvoker` and set `HooksConfig.llm_invoker`. Extend the function signature to `(config: &Config, plugins_manager: &PluginsManager, user_shell: &Shell, model_client: &ModelClient, model_info: ModelInfo, session_telemetry: SessionTelemetry)` — these are available at both call sites via `self.services.model_client` and `self.services.session_telemetry` (`state/service.rs:79` and `:58`). The construction is ~10 additive lines; the invoker itself lives in `hook_llm_invoker.rs` to avoid growing this already-large file. |
| `core/Cargo.toml` | Add `codex-hook-llm-runner = { workspace = true }` (the one narrow place core depends on it). |
| `protocol/src/protocol.rs:1370-1374` | No change — `HookHandlerType::{Prompt,Agent}` already exist. |
| `app-server-protocol/src/protocol/v2/hook.rs:24-28` | No change — already mapped. |

`hooks/src/engine/command_runner.rs` is **untouched** (the shell path is a parallel sibling).

---

## 6. Config & schema changes

1. **`HookHandlerConfig`** gains fields on the `Prompt`/`Agent` variants (`codex-config`). Because the type derives `JsonSchema` and is referenced from `ConfigToml` (via the hooks config), regenerate both:
   - `just write-hooks-schema` — regenerates `codex-rs/hooks/schema/generated/*.schema.json`. Today these are command-only (e.g. `pre-tool-use.command.input.schema.json`); the generator (`hooks/src/bin/write_hooks_schema_fixtures.rs`) may need new fixtures for prompt/agent **output** if we choose to surface a schema for them — for v1 the LLM result is internal (not a stdin/stdout contract), so the existing command fixtures are sufficient and only the config schema changes.
   - `just write-config-schema` — regenerates `core/config.schema.json` (the `ConfigToml` schema includes hook config).
2. **App-server schema** (`just write-app-server-schema`): no change expected, since `HookRunSummary`/`HookHandlerType` are unchanged. Run it to confirm a clean diff.
3. **Bazel reconciliation** (deps changed — new crate + two `Cargo.toml` edits):
   - Add `codex-rs/hook-llm-runner/BUILD.bazel` (srcs, deps: `codex-protocol`, `serde`, `serde_json`, `futures`, `regex`; **no** core/api).
   - Add the new crate to the workspace members list in `codex-rs/Cargo.toml`.
   - `just bazel-lock-update` then `just bazel-lock-check`; commit `MODULE.bazel.lock`.
   - Update `codex-rs/hooks/BUILD.bazel` and `codex-rs/core/BUILD.bazel` deps. No `include_str!`/`include_bytes!` added, so no `compile_data` changes.

---

## 7. Implementation workflow (phased)

### Phase 1 — v1: prompt hook end-to-end through the PreToolUse/PermissionRequest gate

Ordered steps:

1. **New crate skeleton** `codex-rs/hook-llm-runner/`: `LlmHookRequest`, `LlmHookResult`, `DynLlmHookInvoker` (single `BoxFuture` method; no RPITIT layer per Decision B), `substitute_arguments`, `parse_decision`, `hook_output_json_schema`. Add to workspace + BUILD.bazel. Unit tests for `substitute_arguments` and `parse_decision`.
2. **Config fields** (`hook_config.rs`): grow `Prompt {}` (and `Agent {}` for forward-compat, even though Agent dispatch lands Phase 3). `just write-hooks-schema` + `just write-config-schema`; commit fixtures.
3. **Engine plumbing** (`engine/mod.rs`, `registry.rs`): add `HandlerKind`, `ConfiguredHandler.kind`, `handler_type()`, `ClaudeHooksEngine.llm_invoker`, `HooksConfig.llm_invoker`. Add `#[derive(Default)]` to `HandlerKind` (returning `Command`) and `Default` to `ConfiguredHandler`; update all struct-literal construction sites in `dispatcher.rs` and `discovery.rs` tests to include `kind: HandlerKind::Command` (or use `..Default::default()`).
4. **Discovery** (`discovery.rs`): factor the trust/enabled/hash/`hook_entries.push` block out of the `Command` arm; build `ConfiguredHandler { kind: Prompt{…} }` for the `Prompt` arm (still warn-and-skip `Agent` until Phase 3). Reuse `hook_hash` for the trust check.
5. **Dispatcher** (`dispatcher.rs`): `running_summary`/`completed_summary` → `handler.handler_type()`.
6. **LLM dispatcher** (`llm_dispatcher.rs` NEW): `execute_llm_handlers` (concurrent for prompt; the serialization rule matters only once agents exist).
7. **PreToolUse + PermissionRequest** (`pre_tool_use.rs`, `permission_request.rs`): split selected handlers by kind; run LLM leaves; `parse_completed_llm`; merge by `display_order`.
8. **Core adapter** (`core/src/hook_llm_invoker.rs` NEW): `CoreLlmHookInvoker` implementing `DynLlmHookInvoker::invoke_boxed`. `invoke_prompt` calls `self.model_client.new_session()` to get a `ModelClientSession`, then drives `session.stream(prompt, &model_info, &self.session_telemetry, None, ReasoningSummaryConfig::default(), service_tier, None, &InferenceTraceContext::disabled())` with JSON-schema `text.format` and `tokio::time::timeout`.
9. **Wire it** (`session/mod.rs::build_hooks_for_config`): extend signature to accept `model_client: &ModelClient, model_info: ModelInfo, session_telemetry: SessionTelemetry` (from `self.services`), construct `CoreLlmHookInvoker`, and set `HooksConfig.llm_invoker`.

**Phase 1 verification checkpoint:**
- `just fmt`
- `just test -p codex-hook-llm-runner` (substitution + parse unit tests green).
- `just test -p codex-hooks` (existing tests still pass; new test: a `Prompt` handler with a mock `DynLlmHookInvoker` returning `Blocked{reason}` produces `PreToolUseHandlerData { should_block:true, block_reason:Some(_) }`; another returning `Ok` does not block; timeout → `Cancelled` → non-blocking `Stopped` status).
- `just test -p codex-core` then ask before full `just test` (core change).
- Manual: a `hooks.json` with a `prompt` PreToolUse hook on `^Bash$` blocks/allows a `Bash` call; `codex` no longer prints "prompt hooks are not supported yet".

### Phase 2 — prompt hooks for the remaining events

Extend the kind-split to `post_tool_use`, `user_prompt_submit`, `stop`, `session_start`, `compact`. Map `LlmHookResult` to each event's existing outcome fields (e.g. Stop's `should_block`/`continuation_fragments`). Verification: `just test -p codex-hooks` with a per-event prompt-hook test each.

### Phase 3 — agent hook (sub-agent verifier)

1. Implement `AgentHookSpawner` in core over `multi_agents_v2`/`agent::control` spawn (`spawn.rs`): a `dontAsk`, depth-bumped, `isNonInteractiveSession` sub-agent; tool set minus disallowed-agent-tools plus a synthetic structured-output tool; a Stop enforcement gate keyed to the sub-agent id; `MAX_AGENT_TURNS = 50`; 60s default timeout; transcript read access.
2. Implement `CoreLlmHookInvoker::invoke_agent`; watch for the structured-output payload; `Cancelled` on max-turns/no-output; `Blocked`/`Ok` otherwise.
3. Flip the `Agent` discovery arm from warn-and-skip to real construction.

**Phase 3 verification checkpoint:** `just test -p codex-core` with a mock-SSE sub-agent run (use `core_test_support::responses`) that calls the synthetic tool with `{ok:false, reason}` → PreToolUse blocks; a run that hits max turns → Cancelled (no block, no user-facing error).

---

## 8. Test strategy

- **`codex-hook-llm-runner`** (`just test -p codex-hook-llm-runner`): unit tests for `substitute_arguments` (`$ARGUMENTS`, `$ARGUMENTS[0]`, `$0`, append-when-absent), `parse_decision` (`{ok:true}`, `{ok:false,reason}`, malformed JSON → `NonBlockingError`, schema mismatch → `NonBlockingError`), and `hook_output_json_schema` shape.
- **`codex-hooks`** (`just test -p codex-hooks`): a `MockInvoker: DynLlmHookInvoker` returning canned `LlmHookResult`s. Tests: prompt `Blocked` → `should_block:true` with reason; prompt `Ok` → no block; `Cancelled` → non-blocking with `HookRunStatus::Stopped`; `NonBlockingError` → `HookRunStatus::Failed`, no block; mixed command+prompt handlers on one PreToolUse event merge in `display_order` and any block wins (mirror the existing `results.iter().any(should_block)` semantics, `pre_tool_use.rs:115`); discovery builds a `Prompt` `ConfiguredHandler` and respects trust/enabled state.
- **`codex-core`** (`just test -p codex-core`): `CoreLlmHookInvoker::invoke_prompt` against mock SSE (`core_test_support::responses`) asserting the outbound `/responses` body carries the `{ok, reason?}` JSON-schema `text.format` and the evaluator system prompt; timeout path → `Cancelled`. Phase 3: agent-hook spawn integration test (mock SSE driving the synthetic tool).
- **Snapshots:** none required — these handlers produce no TUI output of their own; the existing `HookCompleted` rendering covers the `HookRunSummary` (now with `handler_type: Prompt/Agent`). If a `codex-tui` hook-list snapshot exists, refresh it with `cargo insta` after Phase 1.
- **Schema fixtures:** `just write-hooks-schema` / `just write-config-schema` diffs are committed and CI-validated; treat a dirty diff after build as a failing check.

---

## 9. Upstream-merge safety analysis

**Footprint.** New code is concentrated in a brand-new crate (`hook-llm-runner`, zero merge conflict surface) and a new core module (`hook_llm_invoker.rs`). Edits to existing upstream files are deliberately small and structural:

- `command_runner.rs`, `output_parser.rs`, `schema.rs`, the per-event `*Output` structs — **untouched**.
- `dispatcher.rs` — two one-line changes (`handler_type()` in two summaries).
- `pre_tool_use.rs` / `permission_request.rs` — additive (`parse_completed_llm` + a kind-split in `run`); the existing `parse_completed` and tests are unchanged.
- `discovery.rs` — the riskiest edit: refactoring the trust/enabled/hash block out of the `Command` arm so prompt/agent can share it. Mitigate by keeping the extracted helper a pure function with the same inputs the inline code uses today, so a future upstream change to trust logic re-applies cleanly.
- `engine/mod.rs` `ConfiguredHandler` gains one field with a `Default`-friendly `Command` value — additive; upstream additions to the struct merge alongside.
- `session/mod.rs` `build_hooks_for_config` — additive (~10 lines) in a 3316-line hot upstream file. Low conflict probability, but to honor the prime directive the invoker *construction* logic is extracted into `core/src/hook_llm_invoker.rs`; `session/mod.rs` remains purely wiring (construct + pass).

**Conflict risk on hot files.** `chatwidget.rs`/`app.rs`/`bottom_pane` are **not** touched. `codex-core` growth is limited to one new module + ~10 lines in `session/mod.rs` — within the prime-directive budget because the heavy logic is in the sibling crate.

**Isolation strategy.** The trait seam means upstream changes to `ModelClient` only affect `core/src/hook_llm_invoker.rs` (one file), never the hooks crate. Because `HookHandlerType::{Prompt,Agent}` and the v2 mapping already exist upstream-side, there is no protocol churn to reconcile.

---

## 10. Open design decisions & tradeoffs

**Decision A — Where does the model call physically happen?**
Options: (1) re-derive a `ResponsesClient` from auth+provider inside `hook-llm-runner` (research-pack proposal); (2) implement the trait in `codex-core` over the session's existing `ModelClient` and inject it.
Recommendation: **(2).** Verified that `ResponsesClient` is streaming-only and that `summarize_memories` uses a different client, so (1) would duplicate core-private auth/header/retry/attestation plumbing and is merge-fragile. (2) reuses `ModelClient` and keeps `codex-hooks` model-free.

**Decision B — RPITIT trait vs. boxed-future trait for the seam.**
Options: (1) `LlmHookInvoker` with RPITIT + a `DynLlmHookInvoker` erasure; (2) a single `DynLlmHookInvoker` with a `BoxFuture` method.
Recommendation: **(2) for v1.** The engine must store the invoker behind `Arc<dyn …>`, so a boxed-future trait is the only thing actually stored; the RPITIT layer adds a generic indirection with no caller benefit here. AGENTS.md's RPITIT rule targets ordinary async traits; a deliberately object-safe seam is the idiomatic exception (mirrors the existing `HookFn = Arc<dyn Fn … -> BoxFuture>` in `hooks/src/types.rs:12`).

**Decision C — Agent-hook spawn: reuse `multi_agents_v2` or a bespoke lighter loop?**
Options: (1) route through `agent_control.spawn_agent_with_metadata` (full thread lifecycle); (2) a bespoke minimal turn loop in core.
Recommendation: **(1).** The agent hook genuinely needs tools, a turn loop, and transcript access — exactly what spawn provides. Building a parallel mini-loop would re-implement turn orchestration. Accept the heavier lifecycle for the `Agent` variant only; keep `Prompt` on the flat single-turn path (Decision A). This matches the research pack's nuance that spawn is right for Agent, wrong for Prompt.

**Decision D — Default model for LLM hooks.**
Options: (1) a dedicated small/fast-model tier; (2) the session's own `model_info`; (3) a hardcoded slug.
Recommendation: **(2).** There is no `small_fast_model` concept in the Rust model registry (verified: zero occurrences of `small_fast`/`SmallFast`/`haiku`/`getSmallFast` in `codex-rs/**/*.rs`). Re-inventing a tier here would add new config surface. Safest, upstream-merge-safe choice: use the session's own `model_info` when `hook.model` is `None`. `CoreLlmHookInvoker` holds `model_info: ModelInfo` (the session's resolved model) and calls `models_manager.get_model_info(req.model.as_deref().unwrap_or(self.model_info.slug.as_str()))` at `invoke_boxed` time. A future `hooks.default_model` config key can override without API churn. Hook authors who want a cheaper model must set `model` explicitly in their hook config for now.

**Decision E — Concurrency of LLM hooks within one event.**
Options: (1) run all LLM leaves concurrently like command hooks; (2) serialize agent hooks; (3) serialize all LLM hooks.
Recommendation: **(2).** Prompt hooks are cheap single calls — run concurrently. Agent hooks each drive a multi-turn sub-agent; running several concurrently risks model-context thrash and quota exhaustion (research-pack gotcha). Serialize agent hooks; allow prompt hooks to fan out.

---

## 11. Risks & mitigations

- **Discovery refactor regresses command-hook trust/enabled semantics.** The `Command` arm owns the trust-hash + enabled + `hook_entries.push` logic that the existing tests (`discovery.rs:688-822`) assert. Mitigation: extract it as a pure helper with identical inputs/outputs; keep all existing tests green before adding prompt/agent cases.
- **Layering violation creep.** Someone could be tempted to add `codex-api` to `codex-hooks` for "convenience." Mitigation: the trait lives in `hook-llm-runner` (no model deps); add a CI/grep check or a comment in `hooks/Cargo.toml` forbidding `codex-api`/`codex-core` deps.
- **LLM blocking adds latency to every matched tool call.** A 30s default prompt-hook timeout sits in the PreToolUse critical path. Mitigation: small/fast model default; enforce `tokio::time::timeout`; `Cancelled` on elapse fails **open** (non-blocking), matching command-hook timeout behavior (`command_runner.rs:91-99`).
- **`Cancelled` semantics for PreToolUse.** TS treats agent no-output as silent cancel. Failing open (not blocking) on `Cancelled` is the safe default but means a flaky model never blocks. Mitigation: document; surface a `HookRunStatus::Stopped` entry so the user sees it ran and bailed.
- **JSON-schema enforcement support per provider.** Not all providers honor `text.format: json_schema`. Mitigation: `parse_decision` already tolerates free-form text by extracting the JSON object; mismatch → `NonBlockingError` (non-blocking), never a spurious block.
- **Agent-hook recursion / tool abuse.** A verifier sub-agent with full tools could spawn more agents or make changes. Mitigation: filter the disallowed-agent-tool set (TS `ALL_AGENT_DISALLOWED_TOOLS`), run `dontAsk` with read-scoped transcript access, cap at 50 turns. (Phase 3.)
- **Schema-fixture drift.** Forgetting `just write-hooks-schema`/`write-config-schema` breaks CI. Mitigation: Phase-1 checklist step 2; run both before committing.
- **Bazel/Cargo skew.** New crate must be in workspace members + BUILD.bazel + lock. Mitigation: Section 6 reconciliation steps; `just bazel-lock-check` in the checkpoint.

---

## 12. Out of scope / future work (parity deferred past v1)

- **`http` (webhook) handler** — adjacent, not ported here. Note the TS deadlock guard blocking `http` hooks on `SessionStart`/`Setup` events (a sandbox-consumer ordering issue) would apply.
- **`if` condition filtering** (`permissionRuleValueFromString` / `Bash(git *)` syntax) — TS filters tool-event hooks and silently drops `if` on non-tool events. Add as a per-batch matcher once the LLM handlers are stable.
- **`once`** (self-deregistering hooks) — needs per-session mutable hook state (TS `sessionHooks.ts`); no Rust equivalent yet.
- **Conversation-history priming for prompt hooks** — TS optionally prepends prior messages; v1 sends only the processed prompt.
- **Telemetry `querySource` tags** (`hook_prompt` / `hook_agent`) — wire equivalent source tags into request metadata for analytics attribution.
- **`preventContinuation` propagation to Stop hooks** — preserved as a flag on `LlmHookResult::Blocked` but only consumed by PreToolUse in v1; Stop-event consumption is future work.
- **Async prompt/agent hooks** (`async`/`asyncRewake`) — command async hooks are already skipped (`discovery.rs:443-448`); LLM async is future work.
- **Surfacing a JSON output schema fixture** for prompt/agent in `hooks/schema/generated/` if we later expose the result as a documented contract.
