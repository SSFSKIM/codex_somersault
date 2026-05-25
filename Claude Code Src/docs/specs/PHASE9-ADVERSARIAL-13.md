# Phase 9.5b Adversarial Review — Spec 13 (WebFetch & WebSearch)

Reviewer role: Skeptic. Spec under review: `docs/specs/13-tool-web.md`.
Sources cross-checked: `src/tools/WebFetchTool/{WebFetchTool.ts,utils.ts,preapproved.ts}` and `src/tools/WebSearchTool/WebSearchTool.ts`.

## Severity Counts

- CRITICAL: 0
- HIGH: 1
- MEDIUM: 4
- LOW: 5
- NIT: 3
- BUGS-IN-SOURCE candidates (Phase 9.7): 3

Total findings: 16.

## Top 5 Findings

### 1. [HIGH] WebSearch validateInput dead branch — `min(2)` makes "missing query" path unreachable

Spec §3.2 / §3.3 / §6.3 cite both `query: z.string().min(2)` (input schema) and the validator's `Error: Missing query` for empty `query.length`. These are mutually exclusive in practice: Zod rejects strings of length 0 or 1 at parse time with a different message before `validateInput` ever runs. Spec faithfully copies what's in source (`WebSearchTool.ts:27` and `:237-242`), but it never flags the inconsistency. The `errorCode: 1, "Error: Missing query"` branch is dead code in source. Spec should either flag this as BUGS-IN-SOURCE candidate or note the schema preempts validation.

Verdict: spec accurately mirrors source, but misses an adversarial inconsistency the reviewer is supposed to surface. **BUGS-IN-SOURCE candidate for Phase 9.7.**

### 2. [MEDIUM] DomainCheckFailedError is a `logError` victim, contradicting spec §9

Spec §9 claims `DomainBlockedError` and `DomainCheckFailedError` are "re-thrown without `logError`". Source `utils.ts:199-202` actually calls `logError(e)` *inside* `checkDomainBlocklist` on the catch path before constructing the `check_failed` envelope — the resulting `DomainCheckFailedError` is then thrown out of `getURLMarkdownContent` un-`logError`'d, but the original underlying axios error has already been logged. Spec phrasing "re-thrown without `logError`" is technically true for the wrapper but materially misleading: the same domain failure DOES emit a logError per attempt (via the inner catch). This matters because §10 inventories `logError` surfaces and omits this one. Should be amended.

### 3. [MEDIUM] Cache key claim "pre-upgrade pre-redirect" only half-true

Spec §4.1 / §5.2 / §11.8 say `URL_CACHE` is keyed on the original (pre-upgrade, pre-redirect) URL. Verified at `utils.ts:356,480`: yes, `URL_CACHE.get(url)` and `URL_CACHE.set(url, ...)` use the original `url` arg. **However** there is no separate write under the upgraded or final-redirect URL, so a request that arrives via a hostname-equivalent same-origin redirect chain will miss cache on subsequent direct hits to the redirect target. The spec mentions this only for http→https; the same applies to www-stripping redirects. Worth calling out for fidelity.

### 4. [MEDIUM] WebSearch "passthrough" semantics under-described re: 09 cross-spec

Spec §3.2 says `behavior:'passthrough'` and defers full semantics to 09. But the practical effect — that the network call lives entirely server-side inside Anthropic's API and bypasses local rules entirely — is the security-critical claim. Spec lists this as a footnote in §5.8 and §11.24 but never says: **WebSearch domain allowlist/blocklist is enforced server-side via the `web_search_20250305` tool schema; client never sees URLs**. A reader could believe `allowed_domains`/`blocked_domains` go through the same machinery as WebFetch's `domain:<host>` rules. They don't. Should be explicit.

### 5. [MEDIUM] Spec §9 "binary content + decode" omits HTML-Turndown invocation on binary MIME

Spec §9 says "PDFs are persisted to disk **and** passed through utf-8 decode → Haiku". Verified at `utils.ts:442-466`. But notice line 456: the post-binary decode only routes to Turndown if `contentType.includes('text/html')`. So a PDF that is misidentified upstream as `text/html` (rare but possible via `Content-Type: text/html; charset=binary`-style server quirks) would be sent through Turndown after `persistBinaryContent` already saved the raw bytes. Edge case; spec should at least note that binary detection and HTML decode are independent boolean conditions, not mutually exclusive.

## Cross-Spec Impact

- **Spec 09 (permissions)**: Spec 13 correctly defers `passthrough` semantics. But 09 must clarify that `passthrough` for WebSearch means client-side rule bypass — only the server-tool schema enforces domain filtering.
- **Spec 22 (server tools / streaming)**: Spec 13 §12 already flags the `web_fetch_requests` counter at `utils/messages.ts:367` as out-of-scope. Confirmed: `EMPTY_USAGE` carries this counter but no client code increments it; this implies an unused or future server-side WebFetch beta tool. Phase 9.7 / 22 should investigate.
- **Spec 06 (cost-tracker)**: Spec 13 cites `cost-tracker.ts:271` for `web_search_requests` consumption — confirmed, no impact.
- **Spec 26 (analytics)**: ANT-only `tengu_web_fetch_host` event correctly flagged; routing owned by 26.
- **Spec 02 (settings)**: `skipWebFetchPreflight` deferred correctly.
- **Spec 08 (registry)**: Confirmed both tools are statically registered, no `feature(...)` gate. Spec is accurate.

## BUGS-IN-SOURCE Candidates (Phase 9.7)

1. **WebSearch `Error: Missing query` is unreachable** — Zod `min(2)` on the input schema preempts the validator. Either drop the validator branch or reduce schema to `min(0)`. (`WebSearchTool.ts:27` vs `:237-242`)
2. **`logError` double-emit on domain check failure** — `checkDomainBlocklist` catches and logs; the wrapper then throws `DomainCheckFailedError`. The user-facing error is "expected" but the underlying axios error gets logged once per attempt, polluting telemetry. (`utils.ts:199-202` + `utils.ts:407-413`)
3. **`URL_CACHE` clamp `Math.max(1, contentBytes)` masks empty-body cache misses** — the comment cites lru-cache's positive-integer requirement, but a 1-byte cost on a 0-byte payload makes the cache hold 50M empty entries before evicting. Likely benign (TTL caps it) but the `clamp` math is asymmetric vs eviction accounting. (`utils.ts:480`)

## Hardest-to-Verify Claim

Spec §3.2's WebSearch `isEnabled()` provider matrix: "**vertex** ✅ iff model contains `claude-opus-4`/`claude-sonnet-4`/`claude-haiku-4`; **foundry** ✅ always". Verified at `WebSearchTool.ts:168-193` — the substring check is correct as documented. **But** the claim that "foundry only ships models that already support Web Search" (source comment, line 187) is unverifiable from this leaked tree — no foundry-side model registry exists in `src/`. If foundry ever ships a model without Web Search support, `isEnabled()` returns true and the server-tool call will fail with an opaque error. Spec dutifully reproduces the source comment but cannot assert ground truth. Flagged at §12 (Open Questions).

## Verdict

**ACCEPT WITH MINOR REVISIONS.**

Spec 13 is unusually thorough — verbatim assets, constants table, full preapproved list, redirect template, formatter trailer all match source byte-for-byte where I sampled. The reimplementation checklist (§11) is exhaustive and correct. Source coverage claim "✅ All 319 lines / All 531 lines / All 436 lines" appears credible based on the breadth of cited line numbers.

Required fixes before promotion:
1. Note that `Error: Missing query` validator is dead code (or move to BUGS-IN-SOURCE).
2. Correct §9/§10 `logError` claim re: `DomainCheckFailedError` underlying axios error.
3. Add explicit cross-spec callout that WebSearch domain filtering is server-enforced.
4. Note that binary-content detection and HTML-decode branches are independent.

Optional improvements:
5. Document that same-origin redirect cache key behavior extends beyond http→https.
6. Cross-reference Spec 22 explicitly for the orphan `web_fetch_requests` counter.

No CRITICAL or fabricated content. Numerical constants in §6.13 all verify against source. Verbatim §6.5–6.9 reproduce source comments and string literals exactly. Phase 9.5b confidence: **high**.
