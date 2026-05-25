# 21 — Command Catalog (Index)

> Per-command reverse-engineered specification of every entry in `src/commands/`. Read 20-command-system.md first for the registry, type union, dispatch model, gate semantics (`INTERNAL_ONLY_COMMANDS` mechanics, `availability` carve-out, `feature(...)` DCE) — this catalog enumerates **what** each `/foo` command does, not how the registry routes it. The owner unit of bit-exact reproduction is the **sub-file**; this page is just a router and a class-tally.

## Source-coverage inventory

Confirmed by `ls /Users/new/Downloads/claude-code-main/src/commands/` and the static import block in `commands.ts:1-210`. Class table below is a **count-only** summary; the per-command Gate/source-present table lives in 21c §1 (feature-gated commands' per-flag matrix) and 21a/21b/21d §2 (Source Map per sub-file). Defer to spec 20 for the runtime gate evaluator semantics.

| Class | Gate | Count | Examples |
|---|---|---:|---|
| Public built-in (always-in `COMMANDS()`) | none (registration unconditional; `availability` may self-hide) | 62 | `/clear`, `/help`, `/init`, `/model`, `/plugin`, `/agents` |
| 3P-aware (registration suppressed for Bedrock/Vertex/Foundry) | `!isUsing3PServices()` at `commands.ts:337` | 2 | `/login`, `/logout` (note: `login` is invoked as `login()` — function call yielding a Command) |
| Feature-flag gated (top-level spreads in `COMMANDS()`) | per-flag `feature(...)` at `commands.ts:62-122` (DCE — see 20 §A) | 12 | `/proactive`, `/voice`, `/brief`, `/web-setup`, `/peers`, `/fork`, `/buddy`, `/torch`, `/workflows`, `/remote-control` (BRIDGE_MODE), `/remoteControlServer` (DAEMON ∧ BRIDGE_MODE), `/assistant` (KAIROS) |
| ANT-only (in `INTERNAL_ONLY_COMMANDS`, gated by enclosing spread) | `USER_TYPE === 'ant' && !IS_DEMO` at `commands.ts:343` | 25 | `/commit`, `/commit-push-pr`, `/version`, `/ctx_viz`, `/issue`, `/onboarding`, `/teleport`, `/share`, `/bridge-kick`, `/agents-platform` (also see next row) |
| ANT-only + feature-flag (conditional spreads *inside* `INTERNAL_ONLY_COMMANDS`) | both gates: enclosing ANT spread AND per-flag `feature(...)` | 3 | `/force-snip` (HISTORY_SNIP), `/ultraplan` (ULTRAPLAN), `/subscribe-pr` (KAIROS_GITHUB_WEBHOOKS) |
| ANT top-level conditional require (binding-time gate, IS_DEMO-permissive) | `USER_TYPE === 'ant'` only at `commands.ts:48-50` (no `!IS_DEMO`) — but **also placed in `INTERNAL_ONLY_COMMANDS` at line 252**, so the effective registration gate is the stricter ANT∧!IS_DEMO. The looser binding gate determines whether the symbol is `null` vs the imported module. | (1; counted in the 25 above) | `/agents-platform` |
| Lazy-shim (`getPromptForCommand` defers heavy module) | none — registered unconditionally | 1 | `/insights` (3200 LOC `insights.ts`; shim at `commands.ts:189-202` does `import('./commands/insights.js')` only when invoked. The Command object itself is constructed inline, *not* lazy.) |
| **Total registered with all flags ON** | | **105** | see arithmetic below |

**Total arithmetic (all flags ON, ANT-build, non-3P, non-IS_DEMO):**

`COMMANDS()` body = 62 always-in entries (rows after `addDir`…`vim`, including the 3 dual-entry trio halves: `extraUsage`/`extraUsageNonInteractive`, `context`/`contextNonInteractive`, `resetLimits`/`resetLimitsNonInteractive` — counted individually) + 12 feature-flag spreads (`webCmd`, `forkCmd`, `buddy`, `proactive`, `briefCommand`, `assistantCommand`, `bridge`, `remoteControlServerCommand`, `voiceCommand`, `peersCmd`, `workflowsCmd`, `torch`) + 2 3P-aware (`logout`, `login()`) + 28 entries from `INTERNAL_ONLY_COMMANDS` (25 named + 3 conditional spreads `forceSnip`/`ultraplan`/`subscribePr`, all post-`.filter(Boolean)`) + 1 lazy-shim (`usageReport`) = **62 + 12 + 2 + 28 + 1 = 105**.

Two prior arithmetic claims were wrong: (a) the older "62 public" was right but the older partition row sum (64+27+12 = 103) excluded the lazy-shim and double-bucketed `agents-platform`; (b) "~108" came from counting `agents-platform` twice (once in row 4, once as a separate row). The 21d row (this revision) explicitly absorbs the long-tail public commands into a fourth sub-file; the partition row totals below are therefore **command-file** counts (a.ts file may contribute 0 or 1+ Commands), not registry-entry counts. The two numbers diverge — see the note after the partition table.

`reset-limits` and `extra-usage` ship as TWO Command objects each (interactive + non-interactive); `context`/`contextNonInteractive` are imported from `commands/context/index.ts`. All three pairs are counted as 2 entries above.

Source-read state per file:

| Source | Read fully | Sampled head | Grep-inspected |
|---|---|---|---|
| `commands.ts` | ✅ | | |
| `commands/commit.ts`, `commit-push-pr.ts`, `init.ts`, `init-verifiers.ts`, `review.ts`, `security-review.ts`, `version.ts`, `advisor.ts`, `brief.ts`, `bridge-kick.ts` | ✅ | | |
| `commands/createMovedToPluginCommand.ts`, `cost/cost.ts`, `compact/compact.ts`, `branch/branch.ts`, `clear/clear.ts` | ✅ | | |
| All ~58 `commands/*/index.ts` (or `index.tsx`) public command-metadata files | ✅ (via concat in single batch) | | |
| `statusline.tsx` (single-file public command) | ✅ (compiled head + sourcesContent) | | |
| ANT-only directories (`autofix-pr`, `bughunter`, `issue`, `onboarding`, `share`, `summary`, `teleport`, `ant-trace`, `perf-issue`, `env`, `oauth-refresh`, `debug-tool-call`, `mock-limits`, `ctx_viz`, `good-claude`, `break-cache`, `backfill-sessions`, `reset-limits`) | | | ✅ — **all stubbed in this leak** to `{ isEnabled: () => false, isHidden: true, name: 'stub' }`; documented at registry-citation level only |
| `insights.ts` | | ✅ (head 200 of 3200) | |
| Heavy `*.tsx` impls (login, logout, mcp, model, plugin, ide) | | ✅ (head 40) | |
| Stand-alone files in `INTERNAL_ONLY_COMMANDS` whose **source is missing entirely** (`agents-platform`, `proactive.ts`, `assistant/index.ts`, `remoteControlServer/index.ts`, `subscribe-pr.ts`, `ultraplan.ts`, `torch.ts`, `force-snip.ts`, `peers/index.ts`, `fork/index.ts`, `buddy/index.ts`, `workflows/index.ts`) | | | ✅ — registry-citation only |

## Sub-file partition

The split is by visibility/gating because the citation density and prompt corpus differ sharply across categories:

| File | Scope | Approx. command count |
|---|---|---:|
| **[21a-command-catalog-public.md](./21a-command-catalog-public.md)** | Universally visible commands in `COMMANDS()`: every entry that ships in non-ANT builds without further gates, including `local` / `local-jsx` / `prompt` kinds. Auth-gated (`availability`) commands are here too because their **registration** is unconditional — they self-hide via `isHidden`. Includes `/init`, `/init-verifiers`, `/review`, `/security-review` (prompt commands with full verbatim corpus), and the entire `/commit-push-pr` / `/commit` chain (which are technically ANT-only but their **prompts are the largest verbatim assets** and live alongside their public siblings for traceability). | ~64 |
| **[21b-command-catalog-ant.md](./21b-command-catalog-ant.md)** | `INTERNAL_ONLY_COMMANDS` + `agents-platform` ANT top-level require. Most entries are stubbed in this leak (`{ isEnabled: () => false, isHidden: true, name: 'stub' }`) — those are documented at registry-citation level. Fully-sourced ANT commands (`/version`, `/bridge-kick`, `/files`, `/cost` ANT branch, `/tag`) get full-fidelity entries. | ~27 |
| **[21c-command-catalog-flagged.md](./21c-command-catalog-flagged.md)** | Feature-flag–gated commands: `/proactive`, `/brief`, `/assistant`, `/remote-control` (BRIDGE_MODE), `/remoteControlServer` (DAEMON+BRIDGE_MODE), `/voice`, `/web-setup` (CCR_REMOTE_SETUP), `/peers` (UDS_INBOX), `/fork` (FORK_SUBAGENT), `/buddy` (BUDDY), `/workflows` (WORKFLOW_SCRIPTS), `/torch` (TORCH). Sources for several are missing entirely from the leak; those are registry-citation. The `KAIROS_GITHUB_WEBHOOKS` `/subscribe-pr`, `HISTORY_SNIP` `/force-snip`, and `ULTRAPLAN` `/ultraplan` entries technically live in `INTERNAL_ONLY_COMMANDS` but their gating semantics belong here — the `21b` file forwards by name to here. | ~12 |
| **[21d-command-catalog-plugin-and-misc.md](./21d-command-catalog-plugin-and-misc.md)** | **Phase 10c addition.** Plugin marketplace UI cluster (`src/commands/plugin/` — `ManagePlugins.tsx` 322KB, `PluginSettings.tsx` 128KB, `BrowseMarketplace.tsx`, `ManageMarketplaces.tsx`, `DiscoverPlugins.tsx` + 13 shared) and the `src/commands/install-github-app/` 13-file step machine, plus long-tail public commands originally missed by the 21a/21b/21c partition (`/add-dir`, `/chrome`, `/context`, `/copy`, `/desktop`, `/diff`, `/doctor`, `/effort`, `/extra-usage`, `/fast`, `/feedback`, `/heapdump`, `/help`, `/ide`, `/install`, `/install-slack-app`, `/keybindings`, `/login`, `/logout`, `/memory`, `/mobile`, `/passes`, `/permissions`, `/plan`, `/privacy-settings`, `/rate-limit-options`, `/release-notes`, `/remote-env`, `/resume`, `/review` ultrareview, `/rewind`, `/sandbox-toggle`, `/session`, `/skills`, `/status`, `/stickers`, `/tag`, `/tasks`, `/terminalSetup`, `/theme`, `/thinkback`, `/thinkback-play`, `/upgrade`, `/usage`). | ~76 |

Each sub-file follows the canonical 12-section template (00-overview §6.1) and inlines verbatim assets per command (prompts, schemas, user-facing strings, constants). Citations use `src/<path>:<line-range>`. **Total cataloged: 64 + 27 + 12 + 76 ≈ 179 command files** (Phase 10 expansion). Note that `~108 command registry entries` ≠ command files (one file may register multiple, and many commands have UI/helper sibling files not registered themselves — see 21d for the full long tail).

## Cross-references

- Registry, Command type union, dispatch order, gate evaluator, allowlists, `BRIDGE_SAFE_COMMANDS` / `REMOTE_SAFE_COMMANDS` set definitions → 20.
- Slash-command **alias resolution** (`/plugin` ↔ `/plugins` ↔ `/marketplace`, `/remote-control` ↔ `/rc`, etc.) is owned by sub-files: 21a/21d for public aliases, 21b for ANT, 21c for flag-gated. The parent does not enumerate aliases.
- Skill / plugin / workflow surfaces (commands loaded via those subsystems are NOT in `src/commands/` and are NOT in this catalog) → 17 (skills), **21d → 28** (plugin slash-command UI cluster lives in 21d; the plugin loader / service layer lives in 28), 19 (workflow tool).
- Per-command tool surface for tool-backed commands (`AGENT_TOOL_NAME` invocations, `mcp__*` tools) → 14, 16.
- Permission state mutations (`/permissions`, `/sandbox`, `/hooks`, `/login`, `/logout`) → 09.
- Mode runtime (commands that gate or toggle a mode — `/proactive`, `/remote-control`, `/voice`, `/brief`) defer their *runtime* mechanics to specs 31..36; this catalog documents the **command surface** only.

## Open questions (deferred to sub-files §12)

1. **Resolved.** ANT-only stubs are stubs *at HEAD of the leak* — every directory listed in 21b §2 source-map has an `index.ts` of the form `{ isEnabled: () => false, isHidden: true, name: 'stub' }`. Whether this is a deliberate leak-cleaning pass or genuine HEAD source cannot be determined from the leak alone, but for spec purposes: 21b documents these at registry-citation level only. No further action.
2. `agents-platform` source missing — the `require('./commands/agents-platform/index.js')` site at `commands.ts:50` references a file that does not exist in the leak; documented at registry level. Also note the gate divergence flagged in the class table above (binding-time gate `USER_TYPE === 'ant'` is more permissive than the registration-time `INTERNAL_ONLY_COMMANDS` gate, but the stricter gate dominates because the symbol is placed inside `INTERNAL_ONLY_COMMANDS`).
3. `/insights` is documented from its lazy shim only (`commands.ts:189-202`); the full 3200-line `insights.ts` (read fully) is summarized but its prompt body is too large to inline verbatim — see 21a §6 for the inline strategy. Important distinction: only `getPromptForCommand` is deferred via dynamic `import()` — the Command metadata object is constructed eagerly at module load, so the shim does NOT skip registration.
