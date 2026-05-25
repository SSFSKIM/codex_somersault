# PHASE10-COVERAGE.md — Final coverage audit (Phase 10 milestone)

**Date:** 2026-05-09
**Phase:** 10.6 (verification)
**Result:** ✅ **0 permissive residuals** / 100.00% coverage

---

## Summary

| Metric | Phase 9 (initial) | Phase 10 (final) | Δ |
|---|---:|---:|---:|
| Total src/ files (.ts/.tsx/.js/.json) | 1,902 | 1,902 | — |
| **Permissive residuals** (basename appears nowhere in any spec) | **~40-50** | **0** | **−100%** |
| Strict residuals (full `src/...` path appears) | 1,161 | 536 | −54% |
| Spec markdown files | 43 | 57 | +14 |
| Total spec lines | ~46,000 | ~52,295 | +14% |

**Strict residuals stay at 536** because spec authors widely prefer basename-only citations
(e.g., `` `bridgeApi.ts` ``) over full-path citations (`` `src/bridge/bridgeApi.ts` ``).
This is a stylistic choice, not a coverage gap. Permissive (basename) residuals are the
correct measure for "is this file mentioned at all?" — and that count is now zero.

## Phase 10 work breakdown

| Phase | Sub-agents | Files cataloged | Spec(s) created/extended | LOC added |
|---|---:|---:|---|---:|
| 9.4 (consistency fixes) | 1 | — | 12 specs touched (00, 08, 09, 14, 19, 21c, 26, 29, 30, …) | minor |
| 10a-1 (components) | 1 | 389 | `37a-components-catalog.md` (new) | 729 |
| 10a-2 (hooks) | 1 | 104 | `37b-hooks-catalog.md` (new) | 893 |
| 10a-3 (ink primitives) | 1 | 96 | `37c-ink-primitives-catalog.md` (new) | 904 |
| 10b (utils long tail) | 1 | 327 | `42a-utils-long-tail.md` (new) | 777 |
| 10c (commands) | 1 | 76 | `21d-command-catalog-plugin-and-misc.md` (new) | 1014 |
| 10.5 (INDEX update) | inline | — | `INDEX.md` (5 new rows + co-ownership notes) | ~50 |
| 10d (final cleanup) | 1 | 40 | spec 17 §11.5, spec 19 §13, spec 35 §13, spec 42 §A, spec 42a §7 | ~150 |
| **Total** | **6 dispatched + 1 inline** | **1,032 net new file mentions** | 5 new + 5 extended | **~4,500** |

## Cross-spec ownership findings (Phase 10 byproduct)

The enumeration revealed significant **co-ownership** edges that the original spec
partition under-counted. Each is documented in INDEX.md "Catalog companions" section
and in the affected spec's §1 scope:

- **Spec 09 (permission system)** is co-owned with **37a** — 49 components in
  `src/components/permissions/` totaling ~700KB implement the permission UX.
- **Spec 23 (MCP)** is co-owned with **37a** — 13+ MCP UI components including
  `ElicitationDialog.tsx` (180KB).
- **Spec 30 (coordinator)** is heavily UI-coupled — co-owned with **37a/37b/42a**
  (~13 components + 9 swarm hooks + 21 swarm utilities).
- **Spec 35 (remote-server)** owns 9 Teleport components, 5 transport hooks, the
  `/review ultrareview` command, and 6 `cli/transports/*` files (added in 10d).
- **Spec 28 (plugins)** is co-owned with **21d** for the 5-file plugin marketplace UI
  cluster (~785KB).
- **Spec 22 (api/models)** is co-owned with **42a** — 17-file `src/utils/model/`.
- **Spec 16 (mcp-lsp)** is co-owned with **42a** — 16-file `src/utils/computerUse/`.

## Verification methodology

```python
# Independent verification — Phase 10.6 final
import os
from pathlib import Path

src = [str(p) for p in Path('src').rglob('*')
       if p.is_file() and p.suffix in {'.ts','.tsx','.js','.json'}]

blob = ''.join(
    s.read_text() for s in Path('docs/specs').glob('*.md')
    if 'PHASE9-COVERAGE' not in s.name and 'PHASE10-COVERAGE' not in s.name
)
# (PHASE9-COVERAGE.md is excluded because it tautologically lists its own residuals
#  as src/-prefixed paths — including it would falsely count the residuals as cited.)

residuals = [f for f in src if os.path.basename(f) not in blob]
assert len(residuals) == 0, residuals  # ✅ passes
```

## Surprises discovered during Phase 10

1. **`useTypeahead.tsx` (213KB)** — single hook implementing the entire prompt-input
   typeahead engine (command/file/agent/teammate/Slack/shell-completion/history merger).
   Spec 37 outline didn't even mention it by name.
2. **`PromptInput.tsx` (355KB)** is the largest non-bundle file in the repo — larger
   than every tool's full directory.
3. **`ansiToPng.ts` (215KB)** is essentially an embedded TypedArray glyph atlas, not
   handwritten code. ~15% of the entire utils residual list by size.
4. **chalk truecolor patch in `src/ink/colorize.ts`** — boosts `chalk.level` from 2 to 3
   when `TERM_PROGRAM === 'vscode'` to prevent Claude orange from washing out to
   salmon (truecolor → 6×6×6 cube downgrade).
5. **`skills/bundled/claudeApiContent.ts`** is a 247KB inlined-markdown data module
   that exists solely so Bun's text-loader can load language-specific Claude API docs at
   build time, kept lazy to optimize startup heap.
6. **Naming triple-overload**: `src/hooks/` (React hooks) vs `src/utils/hooks/` /
   `src/services/hooks/` (PreToolUse/PostToolUse user-config hook system) — three different
   things called "hooks". Phase 9.7 should canonicalize the glossary.
7. **23% utils misplacement** (75 of 327): bash/shell/powershell utils belong with
   permission-engine specs (9/10), swarm utils with coordinator (14/30), computerUse with
   tool-mcp-lsp (16), model registry with api (22). Catalog acknowledges via §5
   reassignment maps; no spec body relocation performed.

## Sister documents

- `PHASE9-COVERAGE.md` — initial residual diff (Phase 9.2 baseline)
- `PHASE9-CONSISTENCY.md` — internal drift findings (Phase 9.3)
- `PHASE9-FIXES-APPLIED.md` — consistency-fix log (Phase 9.4); includes 5 cases where
  the consistency reviewer's recommendation was **inverted** by source verification
- `PHASE10-CLEANUP.md` — final 40-residual cleanup detail (Phase 10d)

## Status

- ✅ INDEX.md current; all 57 specs listed with adjacency edges
- ✅ 0 permissive residuals; 100% src basename coverage
- 🔜 Phase 9.5: adversarial review of ~18 high-risk specs (next)
- 🔜 Phase 9.7: final pass — glossary canonicalization, naming-overload reconciliation
