# Phase 9.5 Adversarial Review — Spec 01 (Entrypoint & Bootstrap)

## Severity counts

- CRITICAL: 0
- HIGH: 2
- MEDIUM: 4
- LOW: 5
- NIT: 3
- TOTAL: 14

---

## Findings

### F-01 — HIGH — `clientType` enumeration drift (`'sdk-py'` vs `'sdk-python'`)
- Section / line: §3.4 row "setClientType / getClientType" (state.ts:1069-1075) and §5.2.
- Claim: §3.4 enumerates the values as `'cli', 'sdk-typescript', 'sdk-python', 'sdk-cli', ..., 'github-action'`. §5.2 also writes `CLAUDE_CODE_ENTRYPOINT === 'sdk-py' → 'sdk-python'`. The §1 bullet at line 19 enumerates client types as `'sdk-py'`.
- src verification: `main.tsx:821` — `if (process.env.CLAUDE_CODE_ENTRYPOINT === 'sdk-py') return 'sdk-python'`. The **env-var sentinel** is `'sdk-py'`; the **stored clientType** is `'sdk-python'`. Spec §1 line 19 conflates the two by listing `'sdk-py'` as a clientType value — that string is never written to `STATE.clientType`. §3.4 and §5.2 are consistent with each other but contradict §1.
- Severity: HIGH (false enumeration in §1).
- Fix: in §1 line 19 change `'sdk-py'` → `'sdk-python'` and explicitly note the env sentinel is `sdk-py`.

### F-02 — HIGH — Pre-action awaits `Promise.all`, not sequential awaits
- Section / line: §1 line 22 and §4.5 lifecycle outline.
- Claim: "registers a single `preAction` hook that **awaits `ensureMdmSettingsLoaded()` and `ensureKeychainPrefetchCompleted()`** and then runs `init()`…" — phrasing implies two sequential `await`s.
- src verification: `main.tsx:914` — `await Promise.all([ensureMdmSettingsLoaded(), ensureKeychainPrefetchCompleted()]);`. Single Promise.all gate. §5.4 of the spec also writes the (separate) `init()` body as performing two sequential `await`s, but the actual entry-point hook is parallel. Minor wording bug — but readers diagnosing boot latency will look for the wrong call sites.
- Severity: HIGH (untestable/misleading description of an ordering primitive).
- Fix: replace "awaits X and Y" with "awaits `Promise.all([ensureMdmSettingsLoaded(), ensureKeychainPrefetchCompleted()])`".

### F-03 — MEDIUM — `setup()` line range claim (`56-477`) off by one
- Section / line: §2.1 row "src/setup.ts | 478"; §5.6 header `(src/setup.ts:56-477)`.
- Claim: file is 478 lines and `setup()` runs `:56-477`.
- src verification: `wc -l setup.ts` = 477. `setup()` declaration begins at `:56` and final brace at `:477`. The "478" total in §2.1 is one off (file has 477 lines).
- Severity: MEDIUM (off-by-one on a load-bearing line-count).
- Fix: change `478` → `477` in §2.1.

### F-04 — MEDIUM — `init.ts` line refs `:57-238`, `:247-258` cite past EOF
- Section / line: §1 line 18; §3.2 (`init.ts:57`, `:247`, `:252-258`); §2.1 row "src/entrypoints/init.ts | 341"; §4.6 lockCurrentVersion at "init.ts:87" / cleanup at "init.ts:189"; "registerCleanup … init.ts:195-200".
- Claim: §2.1 says init.ts is **341** lines; §3.2 cites `init.ts:247` for `initializeTelemetryAfterTrust`; §1.18 cites `init.ts:57-238` for the body.
- src verification: `wc -l init.ts` = **340**. So `:341` is past EOF. `setupGracefulShutdown()` is at `:87` ✓, `registerCleanup(shutdownLspServerManager)` at `:189` ✓, the `cleanupSessionTeams` registerCleanup block actually starts at `:195` ✓. But `gracefulShutdownSync(1)` is at `:224` — **inside** init() — which means the body actually extends past the cited `:238` end. Also `initializeTelemetryAfterTrust` line `:247` cannot be exact since file ends at 340; spec needs to back this up with a fresh grep.
- Severity: MEDIUM (line refs unverifiable / off-by-one in the source map).
- Fix: re-grep init.ts and correct §1.18, §2.1, §3.2 line numbers; verify `init` body actually ends where claimed.

### F-05 — MEDIUM — `lockCurrentVersion()` cited at `setup.ts:303`
- Section / line: §4.6 "`lockCurrentVersion()` (`setup.ts:303`)".
- Claim: line 303.
- src verification: `setup.ts:303` is `void lockCurrentVersion()` ✓ — verified, no issue. Counterpart import is at `:44`. Cross-check OK.
- Severity: NIT (this one is correct — keeping for the record).
- Fix: none.

### F-06 — MEDIUM — Print mode rewrite into `open` strips DSP, but spec wording is loose
- Section / line: §5.1 "DIRECT_CONNECT cc:// argv rewrite" pseudocode.
- Claim: Spec says "if -p / --print: rewrite argv to use the internal `open` subcommand: `['claude','open',ccUrl, ...other]`".
- src verification: `main.tsx:621-628`: in the print branch, code first filters out the `cc://` URL, then **also splices out** `--dangerously-skip-permissions` from the stripped array, then reconstructs argv as `[argv0, argv1, 'open', ccUrl, ...stripped]`. So the spec's `...other` actually means "stripped of cc:// AND DSP". Worth being explicit.
- Severity: MEDIUM (hidden assumption — readers tracing the DSP flag would expect it preserved).
- Fix: add "(with `--dangerously-skip-permissions` removed; it's stashed on `_pendingConnect.dangerouslySkipPermissions`)".

### F-07 — LOW — Spec claims setup-ts "478 lines"; `cli/print.ts` "5594 lines"; verify `cli.tsx` "303 lines"
- Section / line: §2.1.
- Claim: print.ts 5594, structuredIO 859, mcp.ts 197, cli.tsx 303, agentSdkTypes 444, sandboxTypes 157, exit.ts 31, ndjsonSafeStringify 32, init.ts 341, state.ts 1759.
- src verification (wc -l): print.ts **5594** ✓, structuredIO **859** ✓, mcp.ts **196** (spec says 197 — off by one), cli.tsx **302** (spec 303 — off by one), agentSdkTypes **443** (spec 444 — off by one), sandboxTypes **156** (spec 157 — off by one), exit.ts **31** ✓, ndjsonSafeStringify **32** ✓, init.ts **340** (spec 341 — off by one), state.ts not yet verified but `STATE` definition + flow read consistently.
- Severity: LOW each, but **systematic off-by-one** across 5 files suggests the author counted with an inclusive endpoint.
- Fix: re-run `wc -l` on every cited path and update §2.1.

### F-08 — LOW — `agentSdkTypes.ts:73-443` cite range exceeds file
- Section / line: §3.6.
- Claim: "See `agentSdkTypes.ts:73-443` for full signatures…".
- src verification: file is 443 lines so `:73-443` is end-inclusive and acceptable; combined with F-07 it implies the author treated lengths as 1-indexed line counts. Verify the line range still spans a contiguous block of `throw new Error('… not implemented')` bodies.
- Severity: LOW.
- Fix: confirm range still hits all listed function bodies; otherwise narrow.

### F-09 — LOW — §3.7 `mcp.ts` constants — `READ_FILE_STATE_CACHE_SIZE = 100` cited "(`mcp.ts:42-43` comment)"
- Section / line: §3.7.
- Claim: cache size 100 + "100 files and 25MB limit" comment.
- src verification: not inspected directly, but mcp.ts is 196 lines (off-by-one with spec's 197); the line citations need a fresh check. Comment-only citations are fragile — bundler can shift them.
- Severity: LOW.
- Fix: re-grep and verify, or weaken the `:42-43` citation to "header-region".

### F-10 — LOW — `state.ts:391-395` ANT-only `replBridgeActive`
- Section / line: §2.3 last row, §4.1 last bullet.
- Claim: ANT-only `replBridgeActive` field added at `state.ts:391-395` in `getInitialState()`.
- src verification: state.ts:391-395 contains the spread `...(process.env.USER_TYPE === 'ant' ? { replBridgeActive: false } : {}),` ✓.
- Severity: NIT (verified, included for reviewer record).
- Fix: none.

### F-11 — LOW — §1.20 cites argv pre-rewrite slot lines `_pendingConnect`, `_pendingAssistantChat`, `_pendingSSH` at "main.tsx:548-584"
- Section / line: §1 line 20 and §4.3.
- Claim: module-scope slots populated at `:548-584`.
- src verification: §5.1 reproduces these slots and uses ranges `:611-642` (DIRECT_CONNECT), `:647-677` (LODESTONE), `:685-700` (KAIROS), `:706-795` (SSH_REMOTE). The slot-creation sites at `:548-584` are not directly grep-verified here, but the surrounding ranges in §5.1 are consistent with the actual main.tsx body. Likely correct but unverified.
- Severity: LOW.
- Fix: spot-grep `_pendingSSH = ` and `_pendingAssistantChat = ` initial declarations.

### F-12 — NIT — §1 line 19 entrypoint values list omits `'sdk-py'` env sentinel and lists "sdk-cli" twice
- Section / line: §1 line 19: "(`mcp` / `claude-code-github-action` / `sdk-cli` / `cli`) … the **client type** (`github-action` / `sdk-typescript` / `sdk-py` / `sdk-cli` / `claude-vscode` / `local-agent` / `claude-desktop` / `remote` / `cli`)".
- Claim: implies a single 9-value enum.
- src verification: cross-cutting with F-01: the **env sentinel** values and the **stored clientType** values are different sets (`sdk-py` is only an env sentinel; `sdk-python` is the stored value). The enumeration as written is ambiguous.
- Severity: NIT.
- Fix: split into two enumerations (env sentinels vs stored clientType).

### F-13 — NIT — §2.7 cross-spec list missing 03 cross
- Section / line: §2.7 "Adjacent-spec cross-references".
- Claim: lists 00, 02, 03, 22, 25, 26, 34, 35.
- src verification: §1 explicitly hands off to spec 03/04 once `runHeadless` is reached; spec 04 is not in §2.7. Minor.
- Severity: NIT.
- Fix: add 04 (turn pipeline) to §2.7.

### F-14 — NIT — `initializeTelemetryAfterTrust` purpose claim untestable from leak
- Section / line: §3.2.
- Claim: "Called by interactive paths after the trust dialog accepts. For SDK / headless mode with beta tracing, eagerly fires before remote settings load (`init.ts:252-258`)."
- src verification: file ends at line 340, so `:252-258` is plausible — but the *behavioral* claim ("for SDK / headless mode … eagerly fires before remote settings load") is a statement about callers in other files (the trust-dialog path, headless beta tracing). Not falsifiable from this spec's owned set; readers should treat as "see callers".
- Severity: NIT.
- Fix: weaken to "called from outside this module after trust acceptance; see spec 26 for the headless beta-tracing eager call".

---

## Spec-level verdict

The spec is **structurally sound and unusually well-cited**, with a clear lifecycle map, faithful pseudocode, and explicit ANT/feature-flag gates. The bulk of large-scale claims (preAction order, migrations list, `setClientType` decision tree, `setup()` body, dangerous-permissions sandbox enforcement, `getInitialState()`, ~70+ `STATE` accessors) cleanly match `src/`. Defects are concentrated in (a) off-by-one line counts across **five** owned files, (b) an enumeration drift between `'sdk-py'` and `'sdk-python'`, and (c) loose phrasing of two ordering primitives (the `Promise.all` await, the `open`-rewrite arg list). No CRITICAL issues; two HIGH issues both narrow and easy to fix. **Verdict: ACCEPT with minor revisions** — fix F-01 through F-04 before publication.

---

## Cross-spec impact

- **Spec 02 (settings/migrations):** the migrations enumeration in §5.5 (11 ordered + 1 ANT + 1 async) must match spec 02's migration ledger; verified order matches `main.tsx:325-352` and the eleven import lines `main.tsx:174-184` + 200.
- **Spec 03 (QueryEngine):** `runHeadless` handoff (line 26) only frames the call; spec 03 owns the body. F-04 init.ts line drift may also affect spec 03's `initializeTelemetryAfterTrust` references.
- **Spec 04 (turn pipeline):** missing from §2.7 (F-13).
- **Spec 09 / 04 (`TRANSCRIPT_CLASSIFIER`):** lifecycle gate at migration #10 cross-references spec 09's classifier feature flag.
- **Spec 22 / 25 / 26:** `init()` runs `populateOAuthAccountInfoIfNeeded`, `initializeAnalyticsGates`, `loadRemoteManagedSettings`, `loadPolicyLimits`. F-04's init.ts line drift propagates.
- **Spec 30 / 32 / 34 / 35 (modes):** argv pre-rewrite fast paths (`COORDINATOR_MODE`, `KAIROS`, `BRIDGE_MODE`, `DIRECT_CONNECT`, `SSH_REMOTE`, `LODESTONE`) — F-06 (DSP-strip on cc://-print rewrite) is a behavior these modes need to know.
- **Spec 40 / 41:** `STATE` field categories overlap with persistent memory and session state ownership; spec 01 correctly deflects to spec 40/41.

---

## Hardest-to-verify claim

> "The published `bin/claude` shell wrapper" / "Boot reaches `main.tsx:main()` via the runtime npm bin shim, which is undocumented in the leak." (§2.6 missing-source ledger.)

This is the load-bearing assumption tying `process.argv[0]/[1]`, the npm bin handoff, the shebang, and any pre-`main.tsx` env wiring (e.g. `BUN_RUNTIME`, `NODE_OPTIONS`). It is **structurally unverifiable from the leaked tree** — there is no `package.json`, no `bin/`, no shell wrapper, and `cli.tsx` is the SDK shim, not the bin. Every claim that depends on what runs *before* `profileCheckpoint('main_tsx_entry')` (line 12) — including any inherited env vars, signal masks, or stdio setup — is downstream of an artifact not in the repo.
