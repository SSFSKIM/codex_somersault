# Phase 9.5 Adversarial Review — Spec 00 (Overview)

**Reviewer**: Opus side, parallel adversarial pass
**Target**: `docs/specs/00-overview.md`
**Method**: Re-verified non-trivial claims against `src/` (READ-ONLY).

## Severity Counts

- **Critical**: 1
- **Major**: 3
- **Minor**: 6

---

## Findings

### CRITICAL-1 — `ScheduleCronTool/{CronCreate,CronDelete,CronList}Tool` is falsely listed as missing source

- **Section/Line**: §2.5 Missing-Source Ledger, table row at line ~173.
- **Claim** (verbatim): `` `ScheduleCronTool/{CronCreate,CronDelete,CronList}Tool` | `tools.ts:31-33` | `feature('AGENT_TRIGGERS')` ``
- **Verification**:
  - `tools.ts:31-33` references `./tools/ScheduleCronTool/CronCreateTool.js`, `CronDeleteTool.js`, `CronListTool.js`.
  - `ls src/tools/ScheduleCronTool/` returns `CronCreateTool.ts CronDeleteTool.ts CronListTool.ts UI.tsx prompt.ts`.
  - **The source IS present.** The "missing" claim is wrong.
- **Severity**: Critical (the missing-source ledger is consumed verbatim by spec 19's §2/§12; a sub-agent will write a phantom "absent source" gap and skip implementing a present tool).
- **Fix**: Delete this row from the §2.5 table. Spec 19 should treat ScheduleCronTool as a normal in-tree tool family.

### MAJOR-1 — Internal contradiction on whether `bubble` is in the runtime permission mode set

- **Section/Line**: §6.2 line 410 vs. §11 line 704.
- **Claim A** (§6.2): "the type-level union `InternalPermissionMode` ALSO includes `bubble`, but `bubble` is NOT in the runtime validation set — it is a type-only mode used internally and is never accepted from settings.json or `--permission-mode`."
- **Claim B** (§11): "Permission modes: `default`, `acceptEdits`, `bypassPermissions`, `dontAsk`, `plan`, optional `auto` (gated by `TRANSCRIPT_CLASSIFIER`), **internal `bubble`**."
- **Verification**: `src/types/permissions.ts:28` confirms `bubble` is in the type union but `INTERNAL_PERMISSION_MODES` (lines 33–36) does NOT include it. §6.2 is correct; §11's "internal `bubble`" entry implies it's part of the runtime set.
- **Severity**: Major (sub-agents lifting the §11 reimplementation checklist will replicate `bubble` as a runtime mode, creating a behavioral divergence in permission validation).
- **Fix**: §11 line 704 → drop `internal `bubble`` or qualify as "type-only `bubble` (NEVER user-addressable, see §6.2)".

### MAJOR-2 — `ToolUseContext` field-count inconsistency (74 vs. ~30+)

- **Section/Line**: §4 line 217 says "**~70+ fields (74 leaf fields counted in `Tool.ts:158-300`)**". §11 line 702 says "**~30+ fields**".
- **Verification**: `awk '/^export type ToolUseContext/,/^}$/' src/Tool.ts | grep -cE '^\s+\w+[?:]'` returns **71** top-level field-like lines (close to the §4 number, far above §11's "30+").
- **Severity**: Major (both numbers can't be right; "30+" is wildly low and will mislead spec 08's §4 sizing).
- **Fix**: Standardize on the §4 count. §11 line 702 should read "~70+ fields" (and replace the misleading "30+").

### MAJOR-3 — `SuggestBackgroundPRTool` registry citation off

- **Section/Line**: §2.5 row `SuggestBackgroundPRTool | tools.ts:21-24 | ANT-only`.
- **Verification**: `tools.ts:215` references it inside the `getAllBaseTools` array. The cited 21–24 range was the import block; that range still references `tungstenTool` line 60 and `SuggestBackgroundPRTool` is not at 21–24 in current source. Confirmed `src/tools/SuggestBackgroundPRTool` does NOT exist (good: missing-source claim is correct).
- **Severity**: Major (citation drift breaks the §2.5 invariant that registry references be precise).
- **Fix**: Re-cite to the exact import line and include the `:215` use site (mirror the TungstenTool entry which already does both `:60, :215`).

### MINOR-1 — File count inconsistency

- **Section/Line**: §1 "**~1,902 source files / ~512K LOC**".
- **Verification**: `find src -type f ! -name .DS_Store | wc -l` = 1902 (matches), but `*.ts/*.tsx` only = 1884. The 1902 includes non-source files (.json, etc.). "Source files" is technically inaccurate.
- **Severity**: Minor.
- **Fix**: Either say "~1,902 files (≈1,884 .ts/.tsx)" or "~1,884 source files".

### MINOR-2 — `DUMP_SYSTEM_PROMPT` listed as a `feature()` flag but is also a CLI argument

- **Section/Line**: §8.1.B "Boot / CLI / install" table row.
- **Verification**: confirmed in `entrypoints/cli.tsx:53`. The flag indeed exists in `feature()` form. Note: spec characterizes it as `--dump-system-prompt` debug flag, but the gate is build-time, not CLI. Slight semantic blur.
- **Severity**: Minor.
- **Fix**: clarify "`feature('DUMP_SYSTEM_PROMPT')` gates the `--dump-system-prompt` CLI option's availability".

### MINOR-3 — §2.4 / Open Question 1 marks `types/message.ts` and `types/tools.ts` as "likely in `src/types/generated/`"

- **Section/Line**: §2.4, §12 item 1.
- **Verification**: `src/types/generated/` contains only protobuf event types (`events_mono/`, `google/protobuf/`). No `message.ts` or `tools.ts`. The "likely generated" hypothesis is unsupported.
- **Severity**: Minor (still flagged as unresolved, but the hint misleads).
- **Fix**: Update §12 item 1: "Not in `src/types/generated/` — likely re-exported elsewhere or a leak gap."

### MINOR-4 — §5.4 multi-agent claim about `tools/shared/spawnMultiAgent.ts (35KB)`

- **Section/Line**: §5.4 line 293.
- **Verification**: not directly checked in this pass (file size unverified). Spec 30 owns it; defer.
- **Severity**: Minor (untestable from this review without measurement; flagged for spec 30).
- **Fix**: spec 30 to confirm exact size.

### MINOR-5 — "user-addressable runtime set is `INTERNAL_PERMISSION_MODES`" naming oddity

- **Section/Line**: §6.2 line 410.
- **Verification**: `permissions.ts:33`: `INTERNAL_PERMISSION_MODES` is exposed as the user-addressable set. The name "INTERNAL" but used for the user-addressable set is genuinely confusing — but accurately matches src. Spec is faithful to a misleading source identifier.
- **Severity**: Minor (cosmetic/explanatory).
- **Fix**: Add a one-line note: "Despite the name `INTERNAL_*`, this is the runtime-validated user-addressable set; the broader type-only union is `InternalPermissionMode`."

### MINOR-6 — The `Tools` type citation `Tool.ts:701` confirmed but worth pinning

- Verified `Tool.ts:701` is the `Tools` type. No issue.

---

## Spec-Level Verdict

**Minor revise** — one Critical (false missing-source row), three Major (one self-contradiction, one number drift, one citation drift). Rest is solid: 89-flag count matches src exactly, line counts for QueryEngine/query/Tool/tools/commands/context all verified to the line, registry refs largely correct, ANT path index plausible, glossary/template authoritative.

The spec is unusually high-fidelity for its size — this is a small number of bugs across ~800 lines, not a fundamental rebuild.

---

## Cross-Spec Impact List

- **Spec 19** (Sleep/Synthetic/Schedule/Brief/AskUserQuestion/Config/REPL/PowerShell): must NOT inherit the false `ScheduleCronTool` missing-source row from §2.5. Its §2/§12 should treat ScheduleCronTool as in-tree.
- **Spec 09** (permissions): correct §6.2 framing of `bubble` must propagate; do not lift the §11 line-704 phrasing.
- **Spec 08** (Tool/registry): use §4 "~70+ fields" sizing; ignore §11's "~30+". Open question 1 (message.ts/tools.ts location) is a real unresolved gap.
- **Spec 30** (coordinator): verify `spawnMultiAgent.ts` size claim.
- **Spec 26** (analytics): StatSig `claude_code_global_system_caching` and `claude_code_system_cache_policy` schemas remain unresolved (Open Question 5).
- **Spec 16/23** (MCP): `CLAUDE_AGENT_SDK_MCP_NO_PREFIX` env semantics still TBD.
- **Spec 36** (voice): voice context wiring location confirmed as `commands/voice/` + `services/voice*.ts` only; `context/voice/` does not exist.

---

## Hardest-to-Verify Claim (smell of unfalsifiability)

> **§5.3**: "**Tool ordering is server-side cache invariant**, controlled by **two related StatSig configs**: `claude_code_global_system_caching` … `claude_code_system_cache_policy` …"

The comments at `tools.ts:191` and `:354-365` confirm the claim WITHIN the leak — but the leak only contains the client side. The actual StatSig config schemas, their version pinning, what "matches" means in the server's cache-key derivation, and whether reordering truly invalidates "all downstream cache keys for all users" cannot be verified from the leaked tree at all. This is the most load-bearing assertion in §5.3 and §11, and a reimplementer must take it on faith. Spec 26's Open Question 5 correctly flags this — but every spec citing tool ordering as cache-load-bearing is implicitly trusting a server contract no spec can prove.
