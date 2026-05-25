# CLAUDE.md

Guidance for working in this codebase. Keep this file lean — pointers and gotchas, not exhaustive docs.

## What this is

**somersault** — a private TypeScript agent harness (terminal coding agent), forked from leaked Claude Code source and extended to be **multi-provider**. The `package.json` name is `somersault`; `Claude Code Src/` is its project root (this directory).

- **Runtime:** Bun (`>=1.1.0`). No bundler/build step — runs source directly.
- **Language:** TypeScript, `strict` (but `noImplicitAny: false`). Module mode `NodeNext`.
- **UI:** React 19 + [Ink](https://github.com/vadimdemedes/ink) (React for the terminal).
- **Other:** Commander.js (CLI), Zod v4, ripgrep (search), MCP + LSP, OpenTelemetry/gRPC (lazy), GrowthBook (flags). Anthropic SDK **and** OpenAI SDK.

## Commands

```bash
bun run dev          # = bun run src/main.tsx  — launches the agent
bun run typecheck    # = tsc --noEmit
bun test             # Bun's test runner (test coverage is currently minimal — see openai.test.ts)
./scripts/setup-ant-stubs.sh   # recreate @ant/* stubs if import resolution fails after `bun install`
```

There is no lint/format script wired into `package.json`; the repo-root tooling (prettier/markdownlint) belongs to the surrounding monorepo, not to this project.

## Critical gotchas

- **`src/main.tsx` (~803 KB, one file) is bundled/minified leaked output.** Treat it as read-mostly. Don't refactor it; find the unbundled equivalent in `entrypoints/`, `bootstrap/`, `cli/`, `setup.ts` and edit there. The same applies to other very large files (`cli/print.ts`, `interactiveHelpers.tsx`, `query.ts`, `QueryEngine.ts`).

- **Feature flags resolve at RUNTIME via `src/bundle-shim.ts`, not at build time.** `import { feature } from 'bun:bundle'` is path-aliased (in `tsconfig.json`) to the shim, which is a plain `Record<string, boolean>` lookup. To enable a gated subsystem, flip its flag in `bundle-shim.ts`.
  - **Do NOT flip these true** — their implementations are missing/type-only stubs and `await import(...)` will crash: `CACHED_MICROCOMPACT`, `CONTEXT_COLLAPSE`, `REACTIVE_COMPACT`, `HISTORY_SNIP`.
  - `TOKEN_BUDGET` is the one cost/context flag with a real impl (`src/query/tokenBudget.ts`) and defaults `true`.
  - Unknown flag names return `false`, so upstream merges degrade gracefully.

- **Imports use `.js` extensions for `.ts`/`.tsx` files** (NodeNext requirement): `import x from './foo.js'` resolves `foo.ts`. Both `src/*`-aliased and relative import styles are used; both are valid (`tsconfig` `paths`).

- **Zod is imported from `'zod/v4'`**, not `'zod'`. Match the existing import style.

- **`@ant/*` packages are Anthropic-internal stubs** (`claude-for-chrome-mcp`, `computer-use-mcp`, …). They're imported at module top-level under `src/utils/claudeInChrome/` and `src/utils/computerUse/` but are runtime-dead (gated by `CHICAGO_MCP = false`). The stubs only exist so module resolution doesn't fail. Recreate with `scripts/setup-ant-stubs.sh`.

- **Top-level side effects in `main.tsx` run before heavy imports on purpose** (keychain prefetch, MDM read) for startup parallelism. A custom eslint rule `no-top-level-side-effects` guards this elsewhere — don't add new top-level side effects casually.

## Architecture map

Source under `src/` is organized in conceptual layers (entrypoint → query loop → tools → commands → services → UI). Largest / highest-traffic areas:

| Area | Where | Notes |
|---|---|---|
| Boot & entry | `src/main.tsx`, `entrypoints/`, `bootstrap/state.ts`, `cli/`, `setup.ts` | Bun entry is `src/main.tsx` |
| Settings / schemas | `src/schemas/`, `src/migrations/`, `src/constants/` | Zod schemas, settings migrations |
| Query loop | `src/QueryEngine.ts` (LLM API loop/streaming/retry), `src/query.ts` + `src/query/` (turn pipeline) | |
| Context & cost | `src/context.ts`, `src/context/`, `src/cost-tracker.ts`, `src/services/compact/` | |
| Tool system | `src/Tool.ts` (interface + `ToolUseContext` god object), `src/tools.ts` (registry), `src/tools/` (~56 tools) | Tool *ordering* is a server-side prompt-cache invariant — see comments in `tools.ts` |
| Permissions | `src/hooks/toolPermission/`, `src/types/permissions.ts`, `src/utils/permissions/` | allow/deny/ask decision tree |
| Commands (`/slash`) | `src/commands.ts` (registry), `src/commands/` (~100 commands) | |
| Services | `src/services/` — `api/` (providers), `mcp/`, `lsp/`, `oauth/`, `analytics/`, `plugins/`, memory services | |
| Multi-agent | `src/coordinator/`, `src/tools/shared/spawnMultiAgent.ts` | |
| UI / Ink | `src/ink/` (primitives), `src/components/` (~140), `src/screens/`, `src/hooks/` (React hooks) | |
| Persistence | `src/state/`, `src/history.ts`, `src/memdir/` (file-based memory) | |

**Providers (multi-provider, fork-specific):** `getAPIProvider()` in `src/utils/model/providers.ts` covers only the Anthropic transport family — `firstParty | bedrock | vertex | foundry` (env-selected; see the header comment in `src/services/api/client.ts` for the env-var matrix). **OpenAI support is a separate path** in `src/services/api/openai.ts` (+ `openaiCodexAuth.ts`), not part of that union — look there, not in `client.ts`, for OpenAI routing.

> **Term overloads** (easy to conflate): "hook" = user-config event hook **vs** Ink React hook **vs** internal runtime hook; "context" = `ToolUseContext` **vs** permission context **vs** system/user context; "session" = transcript file **vs** REPL run **vs** remote connection. Disambiguate before assuming.

## `docs/specs/` — research only, not runtime

`docs/specs/` (~147 markdown files) is a **reverse-engineering research / source-spec project** that maps the leaked Claude Code source subsystem by subsystem (overview, per-tool specs, adversarial-audit phases, etc.). It is documentation about the code, not part of the build or runtime.

Use it as a **reference** to understand subsystems quickly (`docs/specs/00-overview.md` is the architectural anchor; `INDEX.md` lists all specs), but:
- Don't treat its claims as ground truth — verify against actual `src/` before relying on a detail; the source has moved since some specs were written.
- Don't expand or maintain the specs as part of normal feature work, and keep this CLAUDE.md focused on the live codebase rather than the spec project.
