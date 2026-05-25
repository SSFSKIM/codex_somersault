# Phase 9.6c Fix Log — Spec 16 (tool-mcp-lsp)

Source review: `docs/specs/PHASE9-ADVERSARIAL-16.md` (cleanest review of the batch).

## Fixes applied to `docs/specs/16-tool-mcp-lsp.md`

### §2.1 line-count corrections (bit-exact targets)

Both citations off by one (off-by-one likely from inclusive vs. exclusive counting against `wc -l` output).

| File | Spec said | Actual `wc -l` | Fix |
|---|---|---|---|
| `src/tools/MCPTool/MCPTool.ts` | `1-78` | `77` | Updated to `1-77`. |
| `src/tools/LSPTool/LSPTool.ts` | `1-861` | `860` | Updated to `1-860`. |

Verified via:

```
$ wc -l src/tools/MCPTool/MCPTool.ts src/tools/LSPTool/LSPTool.ts
      77 src/tools/MCPTool/MCPTool.ts
     860 src/tools/LSPTool/LSPTool.ts
```

## Optional check — `utils/computerUse` co-ownership claim (spec 42a §5)

Adversarial review flagged: spec 42a §5 lists `utils/computerUse/` as co-owned by spec 16, but spec 16 itself never references the directory.

Verification — searched both tool directories for any `computerUse` reference:

```
$ grep -rln "computerUse" src/tools/MCPTool/ src/tools/LSPTool/
(no matches; exit 1)
```

**Result:** zero imports / zero textual references from MCPTool or LSPTool to `utils/computerUse/*`. The co-ownership in spec 42a is therefore aspirational / based on the broader "MCP/computer-use" grouping in §10 of 42a, not on actual spec-16 in-scope files.

**Decision: no edit to spec 42a.** Rationale:

- The 42a §5 row reads `Computer-use (utils/computerUse/) | 16 | …` and the §10 long-tail row is explicitly labelled `16 — MCP/computer-use`. Both treat spec 16 as the *thematic* owner of computer-use plumbing, even though spec 16's bit-exact §2.1 targets are MCP/LSP tool-shell files only.
- Removing the row would orphan ~16 computer-use files (wrapper, executor, lock, hostAdapter, etc.) from any spec, which is a bigger correctness hit than leaving the slightly loose co-ownership claim.
- A more accurate fix (renaming spec 16's scope, or splitting computer-use into its own spec) is out of scope for a 9.6c small-fix pass.

This is logged here for Phase 10 follow-up rather than edited now.

## Files modified

- `docs/specs/16-tool-mcp-lsp.md` — two line-count edits in §2.1.

## Files NOT modified

- `docs/specs/42a-utils-long-tail.md` — co-ownership row left in place; rationale above.
