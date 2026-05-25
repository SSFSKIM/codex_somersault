# Phase 9.5b Adversarial Review — Spec 16 (MCP & LSP Tools)

Reviewer role: Skeptic. Source-of-truth = `src/tools/{MCPTool,LSPTool,ListMcpResourcesTool,ReadMcpResourceTool,McpAuthTool}/` plus cited helpers in `src/services/mcp/client.ts` and `src/tools.ts`. Read-only sweep, ~14 src reads.

## Severity counts

| Severity | Count |
|---|---|
| Critical (factually wrong, would mislead a re-implementer) | 0 |
| Major (cited line range off by enough to break the cite) | 0 |
| Minor (line-number drift / wording quibble) | 2 |
| Nit (non-load-bearing) | 1 |
| Verified clean (load-bearing claim that holds up) | 11 |

## Top findings

1. **Skip-prefix gate location — VERIFIED.** Spec §3.6 cites `services/mcp/client.ts:1760-1773` for the `client.config.type === 'sdk' && isEnvTruthy(CLAUDE_AGENT_SDK_MCP_NO_PREFIX)` check. Source: lines 1760-1773 contain exactly this guard plus the "use the original name … mcpInfo is used for permission checking" comment. Resolves Phase 9.7 §12 finding correctly.
2. **`ALLOWED_IDE_TOOLS` allowlist — VERIFIED.** Spec cites `client.ts:568` for `['mcp__ide__executeCode', 'mcp__ide__getDiagnostics']`. Confirmed at line 568 verbatim; the surrounding `isIncludedMcpTool` filter is at 569-572. Cross-spec to 23 (lifecycle filter consumer) and 34 (IDE bridge) is consistent.
3. **`getMcpSkillCommands` correctly absent from spec 16 — VERIFIED.** That helper lives in `src/commands.ts:547` and is consumed by `src/utils/attachments.ts:2677`. Spec 16 does not claim it under `MCPTool/`, so the Phase 9.7 spec 17 mis-attribution is *not* duplicated here. Clean.
4. **McpAuthTool factory pattern — VERIFIED.** Spec §3.4 / §4.4 match `tools/McpAuthTool/McpAuthTool.ts:49-215`: factory `createMcpAuthTool(serverName, config)`, `name = buildMcpToolName(serverName, 'authenticate')`, `mcpInfo = { serverName, toolName: 'authenticate' }`, `maxResultSizeChars = 10_000`, `checkPermissions → { behavior:'allow' }`, claudeai-proxy / non-{sse,http} branches return `unsupported`, race between `authUrlPromise` and `oauthPromise`, background `clearMcpAuthCache → reconnectMcpServerImpl → setAppState` with `reject(t.name?.startsWith(prefix))` cleanup of `tools` and `commands`, and `resources` overlay only when `result.resources` set. All branches at 89-205 confirmed.
5. **`utils/computerUse/` co-ownership claim (spec 42a) — N/A here.** Grepped MCPTool/, LSPTool/, McpAuthTool/, ListMcpResourcesTool/, ReadMcpResourceTool/ for `utils/computerUse` — zero hits. Spec 16 makes no co-ownership claim and shouldn't; computerUse is not on the import path of any of these five tools. Clean.

### Minor / Nit

- **(Minor) §2.1 line range for `MCPTool/MCPTool.ts` is "1-78"; file is 77 lines.** Trivial off-by-one.
- **(Minor) §2.1 line range for `LSPTool/LSPTool.ts` is "1-861"; file is 860 lines.** Same.
- **(Nit) §3.6 calls the SEARCH/READ allowlists "two large static allowlists".** Both are large in *aggregate* (READ_TOOLS spans `:142-586`, SEARCH_TOOLS spans `:14-139` covering Slack/GitHub/Linear/Datadog/Sentry/AWS/Terraform), so the wording survives review even though SEARCH is the smaller of the two.

## Other load-bearing claims spot-checked and confirmed

- `MCPTool` template exact shape (§3.1): `MCPTool.ts:27-77` matches — `isMcp: true`, `name: 'mcp'`, `maxResultSizeChars: 100_000`, `outputSchema = z.string()`, `checkPermissions → { behavior:'passthrough', message:'MCPTool requires permission.' }`, `userFacingName: () => 'mcp'`.
- `MAX_MCP_DESCRIPTION_LENGTH = 2048` at `client.ts:218` — confirmed.
- `MAX_SESSION_RETRIES = 1` at `client.ts:1859` — confirmed (spec 16 §5.1 cites this constant; only `McpSessionExpiredError` triggers retry per loop at 1915).
- `ListMcpResourcesTool` empty-result placeholder string `'No resources found. MCP servers may still provide tools even if they have no resources.'` — verified at `:113-117`. Throws `Server "<name>" not found. Available servers: <csv>` at `:75-78` for unknown server filter.
- `ReadMcpResourceTool` blob persistence ID pattern `mcp-resource-${Date.now()}-${i}-${rand36×6}` — verified at `:117`. Three error throws (`not found` / `is not connected` / `does not support resources`) at `:81-100` match spec §4.3 exactly. `getBinaryBlobSavedMessage` text-marker rewrite at `:131-139` matches.
- `LSPTool` validation error codes 1/2/3/4 — verified: code 3 (zod) at `:158-164`, UNC bypass at `:171-173` (`startsWith('\\\\')` or `'//'`), code 1 (ENOENT) at `:179-184`, code 4 (other stat error) at `:193-198`, code 2 (not a regular file) at `:200-206`.
- `MAX_LSP_FILE_SIZE_BYTES = 10_000_000` at `:53` — confirmed.
- `getMethodAndParams` 9-variant dispatch table (§5.1 / §4.5) — verified at `:425-513`. `workspaceSymbol` always passes empty `query: ''`. Both `incomingCalls` and `outgoingCalls` first-step return `textDocument/prepareCallHierarchy`; second step is dispatched in `LSPTool.call`.
- Gitignore filter — verified at `:556-611`: `BATCH_SIZE = 50`, `timeout: 5_000`, `preserveOutputOnError: false`, exit-code-0 stdout split into ignored set, applies cwd-rooted. `LocationLink` adapter (§4.5) at `:622-631`: `{ uri: targetUri, range: targetSelectionRange ?? targetRange }` — confirmed via `||` (functionally equivalent to `??` because both target fields are `Range | undefined`, never falsy non-undefined).
- Tool registry — `tools.ts:74-76` imports the three tools; `:224` ENABLE_LSP_TOOL gate; `:245-246` Mcp resource tools added unconditionally; `:301-307` `specialTools = new Set([ListMcpResourcesTool.name, ReadMcpResourceTool.name])` filter for the standard-tool list — all match spec §2.2.
- `classifyForCollapse.ts` `_serverName` intentionally unused with the documented "stable across installs" rationale at `:1-11` — confirmed.

## Verdict

**APPROVE** — spec 16 holds up to adversarial review. No critical or major errors, two minor line-range off-by-ones, one wording nit. The four load-bearing cross-spec claims (McpAuthTool factory, ALLOWED_IDE_TOOLS, CLAUDE_AGENT_SDK_MCP_NO_PREFIX skip-prefix gate, getMcpSkillCommands correctly *not* attributed) all survive verification. Spec 16 is consistent with Phase 9.7 §12 and Phase 9.7 §17 corrections.

## Cross-spec impact

- **Spec 08 (registry & ToolDef):** `mcpInfo` discriminator + `isMcp`/`isLsp` consumers cited correctly at `Tool.ts:436-455`. No drift.
- **Spec 09 (permissions):** `MCPTool.checkPermissions → passthrough` and `LSPTool.checkPermissions → checkReadPermissionForTool` both verified; spec 09 ownership intact.
- **Spec 23 (MCP server lifecycle):** Spec 16 cites `fetchToolsForClient`, `ensureConnectedClient`, `fetchResourcesForClient`, `callMCPToolWithUrlElicitationRetry`, `reconnectMcpServerImpl`, `clearMcpAuthCache`, `MAX_MCP_DESCRIPTION_LENGTH`, `ALLOWED_IDE_TOOLS`, skip-prefix gate — all by signature only. No leakage of lifecycle into 16; clean boundary.
- **Spec 24 (LSP server manager):** `getLspServerManager`, `waitForInitialization`, `isLspConnected`, `openFile`, `sendRequest`, `isFileOpen` cited by signature only. Clean boundary.
- **Spec 28 (plugins):** No plugin-loaded MCP server discussion in 16. Correctly out of scope.
- **Spec 34 (IDE bridge):** ALLOWED_IDE_TOOLS allowlist is consumed via `services/mcp/client.ts`, not duplicated in 16. Cross-spec consistent.
- **Spec 37 (UI shell):** UI rendering primitives only cited; rich-output unwrap/flatten/Slack-send heuristics correctly carved into spec 16's §6.11 (its own scope).
- **Spec 42a (utils co-ownership):** No `utils/computerUse/` references in any of the five tools. Spec 16 makes no co-ownership claim — correct.
- **Phase 9.7 §17 (`getMcpSkillCommands` mis-attribution):** Spec 16 does not repeat the bug; helper lives at `commands.ts:547`, not under `MCPTool/`. Clean.

## Hardest-to-verify claim

**§3.4 McpAuthTool background continuation race semantics.** The spec asserts that when `Promise.race([authUrlPromise, oauthPromise.then(()=>null)])` resolves, two distinct outcomes are possible: (a) URL captured first → `status:'auth_url'` with URL embedded; (b) OAuth completed silently with no URL (e.g. cached IdP token) → `status:'auth_url'` with "completed silently" message. Verifying this requires reasoning about whether `performMCPOAuthFlow` *can* resolve before `onAuthorizationUrl` fires — that contract lives in `services/mcp/auth.ts` (spec 23 territory) and is not visible from `McpAuthTool.ts` alone. Source at `:174-197` is consistent with both branches but cannot be falsified without reading the auth-flow internals. Cited as a comment ("e.g. XAA with cached IdP token — silent auth") in `McpAuthTool.ts:176`, which is reasonable evidence but not a contract guarantee. Spec 16's claim survives because it accurately describes the *observable* tool behavior; whether the silent-auth branch is reachable at runtime depends on spec-23 internals.
