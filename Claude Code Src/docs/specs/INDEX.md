# docs/specs INDEX

Source of truth for the reverse-spec project. Every spec lives in this directory; status here drives dispatch decisions.

- **Statuses**: `pending`, `wip`, `done`, `review`, `revise`
- **Owner**: `main` for Phase 0/1 specs; sub-agent dispatch label otherwise
- **Last updated**: ISO date of last write
- **Adjacent**: cross-referenced spec IDs (used by sub-agent prompts to scope OUT-of-scope)

Master plan: see `../superpowers/specs/2026-05-08-claude-code-reverse-spec-design.md`.

| #  | Spec                              | Status   | Owner    | Last updated  | Adjacent           |
|----|-----------------------------------|----------|----------|---------------|--------------------|
| 00 | 00-overview.md                    | done     | main     | 2026-05-08    | (root)                         |
| 01 | 01-entrypoint-bootstrap.md        | done     | sub-A1   | 2026-05-08    | 00, 02, 03, 04, 18, 22, 25, 26, 27, 32, 34, 35 |
| 02 | 02-settings-schemas-migrations.md | done     | sub-A2   | 2026-05-08    | 00, 01, 09, 17, 22, 23, 26, 27, 28, 35, 41 |
| 03 | 03-query-engine.md                | done     | sub-B1   | 2026-05-08    | 04, 05, 06, 07, 08, 09, 14, 22, 26, 29, 30, 41 |
| 04 | 04-turn-pipeline.md               | done     | sub-B2   | 2026-05-08    | 03, 05, 07, 08, 09, 22, 26, 29, 41 |
| 05 | 05-context-assembly.md            | done     | sub-B3   | 2026-05-08    | 03, 04, 07, 29, 38, 40         |
| 06 | 06-cost-token-tracking.md         | done     | sub-B4   | 2026-05-08    | 03, 04, 07, 22, 26             |
| 07 | 07-context-compaction.md          | done     | sub-B5   | 2026-05-08    | 03, 04, 05, 09, 22, 26, 29, 41 |
| 08 | 08-tool-base-registry.md          | done     | main     | 2026-05-08    | 03, 04, 09, 10..19, 23         |
| 09 | 09-permission-system.md           | done     | sub-C1   | 2026-05-08    | 02, 03, 04, 08, 10..19, 22, 26, 27, 37, 37a, 37b, 42a |
| 10 | 10-tool-bash.md                   | done     | sub-D0   | 2026-05-08    | 08, 09, 27, 37                 |
| 11 | 11-tool-files.md                  | done     | sub-D1   | 2026-05-08    | 02, 04, 08, 09, 12, 17, 24, 26, 29, 37, 40, 41 |
| 12 | 12-tool-search.md                 | done     | sub-D2   | 2026-05-08    | 08, 09, 11, 17, 23, 26, 28, 37 |
| 13 | 13-tool-web.md                    | done     | sub-D3   | 2026-05-08    | 02, 06, 08, 09, 22, 26         |
| 14 | 14-tool-agent-team.md             | done     | sub-D4   | 2026-05-08    | 08, 09, 15, 26, 30, 37, 37b, 41, 42a |
| 15 | 15-tool-tasks.md                  | done     | sub-D5   | 2026-05-08    | 08, 09, 14, 22, 30, 41         |
| 16 | 16-tool-mcp-lsp.md                | done     | sub-D6   | 2026-05-08    | 08, 09, 23, 24, 28, 37, 37a, 42a |
| 17 | 17-tool-skill.md                  | done     | sub-D7   | 2026-05-08    | 08, 09, 20, 21, 23, 28, 29     |
| 18 | 18-tool-modes.md                  | done     | sub-D8   | 2026-05-08    | 04, 08, 09, 14, 31, 37, 41     |
| 19 | 19-tool-misc.md                   | done     | sub-D9   | 2026-05-08    | 03, 08, 09, 21, 25, 26, 31, 32, 35, 36 |
| 20 | 20-command-system.md              | done     | main     | 2026-05-08    | 17, 21, 28, 31..36             |
| 21 | 21-command-catalog.md (21a/21b/21c/21d) | done | sub-E1+J1 | 2026-05-09 | 10..20, 28, 31..36         |
| 21d| 21d-command-catalog-plugin-and-misc.md | done | sub-J1 | 2026-05-09    | 03, 21, 23, 25, 27, 28, 34, 35, 41 |
| 22 | 22-service-api.md                 | done     | sub-F2   | 2026-05-08    | 01, 02, 03, 04, 06, 07, 25, 26, 27, 34, 35, 42a |
| 23 | 23-service-mcp.md                 | done     | sub-F3   | 2026-05-08    | 16, 22, 25, 28, 32, 34, 37a    |
| 24 | 24-service-lsp.md                 | done     | sub-F4   | 2026-05-08    | 11, 16, 34, 37                 |
| 25 | 25-service-oauth-auth.md          | done     | sub-F5   | 2026-05-08    | 01, 22, 23, 34, 35             |
| 26 | 26-service-analytics-flags.md     | done     | sub-F6   | 2026-05-08    | 01, 02, 03, 06, 09, 22, 27     |
| 27 | 27-service-policy.md              | done     | sub-F7   | 2026-05-08    | 02, 09, 10, 22, 25, 26         |
| 28 | 28-service-plugins.md             | done     | sub-F8   | 2026-05-08    | 02, 17, 20, 21, 21d, 23, 26, 42a |
| 29 | 29-service-memory.md              | done     | sub-F9   | 2026-05-08    | 03, 05, 07, 17, 40, 41         |
| 30 | 30-coordinator-multiagent.md      | done     | sub-G0   | 2026-05-08    | 03, 14, 15, 31, 37a, 37b, 41, 42a |
| 31 | 31-mode-proactive.md              | done     | sub-G1   | 2026-05-08    | 19, 30, 32, 41                 |
| 32 | 32-mode-kairos.md                 | done     | sub-G2   | 2026-05-08    | 17, 19, 21, 23, 26, 30, 31, 35 |
| 33 | 33-mode-daemon.md                 | done     | sub-G3   | 2026-05-08    | 22, 23, 34, 35                 |
| 34 | 34-mode-bridge.md                 | done     | sub-G4   | 2026-05-08    | 16, 23, 25, 33, 35, 37, 41     |
| 35 | 35-mode-remote-server.md          | done     | sub-G5   | 2026-05-08    | 01, 21d, 22, 25, 26, 27, 33, 34, 37a, 41 |
| 36 | 36-mode-voice.md                  | done     | sub-G6   | 2026-05-08    | 01, 02, 25, 26, 37, 39, 41     |
| 37 | 37-ink-ui-shell.md                | done     | sub-H7   | 2026-05-09    | 09, 16, 34, 36, 37a, 37b, 37c, 38, 39, 41 |
| 37a| 37a-components-catalog.md         | done     | sub-J2   | 2026-05-09    | 09, 11, 23, 30, 35, 37         |
| 37b| 37b-hooks-catalog.md              | done     | sub-J3   | 2026-05-09    | 09, 13, 14, 27, 28, 30, 34, 35, 37, 38, 41 |
| 37c| 37c-ink-primitives-catalog.md     | done     | sub-J4   | 2026-05-09    | 37, 38                         |
| 38 | 38-output-styles.md               | done     | sub-H8   | 2026-05-09    | 05, 37                         |
| 39 | 39-vim-keybindings.md             | done     | sub-H9   | 2026-05-09    | 36, 37, 41                     |
| 40 | 40-persistent-memory.md           | done     | sub-H0   | 2026-05-09    | 05, 29, 41                     |
| 41 | 41-session-state-history.md       | done     | sub-H1   | 2026-05-09    | 04, 14, 15, 21d, 29, 34, 35, 37, 40 |
| 42 | 42-misc.md                        | done     | sub-H2   | 2026-05-09    | 00, 01, 20, 37, 41, 42a        |
| 42a| 42a-utils-long-tail.md            | done     | sub-J5   | 2026-05-09    | 09, 10, 14, 16, 22, 28, 30, 42 |

## Update protocol

When a sub-agent completes a spec:
1. Write the spec file to `docs/specs/<NN-name>.md`.
2. Edit this INDEX.md row: `Status` → `done`, set `Owner` and `Last updated`, refine `Adjacent` if cross-references discovered.
3. Reply to the dispatcher with the ≤700-word summary.

When the main agent dispatches:
1. Verify prerequisite rows from the phase graph are `done`; use `Adjacent` rows as scope boundaries, not as a blanket dependency list.
2. Set the row's `Status` to `wip` and `Owner` to a dispatch label (e.g. `sub-A1`).
3. Issue the dispatch with the standard sub-agent prompt template (see master plan §5.3).

When verification flags issues (Phase 9):
1. Set `Status` → `revise`.
2. Re-dispatch with the adversarial findings as input.
3. On fix, set back to `done`.

## Catalog companions (Phase 10 additions)

The following specs are **catalog companions** to a core spec. They enumerate individual files
that the core spec covers at architectural level only. Convention: `NN` (core architectural
spec) + `NNa/NNb/...` (per-submodule enumeration). Catalog specs are the residual sink — every
file in their submodule is cited at least once.

| Catalog            | Core | Files cataloged | LOC of catalog | Submodule covered |
|--------------------|------|----------------:|---------------:|-------------------|
| 21d                | 21   | 76 (plugin family + long-tail commands) |  1014 | `src/commands/plugin/`, `src/commands/install-github-app/`, long-tail of `src/commands/*` |
| 37a                | 37   | 389 components |   729 | `src/components/` (all submodules) |
| 37b                | 37   | 104 hooks      |   893 | `src/hooks/` (incl. notifs/, toolPermission/) |
| 37c                | 37   | 96 primitives  |   904 | `src/ink/` |
| 42a                | 42   | 327 utilities  |   777 | `src/utils/` (28 submodules) |
| **Total**          |      | **992 files**  | **4,317** lines |                  |

### Cross-spec ownership notes (Phase 10 finding)

The enumeration sweep surfaced significant cross-cutting that the original spec partition
under-counted. These are **co-ownership** edges, not relocations:

- **Spec 09 (permission system)** is co-owned with 37a — `src/components/permissions/` holds
  49 component files (~700KB) implementing the permission UX state machines, rule editing,
  and per-tool variants. The two largest dialogs are 119KB and 122KB.
- **Spec 23 (MCP)** has a far larger UI surface than spec 23 alone signals — 13+ components
  in `src/components/mcp/` including `ElicitationDialog.tsx` (180KB).
- **Spec 30 (coordinator)** is heavily UI-coupled — ~13 components plus 9 swarm hooks plus
  21 swarm utilities. Effectively co-owned with 37a/37b/42a.
- **Spec 35 (remote-server)** owns 9 Teleport components plus 5 transport hooks plus the
  `/review ultrareview` command (21d).
- **Spec 28 (plugins)** is co-owned with 21d for the plugin marketplace UI cluster
  (5 huge files totaling ~785KB).
- **Spec 22 (api / models)** is co-owned with 42a for the 17-file `src/utils/model/`
  directory (model registry, capability matrix).
- **Spec 16 (mcp-lsp)** is co-owned with 42a for the 16-file `src/utils/computerUse/` and
  related `src/utils/claudeInChrome/` directories.

These ownership notes are referenced from each catalog spec's §5 (cross-spec map) and from
the affected core spec's §1 (scope). No spec body content was relocated; the catalog acts
as the residual citation registry.
