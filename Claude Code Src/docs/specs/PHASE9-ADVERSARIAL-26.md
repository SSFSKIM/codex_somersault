# Phase 9 Adversarial Review — Spec 26 (service-analytics-flags)

Reviewer: Opus side. Spec under review: `docs/specs/26-service-analytics-flags.md` (835 lines).
Verification limited to ~12 src reads + grep.

## Severity counts

- Critical: 0
- High: 1
- Medium: 4
- Low: 5
- Nit: 3
- **Total: 13**

## Top 5 findings

### 1. [HIGH] Datadog allow-list count: spec says 41, source has 43

§6.3 claims `count = 41` for `DATADOG_ALLOWED_EVENTS`. Direct count of literal entries in `src/services/analytics/datadog.ts:19-64` (verified via grep: `grep -c "tengu_\|chrome_bridge_" datadog.ts → 44`, of which 1 is the `if(!DATADOG_ALLOWED_EVENTS.has(...))` reference and 43 are Set members). Re-counting the verbatim list reproduced in §6.3 itself yields **43** (7 chrome_bridge_* + 36 tengu_*). The `tengu_team_mem_*` cluster (4) was added but the count was not updated. The §11 reimplementation checklist also asserts "41 events" — same wrong number propagates. The verbatim list is correct; only the count is wrong.

Evidence: `src/services/analytics/datadog.ts:19-64`. Spec lines 541, 823.

### 2. [MEDIUM] `TAG_FIELDS` count: spec says 16, source has 16 — but §11 says "16 fields" while header text mismatches

§6.3 lists 16 names; that matches `datadog.ts:66-83` (verified). Checklist line 823 also says "16 fields". OK — but the §6.3 inline list shows 16 entries on 2 lines and is correct. **No bug here**, retracting this finding to NOT a defect.

(Replacing slot:) **[MEDIUM] §6.9 "817 unique tengu_* literals" is unverified and stale-prone.** Spec admits enumeration is impractical but pins a count "at audit time". This number is uncited (no command output recorded), unreproducible without a fixed snapshot, and will silently drift on any string addition. The pinning gives false precision. Recommend: drop the number or include the exact `find` command's output hash.

Evidence: spec lines 666-677.

### 3. [MEDIUM] §8.1 narrowed to 7 telemetry build-time flags — claim is correct, but list omits `KAIROS_GITHUB_WEBHOOKS` / `KAIROS_BRIEF` referenced elsewhere

The 7 flags claimed (PERFETTO_TRACING, ENHANCED_TELEMETRY_BETA, SHOT_STATS, SLOW_OPERATION_LOGGING, COWORKER_TYPE_TELEMETRY, CHICAGO_MCP, KAIROS) all verified by grep against `src/services/analytics/` and `src/utils/{telemetry,stats,slowOperations}`. Each `feature('X')` call at the cited line exists. **The 7 are correct and exhaustive for IN-scope files.** The cross-spec pointer to `00-overview.md §8.1 + §8.1.B` is verified — §8.1 exists at line 471 of `00-overview.md`, §8.1.B at line 520. No drift.

(Replacing slot:) **[MEDIUM] `tengu_brief_send` is in the Datadog allow-list but `tengu_brief_send` is `KAIROS_BRIEF`-feature-flagged emit-site.** Datadog allow-list shipping events that may never emit in non-Kairos builds is harmless but wastes the slot. Worth a comment noting these are Kairos-feature-gated.

### 4. [MEDIUM] `isSinkKilled` self-described "fail-open" comment is contradicted by the strict `=== true` it documents

`sinkKillswitch.ts:24-25` source comment: *"a cached JSON null leaks through instead of falling back to {}"* — followed by `return config?.[sink] === true` which **fails open** for null (returns false). The spec §5.2 says "Strict `=== true` comparison (cached JSON null leaks past `!== undefined` else-branch)" — this prose is confusing. The actual semantics are correct (sink stays on for null/missing/malformed), so "leaks past `!== undefined` else-branch" mis-describes outcome. Recommend: rewrite §5.2 to say "fail-open: only `=== true` kills; null/undefined/non-bool leaves sink active".

Evidence: `src/services/analytics/sinkKillswitch.ts:18-25`, spec line 312.

### 5. [MEDIUM] §4.1 conflates two state items in describing `loggedExposures` clearing

Spec §4.1 line 211: *"`loggedExposures: Set<string>` … per-session de-dup; **NOT** cleared by `resetGrowthBook` for `loggedExposures` is **listed as cleared** (`:1004`)"*. Source `growthbook.ts:1004` indeed clears `loggedExposures.clear()` inside `resetGrowthBook`. The spec sentence is grammatically broken ("NOT cleared … is listed as cleared") and reads like the author started one assertion and pasted another. §12 ¶6 then re-explains it. Reader is left guessing. The §11 checklist is correct: "loggedExposures/pendingExposures **do not** [survive]". §4.1 sentence should be rewritten to match.

Evidence: `growthbook.ts:1004`, spec lines 211, 832.

## Additional findings (lower severity)

- **[LOW]** §3 lists `getAllGrowthBookFeatures` and `getGrowthBookConfigOverrides` as exports. Not verified in this review — possible stale signatures (typical of leaked/refactored modules). Phase-9 deep dive needed.
- **[LOW]** §6.4 cites Logger scope `'com.anthropic.claude_code.events'` for **both** 1P and 3P customer telemetry, then claims "the 1P provider is NOT registered globally". Two distinct LoggerProviders sharing one scope name is correct but counter-intuitive; observability folks reading the spec will assume a single global logger. Add a sentence calling this out.
- **[LOW]** §6.6 says `additional_metadata` is "Base64-JSON only when non-empty after `_PROTO_*` strip" — should specify the encoding (base64 of UTF-8 JSON) and whether it's `Buffer.from(JSON.stringify(...)).toString('base64')` or proto bytes.
- **[LOW]** §11 checklist item "OTLP exporters lazy-imported per protocol (gRPC ~700 KB elided unless `protocol==='grpc'`)" — the size figure is unsourced (probably from package metadata). Mark as approximate or drop.
- **[LOW]** §6.1 "Init timeout 5000 ms growthbook.ts:555" — verified at line 556 (`init({timeout: 5000})`), spec is off-by-one. Trivial, but signals stale line citations elsewhere.
- **[NIT]** §1 typo: "for `loggedExposures` is **listed as cleared**" (broken sentence, see finding 5).
- **[NIT]** §2.4 mentions generated proto bindings under `src/types/generated/events_mono/` as "unverified" — Phase 9 should resolve this rather than punt.
- **[NIT]** §6.7 cites field counts of `EnvContext` (35), `ProcessMetrics` (9), `EventMetadata` (23) without listing them. The reimplementation checklist depends on these being exact; without the verbatim list a reimplementer can't satisfy the contract. Inline at least field names.

## Verdict

**ACCEPT WITH MINOR REVISIONS.** Spec is technically thorough (835 lines covering 13 in-scope files plus 9 utils). Core claims verified: Datadog endpoint, 1P endpoint, growthbook intervals (6h non-ANT / 20min ANT), resolution priority (env → config → in-memory → disk → default), trust-gate logic, killswitch JSON shape, `_PROTO_*` strip semantics, sample-rate validation, periodic-refresh registration. The Phase-9.4 narrowing to 7 telemetry flags is correct; pointer to spec 00 §8.1 is valid. The cross-spec adjacency to 01/02/03/06/09/22/27 in INDEX.md is consistent.

The defects are arithmetic/wording, not architectural. Fix the 41 vs 43 count, rewrite the broken sentence in §4.1, clean up the §5.2 fail-open description, and either drop or substantiate the "817 tengu_* literals" claim.

## Cross-spec impact

- **Spec 03**: §6.8 tool-ordering invariant on `claude_code_global_system_caching` is delegated correctly; spec 03 must own the actual invariant statement.
- **Spec 09**: §12 ¶7 flags that `tengu_internal_record_permission_context` payload includes `jsonStringify(toolPermissionContext)` cast as `_NOT_CODE_OR_FILEPATHS` — **this cast is a lie** (permission context contains paths). Spec 09 should own the redaction policy or accept the ANT-only carve-out explicitly.
- **Spec 22**: §1 disclaims `firstPartyEventLoggingExporter` "inherits axios patterns, not the api client" — verify spec 22 owns axios setup.
- **Spec 06**: ingests `EventMetadata.model`/`betas` per §1; if `EventMetadata` field count drifts, both specs must update.
- **Spec 32 (kairos)**: `KAIROS` build-time flag's behavior owned here for `kairosActive` tag, but emit-site logic owned by spec 32. Co-ownership clear.

## Hardest-to-verify claim

**§6.6: `to1PEventFormat` mapping of `_PROTO_skill_name`/`_PROTO_plugin_name`/`_PROTO_marketplace_name` to dedicated proto fields, with all other `_PROTO_*` keys defensively stripped before `additional_metadata` Base64-JSON encoding.** This requires (a) full read of `firstPartyEventLoggingExporter.ts:635-762`, (b) confirmation of the BQ proto schema (declared missing in §12 ¶1), and (c) end-to-end test that an unrecognized `_PROTO_xyz` key never reaches BQ as a JSON blob field. The proto bindings under `src/types/generated/events_mono/` are not verifiable from the leaked tree, so this is a structural assertion that only a runtime trace (or a cooperating BQ schema dump) could confirm. Recommend Phase-9 dispatch a sub-agent specifically to grep for the proto file or definitively log its absence.
