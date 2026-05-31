# Proposal 01 — Microcompaction (time-based tool-result clearing)

Status: draft / implementation-ready
Audience: Rust engineer implementing this in `codex-rs`
Source-of-truth (behavior): Claude Code TS harness at `Claude Code Src/`
Source-of-truth (target): `codex-rs` Rust workspace

---

## 1. Summary & motivation

**What we port.** Claude Code's *time-based microcompaction*: a pre-request, no-model-call pass that, when the conversation has been idle long enough that the server-side prompt cache has almost certainly expired, replaces the *content* of all but the N most-recent tool-result blocks with a fixed placeholder string (`"[Old tool result content cleared]"`). It runs at the top of every turn, before the existing auto-compact check.

**What it buys us.** When the prompt cache is cold (default: idle gap > 60 minutes, matching the server's 1-hour cache TTL), the entire prompt prefix is retransmitted regardless. Old tool results — file reads, `bash` output, greps — are the bulkiest, lowest-value-per-byte part of that prefix. Clearing them in-place before the request shrinks the bytes/tokens actually sent and lowers the live token estimate that the auto-compact threshold reads, *without an LLM summarization round-trip* and without losing the conversational structure (tool calls, ordering, the recent N results all survive). It is strictly cheaper than auto-compaction and runs first, so it reduces how often auto-compaction (which costs a model call) fires.

**Why now.** Codex already has the matching machinery: per-item `FunctionCallOutput` truncation (`context_manager/history.rs:377`), a single pre-turn compaction hook (`session/turn.rs:711`), and a clean clone-transform-replace history primitive (`Session::replace_history`, `session/mod.rs:2568`). The port is a *new crate* plus a one-call seam into one existing core function — high leverage, low footprint, upstream-merge-safe.

**v1 scope (this proposal's Phase 1).** TIME-BASED clearing only, no model call: track the last-assistant-message wall-clock time; when the idle gap exceeds a configurable threshold, replace the body of all but the N most-recent compactable tool results with the placeholder, running every turn *before* the auto-compact check, so the freed tokens lower the auto-compact estimate. The cached-API variant (server-side `cache_edits`) and the reactive 413-recovery variant are explicitly future phases (§12).

---

## 2. Source-of-truth behavior (Claude Code)

All citations into `Claude Code Src/`.

**Entry & dispatch.** `microcompactMessages` (`src/services/compact/microCompact.ts:253`) clears the compact-warning suppression flag, then calls `maybeTimeBasedMicrocompact` first (`microCompact.ts:267`); if it returns a result, it short-circuits and the cached-MC path is skipped (`microCompact.ts:268-270`). The comment is explicit that the two paths are mutually exclusive: "Cached MC is skipped when this fires: editing assumes a warm cache, and we just established it's cold" (`microCompact.ts:265-266`).

**Gate** (`evaluateTimeBasedTrigger`, `microCompact.ts:422-444`):
1. `config.enabled` must be true (default `false` — `timeBasedMCConfig.ts:31`).
2. `querySource` must be present **and** start with `repl_main_thread` (`microCompact.ts:431`, `isMainThreadSource` at `:249`). `undefined` is explicitly rejected for the time-based path (unlike cached-MC), to block analysis-only callers (`/context`, `/compact`).
3. There must be a last assistant message (`messages.findLast(m => m.type === 'assistant')`, `:434`).
4. `gapMinutes = (Date.now() - new Date(lastAssistant.timestamp).getTime()) / 60_000` must be finite and `>= config.gapThresholdMinutes` (`:438-442`). The `Number.isFinite` check guards unparseable timestamps.

**Tool-ID collection** (`collectCompactableToolIds`, `microCompact.ts:226-241`): a single forward pass over messages collects `tool_use` block IDs from *assistant* messages whose `name` is in `COMPACTABLE_TOOLS`, in encounter (chronological) order.

**Keep/clear split** (`microCompact.ts:461-467`): `keepRecent = Math.max(1, config.keepRecent)` (floor at 1 — `slice(-0)` keeps everything, and clearing all leaves zero working context, `:458-460`). `keepSet` = last `keepRecent` IDs; `clearSet` = the rest. If `clearSet` is empty, return `null`.

**Content replacement** (`microCompact.ts:469-496`): map over messages; for each *user* message, for each `tool_result` block whose `tool_use_id ∈ clearSet` **and** whose content `!== TIME_BASED_MC_CLEARED_MESSAGE` (idempotency guard, `:479`), replace `content` with the placeholder and accumulate `tokensSaved`. Untouched messages keep identity. If `tokensSaved === 0`, return `null`.

**Side effects after firing** (`microCompact.ts:498-529`): log analytics (`tengu_time_based_microcompact`), `suppressCompactWarning()`, `resetMicrocompactState()` (cached-MC module state is stale because IDs were cleared + cache is cold), and `notifyCacheDeletion(querySource)`. The result is `{ messages: result }` — `tokensSaved` is **not** part of `MicrocompactResult` and is **not** passed to autocompact (`microCompact.ts:215-220`, `query.ts` pipeline note).

**Pipeline order** (per the research pack's `query.ts` notes): `applyToolResultBudget` → snip (gated) → **microcompact** → collapses (gated) → `autoCompactIfNeeded`. Autocompact re-estimates tokens from the already-modified messages array; the savings are visible automatically because the modified user messages sit in the suffix that `tokenCountWithEstimation` rough-estimates past the last-assistant anchor.

**Key constants** (`microCompact.ts`, `timeBasedMCConfig.ts`):

| Constant | Value | Cite |
|---|---|---|
| `TIME_BASED_MC_CLEARED_MESSAGE` | `"[Old tool result content cleared]"` | `microCompact.ts:36` |
| `gapThresholdMinutes` (default) | `60` | `timeBasedMCConfig.ts:32` |
| `keepRecent` (default) | `5` | `timeBasedMCConfig.ts:33` |
| `enabled` (default) | `false` | `timeBasedMCConfig.ts:31` |
| `IMAGE_MAX_TOKEN_SIZE` | `2000` | `microCompact.ts:38` |
| `COMPACTABLE_TOOLS` | `{Read, Bash, PowerShell, Grep, Glob, WebSearch, WebFetch, Edit, Write}` | `microCompact.ts:41-50` |
| rough-token bytes-per-token | `4` (`len/4`) | `microCompact.ts:144` via `roughTokenCountEstimation` |

> Note: `COMPACTABLE_TOOLS` is built from constants like `SHELL_TOOL_NAMES`, `FILE_READ_TOOL_NAME`, etc. (`microCompact.ts:41-50`) — these are the **API-facing tool names**. The Rust port must map onto Codex's tool names (§10, decision D2).

---

## 3. Target placement in Codex

**New crate: `codex-microcompaction`** (lib name `codex_microcompaction`), directory `codex-rs/microcompaction/`.

**Why a new crate, not core.** The pure work is a synchronous history transform: scan `&[ResponseItem]`, rewrite the `output` payloads of selected `FunctionCallOutput` / `CustomToolCallOutput` items, return a new `Vec<ResponseItem>` plus a small report struct. It needs only:
- `codex-protocol` — `ResponseItem`, `FunctionCallOutputPayload`, `FunctionCallOutputBody`, `FunctionCallOutputContentItem`.
- `codex-utils-output-truncation` (re-exports `approx_token_count` from `codex-utils-string`) — for the freed-token estimate only.

It needs **no** `Arc<Session>`, `TurnContext`, async runtime, or any core-internal type. The fork prime directive is "RESIST adding to `codex-core`" (`core/CLAUDE.md`); core is already ~44k LoC. Hosting the logic in a leaf crate keeps the only core touch to a single new call inside `run_pre_sampling_compact`. This mirrors how `codex-utils-output-truncation` already lives outside core and is consumed by it.

**Why not extend `codex-utils-output-truncation`.** That crate truncates content to a byte/token *budget*. Microcompaction does a categorically different thing: it replaces a whole tool-result body with a *fixed placeholder* based on recency + tool name, with idempotency and keep-recent semantics, gated on an idle timer. Different concern, different inputs (it must know which call IDs are recent and which tool produced each output). Keep them separate; depend on it for the token estimate.

**The one core seam.** `run_pre_sampling_compact` (`core/src/session/turn.rs:711`) is the exclusive pre-turn hook. We add one call there, immediately before `auto_compact_token_status` (`turn.rs:717`). The crate exposes a pure free function; core does clone → call → `replace_history` when items changed. v1 also adds a `last_assistant_response_at` timestamp field to `SessionState` (the one unavoidable core addition — see §5/§10 D1, since `ResponseItem` carries no timestamp).

---

## 4. Architecture

### 4.1 Crate public API (`microcompaction/src/lib.rs`)

```rust
//! Pure, synchronous history transform: time-based tool-result clearing.
//! No async, no Session, no LLM call.

use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ResponseItem;

/// Exact placeholder written into a cleared tool-result body.
/// Doubles as the idempotency guard (already-cleared bodies are skipped).
pub const CLEARED_PLACEHOLDER: &str = "[Old tool result content cleared]";

/// Configuration for the time-based clearing pass. Constructed by core from
/// `ConfigToml` (§6). A named struct (not bare params) per the no-bool-arg rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MicrocompactConfig {
    pub enabled: bool,
    /// Idle minutes since the last assistant response before clearing fires.
    pub gap_threshold_minutes: u64,
    /// Number of most-recent compactable tool results to preserve.
    /// Effective value is `max(1, keep_recent)` (floor at 1).
    pub keep_recent: usize,
}

impl Default for MicrocompactConfig {
    fn default() -> Self {
        Self { enabled: false, gap_threshold_minutes: 60, keep_recent: 5 }
    }
}

/// Outcome of an `apply` pass. `messages` is `Some` only when something was
/// actually cleared (mirrors CC returning `null` on no-op).
#[derive(Debug, Clone, PartialEq)]
pub struct MicrocompactOutcome {
    /// Rewritten history. Present only when at least one body was cleared.
    pub messages: Option<Vec<ResponseItem>>,
    /// Rough freed-token estimate, for logging / threshold telemetry.
    pub tokens_freed: usize,
    /// Number of tool-result bodies cleared this pass.
    pub cleared: usize,
}

impl MicrocompactOutcome {
    pub fn no_op() -> Self {
        Self { messages: None, tokens_freed: 0, cleared: 0 }
    }
    pub fn changed(&self) -> bool {
        self.messages.is_some()
    }
}

/// Whether the idle gap is wide enough to clear, decoupled from the action so
/// callers can reuse the predicate (and so it is unit-testable without a clock).
pub fn gap_exceeds_threshold(
    idle_gap_minutes: f64,
    config: &MicrocompactConfig,
) -> bool {
    config.enabled
        && idle_gap_minutes.is_finite()
        && idle_gap_minutes >= config.gap_threshold_minutes as f64
}

/// The v1 entry point. Pure and synchronous.
///
/// Precondition: the caller has already decided the idle gap is exceeded and
/// the source is the main thread (core owns the clock + source gate, §5).
/// `items` is the full chronological history (oldest → newest).
/// `compactable_tools` is the set of Codex tool names whose outputs are eligible.
pub fn apply(
    items: &[ResponseItem],
    config: &MicrocompactConfig,
    compactable_tools: &CompactableTools,
) -> MicrocompactOutcome {
    let ids = collect_compactable_call_ids(items, compactable_tools);
    let keep_recent = config.keep_recent.max(1);
    let clear: HashSet<&str> = clear_set(&ids, keep_recent);
    if clear.is_empty() {
        return MicrocompactOutcome::no_op();
    }
    rewrite_cleared_outputs(items, &clear)
}
```

```rust
/// The set of Codex tool names eligible for clearing. Newtype over a set so the
/// caller can not pass a bare collection by mistake, and so the default set is
/// centralized. Names match Codex tool registry names (§10 D2).
#[derive(Debug, Clone)]
pub struct CompactableTools(HashSet<&'static str>);

impl Default for CompactableTools {
    fn default() -> Self {
        // Map of CC API names → verified Codex FunctionCall.name values.
        // Each entry confirmed against the tool registration site cited.
        Self(HashSet::from([
            "exec_command",   // CC Bash family (new unified tool) — shell_spec.rs:84
            "shell_command",  // CC Bash family (legacy shell tool) — shell_spec.rs:206
            "apply_patch",    // CC Edit/Write — apply_patch.rs:303
            // "grep" and "glob" are not confirmed built-in Codex tool names;
            // verify against registry.rs before adding (§10 D2).
            // "web_search" and "web_fetch" are hosted/server-side tools; the
            // string "web_search" appears in router_tests.rs:237 — confirm the
            // actual FunctionCall.name emitted in production before including.
            // CC Read ("read_file") has no matching built-in Codex tool —
            // Codex does not expose a standalone file-read tool; omitted.
        ]))
    }
}

impl CompactableTools {
    pub fn contains(&self, name: &str) -> bool {
        self.0.contains(name)
    }
}
```

### 4.2 Internal transform helpers (same file or a sibling `clear.rs`)

```rust
/// Two-phase join: FunctionCallOutput carries only `call_id`, not the tool name.
/// Walk FunctionCall items to learn which call_ids belong to compactable tools,
/// in chronological order. (CC reads names off the assistant `tool_use` blocks;
/// the Responses model splits name and output across two items — see §5.)
fn collect_compactable_call_ids(
    items: &[ResponseItem],
    tools: &CompactableTools,
) -> Vec<String> {
    let mut ids = Vec::new();
    for item in items {
        match item {
            ResponseItem::FunctionCall { name, call_id, .. } if tools.contains(name) => {
                ids.push(call_id.clone());
            }
            ResponseItem::CustomToolCall { name, call_id, .. } if tools.contains(name) => {
                ids.push(call_id.clone());
            }
            _ => {}
        }
    }
    ids
}

fn clear_set<'a>(ids: &'a [String], keep_recent: usize) -> HashSet<&'a str> {
    let keep_from = ids.len().saturating_sub(keep_recent);
    ids[..keep_from].iter().map(String::as_str).collect()
}

/// Rebuild the history, replacing the body of FunctionCallOutput /
/// CustomToolCallOutput items whose call_id is in `clear` and whose body is not
/// already the placeholder. Preserves call_id, success flag, ordering exactly.
fn rewrite_cleared_outputs(
    items: &[ResponseItem],
    clear: &HashSet<&str>,
) -> MicrocompactOutcome {
    let mut tokens_freed = 0usize;
    let mut cleared = 0usize;
    let out: Vec<ResponseItem> = items
        .iter()
        .map(|item| match item {
            ResponseItem::FunctionCallOutput { call_id, output }
                if clear.contains(call_id.as_str()) && !is_already_cleared(output) =>
            {
                tokens_freed += estimate_body_tokens(&output.body);
                cleared += 1;
                ResponseItem::FunctionCallOutput {
                    call_id: call_id.clone(),
                    output: cleared_payload(output.success),
                }
            }
            ResponseItem::CustomToolCallOutput { call_id, name, output }
                if clear.contains(call_id.as_str()) && !is_already_cleared(output) =>
            {
                tokens_freed += estimate_body_tokens(&output.body);
                cleared += 1;
                ResponseItem::CustomToolCallOutput {
                    call_id: call_id.clone(),
                    name: name.clone(),
                    output: cleared_payload(output.success),
                }
            }
            other => other.clone(),
        })
        .collect();

    if cleared == 0 {
        return MicrocompactOutcome::no_op();
    }
    MicrocompactOutcome { messages: Some(out), tokens_freed, cleared }
}

fn cleared_payload(success: Option<bool>) -> FunctionCallOutputPayload {
    FunctionCallOutputPayload {
        body: FunctionCallOutputBody::Text(CLEARED_PLACEHOLDER.to_string()),
        success,
    }
}

fn is_already_cleared(output: &FunctionCallOutputPayload) -> bool {
    matches!(&output.body, FunctionCallOutputBody::Text(t) if t == CLEARED_PLACEHOLDER)
}

/// Rough freed-token estimate (telemetry only). Text via `approx_token_count`
/// (bytes/4, matching CC's len/4); images flat 2000 (CC IMAGE_MAX_TOKEN_SIZE).
fn estimate_body_tokens(body: &FunctionCallOutputBody) -> usize {
    use codex_utils_output_truncation::approx_token_count;
    use codex_protocol::models::FunctionCallOutputContentItem as Item;
    match body {
        FunctionCallOutputBody::Text(t) => approx_token_count(t),
        FunctionCallOutputBody::ContentItems(items) => items.iter().map(|i| match i {
            Item::InputText { text } => approx_token_count(text),
            Item::InputImage { .. } => 2000,
            Item::EncryptedContent { .. } => 0,
        }).sum(),
    }
}
```

> **D5 / AGENTS.md convention:** The `other => other.clone()` catch-all in `rewrite_cleared_outputs` is a placeholder for scaffold only. **The production implementation must enumerate all `ResponseItem` variants explicitly** (matching the convention in `ContextManager::process_item`, `history.rs:398-411`). This ensures a new upstream `ResponseItem` variant (e.g. a future `ResponseItem::ComputerUseCall`) triggers a compile error rather than silently passing through. Group all non-target variants as `ResponseItem::Message { .. } | ResponseItem::Reasoning { .. } | ResponseItem::FunctionCall { .. } | ResponseItem::CustomToolCall { .. } | ... => item.clone()`. See §10 D5.

### 4.3 Core seam glue (`core/src/session/turn.rs`)

```rust
// New free fn in turn.rs (keep it tiny; the heavy logic lives in the crate).
async fn run_time_based_microcompact(
    sess: &Arc<Session>,
    turn_context: &Arc<TurnContext>,
) -> CodexResult<usize> {
    let config = turn_context.config.microcompaction; // MicrocompactConfig (from ConfigToml, §6)
    if !config.enabled {
        return Ok(0);
    }
    // Source gate: main-thread / regular turns only (sub-agents excluded). §10 D3.
    if !sess.is_main_thread_turn() {
        return Ok(0);
    }
    let Some(idle_gap_minutes) = sess.assistant_idle_gap_minutes().await else {
        return Ok(0); // no prior assistant response / no timestamp
    };
    if !codex_microcompaction::gap_exceeds_threshold(idle_gap_minutes, &config) {
        return Ok(0);
    }

    // `clone_history()` returns a ContextManager (pub(crate) type, history.rs).
    // `raw_items()` is pub(crate) — accessible here inside codex-core.
    // The slice is extracted in core BEFORE crossing the crate boundary;
    // codex-microcompaction never sees a ContextManager. §10 low note.
    let history = sess.clone_history().await;
    let outcome = codex_microcompaction::apply(
        history.raw_items(), // &[ResponseItem] — the only thing the new crate receives
        &config,
        &CompactableTools::default(),
    );
    if let Some(items) = outcome.messages {
        let reference = sess.reference_context_item().await;
        sess.replace_history(items, reference).await;
        // Clear the server-anchored token_info so get_total_token_usage() falls
        // back to pure item-estimation from the mutated history. Without this,
        // replace_history (→ ContextManager::replace, history.rs:187-190) does
        // NOT touch token_info, so auto_compact_token_status would still read
        // the pre-clear server-reported anchor. §10 D6.
        sess.clear_token_info().await;
        // Reset the idle clock so we don't re-clear every turn within the window. §10 D4.
        sess.mark_assistant_response_now().await;
    }
    Ok(outcome.tokens_freed)
}
```

```rust
// Inside run_pre_sampling_compact (turn.rs:711), BEFORE auto_compact_token_status:
async fn run_pre_sampling_compact(/* ... */) -> CodexResult<()> {
    maybe_run_previous_model_inline_compact(sess, turn_context, client_session).await?;
    let _freed = run_time_based_microcompact(sess, turn_context).await?; // NEW (one line)
    let token_status = auto_compact_token_status(sess.as_ref(), turn_context.as_ref()).await;
    if token_status.token_limit_reached { /* unchanged */ }
    Ok(())
}
```

### 4.4 Data-flow diagram

```
turn start (run_turn, turn.rs:133)
      │
      ▼
run_pre_sampling_compact (turn.rs:711)
      │
      ├─ maybe_run_previous_model_inline_compact (unchanged)
      │
      ├─ run_time_based_microcompact  ◄── NEW SEAM
      │     │
      │     ├─ config.enabled? ── no ─► return 0
      │     ├─ main-thread turn? ── no ─► return 0
      │     ├─ idle_gap = now - last_assistant_response_at   (SessionState, NEW field)
      │     ├─ gap_exceeds_threshold(gap, cfg)? ── no ─► return 0
      │     │
      │     ├─ items = sess.clone_history().raw_items()        [no lock held across transform]
      │     │
      │     │   ┌─────────────── codex_microcompaction::apply (PURE) ───────────────┐
      │     │   │ collect_compactable_call_ids: scan FunctionCall/CustomToolCall     │
      │     │   │   for names ∈ CompactableTools → call_ids (chronological)          │
      │     │   │ clear_set = all but last keep_recent (floor 1)                     │
      │     │   │ rewrite_cleared_outputs: for each FunctionCallOutput/              │
      │     │   │   CustomToolCallOutput with call_id ∈ clear and body != placeholder │
      │     │   │   → body := Text(CLEARED_PLACEHOLDER); count tokens_freed          │
      │     │   │ → MicrocompactOutcome { messages?, tokens_freed, cleared }         │
      │     │   └────────────────────────────────────────────────────────────────────┘
      │     │
      │     └─ if outcome.changed():
      │           sess.replace_history(items, reference_context_item)  (mod.rs:2568)
      │             → ContextManager::replace (history.rs:187, bumps history_version)
      │           sess.clear_token_info()   (nulls server-anchored token_info → §10 D6)
      │             → forces get_total_token_usage to use pure item-estimation
      │           sess.mark_assistant_response_now()   (reset idle clock)
      │
      ▼
auto_compact_token_status (turn.rs:717)   ◄── reads shrunk history via pure item-estimation
      │
      └─ if token_limit_reached → run_auto_compact (model call; unchanged)
```

---

## 5. Integration points

Each is a file in `codex-rs/` with the change to make.

1. **`codex-rs/Cargo.toml` workspace members** (`Cargo.toml:2`, members list; `utils/output-truncation` is at `:99`). Add `"microcompaction"` to the `members` array, and add a workspace dependency entry alongside `codex-utils-output-truncation = { path = "utils/output-truncation" }` (`Cargo.toml:229`) and `codex-protocol = { path = "protocol" }` (`Cargo.toml:198`):
   ```toml
   codex-microcompaction = { path = "microcompaction" }
   ```

2. **`codex-rs/microcompaction/Cargo.toml`** (new). Declares `codex-protocol = { workspace = true }` and `codex-utils-output-truncation = { workspace = true }`; dev-deps `pretty_assertions`. No async deps.

3. **`codex-rs/microcompaction/src/lib.rs`** + **`src/tests.rs`** (new). The crate from §4.

4. **`codex-rs/core/Cargo.toml`** — add `codex-microcompaction = { workspace = true }` next to `codex-utils-output-truncation = { workspace = true }` (`core/Cargo.toml:69`).

5. **`codex-rs/core/src/session/turn.rs:711-731`** — `run_pre_sampling_compact`. Insert `run_time_based_microcompact` call after `maybe_run_previous_model_inline_compact` (`:716`) and *before* `auto_compact_token_status` (`:717`). Add the new `run_time_based_microcompact` free fn (§4.3). This is the only logic edit to a hot upstream file; keep it to ~2 inserted lines in the existing fn plus the new helper fn.

6. **`codex-rs/core/src/state/session.rs:22`** (`SessionState`) — add field `last_assistant_response_at: Option<std::time::Instant>` (initialized `None` in `SessionState::new`, `session.rs:46`). Add `SessionState` setter/getter helpers. **This is the unavoidable core addition** — Codex's `ResponseItem` history has no per-message timestamp (verified: variants at `protocol/src/models.rs:760-856` carry no time field), so CC's `lastAssistant.timestamp` has no analogue. We track wall-clock idle time on session state instead.

7. **`codex-rs/core/src/session/mod.rs`** — add async accessors on `Session` (alongside `clone_history` at `mod.rs:2874` and `reference_context_item` at `mod.rs:2879`):
   - `mark_assistant_response_now()` — sets `last_assistant_response_at = Some(Instant::now())`.
   - `assistant_idle_gap_minutes() -> Option<f64>` — `Instant::now().duration_since(t).as_secs_f64() / 60.0` if set, else `None`.
   - `is_main_thread_turn() -> bool` — true for regular (non-sub-agent, non-internal) turns (§10 D3).
   - `clear_token_info()` — calls `self.state.lock().await.set_token_info(None)` (delegating to `state/session.rs:109` which calls `history.set_token_info` at `history.rs:77`). Required so `get_total_token_usage` falls back to pure item-estimation after `replace_history` clears tool-result bodies (§10 D6).

8. **Set the clock on turn completion.** In the turn loop where the assistant response is finalized (the `last_agent_message` path begins at `turn.rs:192`; the assistant message is recorded via `record_conversation_items`), call `sess.mark_assistant_response_now()` once the model's final assistant message for the turn is recorded. Pick the single completion point (end of `run_turn`'s response loop) — not per streamed delta.

9. **`codex-rs/core/src/session/mod.rs:2568`** — `Session::replace_history` is the correct mutation primitive (already `pub(crate)`, does **not** advance the auto-compact window). Used as-is; no change. Do **not** use `replace_compacted_history` (`mod.rs:2577`) — it calls `start_next_auto_compact_window` and persists a `RolloutItem::Compacted`.

10. **`codex-rs/core/src/context_manager/history.rs:187`** — `ContextManager::replace` bumps `history_version` (`:189`). This is reached transitively via `replace_history`; expected and correct. No direct call.

**Reference analogue (read, do not edit):** `ContextManager::process_item` (`history.rs:377-412`) and `truncate_function_output_payload` (`history.rs:462-479`) show the exact variant-match + payload-rewrite pattern. The new crate re-implements the *shape* (it does not import these `pub(crate)` core fns).

---

## 6. Config & schema changes

Add a nested config block so the three knobs are user-tunable. The toggle defaults to `false` (parity with CC, and so the v1 ships dark until validated).

**IMPORTANT — two-struct, two-crate split.** `struct ConfigToml` (the TOML-deserialization struct) lives in `codex-config/src/config_toml.rs:139`, NOT in `core/src/config/mod.rs`. `struct Config` (the resolved runtime struct) lives in `core/src/config/mod.rs:567`. Both must be updated. `core/src/config/mod.rs:27` does `use codex_config::config_toml::ConfigToml`; the existing `model_auto_compact_token_limit` field appears in **both** structs (`config_toml.rs:152` and `core/src/config/mod.rs:567`), and is wired at `mod.rs:3340`. Follow that same pattern.

**Step A — `codex-rs/config/src/config_toml.rs`** (the TOML parse struct; add alongside `model_auto_compact_token_limit` at `config_toml.rs:152`):

```rust
/// Time-based microcompaction: clear old tool-result bodies after an idle gap.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub microcompaction: Option<MicrocompactionToml>,
```

```rust
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct MicrocompactionToml {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_gap_minutes")]
    pub gap_threshold_minutes: u64,
    #[serde(default = "default_keep_recent")]
    pub keep_recent: usize,
}
fn default_gap_minutes() -> u64 { 60 }
fn default_keep_recent() -> usize { 5 }
```

`MicrocompactionToml` uses only primitive types — **no `codex-microcompaction` dep is needed** in `codex-config`. `codex-config` must NOT depend on the new crate (avoid a circular dep path through `codex-core`).

**Step B — `core/src/config/mod.rs`** (the resolved runtime struct; add alongside `model_auto_compact_token_limit` at `mod.rs:567`):

```rust
/// Resolved configuration for time-based microcompaction.
pub microcompaction: codex_microcompaction::MicrocompactConfig,
```

**Step C — Wire the resolution** at the `Self { ... }` constructor (around `mod.rs:3340`, alongside `model_auto_compact_token_limit: cfg.model_auto_compact_token_limit`):

```rust
microcompaction: cfg.microcompaction.map(|t| codex_microcompaction::MicrocompactConfig {
    enabled: t.enabled,
    gap_threshold_minutes: t.gap_threshold_minutes,
    keep_recent: t.keep_recent,
}).unwrap_or_default(),
```

`TurnContext.config.microcompaction` is then available at the seam in `turn.rs`. Reading `MicrocompactConfig` from `codex-microcompaction` in `core` is fine — `core` already depends on the new crate (§5 step 4).

**Regeneration (required because `ConfigToml` in `codex-config` changed):**
```bash
just write-config-schema   # regenerates core/config.schema.json — commit it
```
`just write-app-server-schema` and `just write-hooks-schema` are **not** needed — no wire types or hooks added.

**Bazel reconciliation (required — Cargo deps changed):**
```bash
just bazel-lock-update && just bazel-lock-check   # commit MODULE.bazel.lock
```
Create `codex-rs/microcompaction/BUILD.bazel` for the new crate. It uses no `include_str!`/`include_bytes!`, so no `compile_data` entries.

---

## 7. Implementation workflow (phased)

### Phase 1 — v1: time-based clearing, no model call (the deliverable)

Ordered steps:

1. **Scaffold the crate.** Create `codex-rs/microcompaction/{Cargo.toml,src/lib.rs,src/tests.rs,BUILD.bazel}`. Add to workspace members + workspace deps (§5.1). `just fmt`.
   - **Checkpoint:** `cargo build -p codex-microcompaction` succeeds (empty lib).

2. **Implement the pure transform** (`apply`, `collect_compactable_call_ids`, `clear_set`, `rewrite_cleared_outputs`, `estimate_body_tokens`, `gap_exceeds_threshold`, `CompactableTools`, `MicrocompactConfig`, `MicrocompactOutcome`) per §4.1/§4.2.
   - **Checkpoint:** `just test -p codex-microcompaction` — unit tests in §8 (1–8) pass; `just fix -p codex-microcompaction` clean.

3. **Add the timestamp state.** Add `last_assistant_response_at` to `SessionState` (`state/session.rs`), `Session` accessors `mark_assistant_response_now` / `assistant_idle_gap_minutes` / `is_main_thread_turn` (`session/mod.rs`), and set the clock at turn completion (`turn.rs`, §5.8).
   - **Checkpoint:** `cargo build -p codex-core`; a focused test that after a recorded assistant message `assistant_idle_gap_minutes()` returns `Some(~0.0)`.

4. **Add config.** Add `MicrocompactionToml` to `codex-config/src/config_toml.rs::ConfigToml` (Step A in §6). Add resolved `microcompaction: MicrocompactConfig` field to `core/src/config/mod.rs::Config` and wire it at `mod.rs:3340` (Steps B/C in §6). Run `just write-config-schema` (reads `ConfigToml` from `codex-config`) and commit the updated `core/config.schema.json`.
   - **Checkpoint:** `cargo build -p codex-core`; schema diff shows only the new `microcompaction` block.

5. **Wire the seam.** Add `run_time_based_microcompact` and the one call in `run_pre_sampling_compact` (§4.3).
   - **Checkpoint:** `just fmt`; `just test -p codex-core`; integration test (§8, test 9) proves that with `enabled=true` and a forced idle gap, old tool-result bodies are placeholdered before the request and the auto-compact estimate drops.

6. **Bazel + final gates.** `just bazel-lock-update && just bazel-lock-check`; commit lockfile. `just fix -p codex-microcompaction`, `just fix -p codex-core`.
   - **Checkpoint:** ask the user before running full `just test` (core touched).

### Phase 2 — cached-API microcompaction (future)
Server-side `cache_edits` to delete tool-result entries from the warm cache without resending. Requires Responses-API cache-edit support + per-turn cache-deletion accounting. Out of v1 (§12).

### Phase 3 — reactive 413 recovery (future)
On a context-overflow error from the model, run clearing then retry. Hooks into the client retry loop in `client.rs`. Out of v1 (§12).

---

## 8. Test strategy

All crate tests in `microcompaction/src/tests.rs` (run `just test -p codex-microcompaction`); use `pretty_assertions::assert_eq` on whole `Vec<ResponseItem>` (AGENTS.md). No insta snapshots — pure transform, direct equality is sufficient. No UI surface in v1.

Unit (crate):
1. **Clears old, keeps recent.** 7 compactable FunctionCall/Output pairs, `keep_recent=5` → 2 oldest outputs become placeholder; the 5 newest unchanged; ordering + call_ids preserved.
2. **Floor at 1.** `keep_recent=0` → effective 1; only the newest survives.
3. **Empty clear set.** `keep_recent >= count` → `apply` returns `no_op()` (`messages: None`).
4. **Idempotency.** Run twice; second run returns `no_op()` and `tokens_freed == 0` (already-cleared bodies skipped).
5. **Tool filtering.** A `FunctionCallOutput` for a tool *not* in `CompactableTools` (e.g. an MCP tool) is never cleared even if oldest.
6. **ContentItems body.** A `FunctionCallOutputBody::ContentItems` (text + image) is replaced wholesale with `Text(placeholder)`; `estimate_body_tokens` counts text bytes/4 + 2000 per image.
7. **CustomToolCallOutput** path mirrors FunctionCallOutput (preserves `name`).
8. **`gap_exceeds_threshold`** truth table: disabled, NaN/inf gap, below threshold, at/above threshold.

Integration (core, `just test -p codex-core`, prefer `core_test_support::responses` mock SSE):
9. **End-to-end seam.** Build a session, record several compactable tool calls + outputs and one assistant message, force `last_assistant_response_at` to >gap ago (test hook), set `enabled=true`, start a turn. Assert: (a) the outbound `/responses` body has old tool outputs replaced with the placeholder, recent N intact; (b) `auto_compact_token_status.active_context_tokens` measured after the seam is **lower** than before (this specifically validates the `clear_token_info()` fix — without it, the server-anchored count would be unchanged, see §10 D6); (c) `history_version` incremented (confirming `replace_history` was called). Honor the sandbox early-exit rule (`CODEX_SANDBOX_NETWORK_DISABLED`).
10. **Disabled is a no-op.** Same setup, `enabled=false` → history unchanged, no `replace_history` call (assert `history_version` unchanged).
11. **Below threshold is a no-op.** `enabled=true` but recent assistant timestamp → unchanged.
12. **Sub-agent excluded.** A sub-agent turn does not clear even past threshold (`is_main_thread_turn` gate).

After Rust changes: `just fmt`, then the per-crate `just test`/`just fix`; full `just test` only with user approval (core touched).

---

## 9. Upstream-merge safety analysis

**Footprint.**
- New crate `codex-microcompaction/` — zero merge surface (upstream has no such path).
- Edits to upstream files: `Cargo.toml` (members + 1 dep line), `core/Cargo.toml` (1 dep line), `config/src/config_toml.rs` (1 field + 1 new struct — the TOML-parse side; additive-only), `core/src/session/turn.rs` (1 call line in `run_pre_sampling_compact` + 1 new helper fn), `core/src/session/mod.rs` (3 small accessors + 1 new `clear_token_info` accessor), `core/src/state/session.rs` (1 field + init + 2 helpers), `core/src/config/mod.rs` (1 resolved field + wiring at `:3340`), `core/config.schema.json` (generated).

**Conflict risk.**
- `turn.rs` `run_pre_sampling_compact` is moderately hot. Mitigation: keep the inserted line a single self-contained call (`run_time_based_microcompact(...).await?;`) so a conflicting upstream edit to the surrounding fn is a trivial re-place; put all real logic in the new helper fn appended at the end of the file (low-churn region) and in the crate.
- `SessionState` / `ConfigToml` are additive single-field changes — low conflict risk; additive struct fields rarely textually collide.
- `Cargo.toml` members/deps lists occasionally churn upstream; a one-line addition is a cheap resolve.

**Isolation strategy.** All algorithmic logic, all constants, and all tests live in the leaf crate. The core diff is purely *wiring* (call the crate, store a timestamp, read config). If upstream ever ships its own microcompaction, our crate can be deleted and the seam re-pointed; nothing in `core` encodes the algorithm. The crate depends only on `codex-protocol` + `codex-utils-output-truncation`, both stable lower-layer crates.

---

## 10. Open design decisions & tradeoffs

**D1 — Where does the idle clock live?**
Options: (a) new `last_assistant_response_at: Option<Instant>` on `SessionState`; (b) derive from rollout (`created_at` exists on rollout items, `mod.rs:3272`); (c) wall-clock `SystemTime` for resume-survivability.
Recommendation: **(a) `Instant` on `SessionState`.** `ResponseItem` has no timestamp (verified `models.rs:760-856`), and rollout timestamps are not cheaply queryable from the turn path. `Instant` is monotonic and right for "idle since last response within this process." On resume (`None`), the gate simply doesn't fire until the first response of the resumed session — acceptable for v1. If resume-aware clearing is wanted later, switch to `SystemTime` persisted in rollout (defer).

**D2 — `COMPACTABLE_TOOLS` → Codex tool names.**
Options: (a) hard-coded mapped set in `CompactableTools::default()`; (b) derive from the tool registry; (c) clear *all* tool outputs regardless of name.
Recommendation: **(a), verified against the actual `FunctionCall.name` strings emitted in production.** Confirmed names: `"exec_command"` (`shell_spec.rs:84`), `"shell_command"` (`shell_spec.rs:206`), `"apply_patch"` (`apply_patch.rs:303`). Names that remain unconfirmed and are excluded from the initial default: `"grep"`, `"glob"` (not found as plain Codex built-in tool names in `tools/handlers/`), `"web_search"` / `"web_fetch"` (hosted/server-side — verify the actual `FunctionCall.name` string emitted before adding). **`"unified_exec"` is NOT a valid `FunctionCall.name`** — it is an internal routing label (`parallel.rs:228`); the emitted name is `"exec_command"`. **`"read_file"` is NOT a Codex built-in tool** — Codex has no standalone file-read tool (only an MCP-namespaced `filesystem::read_file` at `mcp.rs:454`); omit it. Do not clear MCP/custom tools (parity with CC: only known bulky built-ins). (c) is rejected — clearing arbitrary tool outputs risks dropping results the model still needs and diverges from CC.

**D3 — Main-thread / source gate.**
Options: (a) gate using `!session_source.is_non_root_agent()`; (b) skip the gate entirely; (c) enumerate only the explicit root sources.
Recommendation: **(a) `!session_source.is_non_root_agent()` from `protocol.rs:2591`.** `is_non_root_agent()` matches `SessionSource::Internal(_) | SessionSource::SubAgent(_)` (`protocol.rs:2591-2595`). This excludes BOTH sub-agents AND internal sessions (compact tasks, review tasks) — which is the correct v1 behavior: internal sessions are short-lived and already performing their own compaction-related work; time-based clearing should not interfere. The `is_main_thread_turn()` accessor on `Session` is implemented as `!state.session_configuration.session_source.is_non_root_agent()` (accessing `SessionState.session_configuration.session_source`, a `pub(crate)` field at `session/mod.rs:403`). CC restricts time-based MC to `repl_main_thread` for the same reason (`timeBasedMCConfig.ts:14-16`, `microCompact.ts:431`). For v1, `!is_non_root_agent()` is the safest conservative choice.

**D4 — Reset the clock after firing?**
Options: (a) `mark_assistant_response_now()` after a successful clear; (b) leave the clock, re-clear every turn; (c) separate "last cleared at" timestamp.
Recommendation: **(a).** After clearing, the cache is cold and we've just rewritten the prefix; resetting the idle clock means we won't re-run the (now no-op, but still O(n)) scan every subsequent turn within the window. Idempotency makes (b) safe but wasteful; (c) is more state for no v1 benefit.

**D5 — Exhaustive match vs catch-all in the rewrite.**
Options: (a) catch-all `other => other.clone()`; (b) fully enumerate every `ResponseItem` variant like `process_item` (`history.rs:398-411`).
Recommendation: **(b) enumerate** to match the codebase convention and AGENTS' match-exhaustiveness guidance, so a new upstream `ResponseItem` variant forces a compile error and a conscious decision. Slightly more verbose; worth it for the safety net.

**D6 — Does `tokens_freed` feed auto-compact, and does `replace_history` make the shrink visible?**
Options: (a) after `replace_history`, also call `sess.clear_token_info()`; (b) pass an explicit `tokens_freed` into `auto_compact_token_status`; (c) switch `auto_compact_token_status` to use `estimate_token_count` instead of `get_total_token_usage`.
Recommendation: **(a).** `replace_history` calls `ContextManager::replace` (`history.rs:187-190`), which only reassigns `self.items` and increments `history_version` — it does **NOT** touch `self.token_info`. `auto_compact_token_status` (`turn.rs:663`) calls `sess.get_total_token_usage()`, which is `last_server_reported_tokens + items_after_last_model_generated` (`history.rs:314-331`). The old cleared outputs precede the last assistant message, so they are under the server-anchored `last_server_reported_tokens` component — they are NOT in the suffix that gets re-estimated. **The shrink is therefore NOT automatically visible** without clearing the anchor. Fix: after `replace_history`, call `sess.clear_token_info().await` (new accessor, §5 item 7). This sets `token_info = None`, causing `get_total_token_usage` to fall back to a pure item-walk of the mutated history, making the freed tokens visible. Integration test 9 must assert the post-clear `active_context_tokens` actually drops. (CC's note that microcompact savings are *not* passed to autocompact still holds — the freed tokens become visible through the pure item-estimation fallback, not by explicit subtraction.)

---

## 11. Risks & mitigations

- **Token estimate vs server reality.** `get_total_token_usage` (`history.rs:314`) = `last_server_reported_tokens + items_after_last_model_generated`. `replace_history` does NOT touch `token_info`, so the server-anchored component is unchanged after a clear. **The spec addresses this explicitly**: after `replace_history`, `sess.clear_token_info()` is called (§4.3, §5 item 7, §10 D6), which nulls the anchor and forces `get_total_token_usage` to fall back to pure item-estimation across all history items — making the freed tokens visible to `auto_compact_token_status`. Integration test 9 MUST assert that `active_context_tokens` drops after the seam runs; this is the correctness gate.
- **Clearing context the model still needs.** Placeholdering an old `read_file` the model later re-references could degrade output. Mitigation: `keep_recent` floor + default 5, default `enabled=false` (ships dark), gate on a long idle gap (cache genuinely cold), and only built-in bulky tools.
- **Normalization invariants.** `replace_history` → `ContextManager::replace` does not re-run pairing normalization; we must preserve every `call_id` and item position exactly (we only swap `output.body`). Mitigation: rewrite reconstructs the variant with identical `call_id`/ordering; test 1 asserts structural equality of all untouched items and IDs.
- **Wrong tool-name mapping (D2).** If the mapped names don't match real `FunctionCall.name` values, the pass silently clears nothing. Mitigation: reconcile against the registry; add a test that uses real Codex tool names; log `cleared`/`tokens_freed` so a misconfiguration is observable.
- **Idle clock never set / sub-agent leakage.** If `mark_assistant_response_now` isn't called at the right completion point, the gate never fires (fail-safe: no clearing) or fires for sub-agents. Mitigation: D3 gate + a focused test for the completion-point set and sub-agent exclusion (tests 9, 12).
- **`Instant` non-persistence across resume.** Acceptable for v1 (gate simply waits for the first post-resume response). Documented in D1; revisit with `SystemTime` if needed.

---

## 12. Out of scope / future work

- **Cached-API microcompaction (Phase 2).** Server-side `cache_edits` deletion of tool-result entries from a *warm* cache, with per-turn `cache_deleted_input_tokens` accounting (CC `cachedMicrocompactPath`, `microCompact.ts:305-399`). Requires Responses-API cache-edit support; not in v1.
- **Reactive 413 recovery (Phase 3).** Clear-then-retry on context-overflow errors in the client retry loop (`client.rs`).
- **Compact-warning suppression UI.** CC suppresses the "context low, run /compact" banner after MC (`compactWarningState.ts`, `microCompact.ts:511`). Codex's equivalent UI signal can be added when/if the TUI grows that banner; v1 emits no UI.
- **Analytics event.** CC logs `tengu_time_based_microcompact`. v1 may log `cleared`/`tokens_freed` via existing tracing; a dedicated analytics event is deferred.
- **GrowthBook-style remote config.** CC drives all three knobs from one flag (`tengu_slate_heron`). v1 uses static `ConfigToml`; remote/dynamic config is out of scope.
- **Resume-aware (persisted) idle clock.** See D1 — `SystemTime` in rollout, deferred.
- **Per-message tool-result byte budget** (CC `applyToolResultBudget`, runs before MC) is a separate feature; Codex already truncates outputs at record time (`history.rs:377`), so no port needed.
