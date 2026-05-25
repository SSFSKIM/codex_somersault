// Runtime shim for `bun:bundle`.
//
// The leaked source uses `import { feature } from 'bun:bundle'` with ~857 call
// sites referencing ~90 unique flag names. `bun:bundle` was a custom virtual
// module in Anthropic's `bun build` config (see docs/specs/00-overview.md §13.3
// — the bundler config is absent from the leak). At build time the bundler
// substituted `feature('X')` for a literal boolean, allowing dead-code
// elimination of disabled branches and their `require(...)`s.
//
// In Somersault we have no such bundler step. This shim returns a boolean at
// RUNTIME instead. Functional equivalence: when `feature('X')` returns false,
// the ternary `feature('X') ? require(...) : null` never evaluates the
// require, so a gated subsystem whose source dir is missing or stripped is
// still safely disabled. The cost of runtime evaluation vs build-time DCE is
// negligible — the boolean lookup happens once at module load and is
// trivially inlinable by V8/JSC.
//
// tsconfig.json path-aliases `bun:bundle` -> this file for typecheck. For
// runtime, Bun honors tsconfig paths for non-`bun:`-prefixed specifiers; the
// `bun:` prefix is reserved by Bun for built-ins. If `bun run` does not
// resolve `bun:bundle` via the tsconfig alias, the next step is to register
// a `Bun.plugin` in a preload entry that intercepts the specifier. Verify at
// first `bun run` (Phase 2).

const FLAGS: Record<string, boolean> = {
  // Cost / context features. Only TOKEN_BUDGET has its implementation
  // present in the leak (src/query/tokenBudget.ts). The other three —
  // cachedMicrocompact, contextCollapse, reactiveCompact, snipCompact —
  // are missing from the leak (Anthropic's build pipeline generated or
  // included them separately). Their flags MUST stay false until we
  // either author the implementations or remove the call sites.
  // src/services/compact/cachedMicrocompact.ts etc. exist as type-only
  // stubs for typecheck; do NOT flip these flags true until real impls
  // land — `await import(...)` at runtime would fail.
  CACHED_MICROCOMPACT: false,
  CONTEXT_COLLAPSE: false,
  REACTIVE_COMPACT: false,
  HISTORY_SNIP: false, // gates snipCompact.ts
  TOKEN_BUDGET: true, // impl present in src/query/tokenBudget.ts

  // Everything else defaults to false. The shape below is for documentation
  // and easy flip-toggling; entries can be omitted (default `false` via the
  // `?? false` fallback in `feature()` below). Grouped by origin.

  // Anthropic-internal modes / surfaces — keep off.
  KAIROS: false,
  KAIROS_BRIEF: false,
  KAIROS_CHANNELS: false,
  KAIROS_DREAM: false,
  KAIROS_GITHUB_WEBHOOKS: false,
  KAIROS_PUSH_NOTIFICATION: false,
  BRIDGE_MODE: false,
  PROACTIVE: false,
  DAEMON: false,
  VOICE_MODE: false,
  TEAMMEM: false,
  COORDINATOR_MODE: false,
  COMMIT_ATTRIBUTION: false,
  COWORKER_TYPE_TELEMETRY: false,
  MEMORY_SHAPE_TELEMETRY: false,
  ENHANCED_TELEMETRY_BETA: false,
  PERFETTO_TRACING: false,
  SLOW_OPERATION_LOGGING: false,
  COMPACTION_REMINDERS: false,
  HARD_FAIL: false,
  ALLOW_TEST_VERSIONS: false,
  BUILDING_CLAUDE_APPS: false,
  ABLATION_BASELINE: false,
  AWAY_SUMMARY: false,
  DUMP_SYSTEM_PROMPT: false,
  HOOK_PROMPTS: false,
  AUTO_THEME: false,
  LODESTONE: false,
  BUDDY: false,
  ULTRAPLAN: false,
  ULTRATHINK: false,
  VERIFICATION_AGENT: false,
  TORCH: false,
  SHOT_STATS: false,

  // CCR / remote / bridge — keep off (no claude.ai/code backend).
  CCR_AUTO_CONNECT: false,
  CCR_MIRROR: false,
  CCR_REMOTE_SETUP: false,
  SSH_REMOTE: false,
  DIRECT_CONNECT: false,

  // Agent triggers / scheduled / monitor — keep off.
  AGENT_TRIGGERS: false,
  AGENT_TRIGGERS_REMOTE: false,
  MONITOR_TOOL: false,

  // Tools gated by feature flags — keep off unless we want them.
  WEB_BROWSER_TOOL: false,
  WORKFLOW_SCRIPTS: false,
  OVERFLOW_TEST_TOOL: false,
  TERMINAL_PANEL: false,
  UDS_INBOX: false,
  REVIEW_ARTIFACT: false,

  // Experimental / lab features — keep off; revisit after Phase 3.
  EXPERIMENTAL_SKILL_SEARCH: false,
  SKILL_IMPROVEMENT: false,
  RUN_SKILL_GENERATOR: false,
  AGENT_MEMORY_SNAPSHOT: false,
  EXTRACT_MEMORIES: false,
  FORK_SUBAGENT: false,
  BUILTIN_EXPLORE_PLAN_AGENTS: false, // CANDIDATE: enable later for built-in explore/plan agents
  CHICAGO_MCP: false,
  MCP_RICH_OUTPUT: false, // CANDIDATE: useful MCP UX
  MCP_SKILLS: false,
  BG_SESSIONS: false,
  TEMPLATES: false,
  CONNECTOR_TEXT: false,
  TRANSCRIPT_CLASSIFIER: false,
  PROMPT_CACHE_BREAK_DETECTION: false, // CANDIDATE: useful cost diagnostic
  ANTI_DISTILLATION_CC: false,
  BASH_CLASSIFIER: false,
  NATIVE_CLIENT_ATTESTATION: false,
  UNATTENDED_RETRY: false,
  POWERSHELL_AUTO_MODE: false,
  HISTORY_PICKER: false,
  MESSAGE_ACTIONS: false,
  STREAMLINED_OUTPUT: false,
  NATIVE_CLIPBOARD_IMAGE: false,
  NEW_INIT: false,
  QUICK_SEARCH: false,
  TREE_SITTER_BASH: false,
  TREE_SITTER_BASH_SHADOW: false,
  FILE_PERSISTENCE: false,
  BREAK_CACHE_COMMAND: false,
  DOWNLOAD_USER_SETTINGS: false,
  UPLOAD_USER_SETTINGS: false,
  BYOC_ENVIRONMENT_RUNNER: false,
  SELF_HOSTED_RUNNER: false,
  SKIP_DETECTION_WHEN_AUTOUPDATES_DISABLED: false,

  // Runtime libc detection — runtime-determined in upstream; the shim returns
  // false. Affects bundled-binary selection paths that are not relevant when
  // running source-mode via `bun run`.
  IS_LIBC_GLIBC: false,
  IS_LIBC_MUSL: false,
}

/**
 * Returns whether a build-time feature flag is enabled.
 *
 * In the leaked source this was a compile-time constant substituted by the
 * Bun bundler. Here it is a runtime lookup. Unknown flag names return false
 * — accepts future merges from upstream gracefully.
 */
export function feature(name: string): boolean {
  return FLAGS[name] ?? false
}
