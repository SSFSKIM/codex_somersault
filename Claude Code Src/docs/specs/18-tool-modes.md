# 18 — Tool Modes (Plan Mode + Worktree Mode)

> **Scope.** The four mode-affecting tools that mutate session state instead of producing tool output: `EnterPlanMode`, `ExitPlanMode` (V2), `EnterWorktree`, `ExitWorktree`. They are registered in the standard pool (`tools.ts`) but their `call()` methods primarily transition `toolPermissionContext.mode` (plan) or session CWD/`currentWorktreeSession` (worktree).
>
> **Out of scope.** Permission-system mechanics → 09. Tool registry/`buildTool` factory → 08. Proactive interactions → 31. Plan-mode interview-phase attachment generation → 04/05. Worktree session persistence → 41. `--worktree` startup path → 01.

---

## 1. Purpose & Scope

`EnterPlanMode` requests user approval to enter `mode='plan'`, after which Claude is constrained to read-only exploration. `ExitPlanMode` (named `ExitPlanModeV2Tool` in source, registered as `'ExitPlanMode'`) presents the on-disk plan for user approval and restores the pre-plan mode. `EnterWorktree` creates a git (or hook-based) worktree under `.claude/worktrees/` and `chdir`s the session into it. `ExitWorktree` reverses that, optionally removing the worktree+branch.

Plan-mode tools are always registered; their `isEnabled()` returns `false` only under the paired KAIROS+channels gate `(feature('KAIROS') || feature('KAIROS_CHANNELS')) && getAllowedChannels().length > 0` (see §5.9, §6.10). On stripped builds where neither KAIROS feature flag is set, `--channels` has **no effect** and the plan tools remain enabled regardless of `getAllowedChannels()`. (Phrasing such as "gated only by `--channels`" is incomplete — the feature-flag dependency is load-bearing for cross-spec consistency with 09 and the Phase 9.6 ripple.) Worktree tools are registered iff `isWorktreeModeEnabled()` returns `true` — which it currently does unconditionally (`utils/worktreeModeEnabled.ts:9-11`); see §3 and §8 for the failure-mode contract if a downstream caller stubs it `false`.

## 2. Source Map

### 2.1 In scope (this spec)

| Path | Lines | Role |
|---|---|---|
| `src/tools/EnterPlanModeTool/EnterPlanModeTool.ts` | 1-126 | Tool def; mutates `toolPermissionContext.mode='plan'` |
| `src/tools/EnterPlanModeTool/prompt.ts` | 1-170 | Default + ANT prompt variants (split at `:167`) |
| `src/tools/EnterPlanModeTool/constants.ts` | 1 | `ENTER_PLAN_MODE_TOOL_NAME = 'EnterPlanMode'` |
| `src/tools/EnterPlanModeTool/UI.tsx` | 1-32 | Renders "Entered plan mode" / "User declined" |
| `src/tools/ExitPlanModeTool/ExitPlanModeV2Tool.ts` | 1-493 | Tool def; teammate fork, gate-fallback, mode restore |
| `src/tools/ExitPlanModeTool/prompt.ts` | 1-29 | External-stub prompt (single variant) |
| `src/tools/ExitPlanModeTool/constants.ts` | 1-2 | `EXIT_PLAN_MODE_TOOL_NAME = EXIT_PLAN_MODE_V2_TOOL_NAME = 'ExitPlanMode'` |
| `src/tools/ExitPlanModeTool/UI.tsx` | 1-81 | Approved/empty/awaiting-leader render branches |
| `src/tools/EnterWorktreeTool/EnterWorktreeTool.ts` | 1-127 | Tool def; calls `createWorktreeForSession` and `chdir`s |
| `src/tools/EnterWorktreeTool/prompt.ts` | 1-30 | Single prompt (no ANT split) |
| `src/tools/EnterWorktreeTool/constants.ts` | 1 | `ENTER_WORKTREE_TOOL_NAME = 'EnterWorktree'` |
| `src/tools/EnterWorktreeTool/UI.tsx` | 1-19 | Renders "Switched to worktree on branch X" |
| `src/tools/ExitWorktreeTool/ExitWorktreeTool.ts` | 1-329 | Tool def; `keep`/`remove`, fail-closed change counting |
| `src/tools/ExitWorktreeTool/prompt.ts` | 1-32 | Single prompt |
| `src/tools/ExitWorktreeTool/constants.ts` | 1 | `EXIT_WORKTREE_TOOL_NAME = 'ExitWorktree'` |
| `src/tools/ExitWorktreeTool/UI.tsx` | 1-24 | Renders "Kept worktree" / "Removed worktree" |
| `src/utils/worktreeModeEnabled.ts` | 1-11 | `isWorktreeModeEnabled() → true` (unconditional) |
| `src/utils/planModeV2.ts` | 50-62 | `isPlanModeInterviewPhaseEnabled()` |

### 2.2 Cited but spec'd elsewhere

| Path | Owner | Why cited here |
|---|---|---|
| `src/tools.ts:202,213,225` | 08 | Registration order; worktree gate site |
| `src/utils/permissions/permissionSetup.ts:1462-1493` | 09 | `prepareContextForPlanMode` (verbatim in §6.5) |
| `src/utils/permissions/permissionSetup.ts:1502-1532` | 09 | `transitionPlanAutoMode` (mid-plan auto-mode reconcile) |
| `src/bootstrap/state.ts:1349-1363` | 09/41 | `handlePlanModeTransition` (verbatim in §6.5) |
| `src/utils/worktree.ts` | (this spec for tool-side; deeper git internals refer here) | `createWorktreeForSession`, `keepWorktree`, `cleanupWorktree`, slug validation, branch naming |
| `src/utils/plans.ts` | (referenced) | `getPlanFilePath`, `getPlan`, `persistFileSnapshotIfRemote`, `getPlanSlug`, `getPlansDirectory` |

## 3. Glossary

- **Plan mode** — `toolPermissionContext.mode === 'plan'`. Read-only writes blocked (`isReadOnly()` enforcement in 09); only Glob/Grep/Read/AskUserQuestion/EnterPlanMode/ExitPlanMode and the plan file itself are writable.
- **`prePlanMode`** — Field on `ToolPermissionContext`. Holds the mode the user was in before entering plan mode, so `ExitPlanMode` restores it. Set by `prepareContextForPlanMode`; cleared on plan exit.
- **Plan slug** — Word slug generated lazily per session (`getPlanSlug` → `generateWordSlug`, retried up to 10× to avoid filename collision in `getPlansDirectory()`). Plan file is `<plansDir>/<slug>.md` (or `<slug>-agent-<agentId>.md` for subagents). `plans.ts:32-49,119-128`.
- **Plans directory** — `settings.plansDirectory` (project-relative; rejected if it escapes cwd) else `<claudeConfigHome>/plans`. Memoized to avoid repeated `mkdirSync`. `plans.ts:79-111`.
- **Worktree session** — Module-level mutable singleton `currentWorktreeSession: WorktreeSession | null` (`worktree.ts:140-156`). Set by `createWorktreeForSession`, nulled by `keepWorktree`/`cleanupWorktree`, persisted to project config and to `saveWorktreeState` for resume.
- **Worktree slug** — User-supplied or auto-generated (`getPlanSlug()` is reused as default, `EnterWorktreeTool.ts:90`). Validated against `^[a-zA-Z0-9._-]+$` per `/`-separated segment, max 64 chars total. `/` allowed for nesting, then flattened to `+` for branch and dir.
- **Hook-based worktree** — When `hasWorktreeCreateHook()` returns true, `createWorktreeForSession` delegates to the WorktreeCreate hook; `cleanupWorktree` delegates to WorktreeRemove. Used outside git repos. Sets `WorktreeSession.hookBased = true`.
- **Awaiting-leader-approval (teammate plan)** — When `isTeammate() && isPlanModeRequired()`, `ExitPlanMode` writes a `plan_approval_request` JSON envelope to the team-lead mailbox instead of restoring mode locally.
- **Channels gate** — `(feature('KAIROS') || feature('KAIROS_CHANNELS')) && getAllowedChannels().length > 0` disables BOTH `EnterPlanMode` and `ExitPlanMode` (paired so plan mode isn't a trap).

## 4. Architecture

```
                        ┌──────────────────────────────┐
EnterPlanMode.call()    │  toolPermissionContext       │
   │                    │    mode: 'default'           │
   ▼                    │    prePlanMode: undefined    │
handlePlanModeTransition│                              │
  ('default','plan')    └──────────────┬───────────────┘
   │                                   │ setAppState
   ▼                                   ▼
applyPermissionUpdate(       ┌──────────────────────────────┐
  prepareContextForPlanMode( │  toolPermissionContext       │
    ctx),                    │    mode: 'plan'              │
  {setMode plan, session})   │    prePlanMode: 'default'    │
                             │    [maybe stripped rules]    │
                             └──────────────┬───────────────┘
                                            │ tool_result instructs
                                            │ "explore + design + ExitPlanMode"
                                            │
                          (model writes plan file at getPlanFilePath())
                                            │
                                            ▼
                          ExitPlanModeV2Tool.call()
                            │
                            ├── teammate+isPlanModeRequired? ──► writeToMailbox(team-lead, plan_approval_request)
                            │                                    setAwaitingPlanApproval(taskId,true)
                            │                                    return {awaitingLeaderApproval:true}
                            │
                            └── else: setHasExitedPlanMode(true)
                                       setNeedsPlanModeExitAttachment(true)
                                       restoreMode = ctx.prePlanMode ?? 'default'
                                       [TRANSCRIPT_CLASSIFIER: gate-fallback to default if auto unavailable]
                                       toggle stripped/restored dangerous rules
                                       prePlanMode := undefined
```

```
EnterWorktree.call()                       ExitWorktree.call()
  │                                          │
  │ if getCurrentWorktreeSession() throw     │ validateInput:
  │ findCanonicalGitRoot → chdir to repo     │   if !session: no-op (errorCode 1)
  │ slug = input.name ?? getPlanSlug()       │   if remove && !discard_changes:
  │ createWorktreeForSession(sid,slug)       │     countWorktreeChanges(); if >0 refuse
  │   ├─ hookBased: WorktreeCreate hook      │ call:
  │   └─ git: getOrCreateWorktree(root,slug) │   action='keep' → keepWorktree()
  │      → .claude/worktrees/<slug+slug>     │   action='remove' → killTmuxSession?
  │      → branch worktree-<slug+slug>       │                     cleanupWorktree()
  │ process.chdir(worktreePath)              │ restoreSessionToOriginalCwd()
  │ setOriginalCwd, saveWorktreeState        │   setCwd, setOriginalCwd,
  │ clearSystemPromptSections                │   setProjectRoot iff projectRootIsWorktree,
  │ clearMemoryFileCaches                    │   updateHooksConfigSnapshot iff so,
  │ getPlansDirectory.cache.clear            │   saveWorktreeState(null),
  │ logEvent('tengu_worktree_created')       │   clearSystemPromptSections,
  ▼                                          │   clearMemoryFileCaches,
  return {worktreePath,branch,message}       │   getPlansDirectory.cache.clear
                                             ▼ logEvent('tengu_worktree_kept' | _removed)
```

## 5. Key Decisions / Algorithms

### 5.1 Plan-mode entry — guards before mode mutation

Cited from `EnterPlanModeTool.ts:77-94`:

1. `if (context.agentId)` → throw `'EnterPlanMode tool cannot be used in agent contexts'`. Subagents must not toggle the parent's mode. **Phase 9.6 cross-link to spec 09 (forked / bubble subagents).** The deferred-tool list announces `EnterPlanMode` regardless of `agentId` (per §5.7 — the announcement is registry-level, not context-aware). Therefore when a forked / "bubble" subagent (spec 09's bubble forked-subagent mechanism) is in scope, the model can still see and call `EnterPlanMode`; the throw at `EnterPlanModeTool.ts:78` propagates as a **normal tool error**, NOT as a permission decline (no `checkPermissions` rejection, no permission-UI prompt). Spec 09's bubble-subagent contract must treat this as the documented failure mode for plan-mode entry attempts inside any non-null `agentId` context.
2. `handlePlanModeTransition(prevMode, 'plan')` runs FIRST — clears any pending plan-mode-exit attachment.
3. `applyPermissionUpdate(prepareContextForPlanMode(ctx), {setMode plan, session})`. The order matters: `prepareContextForPlanMode` records `prePlanMode` and may strip dangerous permissions for auto-mode-during-plan; then `applyPermissionUpdate` flips `mode` to `plan`.

### 5.2 Plan-mode exit — five execution branches

`ExitPlanModeV2Tool.ts:243-417`. After the `validateInput` mode-check (rejects with `errorCode:1` if not in plan mode, `:204-218`):

1. **Edited-plan disk sync** (`:251-261`). If `'plan'` was injected via `permissionResult.updatedInput` (CCR web UI / Ctrl+G edit), write it to `getPlanFilePath(agentId)` and call `persistFileSnapshotIfRemote()` so VerifyPlanExecution and Read see the edit.
2. **Teammate + plan-required leader approval** (`:264-313`). Build `plan_approval_request` JSON; `writeToMailbox('team-lead', …)`; set `awaitingPlanApproval` on the in-process task; return `{awaitingLeaderApproval:true, requestId}`.
3. **Auto-mode gate-fallback notification** (`:328-355`). If `feature('TRANSCRIPT_CLASSIFIER')` and `prePlanMode==='auto'` but `isAutoModeGateEnabled()` is false, build a `gateFallbackNotification` and pass it to `addNotification(key='auto-mode-gate-plan-exit-fallback', priority='immediate', timeoutMs=10000)` BEFORE the setAppState — the user sees the fallback reason before mode actually changes.
4. **Mode restore** (`:357-403`). Inside `setAppState`:
   - If `prev.toolPermissionContext.mode !== 'plan'`, no-op (defensive).
   - `setHasExitedPlanMode(true)` + `setNeedsPlanModeExitAttachment(true)`.
   - `restoreMode = prev.toolPermissionContext.prePlanMode ?? 'default'`. Override to `'default'` if `prePlanMode==='auto'` and gate is disabled.
   - Read `autoModeStateModule.isAutoModeActive()` BEFORE calling `setAutoModeActive(restoringToAuto)` — this is the authoritative signal because `prePlanMode`/`strippedDangerousRules` are stale after `transitionPlanAutoMode` deactivates mid-plan.
   - If `restoringToAuto`, re-strip dangerous permissions. Else if `strippedDangerousRules` is set, restore them.
   - Final ctx: `{...baseContext, mode: restoreMode, prePlanMode: undefined}`.
5. **Tool-result content selection** (`mapToolResultToToolResultBlockParam :419-492`):
   - awaitingLeaderApproval → `"Your plan has been submitted to the team lead for approval"` block.
   - isAgent → `"User has approved the plan. … Please respond with \"ok\""`.
   - empty plan → `"User has approved exiting plan mode."`.
   - default → echo plan with label `"Approved Plan"` or `"Approved Plan (edited by user)"` + optional `teamHint` if `AGENT_TOOL_NAME` is in the tool pool and `isAgentSwarmsEnabled()`.

### 5.3 Plan-mode permission gate (cite-only — owned by 09)

The plan permission gate is enforced by the standard permission system (spec 09). EnterPlanMode does not introduce a new gate; it just sets `mode='plan'`, after which any non-readonly tool invocation is rejected by the existing decision tree. The mode-transition pseudocode lives in §6.5.

### 5.4 Worktree entry — slug→branch→path flattening

From `worktree.ts:48-87, 217-227`:

- Slug regex per segment: `/^[a-zA-Z0-9._-]+$/`. Max total length 64. `.` and `..` rejected. `/` is allowed only as separator.
- Branch name: `worktree-<slug-with-/-replaced-by-+>` (`worktreeBranchName`).
- Directory: `<repoRoot>/.claude/worktrees/<slug-with-/-replaced-by-+>` (`worktreePathFor`).
- Reason for `+` flattening (verbatim comment cited at `worktree.ts:208-216`): nested `user/feature` would create a D/F conflict in git refs and would let `git worktree remove` on a parent destroy a child's uncommitted work.

`EnterWorktreeTool.call` (`:77-118`):

1. Reject if already in a worktree (`getCurrentWorktreeSession() !== null`).
2. `findCanonicalGitRoot(getCwd())` — if non-null and ≠ cwd, `process.chdir` and `setCwd` to that root. (Allows entering a worktree from inside another worktree.)
3. `slug = input.name ?? getPlanSlug()` — auto-name reuses the session's plan slug. **Side effect:** `getPlanSlug()` (`plans.ts:32-49`) is *not pure*. On cache miss it generates a random word slug, retries up to 10× to avoid a `<plansDir>/<slug>.md` collision (`plans.ts:39-45`), and **commits the slug into `getPlanSlugCache()` for the session** (`plans.ts:46`). Therefore calling `EnterWorktreeTool` without `name` *materializes* the session's plan slug, which then leaks into every subsequent plan-file path in the same session (including any later `EnterPlanMode` → `getPlanFilePath()` resolution). Cross-spec impact: 04 (plan attachment), 05 (context assembly), 41 (resume — `setPlanSlug` at `plans.ts:54-55` is the resume-side reciprocal).
4. `createWorktreeForSession(getSessionId(), slug)`:
   - If `hasWorktreeCreateHook()`: run hook, set `hookBased: true` in `WorktreeSession`.
   - Else: `getOrCreateWorktree(gitRoot, slug)` — fast resume via `readWorktreeHeadSha`; else `mkdir(<root>/.claude/worktrees, recursive)` then create branch and add worktree (full algorithm lives in `worktree.ts:235+`, owned at the git layer).
5. `process.chdir(worktreePath)`, `setCwd`, `setOriginalCwd(getCwd())` — note `setOriginalCwd` is set to the *worktree* path, intentional (see ExitWorktree §5.5).
6. `saveWorktreeState(worktreeSession)` (session-state persistence; spec 41).
7. Cache invalidation: `clearSystemPromptSections()`, `clearMemoryFileCaches()`, `getPlansDirectory.cache.clear?.()`.
8. `logEvent('tengu_worktree_created', {mid_session:true})`.
9. Return `worktreePath`, `worktreeBranch`, message string.

### 5.5 Worktree exit — fail-closed change accounting

From `ExitWorktreeTool.ts:79-113, 174-224, 227-321`.

`countWorktreeChanges` returns `null` (treated as unsafe) when:
- `git status --porcelain` exits non-zero (lock/corrupt/bad ref).
- `git rev-list --count <originalHeadCommit>..HEAD` exits non-zero.
- `originalHeadCommit` is undefined but git status succeeded — the hook-based-worktree-wrapping-git case where no baseline exists.

`validateInput` algorithm:

1. If `getCurrentWorktreeSession()` is null → `errorCode:1` no-op.
2. If `action==='remove' && !discard_changes`:
   - `countWorktreeChanges` → if `null`, refuse with `errorCode:3` (verify-failure).
   - If `changedFiles>0 || commits>0`, refuse with `errorCode:2` and a message listing both counts (singular/plural-aware: `"1 file"` vs `"2 files"`, `"1 commit"` vs `"2 commits"`).

`call` algorithm:

1. Capture `originalCwd, worktreePath, worktreeBranch, tmuxSessionName, originalHeadCommit` BEFORE calling `keepWorktree`/`cleanupWorktree` (which null `currentWorktreeSession`).
2. Compute `projectRootIsWorktree = getProjectRoot() === getOriginalCwd()` — this distinguishes `--worktree` startup (where projectRoot was set to the worktree, so must be restored) from mid-session EnterWorktreeTool (where projectRoot was untouched). **Cross-spec contract (owned by 01).** This discriminator is load-bearing and silently relies on the `--worktree` startup block in `src/setup.ts` calling `setCwd(worktreePath)` → `setOriginalCwd(getCwd())` → `setProjectRoot(getCwd())` back-to-back without intervening `chdir`/`setCwd` mutation. Verified at `src/setup.ts:272-277` (the line-number citation in `ExitWorktreeTool.ts:244-248` is a stale comment; the actual setters are at 272/273/277, not 235/239 — confirmed in this leak's tree). If 01 ever reorders or interleaves a `setCwd` between `setOriginalCwd` and `setProjectRoot`, the `projectRootIsWorktree` test would silently flip false and mid-session ExitWorktree would clobber `projectRoot`. Spec 01 must keep the back-to-back invariant — pin the contract by name (`setOriginalCwd(getCwd()); … setProjectRoot(getCwd())` in the `--worktree` post-`createWorktreeForSession` block) rather than by line number.
3. Re-count changes for analytics (null → 0/0; safety gating already happened at validate time).
4. action='keep': `keepWorktree()` → `restoreSessionToOriginalCwd(originalCwd, projectRootIsWorktree)` → `logEvent('tengu_worktree_kept', {mid_session:true,commits,changed_files:changedFiles})`. Returns message including tmux reattach note if `tmuxSessionName` was set.
5. action='remove': `killTmuxSession(tmuxSessionName)` if set → `cleanupWorktree()` → `restoreSessionToOriginalCwd` → `logEvent('tengu_worktree_removed', …)` → message lists discarded `commits`/`files`.

`restoreSessionToOriginalCwd` (`:122-146`):

- `setCwd(originalCwd)`, `setOriginalCwd(originalCwd)` always.
- `setProjectRoot(originalCwd)` and `updateHooksConfigSnapshot()` only when `projectRootIsWorktree` (the `--worktree`-startup case).
- `saveWorktreeState(null)`, `clearSystemPromptSections()`, `clearMemoryFileCaches()`, `getPlansDirectory.cache.clear?.()`.

### 5.6 Worktree permission isolation (refer to 09)

Per-tool worktree isolation is the standard tool-permission model — there is no extra gate in this spec. The worktree mutates `cwd`/`originalCwd`/`projectRoot`, after which existing path-prefix rules in 09 evaluate against the new root. The `EnterWorktreeTool`/`ExitWorktreeTool` themselves do NOT define `checkPermissions`; they rely on `shouldDefer:true` (see §5.7) and the registry-level decision tree.

### 5.7 `shouldDefer: true` (all four tools)

All four tools set `shouldDefer:true`. Per spec 08, deferred tools are surfaced in the deferred-tool list and called via `ToolSearch`. ExitPlanMode's pre-`checkPermissions` `validateInput` mode-check (`:202-218`) explicitly cites the deferred-tool announcement: "the deferred-tool list announces this tool regardless of mode, so the model can call it after plan approval (fresh delta on compact/clear). Reject before checkPermissions to avoid showing the approval dialog."

### 5.8 ExitPlanMode permission UI

`checkPermissions` (`:221-238`):

- If `isTeammate()` → `behavior:'allow'` (skip permission UI; mailbox path handles authorization).
- Else → `behavior:'ask', message:'Exit plan mode?'`. The TUI dialog runs.

`requiresUserInteraction` (`:185-194`): returns `false` for ALL teammates (whether plan-mode-required or voluntary), `true` otherwise.

### 5.9 Channels-mode disable (paired)

Both EnterPlanMode (`isEnabled` `:56-67`) and ExitPlanMode (`isEnabled` `:167-178`) disable themselves when `(feature('KAIROS') || feature('KAIROS_CHANNELS')) && getAllowedChannels().length > 0`. Comments verbatim: EnterPlanMode "Disable entry too so plan mode isn't a trap the model can enter but never leave"; ExitPlanMode "the plan-approval dialog would hang. Paired with the same gate on EnterPlanMode so plan mode isn't a trap."

### 5.10 Interview-phase prompt branching

`isPlanModeInterviewPhaseEnabled()` (`planModeV2.ts:50-62`):

- `USER_TYPE === 'ant'` → always `true`.
- Else: `CLAUDE_CODE_PLAN_MODE_INTERVIEW_PHASE` env var (truthy/falsy), else GrowthBook `tengu_plan_mode_interview_phase` (default `false`).

Used in two places in this spec:
- `EnterPlanModeTool.mapToolResultToToolResultBlockParam` (`:103-118`) — when enabled, replaces the 6-step "In plan mode, you should:" instructions with `"DO NOT write or edit any files except the plan file. Detailed workflow instructions will follow."`.
- `EnterPlanModeTool/prompt.ts` `getEnterPlanModeToolPromptExternal`/`Ant` (`:19-21, 104-106`) — when enabled, omit the `WHAT_HAPPENS_SECTION`.

## 6. Verbatim Assets

### 6.1 Tool names (constants)

```ts
// EnterPlanModeTool/constants.ts
export const ENTER_PLAN_MODE_TOOL_NAME = 'EnterPlanMode'

// ExitPlanModeTool/constants.ts
export const EXIT_PLAN_MODE_TOOL_NAME = 'ExitPlanMode'
export const EXIT_PLAN_MODE_V2_TOOL_NAME = 'ExitPlanMode'

// EnterWorktreeTool/constants.ts
export const ENTER_WORKTREE_TOOL_NAME = 'EnterWorktree'

// ExitWorktreeTool/constants.ts
export const EXIT_WORKTREE_TOOL_NAME = 'ExitWorktree'
```

### 6.2 Worktree path/branch constants and slug regex

From `src/utils/worktree.ts:48-49, 204-227`:

```ts
const VALID_WORKTREE_SLUG_SEGMENT = /^[a-zA-Z0-9._-]+$/
const MAX_WORKTREE_SLUG_LENGTH = 64

function worktreesDir(repoRoot: string): string {
  return join(repoRoot, '.claude', 'worktrees')
}

function flattenSlug(slug: string): string {
  return slug.replaceAll('/', '+')
}

export function worktreeBranchName(slug: string): string {
  return `worktree-${flattenSlug(slug)}`
}

function worktreePathFor(repoRoot: string, slug: string): string {
  return join(worktreesDir(repoRoot), flattenSlug(slug))
}
```

Worktree git-no-prompt env (`worktree.ts:199-202`):

```ts
const GIT_NO_PROMPT_ENV = {
  GIT_TERMINAL_PROMPT: '0',
  GIT_ASKPASS: '',
}
```

### 6.3 Input/Output Zod schemas

**EnterPlanModeTool** (`EnterPlanModeTool.ts:21-33`):

```ts
const inputSchema = lazySchema(() =>
  z.strictObject({
    // No parameters needed
  }),
)
const outputSchema = lazySchema(() =>
  z.object({
    message: z.string().describe('Confirmation that plan mode was entered'),
  }),
)
```

**ExitPlanModeV2Tool** (`ExitPlanModeV2Tool.ts:64-142`):

```ts
const allowedPromptSchema = lazySchema(() =>
  z.object({
    tool: z.enum(['Bash']).describe('The tool this prompt applies to'),
    prompt: z
      .string()
      .describe(
        'Semantic description of the action, e.g. "run tests", "install dependencies"',
      ),
  }),
)

const inputSchema = lazySchema(() =>
  z
    .strictObject({
      allowedPrompts: z
        .array(allowedPromptSchema())
        .optional()
        .describe(
          'Prompt-based permissions needed to implement the plan. These describe categories of actions rather than specific commands.',
        ),
    })
    .passthrough(),
)

export const _sdkInputSchema = lazySchema(() =>
  inputSchema().extend({
    plan: z
      .string()
      .optional()
      .describe('The plan content (injected by normalizeToolInput from disk)'),
    planFilePath: z
      .string()
      .optional()
      .describe('The plan file path (injected by normalizeToolInput)'),
  }),
)

export const outputSchema = lazySchema(() =>
  z.object({
    plan: z.string().nullable()
      .describe('The plan that was presented to the user'),
    isAgent: z.boolean(),
    filePath: z.string().optional()
      .describe('The file path where the plan was saved'),
    hasTaskTool: z.boolean().optional()
      .describe('Whether the Agent tool is available in the current context'),
    planWasEdited: z.boolean().optional()
      .describe(
        'True when the user edited the plan (CCR web UI or Ctrl+G); determines whether the plan is echoed back in tool_result',
      ),
    awaitingLeaderApproval: z.boolean().optional()
      .describe(
        'When true, the teammate has sent a plan approval request to the team leader',
      ),
    requestId: z.string().optional()
      .describe('Unique identifier for the plan approval request'),
  }),
)
```

**EnterWorktreeTool** (`EnterWorktreeTool.ts:23-50`):

```ts
const inputSchema = lazySchema(() =>
  z.strictObject({
    name: z
      .string()
      .superRefine((s, ctx) => {
        try {
          validateWorktreeSlug(s)
        } catch (e) {
          ctx.addIssue({ code: 'custom', message: (e as Error).message })
        }
      })
      .optional()
      .describe(
        'Optional name for the worktree. Each "/"-separated segment may contain only letters, digits, dots, underscores, and dashes; max 64 chars total. A random name is generated if not provided.',
      ),
  }),
)

const outputSchema = lazySchema(() =>
  z.object({
    worktreePath: z.string(),
    worktreeBranch: z.string().optional(),
    message: z.string(),
  }),
)
```

**ExitWorktreeTool** (`ExitWorktreeTool.ts:30-58`):

```ts
const inputSchema = lazySchema(() =>
  z.strictObject({
    action: z
      .enum(['keep', 'remove'])
      .describe(
        '"keep" leaves the worktree and branch on disk; "remove" deletes both.',
      ),
    discard_changes: z
      .boolean()
      .optional()
      .describe(
        'Required true when action is "remove" and the worktree has uncommitted files or unmerged commits. The tool will refuse and list them otherwise.',
      ),
  }),
)

const outputSchema = lazySchema(() =>
  z.object({
    action: z.enum(['keep', 'remove']),
    originalCwd: z.string(),
    worktreePath: z.string(),
    worktreeBranch: z.string().optional(),
    tmuxSessionName: z.string().optional(),
    discardedFiles: z.number().optional(),
    discardedCommits: z.number().optional(),
    message: z.string(),
  }),
)
```

### 6.4 Tool prompts (verbatim)

**EnterPlanMode — default (external)** (`EnterPlanModeTool/prompt.ts:16-99`):

```
Use this tool proactively when you're about to start a non-trivial implementation task. Getting user sign-off on your approach before writing code prevents wasted effort and ensures alignment. This tool transitions you into plan mode where you can explore the codebase and design an implementation approach for user approval.

## When to Use This Tool

**Prefer using EnterPlanMode** for implementation tasks unless they're simple. Use it when ANY of these conditions apply:

1. **New Feature Implementation**: Adding meaningful new functionality
   - Example: "Add a logout button" - where should it go? What should happen on click?
   - Example: "Add form validation" - what rules? What error messages?

2. **Multiple Valid Approaches**: The task can be solved in several different ways
   - Example: "Add caching to the API" - could use Redis, in-memory, file-based, etc.
   - Example: "Improve performance" - many optimization strategies possible

3. **Code Modifications**: Changes that affect existing behavior or structure
   - Example: "Update the login flow" - what exactly should change?
   - Example: "Refactor this component" - what's the target architecture?

4. **Architectural Decisions**: The task requires choosing between patterns or technologies
   - Example: "Add real-time updates" - WebSockets vs SSE vs polling
   - Example: "Implement state management" - Redux vs Context vs custom solution

5. **Multi-File Changes**: The task will likely touch more than 2-3 files
   - Example: "Refactor the authentication system"
   - Example: "Add a new API endpoint with tests"

6. **Unclear Requirements**: You need to explore before understanding the full scope
   - Example: "Make the app faster" - need to profile and identify bottlenecks
   - Example: "Fix the bug in checkout" - need to investigate root cause

7. **User Preferences Matter**: The implementation could reasonably go multiple ways
   - If you would use AskUserQuestion to clarify the approach, use EnterPlanMode instead
   - Plan mode lets you explore first, then present options with context

## When NOT to Use This Tool

Only skip EnterPlanMode for simple tasks:
- Single-line or few-line fixes (typos, obvious bugs, small tweaks)
- Adding a single function with clear requirements
- Tasks where the user has given very specific, detailed instructions
- Pure research/exploration tasks (use the Agent tool with explore agent instead)

${whatHappens}## Examples

### GOOD - Use EnterPlanMode:
User: "Add user authentication to the app"
- Requires architectural decisions (session vs JWT, where to store tokens, middleware structure)

User: "Optimize the database queries"
- Multiple approaches possible, need to profile first, significant impact

User: "Implement dark mode"
- Architectural decision on theme system, affects many components

User: "Add a delete button to the user profile"
- Seems simple but involves: where to place it, confirmation dialog, API call, error handling, state updates

User: "Update the error handling in the API"
- Affects multiple files, user should approve the approach

### BAD - Don't use EnterPlanMode:
User: "Fix the typo in the README"
- Straightforward, no planning needed

User: "Add a console.log to debug this function"
- Simple, obvious implementation

User: "What files handle routing?"
- Research task, not implementation planning

## Important Notes

- This tool REQUIRES user approval - they must consent to entering plan mode
- If unsure whether to use it, err on the side of planning - it's better to get alignment upfront than to redo work
- Users appreciate being consulted before significant changes are made to their codebase
```

The `${whatHappens}` placeholder expands to empty when `isPlanModeInterviewPhaseEnabled()` is true; otherwise it expands to:

```
## What Happens in Plan Mode

In plan mode, you'll:
1. Thoroughly explore the codebase using Glob, Grep, and Read tools
2. Understand existing patterns and architecture
3. Design an implementation approach
4. Present your plan to the user for approval
5. Use AskUserQuestion if you need to clarify approaches
6. Exit plan mode with ExitPlanMode when ready to implement


```

(`prompt.ts:4-14`. Note trailing blank line is part of the constant.)

**EnterPlanMode — ANT variant** (`prompt.ts:101-163`, dispatched at `:166-170` `process.env.USER_TYPE === 'ant' ? Ant : External`):

```
Use this tool when a task has genuine ambiguity about the right approach and getting user input before coding would prevent significant rework. This tool transitions you into plan mode where you can explore the codebase and design an implementation approach for user approval.

## When to Use This Tool

Plan mode is valuable when the implementation approach is genuinely unclear. Use it when:

1. **Significant Architectural Ambiguity**: Multiple reasonable approaches exist and the choice meaningfully affects the codebase
   - Example: "Add caching to the API" - Redis vs in-memory vs file-based
   - Example: "Add real-time updates" - WebSockets vs SSE vs polling

2. **Unclear Requirements**: You need to explore and clarify before you can make progress
   - Example: "Make the app faster" - need to profile and identify bottlenecks
   - Example: "Refactor this module" - need to understand what the target architecture should be

3. **High-Impact Restructuring**: The task will significantly restructure existing code and getting buy-in first reduces risk
   - Example: "Redesign the authentication system"
   - Example: "Migrate from one state management approach to another"

## When NOT to Use This Tool

Skip plan mode when you can reasonably infer the right approach:
- The task is straightforward even if it touches multiple files
- The user's request is specific enough that the implementation path is clear
- You're adding a feature with an obvious implementation pattern (e.g., adding a button, a new endpoint following existing conventions)
- Bug fixes where the fix is clear once you understand the bug
- Research/exploration tasks (use the Agent tool instead)
- The user says something like "can we work on X" or "let's do X" — just get started

When in doubt, prefer starting work and using AskUserQuestion for specific questions over entering a full planning phase.

${whatHappens}## Examples

### GOOD - Use EnterPlanMode:
User: "Add user authentication to the app"
- Genuinely ambiguous: session vs JWT, where to store tokens, middleware structure

User: "Redesign the data pipeline"
- Major restructuring where the wrong approach wastes significant effort

### BAD - Don't use EnterPlanMode:
User: "Add a delete button to the user profile"
- Implementation path is clear; just do it

User: "Can we work on the search feature?"
- User wants to get started, not plan

User: "Update the error handling in the API"
- Start working; ask specific questions if needed

User: "Fix the typo in the README"
- Straightforward, no planning needed

## Important Notes

- This tool REQUIRES user approval - they must consent to entering plan mode
```

(`AskUserQuestion` substitutes from imported constant `ASK_USER_QUESTION_TOOL_NAME`.)

**ExitPlanMode prompt** (`ExitPlanModeTool/prompt.ts:6-29`, single variant — file header notes "External stub for ExitPlanModeTool prompt - excludes Ant-only allowedPrompts section"):

```
Use this tool when you are in plan mode and have finished writing your plan to the plan file and are ready for user approval.

## How This Tool Works
- You should have already written your plan to the plan file specified in the plan mode system message
- This tool does NOT take the plan content as a parameter - it will read the plan from the file you wrote
- This tool simply signals that you're done planning and ready for the user to review and approve
- The user will see the contents of your plan file when they review it

## When to Use This Tool
IMPORTANT: Only use this tool when the task requires planning the implementation steps of a task that requires writing code. For research tasks where you're gathering information, searching files, reading files or in general trying to understand the codebase - do NOT use this tool.

## Before Using This Tool
Ensure your plan is complete and unambiguous:
- If you have unresolved questions about requirements or approach, use AskUserQuestion first (in earlier phases)
- Once your plan is finalized, use THIS tool to request approval

**Important:** Do NOT use AskUserQuestion to ask "Is this plan okay?" or "Should I proceed?" - that's exactly what THIS tool does. ExitPlanMode inherently requests user approval of your plan.

## Examples

1. Initial task: "Search for and understand the implementation of vim mode in the codebase" - Do not use the exit plan mode tool because you are not planning the implementation steps of a task.
2. Initial task: "Help me implement yank mode for vim" - Use the exit plan mode tool after you have finished planning the implementation steps of the task.
3. Initial task: "Add a new feature to handle user authentication" - If unsure about auth method (OAuth, JWT, etc.), use AskUserQuestion first, then use exit plan mode tool after clarifying the approach.
```

**EnterWorktree prompt** (`EnterWorktreeTool/prompt.ts:1-30`, single variant):

```
Use this tool ONLY when the user explicitly asks to work in a worktree. This tool creates an isolated git worktree and switches the current session into it.

## When to Use

- The user explicitly says "worktree" (e.g., "start a worktree", "work in a worktree", "create a worktree", "use a worktree")

## When NOT to Use

- The user asks to create a branch, switch branches, or work on a different branch — use git commands instead
- The user asks to fix a bug or work on a feature — use normal git workflow unless they specifically mention worktrees
- Never use this tool unless the user explicitly mentions "worktree"

## Requirements

- Must be in a git repository, OR have WorktreeCreate/WorktreeRemove hooks configured in settings.json
- Must not already be in a worktree

## Behavior

- In a git repository: creates a new git worktree inside `.claude/worktrees/` with a new branch based on HEAD
- Outside a git repository: delegates to WorktreeCreate/WorktreeRemove hooks for VCS-agnostic isolation
- Switches the session's working directory to the new worktree
- Use ExitWorktree to leave the worktree mid-session (keep or remove). On session exit, if still in the worktree, the user will be prompted to keep or remove it

## Parameters

- `name` (optional): A name for the worktree. If not provided, a random name is generated.
```

**ExitWorktree prompt** (`ExitWorktreeTool/prompt.ts:1-32`, single variant):

```
Exit a worktree session created by EnterWorktree and return the session to the original working directory.

## Scope

This tool ONLY operates on worktrees created by EnterWorktree in this session. It will NOT touch:
- Worktrees you created manually with `git worktree add`
- Worktrees from a previous session (even if created by EnterWorktree then)
- The directory you're in if EnterWorktree was never called

If called outside an EnterWorktree session, the tool is a **no-op**: it reports that no worktree session is active and takes no action. Filesystem state is unchanged.

## When to Use

- The user explicitly asks to "exit the worktree", "leave the worktree", "go back", or otherwise end the worktree session
- Do NOT call this proactively — only when the user asks

## Parameters

- `action` (required): `"keep"` or `"remove"`
  - `"keep"` — leave the worktree directory and branch intact on disk. Use this if the user wants to come back to the work later, or if there are changes to preserve.
  - `"remove"` — delete the worktree directory and its branch. Use this for a clean exit when the work is done or abandoned.
- `discard_changes` (optional, default false): only meaningful with `action: "remove"`. If the worktree has uncommitted files or commits not on the original branch, the tool will REFUSE to remove it unless this is set to `true`. If the tool returns an error listing changes, confirm with the user before re-invoking with `discard_changes: true`.

## Behavior

- Restores the session's working directory to where it was before EnterWorktree
- Clears CWD-dependent caches (system prompt sections, memory files, plans directory) so the session state reflects the original directory
- If a tmux session was attached to the worktree: killed on `remove`, left running on `keep` (its name is returned so the user can reattach)
- Once exited, EnterWorktree can be called again to create a fresh worktree
```

### 6.5 State-transition pseudocode

**`prepareContextForPlanMode(context)`** (`utils/permissions/permissionSetup.ts:1462-1493`):

```
function prepareContextForPlanMode(context):
  currentMode = context.mode
  if currentMode == 'plan':
    return context

  if feature('TRANSCRIPT_CLASSIFIER'):
    planAutoMode = shouldPlanUseAutoMode()
    if currentMode == 'auto':
      if planAutoMode:
        return {...context, prePlanMode: 'auto'}
      autoModeStateModule.setAutoModeActive(false)
      setNeedsAutoModeExitAttachment(true)
      return {...restoreDangerousPermissions(context), prePlanMode: 'auto'}
    if planAutoMode and currentMode != 'bypassPermissions':
      autoModeStateModule.setAutoModeActive(true)
      return {...stripDangerousPermissionsForAutoMode(context),
              prePlanMode: currentMode}

  return {...context, prePlanMode: currentMode}
```

**`handlePlanModeTransition(fromMode, toMode)`** (`bootstrap/state.ts:1349-1363`):

```
function handlePlanModeTransition(fromMode, toMode):
  if toMode == 'plan' and fromMode != 'plan':
    STATE.needsPlanModeExitAttachment = false
  if fromMode == 'plan' and toMode != 'plan':
    STATE.needsPlanModeExitAttachment = true
```

**EnterPlanMode.call (pseudocode)** (`EnterPlanModeTool.ts:77-102`):

```
function call(_input, context):
  if context.agentId:
    throw 'EnterPlanMode tool cannot be used in agent contexts'

  appState = context.getAppState()
  handlePlanModeTransition(appState.toolPermissionContext.mode, 'plan')

  context.setAppState(prev => {
    return {
      ...prev,
      toolPermissionContext: applyPermissionUpdate(
        prepareContextForPlanMode(prev.toolPermissionContext),
        {type: 'setMode', mode: 'plan', destination: 'session'},
      ),
    }
  })

  return {
    data: {
      message: "Entered plan mode. You should now focus on exploring the
              codebase and designing an implementation approach.",
    },
  }
```

**ExitPlanMode.call mode-restore (pseudocode)** (`ExitPlanModeV2Tool.ts:357-403`):

```
context.setAppState(prev => {
  if prev.toolPermissionContext.mode != 'plan':
    return prev

  setHasExitedPlanMode(true)
  setNeedsPlanModeExitAttachment(true)

  restoreMode = prev.toolPermissionContext.prePlanMode ?? 'default'

  if feature('TRANSCRIPT_CLASSIFIER'):
    if restoreMode == 'auto' and not isAutoModeGateEnabled():
      restoreMode = 'default'
    finalRestoringAuto = (restoreMode == 'auto')
    autoWasUsedDuringPlan = isAutoModeActive()  # authoritative
    setAutoModeActive(finalRestoringAuto)
    if autoWasUsedDuringPlan and not finalRestoringAuto:
      setNeedsAutoModeExitAttachment(true)

  restoringToAuto = (restoreMode == 'auto')
  baseContext = prev.toolPermissionContext
  if restoringToAuto:
    baseContext = stripDangerousPermissionsForAutoMode(baseContext)
  elif prev.toolPermissionContext.strippedDangerousRules:
    baseContext = restoreDangerousPermissions(baseContext)

  return {
    ...prev,
    toolPermissionContext: {
      ...baseContext,
      mode: restoreMode,
      prePlanMode: undefined,
    },
  }
})
```

**EnterWorktree.call (pseudocode)** (`EnterWorktreeTool.ts:77-118`):

```
function call(input):
  if getCurrentWorktreeSession() != null:
    throw 'Already in a worktree session'

  mainRepoRoot = findCanonicalGitRoot(getCwd())
  if mainRepoRoot and mainRepoRoot != getCwd():
    process.chdir(mainRepoRoot); setCwd(mainRepoRoot)

  slug = input.name ?? getPlanSlug()
  worktreeSession = await createWorktreeForSession(getSessionId(), slug)

  process.chdir(worktreeSession.worktreePath)
  setCwd(worktreeSession.worktreePath)
  setOriginalCwd(getCwd())                          # = worktree path
  saveWorktreeState(worktreeSession)
  clearSystemPromptSections()
  clearMemoryFileCaches()
  getPlansDirectory.cache.clear?.()
  logEvent('tengu_worktree_created', {mid_session: true})

  branchInfo = worktreeSession.worktreeBranch
              ? ` on branch ${worktreeSession.worktreeBranch}` : ''
  return {data: {
    worktreePath: worktreeSession.worktreePath,
    worktreeBranch: worktreeSession.worktreeBranch,
    message: `Created worktree at ${worktreePath}${branchInfo}. The session
              is now working in the worktree. Use ExitWorktree to leave
              mid-session, or exit the session to be prompted.`,
  }}
```

**ExitWorktree.call (pseudocode)** (`ExitWorktreeTool.ts:227-321`):

```
function call(input):
  session = getCurrentWorktreeSession()
  if not session: throw 'Not in a worktree session'
  {originalCwd, worktreePath, worktreeBranch, tmuxSessionName,
   originalHeadCommit} = session

  projectRootIsWorktree = (getProjectRoot() == getOriginalCwd())

  {changedFiles, commits} =
     (countWorktreeChanges(worktreePath, originalHeadCommit))
     ?? {changedFiles: 0, commits: 0}

  if input.action == 'keep':
    await keepWorktree()
    restoreSessionToOriginalCwd(originalCwd, projectRootIsWorktree)
    logEvent('tengu_worktree_kept',
             {mid_session: true, commits, changed_files: changedFiles})
    tmuxNote = tmuxSessionName
       ? ` Tmux session ${tmuxSessionName} is still running; reattach with:
           tmux attach -t ${tmuxSessionName}` : ''
    return {data: {
      action: 'keep', originalCwd, worktreePath, worktreeBranch,
      tmuxSessionName,
      message: `Exited worktree. Your work is preserved at ${worktreePath}
                ${branch?}. Session is now back in ${originalCwd}.${tmuxNote}`,
    }}

  # action == 'remove'
  if tmuxSessionName: await killTmuxSession(tmuxSessionName)
  await cleanupWorktree()
  restoreSessionToOriginalCwd(originalCwd, projectRootIsWorktree)
  logEvent('tengu_worktree_removed',
           {mid_session: true, commits, changed_files: changedFiles})
  discardNote = (commits>0 or changedFiles>0)
              ? ` Discarded ${parts.join(' and ')}.` : ''
  return {data: {
    action: 'remove', originalCwd, worktreePath, worktreeBranch,
    discardedFiles: changedFiles, discardedCommits: commits,
    message: `Exited and removed worktree at ${worktreePath}.${discardNote}
              Session is now back in ${originalCwd}.`,
  }}
```

### 6.6 User-facing strings

**EnterPlanMode**

- Description: `'Requests permission to enter plan mode for complex tasks requiring exploration and design'`
- Search hint: `'switch to plan mode to design an approach before coding'`
- userFacingName: `''`
- Tool-result message (success): `'Entered plan mode. You should now focus on exploring the codebase and designing an implementation approach.'`
- Tool-result content when interview-phase enabled (`mapToolResultToToolResultBlockParam :104-107`):
  ```
  {message}

  DO NOT write or edit any files except the plan file. Detailed workflow instructions will follow.
  ```
- Tool-result content when interview-phase disabled (`:108-118`):
  ```
  {message}

  In plan mode, you should:
  1. Thoroughly explore the codebase to understand existing patterns
  2. Identify similar features and architectural approaches
  3. Consider multiple approaches and their trade-offs
  4. Use AskUserQuestion if you need to clarify the approach
  5. Design a concrete implementation strategy
  6. When ready, use ExitPlanMode to present your plan for approval

  Remember: DO NOT write or edit any files yet. This is a read-only exploration and planning phase.
  ```
- UI render — entered: `BLACK_CIRCLE` + `' Entered plan mode'` + dim sub-line `'Claude is now exploring and designing an implementation approach.'`.
- UI render — rejected: `BLACK_CIRCLE` + `' User declined to enter plan mode'`.

**ExitPlanMode**

- Description: `'Prompts the user to exit plan mode and start coding'`
- Search hint: `'present plan for approval and start coding (plan mode only)'`
- userFacingName: `''`
- `validateInput` reject (`:213-217`): `'You are not in plan mode. This tool is only for exiting plan mode after writing a plan. If your plan was already approved, continue with implementation.'` + `errorCode:1`.
- Permission ask (`:236`): `'Exit plan mode?'`.
- Tool-result content variants (`:431-491`):
  - awaitingLeaderApproval block (verbatim, includes "Request ID: ${requestId}", lists 4 numbered "What happens next" steps).
  - isAgent: `'User has approved the plan. There is nothing else needed from you now. Please respond with "ok"'`.
  - empty plan: `'User has approved exiting plan mode. You can now proceed.'`.
  - default with plan:
    ```
    User has approved your plan. You can now start coding. Start with updating your todo list if applicable

    Your plan has been saved to: {filePath}
    You can refer back to it if needed during implementation.{teamHint}

    ## {planLabel}:
    {plan}
    ```
    where `planLabel = planWasEdited ? 'Approved Plan (edited by user)' : 'Approved Plan'` and `teamHint`, when `hasTaskTool`, is `\n\nIf this plan can be broken down into multiple independent tasks, consider using the ${TEAM_CREATE_TOOL_NAME} tool to create a team and parallelize the work.`
- UI render strings: `'Exited plan mode'` (empty), `'Plan submitted for team lead approval'` + `'Plan file: {displayPath}'` + `'Waiting for team lead to review and approve...'` (awaiting), `'User approved Claude's plan'` + dim `'Plan saved to: {displayPath} · /plan to edit'` + Markdown-rendered plan (default).
- Auto-mode gate-fallback notification (`:347-354`):
  - `key: 'auto-mode-gate-plan-exit-fallback'`
  - `text: 'plan exit → default · ' + gateFallbackNotification` (string sourced from `permissionSetupModule.getAutoModeUnavailableNotification(reason)` else literal `'auto mode unavailable'`)
  - `priority: 'immediate'`, `color: 'warning'`, `timeoutMs: 10000`
- Plan approval mailbox envelope (`:278-286`):
  ```
  {
    type: 'plan_approval_request',
    from: <agentName>,
    timestamp: new Date().toISOString(),
    planFilePath: <filePath>,
    planContent: <plan>,
    requestId: generateRequestId('plan_approval', formatAgentId(name, team||'default')),
  }
  ```

**EnterWorktree**

- Description: `'Creates an isolated worktree (via git or configured hooks) and switches the session into it'`
- Search hint: `'create an isolated git worktree and switch into it'`
- userFacingName: `'Creating worktree'`
- Throw on already in worktree: `'Already in a worktree session'`
- Success message template: `'Created worktree at ${worktreePath}${branchInfo}. The session is now working in the worktree. Use ExitWorktree to leave mid-session, or exit the session to be prompted.'`
- UI render: `'Creating worktree…'` (use), `Switched to worktree on branch <bold>{branch}</bold>` + dim `{worktreePath}` (result).

**ExitWorktree**

- Description: `'Exits a worktree session created by EnterWorktree and restores the original working directory'`
- Search hint: `'exit a worktree session and return to the original directory'`
- userFacingName: `'Exiting worktree'`
- `validateInput` no-op message (`:184-187`): `'No-op: there is no active EnterWorktree session to exit. This tool only operates on worktrees created by EnterWorktree in the current session — it will not touch worktrees created manually or in a previous session. No filesystem changes were made.'` (`errorCode:1`).
- Verify-failure message (`:198`): `'Could not verify worktree state at ${session.worktreePath}. Refusing to remove without explicit confirmation. Re-invoke with discard_changes: true to proceed — or use action: "keep" to preserve the worktree.'` (`errorCode:3`).
- Has-changes refusal message (`:217`): `'Worktree has ${parts.join(' and ')}. Removing will discard this work permanently. Confirm with the user, then re-invoke with discard_changes: true — or use action: "keep" to preserve the worktree.'` (`errorCode:2`). `parts` are formed with singular/plural rules.
- Race-defense throw on call (`:232`): `'Not in a worktree session'`.
- Keep success: `'Exited worktree. Your work is preserved at ${worktreePath}${branch}. Session is now back in ${originalCwd}.${tmuxNote}'`. tmuxNote: `' Tmux session ${tmuxSessionName} is still running; reattach with: tmux attach -t ${tmuxSessionName}'`.
- Remove success: `'Exited and removed worktree at ${worktreePath}.${discardNote} Session is now back in ${originalCwd}.'`.
- Hook-create error (cited from `worktree.ts:733-736`): `'Cannot create a worktree: not in a git repository and no WorktreeCreate hooks are configured. Configure WorktreeCreate/WorktreeRemove hooks in settings.json to use worktree isolation with other VCS systems.'`
- UI render: `'Exiting worktree…'` (use), `Kept worktree (branch <bold>{branch}</bold>)` or `Removed worktree (branch <bold>{branch}</bold>)` + dim `Returned to {originalCwd}` (result).

### 6.7 Tool-flag matrix (verbatim)

| Tool | shouldDefer | isReadOnly | isConcurrencySafe | requiresUserInteraction | isDestructive | userFacingName |
|---|---|---|---|---|---|---|
| EnterPlanMode | true | true | true | (default) | (default) | `''` |
| ExitPlanMode  | true | false (writes plan to disk) | true | dynamic: `isTeammate() ? false : true` | (default) | `''` |
| EnterWorktree | true | (default) | (default) | (default) | (default) | `'Creating worktree'` |
| ExitWorktree  | true | (default) | (default) | (default) | `input.action === 'remove'` | `'Exiting worktree'` |

`maxResultSizeChars = 100_000` for all four.

### 6.8 Analytics events (verbatim event names)

- `tengu_exit_plan_mode_called_outside_plan` — `{model: mainLoopModel, mode, hasExitedPlanModeInSession}` (ExitPlanMode `:206-211`).
- `tengu_worktree_created` — `{mid_session: true}` (EnterWorktreeTool `:104-106`).
- `tengu_worktree_kept` — `{mid_session: true, commits, changed_files}` (ExitWorktreeTool `:265-269`).
- `tengu_worktree_removed` — `{mid_session: true, commits, changed_files}` (ExitWorktreeTool `:293-297`).

### 6.9 Module-load conditional `require`s (TRANSCRIPT_CLASSIFIER)

`ExitPlanModeV2Tool.ts:51-58`:

```ts
/* eslint-disable @typescript-eslint/no-require-imports */
const autoModeStateModule = feature('TRANSCRIPT_CLASSIFIER')
  ? (require('../../utils/permissions/autoModeState.js') as typeof import('../../utils/permissions/autoModeState.js'))
  : null
const permissionSetupModule = feature('TRANSCRIPT_CLASSIFIER')
  ? (require('../../utils/permissions/permissionSetup.js') as typeof import('../../utils/permissions/permissionSetup.js'))
  : null
/* eslint-enable @typescript-eslint/no-require-imports */
```

### 6.10 Channels-disable predicate (verbatim, both tools)

```ts
if (
  (feature('KAIROS') || feature('KAIROS_CHANNELS')) &&
  getAllowedChannels().length > 0
) {
  return false
}
return true
```

### 6.11 Worktree-mode predicate

`utils/worktreeModeEnabled.ts:1-11`:

```ts
/**
 * Worktree mode is now unconditionally enabled for all users.
 *
 * Previously gated by GrowthBook flag 'tengu_worktree_mode', but the
 * CACHED_MAY_BE_STALE pattern returns the default (false) on first launch
 * before the cache is populated, silently swallowing --worktree.
 * See https://github.com/anthropics/claude-code/issues/27044.
 */
export function isWorktreeModeEnabled(): boolean {
  return true
}
```

Used at `tools.ts:225`: `...(isWorktreeModeEnabled() ? [EnterWorktreeTool, ExitWorktreeTool] : [])`.

### 6.12 Tool-registry placement (verbatim slice)

From `tools.ts` (within `getAllBaseTools` array, around `:200-230`):

```ts
ExitPlanModeV2Tool,
…
EnterPlanModeTool,
…
...(isWorktreeModeEnabled() ? [EnterWorktreeTool, ExitWorktreeTool] : []),
```

Order in the registry array is significant for ANT-only branches and stripped imports — do not reorder (per `tools.ts` header comment, owned by 08).

## 7. Configuration & Environment

| Env / setting | Effect |
|---|---|
| `USER_TYPE === 'ant'` | EnterPlanMode prompt switches to ANT variant (`prompt.ts:166-170`); `isPlanModeInterviewPhaseEnabled()` returns true unconditionally. |
| `CLAUDE_CODE_PLAN_MODE_INTERVIEW_PHASE` | External: truthy/falsy override of the GrowthBook gate. |
| `tengu_plan_mode_interview_phase` (GrowthBook, default `false`) | External default for interview-phase. |
| `feature('TRANSCRIPT_CLASSIFIER')` | Controls the auto-mode-during-plan branches in `prepareContextForPlanMode` and the gate-fallback in ExitPlanMode (§5.2 #3). When stripped, those branches collapse to plain plan/exit. |
| `feature('KAIROS')` / `feature('KAIROS_CHANNELS')` + `getAllowedChannels()` | Disables both EnterPlanMode and ExitPlanMode (§5.9). |
| `settings.plansDirectory` | Project-relative dir for plan files; falls back to `<claudeConfigHome>/plans` if unset or path-traversal-suspect. (`plans.ts:79-111`.) |
| `settings.worktree.sparsePaths` | Sets `WorktreeSession.usedSparsePaths` flag. (`worktree.ts:766-767`.) |
| `WorktreeCreate` / `WorktreeRemove` hooks (`settings.json`) | When `hasWorktreeCreateHook()`, `createWorktreeForSession` delegates here instead of git; `cleanupWorktree` mirrors via `executeWorktreeRemoveHook`. |

## 8. Error Handling

| Site | Error | Behavior |
|---|---|---|
| EnterPlanMode.call when `agentId` set | thrown `Error('EnterPlanMode tool cannot be used in agent contexts')` | Surfaces as a normal tool error. |
| EnterPlanMode `isEnabled()` channels active | returns `false` | Tool not registered for this turn. |
| ExitPlanMode.validateInput when not in plan | `{result:false, message: …, errorCode:1}` + `tengu_exit_plan_mode_called_outside_plan` event | Skips permission UI. |
| ExitPlanMode disk write failure on edited plan | `writeFile(...).catch(e => logError(e))` | Best-effort; tool continues with on-disk plan. |
| EnterWorktree already in session | thrown `Error('Already in a worktree session')` | Tool error. |
| EnterWorktree without git repo and no hook | thrown by `createWorktreeForSession` (`worktree.ts:733-736`) | Tool error with hooks-config hint. |
| ExitWorktree no session | `{result:false, errorCode:1}` no-op | No filesystem change; user-readable message. |
| ExitWorktree remove + dirty + !discard_changes | `{result:false, errorCode:2}` | Listing of files+commits. |
| ExitWorktree git command failed during count | `{result:false, errorCode:3}` | Fail-closed; user must opt in via `discard_changes:true`. |
| ExitWorktree race after validate | thrown `Error('Not in a worktree session')` | Defensive against module-state mutation between validate and call. |
| `cleanupWorktree` failure modes | logged via `logForDebugging({level:'error'})`; never re-thrown | Session restoration still happens. |
| `isWorktreeModeEnabled()` returns `false` (test stub or alternate build) | `tools.ts:225` evaluates the spread `...(false ? […] : [])` → emits `[]` | EnterWorktree/ExitWorktree are NOT in the registry. Any model attempt to call them surfaces as "tool not found" / `ToolNotFoundError` from the registry lookup, **not** as a clean disabled-state error. There is no user-facing "worktree mode is disabled" message; the absence is silent. (Currently moot in shipping builds since `worktreeModeEnabled.ts:9-11` returns `true` unconditionally.) |

## 9. Observability

- Logs (via `logForDebugging`): "Created hook-based worktree at: <path>", "Resuming existing worktree at: <path>", "Linked worktree preserved at: …", "Removed linked worktree at: …", "Deleted worktree branch: …", "Failed to remove linked worktree", "Could not delete worktree branch", "[prepareContextForPlanMode] plain plan entry, prePlanMode=…", "[auto-mode gate @ ExitPlanModeV2Tool] prePlanMode=… but gate is off (reason=…) — falling back to default on plan exit".
- Events: §6.8.
- Notifications: `auto-mode-gate-plan-exit-fallback` (immediate, warning, 10s).

## 10. Testing & Verification

(No tests ship in the leak — see §1 of spec 00.) Manual probes:

- Trigger `EnterPlanMode` → verify `appState.toolPermissionContext.mode === 'plan'` and `prePlanMode` reflects pre-entry mode.
- In plan mode, attempt a non-readonly tool → permission system (09) should reject.
- `ExitPlanMode` → verify `mode` restored to `prePlanMode`; verify `setHasExitedPlanMode(true)` and `setNeedsPlanModeExitAttachment(true)`.
- `ExitPlanMode` after `--channels` → tool should be disabled (not in deferred list).
- `EnterWorktree` from inside an existing worktree → should `chdir` to canonical git root first, then create new sibling.
- `ExitWorktree` with uncommitted file but `discard_changes:false` → must refuse with `errorCode:2`.
- `ExitWorktree` with corrupt git index (synthetically: `chmod -r .git`) → must refuse with `errorCode:3` (fail-closed).

## 11. Source-coverage Inventory

| Source | Line range | Spec section(s) |
|---|---|---|
| `EnterPlanModeTool.ts` | 1-126 | §5.1, §6.3, §6.4, §6.6, §6.7, §6.5 |
| `EnterPlanModeTool/prompt.ts` | 1-170 | §5.10, §6.4 |
| `EnterPlanModeTool/constants.ts` | 1 | §6.1 |
| `EnterPlanModeTool/UI.tsx` | 1-32 | §6.6 |
| `ExitPlanModeV2Tool.ts` | 1-493 | §5.2, §5.7, §5.8, §6.3, §6.6, §6.8, §6.9 |
| `ExitPlanModeTool/prompt.ts` | 1-29 | §6.4 |
| `ExitPlanModeTool/constants.ts` | 1-2 | §6.1 |
| `ExitPlanModeTool/UI.tsx` | 1-81 | §6.6 |
| `EnterWorktreeTool.ts` | 1-127 | §5.4, §6.3, §6.5, §6.6 |
| `EnterWorktreeTool/prompt.ts` | 1-30 | §6.4 |
| `EnterWorktreeTool/UI.tsx` | 1-19 | §6.6 |
| `ExitWorktreeTool.ts` | 1-329 | §5.5, §6.3, §6.5, §6.6, §6.8 |
| `ExitWorktreeTool/prompt.ts` | 1-32 | §6.4 |
| `ExitWorktreeTool/UI.tsx` | 1-24 | §6.6 |
| `worktree.ts` | 48-227, 693-894 | §5.4, §5.5, §6.2 |
| `worktreeModeEnabled.ts` | 1-11 | §1, §6.11 |
| `planModeV2.ts` | 50-62 | §5.10 |
| `permissionSetup.ts` | 1462-1532 | §5.1, §6.5 (cite-only) |
| `bootstrap/state.ts` | 1349-1363 | §6.5 |
| `tools.ts` | 200-230 | §6.12 |
| `plans.ts` | 32-145 | §3 (referenced) |

## 12. Caveats & Open Questions

1. **`autoModeStateModule.isAutoModeActive()` semantics.** The ExitPlanMode comment (`:371-373`) says it's "the authoritative signal — prePlanMode/strippedDangerousRules are stale after `transitionPlanAutoMode` deactivates mid-plan." Spec 09's verification of this exact claim is referenced but not duplicated; may need cross-check during 09 review.
2. **`shouldPlanUseAutoMode` definition.** Cited from `permissionSetup.ts:1450-1454` (top of context), but the full predicate is owned by 09. This spec assumes its current semantics (auto-mode gate + `getUseAutoModeDuringPlan()` setting).
3. **`isTeammate()` / `isPlanModeRequired()` truth tables.** Referenced from `utils/teammate.js`; full ownership is 14 (Agent/Team) or 30 (coordinator) — not re-derived here.
4. **`AGENT_TRIGGERS_REMOTE` / CCR plan-edit injection path.** `_sdkInputSchema` exposes `plan` and `planFilePath` "injected by normalizeToolInput from disk." `normalizeToolInput`'s exact disk-snapshot timing is owned by 04 (turn pipeline); we cite the comment but don't enumerate all CCR edit paths.
5. **`getPlan()` / `getPlanFilePath()` resume semantics.** The plan-slug recovery path (`plans.ts:163-…`) is referenced but not specced — falls under 41 (session state).
6. **`registerPlanVerificationHook`.** ExitPlanMode comment (`:315-316`) notes verification hook is registered in `REPL.tsx` AFTER context clear. The exact REPL hook-registration choreography is owned by 37 (Ink UI shell). Recorded here for cross-reference.
7. **Hook-based-worktree git-tree case.** `countWorktreeChanges` returns `null` when the worktree IS a git repo but `originalHeadCommit` is undefined, citing `worktree.ts:525-532` as the no-baseline site. That line range was inferred from comments; full coverage of `performPostCreationSetup` is owned by the worktree-internals layer (this spec at §5.4 states the contract but does not enumerate 525-532).
8. **`generateRequestId('plan_approval', …)` collision avoidance.** Owned by `utils/agentId.ts`; assumed unique per call.
9. **`saveWorktreeState` / `restoreWorktreeSession` resume cycle.** The persistence end is owned by 41 (session state & history).
10. **`updateHooksConfigSnapshot()` in restore path.** ExitWorktree only calls this when `projectRootIsWorktree`. The setup-time invocation lives in `src/setup.ts`'s `--worktree` block at `setCwd:272 → setOriginalCwd:273 → setProjectRoot:277` (the `:235/:239` numbers in the inline comment at `ExitWorktreeTool.ts:244-248` are stale; the back-to-back invariant — not the line numbers — is the contract). Owned by 01.
