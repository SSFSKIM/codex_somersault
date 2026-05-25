# PHASE 9 Adversarial Review — Spec 21 (Command Catalog parent/index)

Reviewer: Opus, parallel x18.
Scope: parent `21-command-catalog.md` only. 21a/21b/21c/21d are out of scope for this pass.

## Severity counts

- Critical: 0
- High: 3
- Medium: 4
- Low: 3
- Nit: 2

## Top 5 findings

### H1 — Parent does not mention 21d at all (taxonomy completeness broken)

`21-command-catalog.md` § "Sub-file partition" lists only **21a / 21b / 21c** with counts 64+27+12 ≈ 103. There is no row for 21d, no narrative paragraph naming it, and no acknowledgement that the public/ant/flagged taxonomy is *incomplete by itself*. Yet `INDEX.md:35-36` registers `21d-command-catalog-plugin-and-misc.md` as "76 (plugin family + long-tail commands)" and 21d's own header openly states it "picks up the **73 command files** the original public/ant/flagged split missed". The parent's promise that the three sub-files enumerate every entry is therefore false at HEAD; readers following the parent will believe `/plugin`, `/install-github-app`, `/extra-usage`, `/context`, `/add-dir`, `/help`, `/doctor`, `/login`, `/logout`, `/mcp`, `/model`, `/ide`, `/permissions`, `/plan`, `/memory`, `/skills`, `/usage`, `/upgrade`, `/theme`, `/vim`, `/resume`, `/rewind`, `/stickers`, etc. are catalogued in 21a when they are actually in 21d. This is the dominant defect of this pass.
Evidence: `21-command-catalog.md:38-44` (3-row sub-file table), `INDEX.md:35-36`, `21d-command-catalog-plugin-and-misc.md:7-14`.

### H2 — "Gate column" claim from Phase 9.4 is unverifiable in the parent

The review prompt asserts Phase 9.4 added a "Gate column". The parent file contains zero occurrences of "Gate" (verified by grep of the file: only `feature-gated`, `auth-gated`, `gates`/`gating` prose). The single Class-table at lines 9-18 has columns `Class | Count | Examples | Notes` — no Gate column. If a Gate column was meant to land in the parent, it did not. If it was meant to land in the sub-files, the parent's claim of being a "router" should reference it; it does not.
Evidence: `21-command-catalog.md:9-18`.

### H3 — Class counts contradict themselves and source

Row "Public built-in" claims 62; Row "ANT-only top-level require" claims 1 (`agents-platform`); Row "ANT-only conditionally + feature-flag" claims 3; Row "ANT-only" claims 27 — but `INTERNAL_ONLY_COMMANDS` in `commands.ts:225-254` contains exactly 28 named entries before `.filter(Boolean)` (with 3 `...spread` conditionals: forceSnip, ultraplan, subscribePr — counted in row 5). Subtracting the 3 spreads gives 25, not 27, and `agentsPlatform` is *also inside* `INTERNAL_ONLY_COMMANDS` (line 252) — so claiming it as a separate "top-level require" row double-counts: it appears both in row 4 and row 6. The claimed total `~108` does not match either 64+27+12 (103) from the partition table, the 101 entries in `ls src/commands/`, or any consistent enumeration.
Evidence: `commands.ts:225-254`; `21-command-catalog.md:11-18`; partition row totals 40-42.

### H4 — "USER_TYPE === 'ant' && !IS_DEMO" gate description is incomplete

Line 14 says ANT-only = `USER_TYPE === 'ant' && !IS_DEMO`. The actual check at `commands.ts:343` is `process.env.USER_TYPE === 'ant' && !process.env.IS_DEMO`. More importantly, `agentsPlatform` is gated by a *different* check at `commands.ts:48-50`: `process.env.USER_TYPE === 'ant'` only — *no* IS_DEMO clause. The parent's claim that `agents-platform` belongs in the same ANT bucket is therefore wrong: it ships in IS_DEMO ANT builds while the rest of `INTERNAL_ONLY_COMMANDS` does not. This is a real semantic divergence the parent flattens.
Evidence: `commands.ts:48-50` vs `commands.ts:343-345`.

### H5 — Spec 21 vs spec 20 boundary is asserted but not enforced

The parent says "Read 20-command-system.md first ... this catalog enumerates **what** each `/foo` command does, not how the registry routes it" (line 3). Yet § "Source-coverage inventory" then re-derives the registry order from `commands.ts:258-319`, re-states the `INTERNAL_ONLY_COMMANDS` mechanics, re-cites `commands.ts:2-186`, and re-explains the auth-gated `availability` carve-out — all of which spec 20 owns per the boundary statement. The "router" framing claims minimality but the file is ~60 lines of overlap with 20.
Evidence: `21-command-catalog.md:3,7,10-18`.

## Other findings (M / L / Nit)

- **M1**: Line 18 says total `~108` but lines 11-17 sum to 62+2+12+27+3+1+1 = 108 *if* you count the lazy-shim `/insights` separately and exclude that the 3 conditional-flag-ANT entries are *inside* the 27, and exclude the agents-platform double-count — the math is fragile and undocumented.
- **M2**: Row 2 "3P-aware" example list is `/login`, `/logout`. The carve-out at `commands.ts:337` is `!isUsing3PServices() ? [logout, login()] : []` — note `login()` is a function call, not the import; the parent does not flag this. Reimplementers reproducing the registry need this.
- **M3**: § "Sub-file partition" gives `~12` for 21c but the per-flag table in 21c lists 17 entries (3 of which are non-command wirings). Parent says "12" without disambiguating "command-shaped feature gates" vs "all `feature(...)` sites".
- **M4**: "Lazy-shim only (heavy deferred) | 1 | `/insights`" — but `commands.ts:188-202` shows `usageReport` is *the* lazy shim entry (which is a `prompt` command, not lazy in the dynamic-import sense the parent suggests). The shim only defers *getPromptForCommand*, not the entire registration; framing as "lazy import" without that nuance misleads.
- **L1**: Open question §1 ("ANT-only stub bypass") — the answer is visible in 21b §2 source-map (most ANT files are stubbed); parent should resolve or cross-reference instead of leaving open.
- **L2**: Cross-reference §3 ("Skill / plugin / workflow surfaces ... → 17, 28, 19") does not mention 21d which actually catalogues the *plugin slash command UI* (distinct from the plugin loader at 28). Reader following 21 → 28 misses the UI surface.
- **L3**: Parent does not mention `BRIDGE_SAFE_COMMANDS` or `REMOTE_SAFE_COMMANDS` — both exported from `commands.ts:619-660` and both are command-catalog facts (not registry mechanics). They are command-set definitions, so they likely belong here or in a sub-file with a forwarding pointer here.
- **Nit-1**: Path "`commands.ts:2-186`" in line 7 — the static import block actually runs to line 210 (the imports trail past the `INTERNAL_ONLY_COMMANDS` const). Minor citation drift.
- **Nit-2**: Slash-command alias resolution is not mentioned in the parent at all (e.g. `/plugin` ↔ `/plugins` ↔ `/marketplace`, `/remote-control` ↔ `/rc`). The review prompt explicitly asks about this; parent defers entirely to sub-files without even saying so.

## Verdict

**Needs revision** before merge. H1 alone breaks the parent's central promise. H2/H3/H4 compound it. The parent should: (a) add a 21d row to the partition table, (b) reconcile the class counts to a single arithmetic identity, (c) split the `agents-platform` gate from the `INTERNAL_ONLY_COMMANDS` gate, (d) actually add the Gate column or remove the Phase 9.4 claim, (e) explicitly state alias resolution is deferred to 21a/d.

## Cross-spec impact

- **Spec 20**: H5 means 20 and 21 overlap; the boundary needs sharpening on either side.
- **Spec 28** (plugin service layer): cross-ref from 21 to 28 should be `21d → 28`, not `21 → 28`.
- **INDEX.md**: already correctly registers 21d. The parent is the lagging artifact, not the index.
- **Specs 31..36** (modes): correctly cited; no impact.

## Hardest-to-verify claim

> "**Total registered ~108** ... A few commands have two entries (interactive vs non-interactive variants — `extra-usage`, `context`, `reset-limits`)"

To verify this requires: (1) enumerating `COMMANDS()` static order with all flags ON, (2) adding the *spread* conditionals (`webCmd`, `forkCmd`, `buddy`, `proactive`, `briefCommand`, `assistantCommand`, `bridge`, `remoteControlServerCommand`, `voiceCommand`, `peersCmd`, `workflowsCmd`, `torch`), (3) adding `INTERNAL_ONLY_COMMANDS` (which itself has 3 internal `feature(...)` spreads), (4) accounting for `usageReport` being *constructed inline* not imported, (5) accounting for `login()` being a function call yielding one Command, (6) deciding whether `agentsPlatform` counts once or twice (it's inside INTERNAL_ONLY_COMMANDS *and* gated by its own ANT check at import), (7) counting both halves of the dual-entry trios (extra-usage, context, reset-limits). I cannot reproduce 108 from the source consistently — I get 105, 106, or 109 depending on assumption. Parent does not show its work.
