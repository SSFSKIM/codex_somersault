# Phase 9.6c Fixes — Spec 13 (WebFetch & WebSearch)

Source of fixes: `docs/specs/PHASE9-ADVERSARIAL-13.md` (Phase 9.5b reviewer findings).
Spec edited in-place: `docs/specs/13-tool-web.md`.
Verification: source files re-read at `src/tools/WebFetchTool/utils.ts` (lines 170-203, 405-482) and `src/tools/WebSearchTool/WebSearchTool.ts` (lines 25-37, 230-298) before each edit.

## Fixes Applied

### HIGH — `Error: Missing query` is dead code (§3.3)
- **Finding**: Zod `query: z.string().min(2)` (`WebSearchTool.ts:27`) preempts `!query.length` validator (`WebSearchTool.ts:237-242`). Empty queries fail at parse time with a Zod message, never reaching `validateInput`. Spec previously documented both as if both fired.
- **Fix**: Added a "Note (BUGS-IN-SOURCE candidate)" block in §3.3 explicitly flagging the unreachable branch and pointing to §X. Cross-listed in §X.

### MEDIUM — §9/§10 wrongly claimed `DomainCheckFailedError` skips `logError`
- **Finding**: `utils.ts:200` calls `logError(e)` inside `checkDomainBlocklist`'s catch *before* returning the `check_failed` envelope. The outer `getURLMarkdownContent` catch (`utils.ts:407-413`) re-throws the `DomainCheckFailedError` without an *additional* log, but the underlying axios error has already been logged once.
- **Fix**: §9 "Domain blocklist failures" bullet rewritten to clarify wrapper-vs-inner-catch behavior, with explicit `utils.ts:200` citation. §10 "WebFetch logError surface" amended to include the preflight inner catch. Cross-listed in §X.

### MEDIUM — `URL_CACHE` "pre-upgrade" caveat too narrow
- **Finding**: Spec §9 mentioned only `http://` → `https://` upgrade. The same cache-miss-on-redirect-target behavior also applies to `isPermittedRedirect`'s www-strip equivalence (`utils.ts:236-239`). No second `URL_CACHE.set` under the upgraded or final-redirect URL exists in `getURLMarkdownContent` (verified at `utils.ts:480`).
- **Fix**: §9 "HTTP→HTTPS upgrade" bullet extended to call out same-origin (incl. www-strip) redirect targets explicitly.

### MEDIUM — WebSearch `allowed_domains`/`blocked_domains` are server-enforced
- **Finding**: Spec §3.2 / §5.8 / §11.24 mention `behavior:'passthrough'` but never say the domain filtering is enforced by the `web_search_20250305` server-tool schema (§6.15), not by client-side rule machinery. A reader could conflate these with WebFetch's `domain:<host>` rules.
- **Fix**: Added a "Cross-spec note (server enforcement)" block in §3.3 stating the filters are passed verbatim into the server-tool schema and the client never sees individual result URLs at decision time.

### MEDIUM — Binary-content and HTML-decode branches are independent
- **Finding**: Spec §9 "Binary content + decode" implied mutual exclusivity. Source `utils.ts:442` (`isBinaryContentType`) and `utils.ts:456` (`contentType.includes('text/html')`) are independent `if` blocks; a response matching both predicates is persisted *and* sent through Turndown.
- **Fix**: §9 bullet expanded to state the conditions are independent boolean checks and to give the misidentified-PDF example.

### Structural — §X BUGS-IN-SOURCE cross-link
- Added a new §X section at the end of the spec listing the three Phase 9.7 candidates surfaced by adversarial review (dead `Missing query` branch, double-attribution `logError` on preflight failure, and the `Math.max(1, contentBytes)` cache-eviction asymmetry). Each item links back to the §3.3 / §9 / §10 callouts and to `docs/specs/BUGS-IN-SOURCE.md`.

## Not Applied

- **Adversarial finding 6** (cross-reference Spec 22 explicitly for orphan `web_fetch_requests` counter): already covered in §12 ("Server-tool `web_fetch_requests` accounting") with a 22 hand-off. No further edit required.
- **Hardest-to-verify claim** (foundry provider matrix): already deferred to §12 Open Questions.

## Verification Summary

- HIGH: 1 fix (§3.3 dead-branch note + §X entry).
- MEDIUM: 4 fixes (§3.3 server-enforcement note, §9 logError correction, §9 redirect-cache caveat, §9 binary/HTML independence).
- Structural: §X added.
- Source re-reads: 3 (`utils.ts:170-203`, `utils.ts:405-482`, `WebSearchTool.ts:230-298`).
- No CRITICAL findings; spec already byte-for-byte faithful on verbatim assets.
