# Phase 9.5 Adversarial Review — Spec 10 (BashTool)

Reviewer: Opus fallback (original codex agent failed exit 1 @ 813s).
Scope verified against: `src/tools/BashTool/`, `src/utils/bash/`,
`src/utils/shell/`, `src/utils/permissions/dangerousPatterns.ts`.
Files read: 12 (under the ~25 budget).

## Severity Counts
- Critical (security/correctness): 0
- High (factual errors caller will trip on): 1
- Medium (off-by-one / drift / contradiction): 4
- Low (cosmetic / stale-comment annotations): 4
- Untestable claims: 1

## Top 5 Findings

1. **HIGH — `getMaxTimeoutMs` semantics misstated.**
   Spec §6.4 says "`BASH_MAX_TIMEOUT_MS` env override clamped to ≥ default".
   Source `src/utils/timeouts.ts:38`:
   `return Math.max(MAX_TIMEOUT_MS, getDefaultBashTimeoutMs(env))`.
   That is the **fallback when the env var is missing/invalid**, not the
   clamping rule. The actual override branch (lines 29-37, not shown but
   inferable) parses the env var; the clamp floor is `MAX_TIMEOUT_MS`
   (600_000) AND the user-set default — meaning a user who sets only
   `BASH_DEFAULT_TIMEOUT_MS=900_000` gets a max of 900_000 with no env
   max set, which contradicts the spec's "clamped to ≥ default" phrasing
   (it's clamped to ≥ max(MAX_TIMEOUT_MS, BASH_DEFAULT_TIMEOUT_MS)). Reword.

2. **MEDIUM — `parseCommandRaw` gating description is incomplete.**
   Spec §5.11 says `parseCommandRaw` is gated on
   `TREE_SITTER_BASH || TREE_SITTER_BASH_SHADOW` (correct, parser.ts:108).
   But spec §5.1 step 0 reads: `astRoot = injectionCheckDisabled ? null
   : (TREE_SITTER_BASH_SHADOW && !shadowEnabled) ? null : await
   parseCommandRaw(...)`. This omits that when **neither** flag is on,
   `parseCommandRaw` itself returns `null` (line 132), so external builds
   silently take the `parse-unavailable` branch. The pseudocode reads as
   though `parseCommandRaw` always parses — a reimplementer following the
   pseudocode literally would parse in external builds. Add a fall-through
   note for `!TREE_SITTER_BASH && !TREE_SITTER_BASH_SHADOW`.

3. **MEDIUM — Stale "WASM init" comment is contradicted in spec but the
   spec itself reproduces the misleading WASM language.** Source
   `parser.ts:38-46` says "Awaits WASM init (Parser.init + Language.load)";
   spec §7.5 correctly flags it as stale ("pure-TypeScript ...
   no native module is loaded; the comments in `parser.ts` referring to
   'WASM init' are stale"). However the verbatim citation in §5.11 says
   "tree-sitter WASM is unavailable" mirroring the comment. The spec has
   it both ways. The bashParser.ts header is the trust boundary;
   confirmed pure-TS via `MAX_NODES`/`PARSE_TIMEOUT_MS` constants in
   `bashParser.ts:29,32`. Pick one description and stick to it.

4. **MEDIUM — `MAX_PERSISTED_SIZE` truncation order.** Spec §11 item 12:
   "hard-link first, copyFile fallback, **truncate to 64 MiB before
   linking**". Source `BashTool.tsx:741-748` order is: stat → IF
   size>64MiB **truncate the source** → THEN link (fallback copy). So
   "before linking" is correct, but the comment in §5.9 says "If > 64MB:
   truncate(outputFilePath, 64MiB)" — note this truncates the **source**
   file, not the dest. A reader may assume the truncate is on `dest`.
   Make explicit that the source `outputFilePath` is truncated in-place;
   then the (already-truncated) file is linked.

5. **MEDIUM — `bashSecurity.ts` line count drift.** Spec §2.1 lists
   `bashSecurity.ts` at 2592 lines (read: sampled). My `wc -l` returns
   2592. Same for `bashPermissions.ts` (2621). `ast.ts` is reported in
   §2.4-style narrative as "112KB" but its real line count is 2679.
   `readOnlyValidation.ts` is reported 1990 — confirmed 1990. Sizes
   match; the dispatch prompt's "112KB" claim for `ast.ts` is *external*
   (from the reviewer dispatch), not in the spec. No spec error, but
   the spec should add `ast.ts` to §2.1 explicitly — currently it's only
   referenced via `parseForSecurityFromAst` in §5 without a line-count
   row, despite being the AST trust boundary.

## Other Verified-Correct Claims (sample)
- All cited line numbers in §2.4 feature-flag table match source
  (`bashPermissions.ts:1683,1684,1690,1707; parser.ts:51,65,108;
  BashTool.tsx:525,976; prompt.ts:49,56,312,320; shouldUseSandbox.ts:23;
  readOnlyValidation.ts:1211,1212; dangerousPatterns.ts:58`).
- `MAX_SUBCOMMANDS_FOR_SECURITY_CHECK=50` (`bashPermissions.ts:103`),
  `MAX_SUGGESTED_RULES_FOR_COMPOUND=5` (`:110`),
  `MAX_PERSISTED_SIZE = 64 * 1024 * 1024` (`BashTool.tsx:732`),
  `ASSISTANT_BLOCKING_BUDGET_MS=15_000` (`:57`),
  `PROGRESS_THRESHOLD_MS=2000` (`:55`),
  `BASH_MAX_OUTPUT_DEFAULT=30_000`, `BASH_MAX_OUTPUT_UPPER_LIMIT=150_000`
  (`outputLimits.ts:3-4`) — all verbatim correct.
- `DANGEROUS_BASH_PATTERNS` enumeration §6.4 matches
  `dangerousPatterns.ts:18-80` exactly (CROSS_PLATFORM_CODE_EXEC list,
  ANT extension list, ordering).
- `_simulatedSedEdit` schema-strip + security comment at
  `BashTool.tsx:249-253` correct.
- `PARSE_ABORTED` symbol semantics §5.11 (fail-closed → too-complex,
  module-not-loaded → null) correctly mirrors `parser.ts:93,124,132`
  comments and `bashPermissions.ts:1692,1741`.
- ANT-only `tengu_sandbox_disabled_commands` substring AND command-token
  matching at `shouldUseSandbox.ts:23-49` — accurate.
- `containsExcludedCommand` is **not a security boundary** (per source
  comment line 18-20). Spec §5.7 captures this correctly.
- `ACCEPT_EDITS_ALLOWED_COMMANDS = [mkdir,touch,rm,rmdir,mv,cp,sed]`
  matches `modeValidation.ts:7-15` exactly.

## Low / Cosmetic
- §3.4 `validateInput` cites `:524-538` — actual range `:524-538` ✓.
- §5.9 narrative says "shell run-time `shouldUseSandbox` option" passed
  to `Shell.exec`; `BashTool.tsx:896` is inside `runShellCommand` —
  confirmed at the call site, but spec gives 502/896 with 502 being the
  UI label site. Acceptable.
- §6.5 string `Permission to use ${BashTool.name} with command ...` is
  in `bashPermissions.ts` at the cited lines, but is rendered via
  `createPermissionRequestMessage` — readers who grep for the literal
  string in BashTool/ get the indirect callsites (createPermissionRequestMessage
  is in `src/utils/permissions/PermissionResult.ts`). Spec doesn't claim
  origin location, so OK.
- `validateBoundedIntEnvVar` is referenced indirectly; spec doesn't
  mention it. Minor — `outputLimits.ts:7` explicitly returns
  `result.effective` from this helper. Adds clamp behavior.

## Untestable Claim
- §11 invariant 16: "ANT-only paths are bundle-eliminated in external
  builds via the `process.env.USER_TYPE === 'ant'` constant-fold; no
  ANT_ONLY_* string appears in external bundles." Cannot verify — no
  build artifact ships with this leak (per repo CLAUDE.md). Whether
  Bun's bundler actually constant-folds `process.env.USER_TYPE` at
  build time is asserted, not demonstrable from source. Treat as a
  build-config dependency that 02 / 27 should backstop.

## Verdict
**ACCEPT WITH MINOR REVISIONS.** Spec 10 is high-fidelity to source.
No security holes uncovered. The legitimate concerns are clarity issues
(parseCommandRaw gating, truncate-source-vs-dest, max-timeout phrasing)
and one stale-WASM-comment ambiguity inherited from source. The
verbatim asset section (§6) survives spot-checking against
`bashPermissions.ts`, `dangerousPatterns.ts`, `outputLimits.ts`,
`modeValidation.ts`, `parser.ts`, `BashTool.tsx`. The control-flow
pseudocode in §5.1 is faithful to source order at `bashPermissions.ts:
1663-2557`. The PARSE_ABORTED fail-closed invariant — the riskiest
single claim — is correctly captured.

## Cross-Spec Impact
- **Spec 09 (permission system):** Spec 10 §5.2-5.4 documents the
  decision order BashTool feeds into 09's global tree. The "deny → ask
  → allow → passthrough" per-subcommand ordering is owned by 10; 09
  must reflect this entry shape. No drift detected from this side.
- **Spec 27 (policy / settings):** §6.4 `sandbox.excludedCommands` and
  §8 ANT GrowthBook `tengu_sandbox_disabled_commands` are read-only
  settings consumers; spec 27 should document the schema. Spec 10
  correctly defers this (§12 open question 6).
- **Spec 08 (registry):** name `'Bash'`, registration at
  `src/tools.ts:197` confirmed. No drift.
- **Spec 19 (PowerShell):** §1 correctly flags `gitOperationTracking.ts`
  as shared. CROSS_PLATFORM_CODE_EXEC is shared via
  `dangerousPatterns.ts` — spec 19 must re-cite the same list.
- **Spec 24 (LSP) and 41 (file history):** spec 10 defers
  `_simulatedSedEdit` history wiring; correct deferral.

## Hardest-to-Verify Claim
**Invariant 16** ("ANT_ONLY_* strings DCE'd from external bundles via
`process.env.USER_TYPE === 'ant'` constant-fold"). This requires a
build artifact and a bundler-aware static-analysis pass that this leak
does not enable. Source confirms the *pattern* is uniformly applied
(`bashPermissions.ts:174,250,329,591`; `readOnlyValidation.ts:1211`;
`shouldUseSandbox.ts:23`; `prompt.ts:49,56`; `dangerousPatterns.ts:58`)
— but whether DCE actually fires depends on Bun configuration outside
this repo. The CLAUDE.md "feature-flag gated imports" pattern only
covers `feature(...)` calls; the `process.env.USER_TYPE === 'ant'`
fold is a **separate** convention not documented in the bundler
config (which we don't have).
