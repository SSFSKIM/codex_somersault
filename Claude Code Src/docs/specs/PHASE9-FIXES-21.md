# PHASE 9.6 B-full Fix Log — Spec 21 (Command Catalog parent)

Reviewer's findings from `PHASE9-ADVERSARIAL-21.md` resolved in this pass:

## H1 — Parent missing 21d row in partition table
**Status:** Already addressed in B-mini (prior commit). 21d row present in partition table. No action this pass.

## H2 — "Gate column" claim
**Status:** Re-verified.
- `21c-command-catalog-flagged.md` §1 already has a per-flag gate matrix as its first table (column 1 is `Flag`, which IS the gate). This was the artifact the Phase 9.4 claim referenced. Non-finding for the sub-file.
- For the parent: original class table at lines 9-18 had columns `Class | Count | Examples | Notes` with no Gate column. **Fixed** by reorganising to `Class | Gate | Count | Examples` and putting the precise gate clause (`commands.ts:343`, `commands.ts:48-50`, etc.) into a dedicated column. This both addresses H2 and disambiguates the gate-divergence at the heart of H4.

## H3 — Class counts contradict source
**Recomputed enumeration** of `INTERNAL_ONLY_COMMANDS` from `src/commands.ts:225-254`:
- 25 named entries: `backfillSessions, breakCache, bughunter, commit, commitPushPr, ctx_viz, goodClaude, issue, initVerifiers, mockLimits, bridgeKick, version, resetLimits, resetLimitsNonInteractive, onboarding, share, summary, teleport, antTrace, perfIssue, env, oauthRefresh, debugToolCall, agentsPlatform, autofixPr`.
- 3 conditional spreads: `forceSnip` (HISTORY_SNIP), `ultraplan` (ULTRAPLAN), `subscribePr` (KAIROS_GITHUB_WEBHOOKS).
- Pre-`.filter(Boolean)` total: **25 + 3 = 28**. (Reviewer was right; original spec said "27 + 1 separate top-level + 3" double-counting `agentsPlatform`.)

**Total registered (all flags ON arithmetic) recomputed:**
- 62 always-in entries in `COMMANDS()` body
- 12 feature-flag spreads
- 2 3P-aware (`logout`, `login()`)
- 28 from `INTERNAL_ONLY_COMMANDS`
- 1 lazy-shim (`usageReport`)
- **62 + 12 + 2 + 28 + 1 = 105**

Old "~108" figure was wrong (counted `agents-platform` twice). Replaced with `105` and showed-the-work arithmetic inline.

## H4 — ANT gate semantics flattened
**Status:** Fixed.
- `INTERNAL_ONLY_COMMANDS` registration spread at `commands.ts:343`: `process.env.USER_TYPE === 'ant' && !process.env.IS_DEMO`.
- `agentsPlatform` binding at `commands.ts:48-50`: `process.env.USER_TYPE === 'ant'` only — **no** `!IS_DEMO` clause.
- Spec previously treated as same gate. Now the class table calls out the divergence in two rows: (a) the row for `INTERNAL_ONLY_COMMANDS` cites `commands.ts:343` with the conjunction; (b) a dedicated row for the `agentsPlatform` top-level `require` cites `commands.ts:48-50` with just the `USER_TYPE` check. Note added that the looser binding-time gate is dominated by the stricter registration-time gate because `agentsPlatform` is placed inside `INTERNAL_ONLY_COMMANDS` (line 252) — so registration only happens when both gates pass.

## H5 — Spec 20/21 boundary
**Status:** Tightened.
- Lead paragraph rewritten: "Read 20-command-system.md first for the registry, type union, dispatch model, **gate semantics (`INTERNAL_ONLY_COMMANDS` mechanics, `availability` carve-out, `feature(...)` DCE)**".
- Source-coverage inventory paragraph now says "Class table below is a **count-only** summary; the per-command Gate/source-present table lives in 21c §1 and 21a/21b/21d §2. Defer to spec 20 for the runtime gate evaluator semantics."
- Removed the prose re-derivation of `INTERNAL_ONLY_COMMANDS` mechanics (compressed into the class table's Gate column citations).

## Open questions
- Q1 ("ANT stub bypass") **resolved in-place** — answer is "stubs at HEAD of leak; documented at registry level in 21b". (Per L1 from the review.)
- Q2 (`agents-platform` source missing) updated with the gate-divergence note.
- Q3 (`/insights` lazy-shim) tightened: clarified that ONLY `getPromptForCommand` is deferred via dynamic `import()` — the Command metadata is constructed eagerly. Addresses M4.

## Cross-references
- Added explicit alias-resolution forwarding ("21a/21d for public aliases, 21b for ANT, 21c for flag-gated") — addresses Nit-2.
- Updated plugin cross-ref to `21d → 28` rather than `21 → 28` — addresses L2.
- `BRIDGE_SAFE_COMMANDS` / `REMOTE_SAFE_COMMANDS` cited as owned by spec 20 (set definitions) — addresses L3 (deferred to 20 rather than expanding parent).

## Items skipped / not touched this pass
- M1 (fragile arithmetic): now explicitly shown — superseded.
- M2 (`login()` is a function call): now noted in the 3P-aware row of the class table.
- M3 (21c "12" vs "17"): partition table still says "~12" because the 17 in 21c §1 includes 2 non-command flag wirings (`EXPERIMENTAL_SKILL_SEARCH`, `MCP_SKILLS`). 12 = command count; left as-is with the "~" caveat — explicit disambiguation belongs in 21c, not the parent.
- Nit-1 (`commands.ts:2-186` should be `:1-210`): fixed inline (`commands.ts:1-210`).

## Top 3 fixes (summary)
1. **Class table redesigned** with Gate column citing exact `commands.ts` lines, splitting the `agents-platform` gate from the `INTERNAL_ONLY_COMMANDS` gate (H2 + H4).
2. **Enumeration arithmetic shown** with `25 + 3 = 28` for `INTERNAL_ONLY_COMMANDS` and `62 + 12 + 2 + 28 + 1 = 105` for registered total — replaces the wrong `~108` (H3).
3. **Boundary with spec 20 tightened** — gate semantics, set definitions (`BRIDGE_SAFE_COMMANDS`, `REMOTE_SAFE_COMMANDS`), and runtime evaluator deferred to 20; 21 parent is purely router + class-tally (H5).
