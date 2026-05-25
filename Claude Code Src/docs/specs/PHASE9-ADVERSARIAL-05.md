# Phase 9.5 Adversarial Review — Spec 05 (Context Assembly)

Reviewer: Opus side, 18-spec parallel review.
Subject: `docs/specs/05-context-assembly.md`
Source verified: `src/context.ts` (189 lines, full read), `src/utils/api.ts:425-563`, `src/utils/queryContext.ts`, `src/commands/clear/caches.ts`, `src/services/compact/postCompactCleanup.ts`, `src/utils/gitSettings.ts`, `src/constants/common.ts`, `src/utils/envUtils.ts`, plus targeted greps in `src/QueryEngine.ts`, `src/query.ts`, `src/tools/AgentTool/{forkSubagent,runAgent}.ts`, `src/Tool.ts`, `src/screens/REPL.tsx`, `src/commands/compact/compact.ts`.

## Severity counts

| Severity | Count |
|---|---|
| Critical | 0 |
| High     | 1 |
| Medium   | 2 |
| Low      | 4 |
| Nit      | 3 |

## Findings

### H1 — `appendSystemContext` "main turn pipeline uses a richer per-key block" claim is wrong

Spec §3 (`05-context-assembly.md:112`):
> `appendSystemContext` (`api.ts:437-447`) appends a single newline-joined `key: value` block to the system prompt array (used by SDK fallback path; the main turn pipeline uses a richer per-key block — see §6.2).

Source: `src/query.ts:450`:

```ts
appendSystemContext(systemPrompt, systemContext),
```

`query.ts` IS the main turn pipeline. It calls `appendSystemContext` directly with the same flat `key: value` join. There is no "richer per-key block" alternative path for `systemContext` — only `prependUserContext` uses the `# key\nvalue` form, and that is for `userContext`, not `systemContext`. Section §6.2's "see §6.2" reference points at the `prependUserContext` block, conflating the two maps.

Impact: misleads any reimplementer into looking for a non-existent main-pipeline systemContext serializer. The bit-exact claim in §11 reimplementation checklist is unaffected (single block, `\n`-joined), but the prose explanation is incorrect.

Fix: delete the parenthetical "(used by SDK fallback path; the main turn pipeline uses a richer per-key block — see §6.2)" or rewrite to "used by both the main pipeline (`query.ts:450`) and the SDK fallback (`buildSideQuestionFallbackParams`); `prependUserContext` is the per-key variant for `userContext` only."

### M1 — REPL.tsx line citations include three non-existent sites

Spec §2 source map and §5.6 cite `src/screens/REPL.tsx:2535, 2543, 2772, 2788, 4942` for the `renderedSystemPrompt` snapshot. Direct grep returns only `:2543` and `:2788` for the assignment. 2535 and 2772 may be adjacent context (the systemPrompt construction lines) but the spec does not say so; 4942 was not findable in the snapshot search. Either stale line numbers or false enumerated callsites.

Impact: Phase-10 reimplementation auditors will follow these citations and find nothing.

Fix: Re-derive citations. Confirmed real: `2543`, `2788`. Verify or drop `2535`, `2772`, `4942`.

### M2 — `setCachedClaudeMdContent(claudeMd || null)` rationale uses "intentional `||` not `??`"

Spec §4 and §11 emphasize: "intentional `||` not `??`: empty string is normalized to `null`."

Source `context.ts:176`:

```ts
setCachedClaudeMdContent(claudeMd || null)
```

But at that point `claudeMd` is `string | null` (assigned from `getClaudeMds(...)` or `null`). `getClaudeMds([])` returns `''` (empty string). The spec correctly identifies that `||` collapses both `''` and `null` to `null`. **This is correct**, but flagging because the spec then says (§9) "Both paths still call `setCachedClaudeMdContent(null)` because of the `||` normalization" — which is true. No bug, but the rationale comment in source (lines 173–175) describes the cycle break, not the `||` choice; the `||` is load-bearing per the spec's claim. Worth a one-line code comment in the source (out of scope for review) but the spec's claim is fine.

Drop to Low.

### L1 — `getSessionStartDate` open question (§12 #1) under-explored

Spec §12 #1 admits source-only review cannot confirm whether `getSessionStartDate` is read inside `getSystemPrompt` (spec 38 territory). I confirmed: `clearSessionCaches` at `caches.ts:55` clears `getSessionStartDate.cache`, and `constants/common.ts:17-23` says simple mode (`--bare`) calls `getSystemPrompt` per-request and uses `getSessionStartDate` for cache stability. Cross-spec drift risk with spec 38; the comment in `common.ts:17-23` is an authoritative claim about another subsystem's behavior. Spec 05 should explicitly defer this — currently it does, but the §6.5 cache table lists `getSessionStartDate.cache` as cleared by `/clear` without flagging that the producer is owned elsewhere. Minor.

### L2 — `getMemoryFiles` memoize key claim incomplete

Spec §4: "memoize key is `forceIncludeExternal: boolean` (default `false`)". `lodash-es/memoize` with no resolver function uses **only the first argument coerced to string** as the key. A boolean first arg yields keys `"false"` / `"true"`, which works. But the spec claim "production path hits a separate slot from the approval-check path" depends on this default-resolver behavior. Worth a one-line note: `getMemoryFiles` does not pass a custom resolver, so default-arg coercion handles slot separation. Low impact.

### L3 — `gitExe()` "memoized on first call" claim unverified

§6.3 footnote claims `gitExe()` is memoized. Not verified in this review (would need `src/utils/git.ts:212-216`). The functional claim is irrelevant to context assembly — three calls in the same `Promise.all` would resolve `gitExe()` either way — so even if false the spec is still bit-correct. Low.

### L4 — `getIsGit()` "memoized" claim under-specified

§5.1 says "`getIsGit() // findGitRoot(getCwd()) !== null, memoized`". The memoization scope (per-cwd? once-forever?) is not specified. Affects reasoning about EnterWorktreeTool / cwd-changing flows. Cross-spec to 11/14. Low.

### N1 — Spec calls `--no-optional-locks` "load-bearing" but only documents *why* in §5.1 invariants

Both `status` and `log` use the flag; the rationale (avoiding race against user's concurrent git) is correct. Nit: the spec says "NOT to `config user.name`" three times. Once would suffice.

### N2 — Spec §3 return-shape claim for `getSystemContext`

> Possible keys (and only these): `gitStatus`, `cacheBreaker`. Both are conditional; the map may be `{}`.

Verified. Bit-exact.

### N3 — Inline-doc consistency

Spec §5.3 quotes a "synchronous after the await" comment. The source comment at `context.ts:168-169` actually says "Await the async I/O (readFile/readdir directory walk) so the event loop yields naturally at the first fs.readFile." Different framing but no semantic conflict.

## Verdict

**APPROVE WITH FIXES.** Spec 05 is unusually thorough — the cache-invalidation matrix (§5.5), verbatim assets (§6.x), and reimplementation checklist (§11) are bit-correct against source. The one substantive bug is H1 (false claim about main-pipeline serialization). The rest are line-citation hygiene and minor rationale drift. Phase-10 should not block on this spec; H1 and M1 are 5-minute fixes.

## Cross-spec impact

- **Spec 03 (query engine)**: Should claim `query.ts:450` `appendSystemContext` callsite explicitly so spec 05's H1 is consistent with whatever 03 says about the serialization step. Verify spec 03 doesn't repeat the "richer per-key block" claim.
- **Spec 04 (turn pipeline)**: Owns `prependUserContext` callsite at `query.ts:660`. Verify 04 records the synthetic user-message at index 0 and doesn't double-document the verbatim body (already in spec 05 §6.2).
- **Spec 07 (compaction)**: Cache-invalidation rules between 05's table and 07's cleanup must agree on main-thread vs subagent semantics (`postCompactCleanup.ts:36-39` divergence). Source confirms 05's account; 07 should match.
- **Spec 29 (memory services)**: Owns `getMemoryFiles`, `getClaudeMds`, `filterInjectedMemoryFiles`. Verify 29's memoize-key documentation matches L2 above.
- **Spec 38 (output styles)**: Owns `getSystemPrompt` + `getSessionStartDate` interaction (L1 / spec 05 open question §12 #1). Spec 38 must claim this.
- **Spec 40 (persistent memory)**: Owns `MEMORY.md` / TeamMem / AutoMem on-disk schema; spec 05 correctly defers per-file content rules.
- **Spec 14 (AgentTool/TeamCreate)**: Fork-subagent override path at `runAgent.ts:381-382, 392-409` — spec 05's §5.6 is consistent with what I verified, but spec 14 owns the full override semantics (claudeMd/gitStatus stripping for non-fork agents).

## Hardest-to-verify claim

> "Date is captured **once** at first cache-miss and never refreshed. After-midnight handling is delegated: `getDateChangeAttachments` (`utils/attachments.ts:1415-1444`) emits a tail `date_change` attachment instead of busting the prefix cache. The trade-off is acknowledged in `attachments.ts:1408-1412`: stale prefix-date wins over re-creating ~920K tokens of cache_creation per midnight crossing per overnight session."

Three reasons it's hard to verify from this spec alone:
1. The numerical claim "~920K tokens of cache_creation" is not citable from `context.ts` itself — it requires reading `attachments.ts:1408-1412` *and* reasoning about token sizes downstream of the entire prefix (system prompt + tools + claudeMd + gitStatus + memory files), which is a compaction/cache-pricing argument owned by spec 06/07.
2. The "stale wins" trade-off is a runtime behavioral assertion: it would manifest only on overnight sessions and produce no in-process telemetry distinguishing "stale date" from "fresh date" because `getLocalISODate()` is read once per cache slot.
3. The midnight-crossing edge case interacts with `getSessionStartDate` (§12 #1), `setLastEmittedDate` (caches.ts:9, 69), and `getDateChangeAttachments` — three independent sources of truth for "today's date" that must agree.

Verification would require either: (a) running an overnight session with telemetry, or (b) reading `attachments.ts` + spec 06/07 + spec 38 in concert. Source-only review can confirm the *mechanism* (memoize captures once, tail attachment exists) but not the *numerical trade-off*.
