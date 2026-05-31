# 00 — Codex Port Proposals: Index & Build Order

These are **fork-local design proposals**. They port Claude Code capabilities into Codex and will **not** arrive via upstream merge — so each is engineered for minimal core footprint and clean re-application across upstream merges. This index sequences them and consolidates the cross-cutting decisions.

## 1. Purpose (one line each)

- **[01 — Microcompaction](./01-microcompaction.md)** — A pre-turn, no-model-call pass that clears old tool-result bodies to a placeholder once the conversation has been idle long enough that the prompt cache is cold, shrinking the prefix before the auto-compact check.
- **[02 — LLM Hook Handlers](./02-llm-hook-handlers.md)** — Implements the `prompt` (v1) and `agent` (Phase 3) LLM-backed hook handler types so hook authors can gate tool calls with an LLM judgment instead of a shell script.
- **[03 — LSP Crate](./03-lsp-crate.md)** — A new optional `codex-lsp` crate that spawns config-driven language servers and exposes diagnostics / definition / hover / references to the model via a single namespaced tool, with automatic document sync from apply-patch.

## 2. Recommended build order

**Build order: 01 Microcompaction → 03 LSP → 02 LLM Hooks.** Rationale, tied to the fork "minimal-footprint, merge-safe" priority:

### First — 01 Microcompaction (most isolated, lowest risk)
The algorithm is a pure, synchronous `&[ResponseItem] -> Vec<ResponseItem>` transform living entirely in a new leaf crate (`codex-microcompaction`) that depends only on `codex-protocol` + `codex-utils-output-truncation`. The core diff is pure *wiring*: one call line in `run_pre_sampling_compact`, a few small `Session` accessors, one `SessionState` field, and additive config. No async surface, no shared subsystem, no dependency on the other two proposals. It ships dark (`enabled=false`), so it can land without behavioral risk. It is the natural warm-up: smallest blast radius, fastest to a green `just test`, and it exercises the `replace_history`/token-accounting seam that builds intuition for the larger work. **Caveat before merge:** resolve the two flagged blockers — `MicrocompactionToml` must be added to `codex-config/src/config_toml.rs` (an upstream-file edit the spec's file list under-states), and the `clear_token_info()` token-accounting fix must be in place or the feature is a silent no-op.

### Second — 03 LSP (self-contained new crate, but more surface area)
LSP is also a brand-new crate (`codex-rs/ext/lsp-client/`) and rides the established `ext/` extension machinery (`ToolContributor`, thread-scoped `ExtensionData`), so it needs **zero** core tool-registration edits. That isolation is why it ranks above the hooks work. It is sequenced *after* microcompaction because it is materially larger (transport framer, server lifecycle state machine, diagnostics registry, 9-op tool) and carries one genuinely hot-file edit (`apply_patch.rs`) plus a **must-resolve-first** circular-dependency blocker: the doc-sync hook requires a new `ApplyPatchDocSync` trait in `codex-extension-api` (adding a fifth upstream touch-point) — without it the spec will not compile. It does not depend on 01 or 02.

### Third — 02 LLM Hook Handlers (highest core entanglement; may reuse multi_agents_v2)
Ranked last because it has the deepest entanglement with existing core/hooks internals despite its new crate (`hook-llm-runner`). The risky edit is the `discovery.rs` refactor — extracting the trust/enabled/hash block out of the command-only arm, which existing tests assert — plus a wide-surface `ConfiguredHandler` field addition that touches every constructor site. The concrete invoker must be wired into `session/mod.rs::build_hooks_for_config` over the live `ModelClient`. Critically, its **Phase 3 (`agent` handler) reuses `multi_agents_v2` spawn machinery** for the verifier sub-agent — so doing it after the other two means the core sub-agent/turn seams are already familiar, and the v1 (`prompt`-only) slice can land independently while Phase 3 follows. No hard dependency on 01 or 03, but the highest cognitive load and the most merge-fragile core edits argue for doing it when the simpler seams are behind you.

## 3. Combined upstream-merge-safety table

| Feature | New crate? | Hot upstream files touched | Conflict risk |
|---|---|---|---|
| 01 Microcompaction | `codex-microcompaction` (leaf) | `core/src/session/turn.rs` (1 call in `run_pre_sampling_compact` + 1 helper fn); `session/mod.rs` (small accessors); `state/session.rs` (1 field); `config/src/config_toml.rs` + `core/src/config/mod.rs` (additive config) | **Low.** All algorithm/logic in the leaf crate; core edits are wiring. `turn.rs` is moderately hot but the insertion is a single trivially re-placeable line. Watch: the `config_toml.rs` edit is under-stated in the spec's file list. |
| 02 LLM Hook Handlers | `codex-hook-llm-runner` (trait + helpers, no model deps) + new core module `hook_llm_invoker.rs` | `hooks/src/engine/discovery.rs` (refactor trust/enabled/hash out of Command arm — riskiest); `hooks/src/engine/mod.rs` (`ConfiguredHandler` gains a field → every constructor site); `dispatcher.rs` (2 one-liners); `pre_tool_use.rs`/`permission_request.rs` (additive); `session/mod.rs::build_hooks_for_config` (~10 lines in a 3316-line file) | **Medium.** `discovery.rs` refactor will re-apply as a conflict if upstream changes trust logic (mitigate: pure helper). `ConfiguredHandler` field is a wide compile-time migration. Protocol/app-server-protocol need **no** changes. |
| 03 LSP Crate | `codex-lsp` (`ext/lsp-client/`) + new `config/src/lsp_config.rs` | `core/src/tools/runtimes/apply_patch.rs` (rename `_ctx`→`ctx` + 1 delegating call); `config_toml.rs` (1 tail field); `config/src/lib.rs` (1 mod+use); `app-server/src/extensions.rs` (1 install line); `codex-extension-api` (new `ApplyPatchDocSync` trait — 5th touch-point) | **Low** (conditional). All edits additive/tail-positioned or in fork-local glue. Only genuinely hot file is `apply_patch.rs` (2-line diff, logic in fork-local `lsp_sync.rs`). **Blocker:** the circular-dep fix (`ApplyPatchDocSync` trait) must land or it won't compile. |

## 4. Open decisions needing user input

Grouped/deduped across the three specs. Most have a strong recommendation in-spec; these are the ones worth an explicit confirmation before implementation.

### A. Cross-cutting / fork-policy
- **Default-off vs default-on shipping.** Microcompaction ships `enabled=false` (parity with CC). Confirm the same dark-launch posture applies to LSP (gated on a non-empty `[lsp_servers]` table) and to LLM hooks (only fire when configured). _(01 §6, 03 §6, 02 implicit)_
- **Default model for LLM hooks.** No `small_fast_model` concept exists in the Rust registry. Spec recommends falling back to the **session's own `model_info`** when `hook.model` is unset (vs. CC's small/fast tier). This affects hook cost/latency in the PreToolUse critical path — confirm acceptable, or decide whether to add a `hooks.default_model` config key now. _(02 Decision D)_

### B. State placement & lifecycle
- **Microcompaction idle clock representation.** `Instant` on `SessionState` (recommended; not resume-persistent) vs. `SystemTime` in rollout (resume-aware, deferred). Confirm v1 may skip resume-survivability. _(01 Decision D1)_
- **Where the LSP manager lives.** Thread-scoped `ExtensionData` (recommended, zero core struct edits) vs. a dedicated `Services.lsp_manager` field vs. process-global. _(03 Decision B)_

### C. Token accounting & correctness gates (must-confirm)
- **Microcompaction token visibility.** The freed tokens are **not** visible to auto-compact unless `clear_token_info()` is called after `replace_history` (the spec's key correctness fix). Confirm this is in scope for v1 and gated by integration test 9. _(01 Decision D6, Risks)_
- **Compactable tool-name map.** Hard-coded set verified against real `FunctionCall.name` values (`exec_command`, `shell_command`, `apply_patch`); `grep`/`glob`/`web_search`/`web_fetch`/`read_file` are unconfirmed/absent and excluded. Confirm the initial set, since a wrong map silently clears nothing. _(01 Decision D2)_

### D. Subsystem-specific behavior placement
- **LSP diagnostics injection point.** `ContextContributor` at prompt assembly (recommended; matches memories ext) vs. a between-turns turn-lifecycle event vs. attaching to tool output. Flagged as the one behavior whose exact placement should be confirmed against where Codex drains between-turn state. _(03 Decision E)_
- **LLM-hook agent-spawn mechanism (Phase 3).** Route through `multi_agents_v2` `spawn_agent_with_metadata` (recommended — needs full tools/turn-loop/transcript) vs. a bespoke minimal turn loop. Confirm the heavier lifecycle is acceptable for the `agent` variant. _(02 Decision C)_
- **LLM-hook concurrency.** Prompt hooks fan out concurrently; agent hooks serialized (recommended) to avoid quota/context thrash. _(02 Decision E)_

### E. Lower-risk decisions (recommendation likely sufficient; flag only if you disagree)
- Microcompaction: exhaustive `match` vs catch-all in the rewrite (recommend enumerate); reset idle clock after firing (recommend yes). _(01 D5, D4)_
- LLM hooks: object-safe `DynLlmHookInvoker` boxed-future trait vs RPITIT layer (recommend boxed-future); model call located in core over `ModelClient` vs re-derived `ResponsesClient` (recommend core/`ModelClient`). _(02 Decisions A, B)_
- LSP: hand-rolled Content-Length framer + `lsp-types` vs `async-lsp`/`lsp-server` (recommend hand-rolled); single namespaced `lsp/query` tool vs one tool per op (recommend single). _(03 Decisions A, C)_

## 5. The three specs

- [01-microcompaction.md](./01-microcompaction.md)
- [02-llm-hook-handlers.md](./02-llm-hook-handlers.md)
- [03-lsp-crate.md](./03-lsp-crate.md)
