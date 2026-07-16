# Replicate Codex's native review workflow as a Claude Code plugin (`codex-review`)

This ExecPlan is a living document. The sections `Progress`, `Surprises & Discoveries`, `Decision Log`, and `Outcomes & Retrospective` must be kept up to date as work proceeds. This document is maintained in accordance with the PLANS.md contract vendored in the doperpowers `execplan` skill (there is no PLANS.md checked into this repository).

## Purpose / Big Picture

OpenAI's Codex CLI ships a native code-review workflow (`codex review --base <branch>`, also reachable as `codex exec review` and the TUI `/review` command). Its mechanism, traced in full from the Rust source in this repository (`codex-rs/`), is: the parent session resolves a "review target" into a short seed instruction (for a base branch it precomputes the git merge-base SHA and instructs "run `git diff <SHA>`"), then spawns an isolated one-shot child agent whose system prompt is a review rubric, with no inherited conversation history, restricted tools, and no approval prompts; the child investigates the diff itself with git commands, emits prioritized findings plus an overall correctness verdict in a mandated output format, and the parent parses that, records it into its own conversation history, and renders it to the user.

After this plan is implemented, a Claude Code user can install a self-contained plugin named `codex-review` (repository `/Users/new/Documents/GitHub/codex-review`, to be published as `SSFSKIM/codex-review` on GitHub) and get the same workflow natively in Claude Code: the main agent — proactively after substantial implementation work, or on user request like "review my changes against main" — dispatches a `codex-reviewer` subagent that reviews the change in a fresh, isolated context using the ported Codex rubric, and returns a `## Findings` block (`[P1] title — file:lines` entries) plus a `## Verdict` block (`patch is correct|incorrect`, explanation, confidence) that the main agent relays verbatim. To see it working: load the plugin with `claude --plugin-dir /Users/new/Documents/GitHub/codex-review` in any git repository, ask "review my changes against main", and observe the subagent run `git diff <merge-base>` and return the two blocks.

The user's prime directive for this work: replicate the Codex native mechanism with the minimum possible deviation. Every deviation must be justified in the Decision Log.

## Progress

- [x] (2026-07-16 19:40Z) Grill completed in-session: trigger posture (autonomous + on-request), verdict format (hybrid text), plugin home (new standalone repo) confirmed by the user.
- [x] (2026-07-16 19:50Z) ExecPlan authored and committed to `docs/doperpowers/execplans/2026-07-17-codex-review-plugin.md` in the codex_somersault repo.
- [x] (2026-07-16 19:52Z) Milestone 1: scaffolded the `codex-review` repo (git init, MIT license, manifests, README stub) on branch `build/initial-plugin`; both manifests JSON-validated.
- [x] (2026-07-16 20:05Z) Milestone 2: `agents/codex-reviewer.md` authored, committed, and validated — fixture Test A PASSED (planted off-by-one flagged as [P1] at calc.py:4-5 with a rubric-compliant suggestion block, plus a legitimate bonus [P2] IndexError-regression finding; Verdict block exact: "patch is incorrect", Confidence 0.98; util.py not flagged; main agent relayed verbatim). Transcript in Artifacts and Notes.
- [x] (2026-07-16 19:58Z) Milestone 3: authored `skills/codex-review/` (SKILL.md, resolve_target.sh, codex-parity.md); resolve_target.sh unit-checked against the fixture (correct SHA; exit 1 on unknown branch).
- [ ] Milestone 4: full validation matrix — Tests B through F (skill-path base review, uncommitted, commit, custom, no-bug control); complete README.
- [ ] Milestone 5: exit gate — `codex exec review --base main` on the plugin repo, fix findings, merge to `main`, create and push `SSFSKIM/codex-review` on GitHub; write retrospective.

## Surprises & Discoveries

- Observation: Codex itself already ships a skill-ified version of its review workflow. The app-server's "detached" review delivery injects a `review-agent` skill instead of running the native pipeline; its source sample is `codex-rs/skills/src/assets/samples/review-agent/SKILL.md`.
  Evidence: `codex-rs/app-server/src/request_processors/turn_processor.rs:1374-1381` builds the prompt "Use [$review-agent](<path>/review-agent/SKILL.md) for this review." This is a first-party precedent that the mechanism survives translation into a prompt artifact, and its adaptation choices (JSON schema dropped in favor of `[P1] title — path:line` text; merge-base ritual folded into prose) directly informed this plan.
- Observation: the native pipeline does NOT enforce its JSON output schema mechanically. `run_codex_thread_one_shot` is called with `final_output_json_schema: None` (`codex-rs/core/src/tasks/review.rs:135`); format compliance comes entirely from the rubric prompt plus a tolerant parser (`parse_review_output_event`, `tasks/review.rs:197`). A prompt-enforced format in a Claude Code agent therefore has the same trust model as native — this is parity, not a compromise.
- Observation: the native text rendering of findings drops most of the JSON structure anyway. `format_review_findings_block` (`codex-rs/protocol/src/review_format.rs:23`) renders only title, `path:start-end`, and body (priority survives only as the `[P1]` tag inside the title); `render_review_output_text` (`review_format.rs:64`) renders only `overall_explanation` plus that block — `overall_correctness` and the confidence scores never reach the parent-visible text. The hybrid format chosen here is therefore a superset of what native users actually see.

## Decision Log

- Decision: trigger posture is autonomous + on-request (agent description tuned so the main agent dispatches proactively after substantial implementation work and before commits/PRs, and reactively on user request).
  Rationale: the user's stated workflow is "when the main agent needs a review"; native Codex is user-triggered only, but Claude Code's description-driven dispatch enables the autonomous half at zero mechanism cost. Rejected: user-request-only (drops the half the user explicitly asked for); mandatory pre-commit gate (too aggressive; reviews on trivial changes waste tokens).
  Date/Author: 2026-07-16, session grill (user-confirmed).
- Decision: verdict format is hybrid text — `## Findings` with `[P1] <title> — <path>:<start>-<end>` entries and one-paragraph bodies, then `## Verdict` with fixed lines `Overall correctness:`, `Explanation:`, `Confidence:`.
  Rationale: mirrors what native actually records into parent history (rendered text, not JSON — see Surprises), keeps a machine-greppable correctness line the user explicitly wants ("verdict를 메인 에이전트에게 전달"), and follows Codex's own skill-ification precedent of dropping JSON. Rejected: verbatim rubric JSON (native itself never shows it to the parent as JSON; brittle over long reviews); pure prose like the review-agent sample (no fixed verdict line for the main agent to key off).
  Date/Author: 2026-07-16, session grill (user-confirmed with preview).
- Decision: per-finding `confidence_score` is dropped from the output format; only the overall confidence survives, and per-finding priority survives as the `[P0]`-`[P3]` title tag.
  Rationale: native's own text renderer (`format_review_findings_block`) drops per-finding confidence and the numeric priority field; the hybrid format replicates the parent-visible surface, not the internal wire type. Rejected: appending "(confidence: 0.8)" per finding — more deviation from the user-approved preview, no consumer for the number.
  Date/Author: 2026-07-16, plan author.
- Decision: the `## Verdict` block includes `Overall correctness:` and `Confidence:` lines even though native's text render drops them.
  Rationale: the user's core requirement is that a verdict reaches the main agent; these two lines carry `overall_correctness` and `overall_confidence_score` from the rubric's schema into the text surface. This is the plan's one deliberate enrichment over native's parent-visible text, and it is still strictly rubric-derived.
  Date/Author: 2026-07-16, plan author.
- Decision: plugin lives in a new standalone repository `/Users/new/Documents/GitHub/codex-review`, published as `SSFSKIM/codex-review`, MIT license, plugin and repo both named `codex-review`, agent named `codex-reviewer`, skill named `codex-review`.
  Rationale: user chose "new standalone plugin repo (like claude-plugin-codex)"; that precedent was published publicly under MIT. Rejected names: `claude-plugin-codex-review` (confusable with the user's existing `claude-plugin-codex`, which is a plugin for the Codex host, not for Claude Code). Rejected homes: inside the codex_somersault monorepo (couples an installable artifact to a fork), `~/.claude` local-only (not shareable).
  Date/Author: 2026-07-16, session grill (user-confirmed).
- Decision: merge-base resolution is parent-side and deterministic via `skills/codex-review/scripts/resolve_target.sh` (a shell port of `merge_base_with_head`), with a reviewer-side prose fallback inside the agent for dispatches that arrive without a precomputed SHA.
  Rationale: native's parent (`resolve_review_request`) precomputes the SHA so the reviewer runs one deterministic `git diff <SHA>`; the script preserves that determinism. The agent-side fallback mirrors native's own `BASE_BRANCH_PROMPT_BACKUP` (used when the parent cannot compute a merge base) and Codex's skill-ified review-agent, which embeds the resolution ritual in prose. Rejected: prose-only (loses determinism); script-only (a lazy dispatch without the skill loaded would strand the reviewer).
  Date/Author: 2026-07-16, plan author.
- Decision: reviewer agent toolset is `["Read", "Grep", "Glob", "Bash"]`, `model: inherit`.
  Rationale: native disables web search, collab/multi-agent spawning, and image viewing for the review child and runs with approval policy Never; omitting WebFetch/WebSearch/Task/Write/Edit from the tools list is the declarative Claude Code equivalent (stronger than native's runtime flags). Bash stays because the reviewer must run `git diff` itself — the diff is never injected as data, in native or here. `model: inherit` is exact parity with native's fallback when `review_model` is unset (`tasks/review.rs:121-124`).
  Date/Author: 2026-07-16, plan author.
- Decision: the rubric is ported verbatim into the agent body except for (a) the OUTPUT FORMAT section (replaced by the hybrid format), (b) the one sentence mandating a numeric `priority` JSON field (subsumed by the `[P0]`-`[P3]` title tag), and (c) three appended sections that carry native behavior the rubric text itself does not: conduct constraints (= approval Never + feature disables), target-resolution fallback (= BASE_BRANCH_PROMPT_BACKUP), and a "When to invoke" section (Claude Code agent convention).
  Rationale: minimum-deviation directive; every non-verbatim piece maps to a specific native mechanism documented in `references/codex-parity.md`.
  Date/Author: 2026-07-16, plan author.

## Outcomes & Retrospective

Pending — written at finish.

## Context and Orientation

Read this section as if you know nothing about either codebase.

Codex is OpenAI's Rust coding-agent CLI. This repository (`/Users/new/Documents/GitHub/codex_somersault`) is a fork of it; the Rust workspace lives under `codex-rs/`. Codex's review feature was traced end-to-end in a prior session; the authoritative source files, all under `codex-rs/`, are:

- `prompts/templates/review/rubric.md` — the reviewer's system prompt ("the rubric"). Ported in full below; you do not need to open it, but it is the ground truth if a discrepancy is suspected.
- `prompts/src/review_request.rs` — the parent-side target resolution: four review-target kinds and the verbatim seed-prompt templates (ported below).
- `git-utils/src/branch.rs` — `merge_base_with_head`: resolve the base branch, prefer its upstream when the upstream exists and is ahead, then `git merge-base HEAD <ref>` (ported below as a shell script).
- `core/src/session/review.rs` and `core/src/tasks/review.rs` — the child-agent spawn: rubric installed as base instructions, fresh history, web search and collaboration tools disabled, approval policy Never, `review_model` fallback to the parent model.
- `protocol/src/review_format.rs` — how findings are rendered as text for the parent ("Full review comments:" block).
- `skills/src/assets/samples/review-agent/SKILL.md` — Codex's own first-party skill-ification of this workflow (precedent).

Claude Code is Anthropic's coding-agent CLI (binary `claude`, version 2.1.204 verified on this machine). Its plugin system: a plugin is a directory with a `.claude-plugin/plugin.json` manifest; subdirectories `agents/` (each `.md` file is a dispatchable subagent: YAML frontmatter `name`/`description`/`model`/`tools` + a markdown body that becomes the subagent's system prompt) and `skills/<name>/SKILL.md` (YAML frontmatter `name`/`description` + a body of instructions that loads into the main agent's context when the description matches the task; may bundle `scripts/` and `references/`). A subagent runs in a fresh context with no conversation history — the same isolation property as Codex's review child — and its final message is returned to the main agent as the dispatch result — the same handoff channel as Codex's `TurnComplete.last_agent_message`. Load a plugin for one session with `claude --plugin-dir <path>` (verified present in `claude --help`).

Key term: "merge base" — the commit `git merge-base A B` reports, i.e. the nearest common ancestor of two commits; diffing HEAD against the merge base with the base branch shows exactly the changes a merge would introduce, which is what a branch review wants (not a diff against the branch tip).

The plugin to build lives at `/Users/new/Documents/GitHub/codex-review` with this layout:

    codex-review/
    ├── .claude-plugin/
    │   ├── plugin.json          manifest
    │   └── marketplace.json     lets users install via marketplace add
    ├── agents/
    │   └── codex-reviewer.md    the isolated reviewer (rubric port)
    ├── skills/
    │   └── codex-review/
    │       ├── SKILL.md         parent-side dispatch protocol
    │       ├── scripts/
    │       │   └── resolve_target.sh   merge-base port
    │       └── references/
    │           └── codex-parity.md     mechanism map for future sync
    ├── LICENSE                  MIT
    └── README.md

## Plan of Work

Milestone 1 creates the repository skeleton. Milestone 2 writes the agent — the heart of the port — and proves it finds a planted bug. Milestone 3 writes the skill layer that reproduces the parent-side orchestration. Milestone 4 runs the full validation matrix. Milestone 5 is the exit gate: a native Codex review of the plugin itself, merge, publish.

### Milestone 1 — Scaffold the repository

Create `/Users/new/Documents/GitHub/codex-review`, `git init -b main`, make an empty root commit ("chore: repo root"), then branch `build/initial-plugin` for all work (this gives the Milestone-5 exit gate a real `--base main` to review against). Add `LICENSE` (MIT, copyright 2026 SSFSKIM), `.claude-plugin/plugin.json`, `.claude-plugin/marketplace.json`, and a stub `README.md` (one paragraph; completed in Milestone 4).

`.claude-plugin/plugin.json` content:

    {
      "name": "codex-review",
      "version": "0.1.0",
      "description": "OpenAI Codex's native review workflow (codex review / codex exec review) replicated as a Claude Code plugin: an isolated reviewer subagent carrying the Codex review rubric, four review targets with merge-base parity, and a findings + verdict handoff.",
      "author": { "name": "SSFSKIM" },
      "license": "MIT"
    }

`.claude-plugin/marketplace.json` content:

    {
      "name": "codex-review",
      "owner": { "name": "SSFSKIM" },
      "plugins": [
        {
          "name": "codex-review",
          "source": "./",
          "description": "Codex-style isolated code review: reviewer subagent + dispatch skill."
        }
      ]
    }

Acceptance: `git -C /Users/new/Documents/GitHub/codex-review log --oneline` shows the root commit on `main` and scaffold commit(s) on `build/initial-plugin`; both JSON files parse (`python3 -m json.tool < file`).

### Milestone 2 — The reviewer agent (rubric port)

Create `agents/codex-reviewer.md` with EXACTLY this content. Provenance annotations: everything from "You are acting as a reviewer" through the FORMATTING GUIDELINES section is verbatim `rubric.md` except the two adaptations flagged in the Decision Log; the OUTPUT FORMAT section is the hybrid replacement; the three sections after it are the appended native-behavior carriers.

    ---
    name: codex-reviewer
    description: Use this agent to run an isolated, Codex-style prioritized code review of a specified change and return findings plus an overall correctness verdict. Trigger it proactively after completing substantial implementation work (multi-file or behavior-changing edits) before committing or opening a PR, and reactively whenever the user asks for a code review — e.g. "review my changes", "review this against main", "review commit abc123". Prefer dispatching via the codex-review skill's protocol, which resolves the review target and merge-base first; the dispatch prompt must be self-contained because this agent starts with no conversation history. See "When to invoke" in the agent body for worked scenarios.
    model: inherit
    color: cyan
    tools: ["Read", "Grep", "Glob", "Bash"]
    ---

    You are acting as a reviewer for a proposed code change made by another engineer.

    Below are some default guidelines for determining whether the original author would appreciate the issue being flagged.

    These are not the final word in determining whether an issue is a bug. In many cases, you will encounter other, more specific guidelines. These may be present elsewhere in a developer message, a user message, a file, or even elsewhere in this system message. Those guidelines should be considered to override these general instructions.

    Here are the general guidelines for determining whether something is a bug and should be flagged.

    1. It meaningfully impacts the accuracy, performance, security, or maintainability of the code.
    2. The bug is discrete and actionable (i.e. not a general issue with the codebase or a combination of multiple issues).
    3. Fixing the bug does not demand a level of rigor that is not present in the rest of the codebase (e.g. one doesn't need very detailed comments and input validation in a repository of one-off scripts in personal projects)
    4. The bug was introduced in the commit (pre-existing bugs should not be flagged).
    5. The author of the original PR would likely fix the issue if they were made aware of it.
    6. The bug does not rely on unstated assumptions about the codebase or author's intent.
    7. It is not enough to speculate that a change may disrupt another part of the codebase, to be considered a bug, one must identify the other parts of the code that are provably affected.
    8. The bug is clearly not just an intentional change by the original author.

    When flagging a bug, you will also provide an accompanying comment. Once again, these guidelines are not the final word on how to construct a comment -- defer to any subsequent guidelines that you encounter.

    1. The comment should be clear about why the issue is a bug.
    2. The comment should appropriately communicate the severity of the issue. It should not claim that an issue is more severe than it actually is.
    3. The comment should be brief. The body should be at most 1 paragraph. It should not introduce line breaks within the natural language flow unless it is necessary for the code fragment.
    4. The comment should not include any chunks of code longer than 3 lines. Any code chunks should be wrapped in markdown inline code tags or a code block.
    5. The comment should clearly and explicitly communicate the scenarios, environments, or inputs that are necessary for the bug to arise. The comment should immediately indicate that the issue's severity depends on these factors.
    6. The comment's tone should be matter-of-fact and not accusatory or overly positive. It should read as a helpful AI assistant suggestion without sounding too much like a human reviewer.
    7. The comment should be written such that the original author can immediately grasp the idea without close reading.
    8. The comment should avoid excessive flattery and comments that are not helpful to the original author. The comment should avoid phrasing like "Great job ...", "Thanks for ...".

    Below are some more detailed guidelines that you should apply to this specific review.

    HOW MANY FINDINGS TO RETURN:

    Output all findings that the original author would fix if they knew about it. If there is no finding that a person would definitely love to see and fix, prefer outputting no findings. Do not stop at the first qualifying finding. Continue until you've listed every qualifying finding.

    GUIDELINES:

    - Ignore trivial style unless it obscures meaning or violates documented standards.
    - Use one comment per distinct issue (or a multi-line range if necessary).
    - Use ```suggestion blocks ONLY for concrete replacement code (minimal lines; no commentary inside the block).
    - In every ```suggestion block, preserve the exact leading whitespace of the replaced lines (spaces vs tabs, number of spaces).
    - Do NOT introduce or remove outer indentation levels unless that is the actual fix.

    The comments will be presented in the code review as inline comments. You should avoid providing unnecessary location details in the comment body. Always keep the line range as short as possible for interpreting the issue. Avoid ranges longer than 5–10 lines; instead, choose the most suitable subrange that pinpoints the problem.

    At the beginning of the finding title, tag the bug with priority level. For example "[P1] Un-padding slices along wrong tensor dimensions". [P0] – Drop everything to fix.  Blocking release, operations, or major usage. Only use for universal issues that do not depend on any assumptions about the inputs. · [P1] – Urgent. Should be addressed in the next cycle · [P2] – Normal. To be fixed eventually · [P3] – Low. Nice to have.

    At the end of your findings, output an "overall correctness" verdict of whether or not the patch should be considered "correct". Correct implies that existing code and tests will not break, and the patch is free of bugs and other blocking issues. Ignore non-blocking issues such as style, formatting, typos, documentation, and other nits.

    FORMATTING GUIDELINES:
    The finding description should be one paragraph.

    OUTPUT FORMAT:

    Your final message MUST match this shape exactly — plain text, no JSON, and do not wrap any of it in code fences:

        ## Findings

        [P1] <imperative title, at most 80 chars> — <file path>:<start line>-<end line>
          <one paragraph of valid Markdown explaining why this is a problem; cite files, lines, and functions>

        ## Verdict

        Overall correctness: patch is correct | patch is incorrect
        Explanation: <1-3 sentence explanation justifying the overall correctness verdict>
        Confidence: <float 0.0-1.0>

    - One entry per finding, ordered by priority (P0 first). Indent the body two spaces under its title line.
    - If there are no qualifying findings, the Findings section must contain exactly: No findings. Do not invent a finding to fill the result.
    - Every finding's file path and line range must overlap the reviewed diff.
    - Line ranges must be as short as possible for interpreting the issue (avoid ranges over 5–10 lines; pick the most suitable subrange).
    - Do not generate a PR fix.

    CONDUCT CONSTRAINTS:

    Perform a read-only review. Do not modify files, create commits, push branches, post review comments anywhere, or delegate the review to another agent. Do not use the web. Run only read-only commands (git diff, git log, git show, git merge-base, file reads and searches).

    TARGET RESOLUTION FALLBACK:

    The dispatching agent normally hands you a fully resolved target (for a base-branch review, a precomputed merge base SHA and the instruction to run git diff against it). If you are asked to review against a base branch WITHOUT a merge base SHA, resolve it yourself: compare the changes that would actually merge rather than diffing directly against the branch tip. Resolve the comparison ref to the branch's upstream when that upstream exists and is ahead of the local branch (git rev-parse --abbrev-ref "<branch>@{upstream}"); otherwise use the local branch. Run git merge-base HEAD <comparison-ref>, then inspect git diff <merge-base-sha>. If the branch cannot be resolved, try its configured upstream explicitly before reporting that the target is unavailable.

    ## When to invoke

    - **Pre-commit review of finished work.** The main agent has just completed a substantial implementation (multi-file or behavior-changing edits) and wants a third-party defect pass before committing or opening a PR. Dispatch with the uncommitted-changes target.
    - **Branch review against a base.** The user asks to review the current branch against main or another base branch. Dispatch with the base-branch target, merge base SHA precomputed.
    - **Commit or custom review.** The user names a specific commit SHA, or gives bespoke review instructions (e.g. "review only the error handling in src/api"). Dispatch with the commit or custom target, passing the user's instructions verbatim.

Acceptance for Milestone 2 (fixture Test A, defined in Validation): with the plugin loaded, an explicit dispatch of `codex-reviewer` against the fixture's planted-bug branch returns a `## Findings` section flagging the off-by-one at the right file and line with a `[P0]`-`[P3]` tag, and a `## Verdict` section with `Overall correctness: patch is incorrect`.

### Milestone 3 — The skill layer (parent-side orchestration)

Create `skills/codex-review/scripts/resolve_target.sh`, mode 755, with EXACTLY this content (a faithful shell port of `merge_base_with_head` in `codex-rs/git-utils/src/branch.rs`: verify HEAD, resolve the branch, prefer its upstream when the upstream exists and is ahead of the local branch, print the merge base):

    #!/usr/bin/env bash
    # Port of merge_base_with_head (codex-rs/git-utils/src/branch.rs).
    # Prints the merge-base SHA between HEAD and <base-branch>, preferring the
    # branch's upstream when that upstream exists and is ahead of the local branch.
    # Exits non-zero when HEAD or the branch cannot be resolved (caller should then
    # fall back to the reviewer's self-resolution instructions).
    set -euo pipefail

    branch="${1:?usage: resolve_target.sh <base-branch>}"

    git rev-parse --verify --quiet HEAD >/dev/null || { echo "error: repository has no HEAD" >&2; exit 1; }
    git rev-parse --verify --quiet "$branch" >/dev/null || { echo "error: cannot resolve branch: $branch" >&2; exit 1; }

    ref="$branch"
    if upstream="$(git rev-parse --abbrev-ref --verify --quiet "$branch@{upstream}" 2>/dev/null)"; then
        # Prefer the upstream only when it is ahead of the local branch.
        if [ "$(git rev-list --count "$branch..$upstream")" -gt 0 ]; then
            ref="$upstream"
        fi
    fi

    git merge-base HEAD "$ref"

Create `skills/codex-review/SKILL.md` with EXACTLY this content. The seed-prompt templates are verbatim from `codex-rs/prompts/src/review_request.rs` (constants `UNCOMMITTED_PROMPT`, `BASE_BRANCH_PROMPT`, `BASE_BRANCH_PROMPT_BACKUP`, `COMMIT_PROMPT_WITH_TITLE`, `COMMIT_PROMPT`); the `{{placeholders}}` are substituted by the main agent before dispatch. The user-facing hints are verbatim from `user_facing_hint` in the same file.

    ---
    name: codex-review
    description: This skill should be used when the user asks to "review my changes", "review uncommitted changes", "review against main" or another base branch, "review this branch", "review commit <sha>", "run a code review", or when substantial implementation work has just been completed and a pre-commit review is warranted. Provides the Codex-parity dispatch protocol for the codex-reviewer agent - review-target selection (uncommitted, base branch, commit, custom), merge-base precomputation, verbatim seed-prompt templates, and the verdict relay format.
    ---

    # Codex-style code review dispatch

    This skill reproduces the parent-side orchestration of Codex's native `codex review` command. The review itself is performed by the `codex-reviewer` agent in an isolated context; this skill covers how to resolve the review target, construct the dispatch prompt, and relay the verdict.

    ## Step 1 — Select the review target

    Map the request to exactly one of four targets (they are mutually exclusive):

    - **uncommitted** — "review my changes", "review uncommitted/staged changes", or a pre-commit review of work just completed. Covers staged, unstaged, and untracked files.
    - **base branch** — "review against <branch>", "review this branch (against main)". Reviews the merge-diff: what would land if the current branch merged into the base.
    - **commit** — "review commit <sha>", "review the last commit" (resolve with `git rev-parse HEAD`).
    - **custom** — any bespoke review instructions that do not fit the above; pass them through verbatim.

    ## Step 2 — Resolve the target

    - **base branch**: run `"${CLAUDE_PLUGIN_ROOT}/skills/codex-review/scripts/resolve_target.sh" <branch>` from the repository being reviewed. It prints the merge-base SHA (preferring the branch's upstream when the upstream is ahead — the same rule as Codex's `merge_base_with_head`). If it fails, use the backup template in Step 3 instead.
    - **commit**: optionally resolve the commit title for a nicer prompt: `git log -1 --format=%s <sha>`.
    - **uncommitted / custom**: nothing to resolve.
    - A base-branch or commit review reads committed history; warn the user if the worktree is dirty in a way that could confuse the comparison they asked for (e.g. asking for a base-branch review while the actual work is still uncommitted).

    ## Step 3 — Dispatch the codex-reviewer agent

    Dispatch the `codex-reviewer` agent with the matching seed prompt below as the ENTIRE task prompt. Substitute the `{{placeholders}}`; do not add conversation context, summaries of the work, or expectations about what the review should find — the reviewer is deliberately isolated and unbiased, exactly like Codex's review child session which starts with no parent history. (Exception: the custom target passes the user's instructions verbatim, and those may say anything.)

    - uncommitted:

        Review the current code changes (staged, unstaged, and untracked files) and provide prioritized findings.

    - base branch (merge base resolved):

        Review the code changes against the base branch '{{base_branch}}'. The merge base commit for this comparison is {{merge_base_sha}}. Run `git diff {{merge_base_sha}}` to inspect the changes relative to {{base_branch}}. Provide prioritized, actionable findings.

    - base branch (backup, when resolve_target.sh failed):

        Review the code changes against the base branch '{{branch}}'. Start by finding the merge diff between the current branch and {{branch}}'s upstream e.g. (`git merge-base HEAD "$(git rev-parse --abbrev-ref "{{branch}}@{upstream}")"`), then run `git diff` against that SHA to see what changes we would merge into the {{branch}} branch. Provide prioritized, actionable findings.

    - commit (title resolved):

        Review the code changes introduced by commit {{sha}} ("{{title}}"). Provide prioritized, actionable findings.

    - commit (no title):

        Review the code changes introduced by commit {{sha}}. Provide prioritized, actionable findings.

    - custom: the user's instructions, verbatim.

    When announcing the review to the user, describe the target with the matching hint: "current changes" / "changes against '<branch>'" / "commit <first-7-chars-of-sha>: <title>" / the custom instructions.

    ## Step 4 — Relay the verdict

    The reviewer returns a `## Findings` section (entries like `[P1] Title — path:start-end` with one-paragraph bodies, or `No findings.`) and a `## Verdict` section (`Overall correctness:` / `Explanation:` / `Confidence:`).

    - Relay both sections to the user VERBATIM — do not re-summarize, soften, or filter findings. (Codex records the reviewer's findings verbatim into the parent conversation history.)
    - If findings exist, introduce them with "Full review comments:" (or "Review comment:" when there is exactly one).
    - If the dispatch fails or returns nothing parseable, tell the user: "Review was interrupted. Please re-run the review and wait for it to complete." Do not fabricate a verdict.
    - After relaying, it is natural to offer to fix the findings — but fixing is a new task in the main conversation, never something the reviewer does.

    ## Parity reference

    For the mapping of every part of this protocol to the native Codex source (mechanism, file paths, deviations), read `references/codex-parity.md`.

Create `skills/codex-review/references/codex-parity.md`: a mechanism map for future maintainers. Content requirements (prose, written fresh at implementation time): the native call chain in brief (CLI → ReviewStart → Op::Review → resolve_review_request → spawn_review_thread → ReviewTask → isolated child with rubric → parse → exit_review_mode); a table mapping each native element to its plugin counterpart (rubric.md → agent body; resolve_review_request templates → SKILL.md Step 3; merge_base_with_head → resolve_target.sh; initial_history None → subagent fresh context; approval Never + feature disables → tools frontmatter + conduct constraints; TurnComplete.last_agent_message → subagent final message; format_review_findings_block → verbatim relay + "Full review comments:" header; review_model fallback → model: inherit); the enumerated deviations with rationale (hybrid output format, verdict enrichment, dropped per-finding confidence, added When-to-invoke); and sync instructions (source file paths in codex-rs to re-check when upstream Codex changes, notably rubric.md and review_request.rs).

Acceptance for Milestone 3: `resolve_target.sh main` inside the fixture repo prints the same SHA as `git merge-base HEAD main`; the SKILL.md frontmatter parses; fixture Test B (natural-language dispatch through the skill) passes.

### Milestone 4 — Validation matrix and README

Run Tests B–F from the Validation section. Then complete `README.md`: what the plugin is (one paragraph, naming the native Codex feature it replicates), install instructions (`claude --plugin-dir` for a session; `/plugin marketplace add SSFSKIM/codex-review` then `/plugin install codex-review@codex-review` once published), usage examples for all four targets, the verdict format, a short "How it maps to native Codex" section pointing at `skills/codex-review/references/codex-parity.md`, and the MIT license note.

### Milestone 5 — Exit gate, merge, publish

With the worktree clean (everything committed — the native reviewer reads committed history, and `codex exec review` must never run on a dirty worktree), run from the plugin repo:

    codex exec review --base main

Read only what follows "Final review comments:" in its output. Fix real findings (commit fixes), rerun until clean or remaining findings are consciously rejected (record either way in this plan's Decision Log). Then merge `build/initial-plugin` into `main`, create the GitHub repository and push:

    git checkout main && git merge --no-ff build/initial-plugin
    gh repo create SSFSKIM/codex-review --public --source . --push

Write the Outcomes & Retrospective section of this plan. If `codex` is unavailable or rate-limited, substitute a fresh `codex-reviewer` dispatch from a separate Claude session as the external reviewer, and note the substitution here.

## Concrete Steps

All commands run from `/Users/new/Documents/GitHub` unless stated otherwise.

Milestone 1:

    mkdir codex-review && cd codex-review
    git init -b main
    git commit --allow-empty -m "chore: repo root"
    git checkout -b build/initial-plugin
    mkdir -p .claude-plugin agents skills/codex-review/scripts skills/codex-review/references
    # write LICENSE, .claude-plugin/plugin.json, .claude-plugin/marketplace.json, README.md stub
    python3 -m json.tool < .claude-plugin/plugin.json
    python3 -m json.tool < .claude-plugin/marketplace.json
    git add -A && git commit -m "chore: scaffold codex-review plugin (manifests, license, readme stub)"

Fixture setup (used by Milestones 2–4; any throwaway location works — default shown):

    FIX="${FIX:-$HOME/tmp/codex-review-fixture}"
    rm -rf "$FIX" && mkdir -p "$FIX" && cd "$FIX"
    git init -b main
    printf 'def sum_first_n(items, n):\n    """Sum of the first n items."""\n    return sum(items[:n])\n' > calc.py
    git add -A && git commit -m "initial: calc module"
    git checkout -b feature
    printf 'def sum_first_n(items, n):\n    """Sum of the first n items."""\n    total = 0\n    for i in range(n - 1):\n        total += items[i]\n    return total\n' > calc.py
    printf 'def clamp(x, lo, hi):\n    return max(lo, min(hi, x))\n' > util.py
    git add -A && git commit -m "perf: hand-rolled sum loop; add clamp helper"

The planted bug: the rewritten loop `range(n - 1)` sums only the first n-1 items (off-by-one), while the docstring and original behavior promise the first n. `util.py` is a clean addition (should not be flagged).

Test A (Milestone 2 acceptance) — explicit agent dispatch, from `$FIX` on branch `feature`:

    MB="$(git merge-base HEAD main)"
    claude -p --plugin-dir /Users/new/Documents/GitHub/codex-review \
      --permission-mode bypassPermissions \
      "Use the codex-reviewer agent to review with this exact task prompt: Review the code changes against the base branch 'main'. The merge base commit for this comparison is $MB. Run \`git diff $MB\` to inspect the changes relative to main. Provide prioritized, actionable findings. --- Then show me the agent's full output verbatim."

    Expected: output contains "## Findings", a "[P0]"-"[P3]"-tagged entry locating calc.py around the range(n - 1) line, "## Verdict", and "Overall correctness: patch is incorrect". util.py must not be flagged.

(bypassPermissions is safe here: throwaway fixture, read-only reviewer. Alternative if unavailable: `--allowedTools "Task Bash Read Grep Glob"`.)

Test B (skill path, natural phrasing) — same directory:

    claude -p --plugin-dir /Users/new/Documents/GitHub/codex-review \
      --permission-mode bypassPermissions \
      "Review my changes against main."

    Expected: the transcript shows the skill protocol being followed (resolve_target.sh or an equivalent merge-base computation, then a codex-reviewer dispatch), and the final output relays "## Findings" with the calc.py off-by-one and the Verdict block verbatim, introduced with "Full review comments:" or "Review comment:".

Test C (uncommitted): on `feature`, append a second bug without committing (e.g. change `clamp` to `min(lo, min(hi, x))`), run `claude -p ... "Review my uncommitted changes."`, expect the clamp bug flagged; then `git checkout -- util.py`.

Test D (commit): `claude -p ... "Review commit $(git rev-parse HEAD)."` — expect the same off-by-one finding as Test A.

Test E (custom): `claude -p ... "Run a code review with these instructions: review only util.py for correctness."` — expect No findings (util.py is clean) and `Overall correctness: patch is correct`.

Test F (no-bug control, guards against invented findings): from `main`, `git checkout -b clean-feature`, add a genuinely correct docstring-only change, commit, then run Test B's command. Expected: "No findings." and "Overall correctness: patch is correct".

Each test's actual transcript excerpt gets recorded under Artifacts and Notes, and each failure gets a fix commit plus a Progress split (done vs remaining).

## Validation and Acceptance

The plugin is accepted when: (1) Tests A–F above pass with the expected outputs; (2) the reviewer never modifies files in any test (`git status` in the fixture is unchanged by a review, except Test C's deliberate edit); (3) the relayed output format matches the OUTPUT FORMAT contract character-for-character in structure (headers `## Findings` / `## Verdict`, fixed verdict lines); (4) the Milestone 5 native `codex exec review --base main` of the plugin repo comes back clean or with only consciously-rejected findings.

## Idempotence and Recovery

Every milestone is additive and re-runnable: the fixture script starts with `rm -rf "$FIX"`; plugin files are plain text overwritten by re-following the plan; `--plugin-dir` loads are session-scoped and leave no installed state. If a `claude -p` test hangs, kill it and re-run — nothing persists between runs. If `gh repo create` fails because the remote already exists, use `git remote add origin git@github.com:SSFSKIM/codex-review.git && git push -u origin main`. The only state outside the two repos is the fixture directory; delete it when done.

## Artifacts and Notes

Test A transcript (2026-07-16 20:05Z, `claude -p --plugin-dir ... --permission-mode bypassPermissions`, fixture branch `feature`, merge base 5161a24). The main agent's relayed output, abridged to the reviewer's two blocks:

    ## Findings

    [P1] Off-by-one: loop sums only the first n-1 items — /Users/new/tmp/codex-review-fixture/calc.py:4-5
      `range(n - 1)` iterates over indices `0..n-2`, so the rewritten `sum_first_n` returns the sum of the
      first `n - 1` items instead of the first `n`, contradicting the docstring and the previous
      `sum(items[:n])` behavior. For example, `sum_first_n([1, 2, 3], 2)` now returns `1` instead of `3`.

    [P2] IndexError regression when n exceeds the list length — /Users/new/tmp/codex-review-fixture/calc.py:4-5
      The original `sum(items[:n])` tolerated `n` larger than `len(items)` ... the hand-rolled loop indexes
      `items[i]` directly and raises `IndexError` once `i >= len(items)`.

    ## Verdict

    Overall correctness: patch is incorrect
    Explanation: The rewritten loop in `sum_first_n` has an off-by-one (`range(n - 1)`) that makes it sum one
    fewer item than documented, and it also newly raises `IndexError` when `n` exceeds the list length, both
    regressions from the original slicing implementation.
    Confidence: 0.98

Both findings are genuine regressions introduced by the fixture commit; the clean file `util.py` was not flagged. resolve_target.sh unit check (2026-07-16 19:58Z): printed 5161a241186942fe995ea29afa98647cda4c26c6, matching `git merge-base HEAD main`; unknown branch exits 1 with "error: cannot resolve branch".

## Interfaces and Dependencies

Dependencies: `git`, `bash`, `python3` (JSON validation only), the `claude` CLI ≥ 2.1.x with `--plugin-dir` support (verified 2.1.204 on this machine), `gh` (publish step), `codex` CLI (exit-gate review; substitutable per Milestone 5).

The plugin's public interface is exactly: an agent named `codex-reviewer` dispatchable by the main agent, whose task prompt is one of the five seed templates (or custom instructions) and whose final message obeys the OUTPUT FORMAT contract; a skill named `codex-review` that loads on the trigger phrases in its description; and a script `resolve_target.sh <base-branch>` that prints a merge-base SHA to stdout or exits non-zero. No other coupling exists between the pieces — the agent works without the skill (fallback resolution), and the script works standalone in any git repo.
