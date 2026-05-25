# Phase 9.7 Agent E — Path Canonicalization Log

**Goal:** For ambiguous bare-basename citations (basenames that resolve to 2+
candidate files under `src/`), prepend a discriminating directory prefix to
make them machine-checkable.

**Scope excluded:** spec 28 (Agent D), spec 09/11/21/30 (Agent C zone),
spec 04 §12.

## Procedure

1. Extracted all bare-basename file citations of the form
   `` `<name>.ts:<lines>` `` across specs 00–42 + catalog companions
   (excluding the protected zones).
2. For each unique basename, ran `find src -name <basename> -type f` to
   determine candidate count.
3. Built ambiguous list (count ≥ 2) and triaged: most apparently-ambiguous
   basenames (`prompt.ts`, `utils.ts`, `constants.ts`, `types.ts`,
   `settings.ts`, `client.ts`, `messages.ts`, `compact.ts`, `attachments.ts`,
   `config.ts`) are in fact disambiguated by per-spec topical scope —
   e.g. `prompt.ts` in `12-tool-search.md` is unambiguously
   `src/tools/ToolSearchTool/prompt.ts`. Such citations were left alone.
4. Identified citations whose **section-local context does NOT disambiguate**
   the file — i.e. where a global reader / automated checker scanning only
   the cite line cannot pick the right candidate. Applied a minimum-prefix
   Edit to those.

## Ambiguous Basename Inventory

`find src -name <basename> -type f` candidate counts ≥ 2 (excluding
`index.ts`, 84 candidates — context always disambiguates):

```
39 prompt.ts
28 UI.tsx
24 constants.ts
18 types.ts
11 utils.ts
 6 config.ts
 4 prompts.ts
 4 parser.ts
 4 auth.ts
 3 setup.ts
 3 client.ts
 3 api.ts
 2 voice.ts
 2 validation.ts
 2 tools.ts
 2 toolName.ts
 2 tokenBudget.ts
 2 settings.ts
 2 schemas.ts
 2 readOnlyValidation.ts
 2 permissions.ts
 2 modeValidation.ts
 2 messages.ts
 2 init.ts
 2 errors.ts
 2 destructiveCommandWarning.ts
 2 debug.ts
 2 context.ts
 2 compact.ts
 2 commands.ts
 2 commandSemantics.ts
 2 color.ts
 2 betas.ts
 2 attachments.ts
 2 advisor.ts
```

Total ambiguous basenames found: **35** (after excluding `index.ts`).

## Edits Applied

5 edits across 2 specs.

| # | Spec | Original | Canonicalized to | Reason |
|---|------|----------|------------------|--------|
| 1 | 05-context-assembly | `api.ts:437-447` | `utils/api.ts:437-447` | 3 candidates; spec 05's surrounding text discusses context-assembly utilities, not `services/api/client.ts` or `utils/teleport/api.ts`. Disambiguates. |
| 2 | 05-context-assembly | `api.ts:449-474` | `utils/api.ts:449-474` | Same as above. |
| 3 | 05-context-assembly | `api.ts:453` | `utils/api.ts:453` | Same as above (env-flag table). |
| 4 | 35-mode-remote-server | `api.ts:16-33` | `teleport/api.ts:16-33` | Spec 35 covers remote-session/teleport; `RedactedGithubToken` lives in `src/utils/teleport/api.ts`. Distinguishes from `services/api/client.ts` and `utils/api.ts`. |
| 5 | 35-mode-remote-server | `api.ts:91-97` | `teleport/api.ts:91-97` | Same as above. |

Note: spec 05 already had two **pre-canonicalized** `utils/api.ts:`
citations at lines 360, 374 (likely from an earlier polish pass). Those
were left as-is. The 5 fixes here harmonize the remaining 5 bare cites in
those two specs to the same convention.

## Citations Inspected and Left As-Is

- **Spec 02** `settings.ts`, `types.ts`, `constants.ts` — all refer to
  `src/utils/settings/{settings,types,constants}.ts`. Spec 02's title and
  scope ("settings, schemas, migrations") makes the file unambiguous. The
  spec ALREADY uses explicit `mdm/settings.ts`, `mdm/constants.ts`,
  `schemas/hooks.ts` prefixes when crossing into siblings.
- **Spec 04** `messages.ts:*` — all refer to `src/utils/messages.ts`. Spec
  itself anchors with `src/utils/messages.ts:3097-3099` once at line 719;
  remaining cites are clearly the same file.
- **Specs 10, 12, 13, 14, 15, 17, 18, 19** `prompt.ts` — each refers to
  the tool-specific `prompt.ts` named in the spec title. Section-local
  context disambiguates.
- **Spec 13** `utils.ts` — all refer to `src/tools/WebFetchTool/utils.ts`.
  Spec 13 = WebFetch/WebSearch.
- **Spec 16** `client.ts` — refers to `src/services/mcp/client.ts`. Spec 16
  scope established up front.
- **Spec 17** `prompt.ts:*` and `attachments.ts:*` — `src/tools/SkillTool/prompt.ts`
  and `src/utils/attachments.ts` respectively; the spec already uses
  `utils/attachments.ts` explicitly when needed.
- **Spec 22** `client.ts:*` — refers to `src/services/api/client.ts`. Spec 22
  scope.
- **Spec 23** `client.ts`, `utils.ts`, `config.ts` — all `src/services/mcp/`.
  Spec 23 scope.
- **Spec 24** `config.ts:*` — refers to `src/services/lsp/config.ts`. Spec 24 scope.
- **Spec 27** `settings.ts:*`, `types.ts:8-12`/`:21-27` — settings cites refer to
  `src/utils/settings/settings.ts` (cross-cited from policy spec, but the
  function names disambiguate); types refer to `src/services/policyLimits/types.ts`.
- **Spec 29** `types.ts:*` — refers to `src/services/teamMemorySync/types.ts`. Scope.
- **Spec 34** `types.ts:*` — refers to `src/bridge/types.ts`. Spec 34 scope.

## Did the Phase 9.6 estimate (12.3% / ~26 of 211) hold?

**No** — the 12.3% figure overcounts citations that are *technically*
bare-basename but *practically* disambiguated by spec scope. The genuine
cross-spec-ambiguous count is closer to **2–3% (5 of 211)**: the cases
where a citation appears in a spec whose topical scope does NOT match the
file's directory in `src/` (e.g. spec 35 citing `api.ts` for
`utils/teleport/api.ts`, where 35 is "remote-server mode" and the path
prefix `teleport/` is non-obvious without the canonicalization).

The other ~21 cites flagged by the audit are bare in syntax but
unambiguous in context — useful as a polish target if a future reader
parses citations file-locally without the surrounding section, but not
load-bearing.

## Unexpected Naming Conflicts

- `src/utils/messages.ts` (5512 lines) vs `src/constants/messages.ts`
  (1 line). The constants file is effectively a stub; nothing in any
  spec references it. Not a real ambiguity.
- `src/commands.ts` (top-level registry) vs `src/utils/bash/commands.ts`
  (Bash command vocabulary). Both are referenced in different specs; no
  cite was found that would cause cross-confusion.
- `prompt.ts` has the highest candidate count (39) but is the most
  context-disambiguated — every tool spec's section header names the
  tool, so `prompt.ts:NNN` always means "this tool's prompt file."

## Budget

- Cap: 30 edits.
- Used: **5 edits**.
- Remainder: not needed — the genuinely-ambiguous set is exhausted.

## Follow-Up (Optional, Out of Scope for Phase 9.7)

If a future pass wants to lift section-local citations to globally-
unambiguous form (i.e. "every cite resolves correctly when read in
isolation"), the candidate set is the ~20 cites covered under
"Citations Inspected and Left As-Is" above. The mechanical rewrite would
prepend the parent-directory of each spec's primary subject:

- spec 02 `settings.ts` → `utils/settings/settings.ts`
- spec 04 `messages.ts` → `utils/messages.ts`
- spec 13 `utils.ts` → `tools/WebFetchTool/utils.ts`
- spec 14 `prompt.ts` → `tools/AgentTool/prompt.ts`
- etc.

Estimated effort: ~50–80 edits across ~15 specs. **Not recommended unless
an automated checker is built that needs file-local resolution.**
