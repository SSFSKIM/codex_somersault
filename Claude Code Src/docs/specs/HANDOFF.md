# Project Handoff — Claude Code Reverse-Spec

> Read this FIRST before doing anything. Then read INDEX.md and the master plan.

You are inheriting a long-running reverse-engineering documentation project at `/Users/new/Downloads/claude-code-main/`. The previous session compacted at the end of Phase 2; this doc captures everything you need to resume without losing momentum.

---

## 1. The 60-Second Pitch

The user is reverse-engineering the **leaked Claude Code CLI source** (~1,902 source files / ~512K LOC at `src/`) into 43 modular spec docs under `docs/specs/`. Each spec is bit-exact: verbatim system prompts, regexes, schemas, decision trees, and constants are inlined. The end goal is that an engineer can rebuild a behaviorally and architecturally equivalent harness from the docs alone — with the freedom to swap models, tools, and system prompts.

This is the inverse of normal SDD: instead of `spec → plan → code`, we run `code → spec ← plan`.

---

## 2. Where to Start Reading

Read these in order. They give you everything you need.

1. **`docs/superpowers/specs/2026-05-08-claude-code-reverse-spec-design.md`** — the master plan. ALL operating rules, dispatch protocol, phase graph, sub-agent prompt template (§5.3), verification strategy, INDEX convention. **Treat this as the source of truth for "how we work".**
2. **`docs/specs/INDEX.md`** — the progress tracker. Every spec's status, owner, last-updated, and adjacent-spec cross-refs. **The single source of truth for "what is done"**.
3. **`docs/specs/00-overview.md`** — the foundational anchor. Architecture map, source ownership, glossary, 89-flag feature matrix, ANT-vs-prod table, canonical 12-section spec template (§6.1), repo conventions appendix.
4. **`docs/specs/08-tool-base-registry.md`** — the Tool interface, registry, and on-disk pattern. Every tool spec (10..19) references this.
5. **`docs/specs/20-command-system.md`** — the Command type union, registry, and three command kinds. Spec 21 (catalog) references this.
6. **`docs/specs/01-entrypoint-bootstrap.md` and `02-settings-schemas-migrations.md`** — Phase 2 outputs from sub-agents. Skim only if your next task touches them.

The leaked source you are reverse-engineering is at `src/`. **`src/` is gitignored** in this nested repo — use `Read`/`Grep` directly on those paths, not git tools.

---

## 3. Current State (as of Phase 2 completion)

**Commit `2272c69`** is the head. 8 commits total:

```
2272c69  spec(01, 02): entrypoint+bootstrap, settings+schemas (Phase 2, parallel)
47448c2  spec(08, 20): tool registry + command system (Phase 1, main agent)
d68d5eb  spec(00): apply codex review fixes (10 critical + 3 warn + 1 info)
d814e84  spec(00): self-review fixes — flag matrix, ToolUseContext count, cache config
9241ed2  spec(00): write overview — architecture map, source ownership, flag matrix
98ca681  docs: apply self-review fixes (coverage gaps, dispatch hardening)
27c2cf9  docs: incorporate review-agent feedback on master plan and INDEX
aaf32f1  docs: bootstrap reverse-spec project
```

**Specs done**: 00, 01, 02, 08, 20 (5 of 43). All other rows in INDEX.md are `pending`.

**Phase progress**:
- ✅ Phase 0: 00-overview (4-tier verified: main + review-agent + main self-review + codex + sub-A2 correction)
- ✅ Phase 1: 08-tool-base-registry, 20-command-system (main agent direct)
- ✅ Phase 2: 01-entrypoint-bootstrap (sub-A1), 02-settings-schemas-migrations (sub-A2), parallel
- ⏳ **Phase 3 is the next dispatch**: parallel x5 — 03, 04, 05, 06, 07 (query loop & context)

---

## 4. Repo & Environment Specifics (Read Carefully)

### 4.1 Nested git layout

`/Users/new/Downloads/` is the **outer** git repo (1800+ unrelated files staged, 0 commits — long-running user state, NOT ours to touch). `/Users/new/Downloads/claude-code-main/` is a **nested** git repo we created for this project. **All git commands run from inside `claude-code-main/` operate on our nested repo only.**

`src/` and `.omc/` and `AGENTS.md` are gitignored — do not stage them. The `src/` tree is the leaked source we reverse-engineer; treat it as read-only reference.

### 4.2 The `codex` CLI

The user's `codex` shell alias is broken (points at a missing peon script). The real binary is at:

```
/Applications/Codex.app/Contents/Resources/codex
```

Version: codex-cli 0.128.0-alpha.1. Authenticated via `~/.codex/auth.json` (already present). To use:

```bash
/Applications/Codex.app/Contents/Resources/codex exec "<prompt>" \
  -C /Users/new/Downloads/claude-code-main \
  -s read-only --skip-git-repo-check \
  -c 'model_reasoning_effort="medium"' < /dev/null
```

Use codex for adversarial review of foundational specs (we did it for 00; consider it for 03, 04, 09, 21). Do NOT route through the gstack `/codex` skill — its preamble has many one-time interactive prompts.

### 4.3 Bash hook noise

Many Bash/Edit/Write tool calls return system-reminder hooks like "Tool 'Bash' failed" or "Edit operation failed" even when the actual tool output says "completed/updated successfully". **Trust the tool output, not the hook reminder.** Hooks are misfiring in this environment.

### 4.4 Memory files

Three CLAUDE.md files load automatically: `~/CLAUDE.md`, `~/Downloads/CLAUDE.md`, `~/Downloads/claude-code-main/CLAUDE.md`. They contain reference info. The leaked-repo CLAUDE.md describes navigation tips for the source. Useful but **not authoritative** — verify everything.

---

## 5. Operating Rules (Hard)

### 5.1 The verbatim policy

Spec docs MUST inline:
- Full system prompts (no paraphrasing)
- Zod schemas verbatim
- Critical regexes with anchor explanations
- Permission decision-tree pseudocode where applicable
- Constants tables (timeouts, caps, paths, env vars)

Specs cite source via `src/<path>:<line-range>` for every behavioral claim. Prefer ≤25-line ranges.

### 5.2 Sub-agent dispatch protocol

When you dispatch via the `Agent` tool:
- **Always**: `subagent_type: general-purpose` (read-only types like `Explore` cannot write spec files and will fail)
- **Always**: `model: opus`
- **Multiple Agent calls in a single message** = parallel concurrent execution
- Each sub-agent prompt must include the §5.3 master-plan template fields: project context, reference docs to read, scope (IN/OUT), required output (12-section template + N/A rule), source-coverage inventory step, verbatim asset rules, citation rules, output protocol (disk write + ≤700-word reply + INDEX row update), hard rules (no hallucination, no paraphrase, no chat-verbatim).
- **No-chat-verbatim is critical**: sub-agents must write spec content ONLY to disk and reply with a 400-700 word summary. Without this, main agent context is poisoned by 5K-30K-token verbatim blocks.

### 5.3 INDEX.md update protocol

When a sub-agent finishes, it edits ITS OWN existing row in INDEX.md (Status → done, set Owner, Last updated, refine Adjacent). **Do not add duplicate rows**. The dispatcher (main agent) verifies on receipt of the reply.

### 5.4 Verification habits (from this session's lessons)

Treat EVERYTHING as hypothesis until source-cited:
- README claims are hypotheses (e.g., README says "Tool.ts ~29K lines" — actually 792 lines / 29K bytes).
- CLAUDE.md feature flag list is hypothesis (~25 listed; actual is 89).
- Earlier specs are hypotheses (sub-A2 found 00-overview's settings precedence chain was wrong; we fixed it).
- Your own previous Phase grep is hypothesis (main missed half the feature flags by greping only hub files).

Before claiming a behavior, run a verifying grep or re-read the cited line.

### 5.5 Commit message convention

Use HEREDOC, end with:
```
Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

Stage explicit paths only: `git add docs/specs/<files>`. Never `git add -A` (could pull in untracked tooling state).

---

## 6. Critical Accuracy Gotchas Observed (Do Not Repeat)

| Mistake | Reality | Cite |
|---|---|---|
| "Tool.ts ~29K lines" (from README) | 792 lines (29K bytes) | `src/Tool.ts` |
| "42 feature flags" (Phase 0 first draft) | 89 unique `feature('X')` references | grep src/ |
| "settings precedence: env > project > user > MDM > defaults" | Actual: pluginSettingsBase < userSettings < projectSettings < localSettings < flagSettings < policySettings (NO env source); policySettings internally cascades remote > HKLM/plist > file > HKCU | `src/utils/settings/settings.ts:241-352` |
| "Tool.ts imports `../types/message.js`" | Actual: `./types/message.js` (Tool.ts is in `src/`, types is its sibling) | `src/Tool.ts:40,58` |
| "BASH_CLASSIFIER gate at bashPermissions.ts:84" | Line 84 is a comment; actual gates at lines 1576 and 1645 | grep |
| "EXTRACT_MEMORIES gate at memdir/paths.ts:65" | That line is a comment; actual gates at `utils/backgroundHousekeeping.ts:7,34` and `query/stopHooks.ts:142` | grep |
| "ANT-only setup at setup.ts:337,417 = UDS init" | Actually ANT repo classification + bypass-permission gate; UDS_INBOX init is at setup.ts:95 | `src/setup.ts` |
| "GrowthBook init in parallel pre-module" | False — pre-module side effects are profileCheckpoint, startMdmRawRead, startKeychainPrefetch only. GrowthBook resolves at build time via bun:bundle DCE. | `src/main.tsx:1-15` |
| "ToolUseContext has 30+ fields" | 74 leaf fields | `src/Tool.ts:158-300` |
| "Status truncation suffix starts with leading space" | Actual: leading `\n` (newline). Verbatim string in `src/context.ts:84-89` |

The `src/types/message.ts` and `src/types/tools.ts` files are imported by `Tool.ts` but **do not exist at that path**. They likely live in `src/types/generated/` (not enumerated in Phase 0). Spec 08 §12 records this. Sub-agent for any spec that depends on these types should locate or record as missing-source.

---

## 7. Immediate Next Action: Phase 3 Dispatch

Master plan §7 Phase 3: parallel x5 dispatch of 03..07.

**MANDATORY user checkpoint AFTER Phase 3** (master plan §10): the query engine (03) is the algorithmic heart of the harness; the user MUST scan it before Phase 4.

### 7.1 Phase 3 sub-agent assignments

| Spec | Scope (IN) | Scope (OUT — cite spec #) | Critical care |
|---|---|---|---|
| **03-query-engine.md** | `src/QueryEngine.ts` (1295 lines) | tools (08), permission (09), turn pipeline (04) | **Highest algorithmic complexity.** Must capture: streaming SSE event order, tool-use loop structure, retry/backoff with exact timeouts, thinking mode interaction, token counting algorithm, prompt-cache breakpoint integration. Queue for Phase 9 adversarial review. |
| **04-turn-pipeline.md** | `src/query.ts` (1729 lines), `src/query/`, `src/services/tools/` | query engine (03), permission (09), context (05), compaction (07) | message → tool-use → tool-result lifecycle, system-reminder injection, hook fan-out |
| **05-context-assembly.md** | `src/context.ts` (189), `src/context/` | settings (02), persistent memory (40), services (29), output styles (38) | system prompt assembly, env/project memory load, CLAUDE.md chaining (verbatim), `setSystemPromptInjection` for cache breaking (BREAK_CACHE_COMMAND) |
| **06-cost-token-tracking.md** | `src/cost-tracker.ts`, `src/costHook.ts`, `src/services/tokenEstimation.ts` | api (22), analytics (26), TOKEN_BUDGET flag handling in 04 | usage aggregation handoff |
| **07-context-compaction.md** | `src/services/compact/` | query engine (03), turn pipeline (04), context (05), services/memory (29), session state (41) | auto-compact triggers, retention policy, microcompact/snip variants (HISTORY_SNIP, REACTIVE_COMPACT, CACHED_MICROCOMPACT) |

### 7.2 Dispatch checklist

For each of the 5 sub-agents, build the prompt from the §5.3 template, including:
- Reference docs to read FIRST: `docs/specs/00-overview.md` (template + glossary + flag matrix), `docs/specs/08-tool-base-registry.md`, `docs/specs/20-command-system.md`, the master plan
- Source-coverage inventory step
- IN scope and OUT scope (table above)
- ≤700-word reply
- Disk-write only; INDEX row self-update
- `subagent_type: general-purpose`, `model: opus`

Send all 5 in a **single message** with 5 Agent tool calls so they run concurrently.

### 7.3 After Phase 3

1. Verify all 5 spec files exist on disk and INDEX.md rows show `done`.
2. Spot-check 03's algorithm sections against `QueryEngine.ts`. **Run a codex consult review of 03** specifically (see §4.2 above for the codex CLI command). Apply fixes.
3. Commit Phase 3 outputs.
4. **Stop and ask the user to review** before proceeding to Phase 4.

---

## 8. Roadmap After Phase 3

Per master plan §7:

| Phase | What | Parallelism |
|---|---|---|
| 4 wave 1 | 09-permission-system | sequential, alone |
| 4 wave 2 | 10..19 (10 tool docs) | parallel x10 |
| 5 | 21-command-catalog | 1 agent, may split into 21a/b/c |
| 5.5 | catalog sanity pass (main agent, lightweight) | — |
| 6 | 22..29 (8 services) | parallel x8 |
| 7 | 30..36 (7 modes) | parallel x7 |
| 8 | 37..42 (UI/state/misc) | parallel x6 |
| 9 | per-spec adversarial reviews + cross-spec consistency + coverage audit | mixed |

**Mandatory user checkpoints** are at: after Phase 0 (done), after Phase 3 (next!), after Phase 4 wave 1 (09 permission), after Phase 9 final acceptance.

After all 43 specs are `done` in INDEX, run the Phase 9 coverage audit: `find src -type f` minus all spec citations → residuals must be claimed in 42-misc or explicitly marked "no behavioral content".

---

## 9. State Files (do not delete)

- `docs/superpowers/specs/2026-05-08-claude-code-reverse-spec-design.md` — master plan, ~700 lines
- `docs/specs/INDEX.md` — progress tracker, the single source of truth for status
- `docs/specs/00-overview.md` — foundational anchor for ALL subsequent specs
- `docs/specs/HANDOFF.md` — this doc
- `.gitignore` — excludes src/, .omc/, AGENTS.md
- All `docs/specs/<NN>-*.md` — completed specs

`/tmp/codex-review-output.txt` exists from the Phase 0 codex review (already applied; can be deleted).

`TaskCreate` task list inside the conversation is short-horizon only — INDEX.md is authoritative across sessions.

---

## 10. If Things Look Wrong

- **Hook says tool failed but the output succeeded**: trust the output. See §4.3.
- **codex: aliased to peon script that doesn't exist**: use the explicit binary path. See §4.2.
- **A spec claim contradicts source**: source wins. Update the spec, do NOT update the source. Add a §12 entry if you found ambiguity.
- **An earlier spec contradicts your sub-agent's finding**: the sub-agent (closer to the source) is usually right. Update the earlier spec, cite the new source evidence, and recommit. (See sub-A2's settings-precedence correction for the pattern.)
- **You can't find a path that's referenced in source**: it's a missing-leaked-source case. Document in §12 of the owning spec; do not silently drop the reference.
- **The user asks for something that contradicts the master plan**: the user wins. Master plan rules can be overridden by explicit user instruction; record the deviation in the relevant spec or commit message.

---

## 11. Style Notes

- Output language: English for spec docs, but the user converses in Korean. Match their language in conversation; keep specs in English.
- Each turn ends with a tight 1-2 sentence summary unless deep technical content was just produced.
- Use the `★ Insight ─────────────────────────────────────` callout block sparingly for genuine non-obvious observations (this is enabled by the explanatory output style; do not overuse).
- Korean term `"진행"` = proceed/go.

---

## 12. The User's Working Style (observed)

- High trust, low ceremony. If you have a strong recommendation, say so directly with rationale; the user will say yes/no.
- Wants to be informed of decisions, not asked permission for every small fix.
- Comfortable with deep technical detail and long sessions.
- Will explicitly request review or pause when needed (`"마지막으로 너스스로 한번 리뷰해보도록 하자"` = "let's have you review yourself one last time").
- Welcomes external review (codex, review agents) when stakes are high.
- Confident interruptions: `"여기까지 있으면 아마도 컴팩션을 하거나..."` triggered this handoff.
- Approves option lists with single-letter answers (A/B/C).

---

Good luck. The system is in good shape — keep the source-citation discipline, dispatch sub-agents in parallel, and hand off cleanly when context fills again.
