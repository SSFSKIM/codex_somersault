# Phase 9.5b Adversarial Review — Spec 32 (Mode: Kairos)

**Reviewer role**: Skeptic. **Scope**: KAIROS family flags + AGENT_TRIGGERS\*, brief mode, channels, cron, assistant-mode latch, workerType, missing-source ledger.
**Verdict**: **PASS with minor corrections**. Spec is unusually well-grounded; nearly every cited line and constant verified. Two factual errors in the missing-source ledger and a small set of nit-level issues.

---

## Severity counts

| Severity | Count |
|---|---|
| CRITICAL | 0 |
| HIGH | 1 |
| MEDIUM | 2 |
| LOW | 4 |
| INFO | 3 |

---

## Top 5 findings

### 1. HIGH — `RemoteTriggerTool/` is **present**, not missing-source

Spec §1.4 row "AGENT_TRIGGERS_REMOTE" marks this **partial — `src/tools/RemoteTriggerTool/` absent**. Spec §12 item 10 also enumerates it as missing.

**Verified false.** `ls src/tools/RemoteTriggerTool/` returns `RemoteTriggerTool.ts`, `UI.tsx`, `prompt.ts` — the tool surface is fully present. The flag (`AGENT_TRIGGERS_REMOTE`) only gates the import, not the source. Spec must demote this from "partial/missing" to "present" and remove §12.10. The skill registrar (`scheduleRemoteAgents.ts`) is also present (referenced from `skills/bundled/index.ts:56-63`) — needs verifying whether the file truly exists, but the tool absolutely does.

### 2. MEDIUM — Spec §3.3 `description` field claim is unverified

Spec §3.3 declares `ChannelPermissionRequestParams.description: string`. Source (`channelNotification.ts:87-95`) confirms the field exists, but the spec implies the string comes from `tool.description(...)` (§5.3 pseudocode). I did not verify the call-site in `interactiveHandler.ts` produces a description from the tool object vs. a generic string — spec marks this as "tool.description(...)" without citation. Cross-spec to 23 (MCP). Mark as **inferred**, not verified.

### 3. MEDIUM — `BridgeWorkerType` cross-spec note (Phase 9.5 spec 34 finding) confirmed

Phase 9.5 spec 34 found `workerType='claude_code_assistant'` at `initReplBridge.ts:477-485`. **Confirmed bit-exact**: `BridgeWorkerType = 'claude_code' | 'claude_code_assistant'` (`bridge/types.ts:79`); the latch at `initReplBridge.ts:476-484` flips on `feature('KAIROS') && isAssistantMode()`. Spec 32 does not document this, even though it claims "registry-level" coverage of the assistant-mode latch. **Recommendation**: add cross-spec line under §1.2 / §5.1 noting that the assistant-mode latch ALSO mutates `workerType` for bridge metadata, citing `bridge/initReplBridge.ts:476-484`. Currently a hole in the consistency story between spec 32 and spec 34.

### 4. LOW — Spec §1.4 KAIROS row line citation drift

Spec lists `main.tsx:1034-1089` as the assistant-mode activation block. Verified: 1030–1089 is the correct range, and the `tengu_kairos` GB key is mentioned as a comment at line 1034. The "(comment)" annotation in the constants table (§6.5) for `tengu_kairos` is correct — the literal string is *not* in that block. Fine, but adversarial: the spec implies a runtime call to `tengu_kairos`; in fact the call is inside missing-source `assistant/gate.js:isKairosEnabled()` (and the comment alone). Phase 9.7 §13.2 GrowthBook unfalsifiability concern applies: `tengu_kairos`'s exact signature cannot be confirmed from the leak.

### 5. LOW — `commands.ts:324-325` line citation

Spec §2.2 says briefCommand pushed at `:324`. Source: line 324 is `...(briefCommand ? [briefCommand] : [])` — **correct**. (Adjacent `assistantCommand` push at :325 is undocumented in spec §2.2; minor omission.)

---

## Other findings

- **INFO**: `tools.ts:25-52` citations all verified — SleepTool (PROACTIVE||KAIROS), cronTools (AGENT_TRIGGERS), RemoteTriggerTool (AGENT_TRIGGERS_REMOTE), SendUserFileTool (KAIROS), PushNotificationTool (KAIROS||KAIROS_PUSH_NOTIFICATION), SubscribePRTool (KAIROS_GITHUB_WEBHOOKS) — all bit-exact at the cited lines.
- **INFO**: `BriefTool.ts:88-134` `isBriefEntitled()` / `isBriefEnabled()` — verified verbatim including the DCE-load-bearing positive ternary and the `KAIROS_BRIEF_REFRESH_MS = 5*60*1000` constant. The `isEnvTruthy(process.env.CLAUDE_CODE_BRIEF)` branch is real.
- **INFO**: Channel constants (`PERMISSION_REPLY_RE`, `ID_ALPHABET`, `ID_AVOID_SUBSTRINGS`, FNV-1a hash, 5-letter base-25, 10 retries) — all verified verbatim (`channelPermissions.ts:75-152`).
- **LOW**: §6.5 `recurringCapMs = 15*60*1000` — verified at `cronTasks.ts:350`. All seven jitter constants verified (`:348-354`).
- **LOW**: §3.5 session-events headers `anthropic-beta: ccr-byoc-2025-07-29` and `timeout: 15000` — verified at `assistant/sessionHistory.ts:39,54`. `HISTORY_PAGE_SIZE = 100` at `:7`. 
- **LOW**: §6.3 `wrapChannelMessage` envelope shape — verified verbatim (`channelNotification.ts:106-116`); `SAFE_META_KEY` regex matches.
- **LOW**: `SubscribePRTool/`, `PushNotificationTool/`, `SendUserFileTool/`, `commands/subscribe-pr.ts`, `commands/assistant/`, `skills/bundled/dream.ts` confirmed **absent** — missing-source ledger items 1–9, 11 are correct.

---

## Cross-spec impact

- **Spec 19 (tools)**: Spec 19 must reflect that `RemoteTriggerTool` source IS present (counter to spec 32 §1.4 / §12.10). Phase 9.6 confirmed `ScheduleCronTool` present — same status applies to RemoteTriggerTool now.
- **Spec 21c (slash commands)**: `briefCommand`, `assistantCommand`, `subscribePr` registry locations confirmed at `commands.ts:66-72,101-103,240,324-325`. `force-snip` (HISTORY_SNIP) and `ultraplan` (ULTRAPLAN) flags verified — these are NOT KAIROS family, but spec 21c referenced them; correct.
- **Spec 23 (MCP)**: Channel transport schemas verbatim verified; spec 23's MCP-channel cross-reference is sound.
- **Spec 26 (analytics)**: `tengu_brief_send`, `tengu_brief_mode_toggled` — spec doesn't show them at the cited line numbers in this audit, but pattern is consistent.
- **Spec 31 (PROACTIVE)**: `SleepTool` shared import gate (`PROACTIVE || KAIROS`) — verified at `tools.ts:25-28`. AutoDream short-circuit on `getKairosActive()` claim was not verified directly but file path is correct.
- **Spec 34 (bridge/replbridge)**: workerType latch confirmed; spec 32 should add a cross-spec line.

---

## Hardest-to-verify claim

**§3.3 `notifications/claude/channel/permission_request` outbound protocol.** The spec's §5.3 pseudocode says `c.send(CHANNEL_PERMISSION_REQUEST_METHOD, {request_id, tool_name, description: tool.description(...), input_preview: truncateForPreview(input)})`. The schema constants are present in `channelNotification.ts:85-95` (verified), but the actual *callsite* that constructs and sends this notification lives in `hooks/toolPermission/handlers/interactiveHandler.ts` (per spec §2.4), which I did not read. The spec's claim that it races against `localUIDecision`, `bridgeDecision`, `hookDecision`, `classifierDecision`, and the channel onResponse is plausible but unverified in this audit. Phase 9.7 §13.2 unfalsifiability does not apply (this is a callsite question, not a GB key), but the spec's own traceability would be stronger with an interactiveHandler line citation.

Also unverifiable bit-exact: the missing-source enumeration in §12 — assistant engine, gate, sessionDiscovery, install, dream skill, SubscribePR/PushNotification/SendUserFile tools — these are correctly marked as missing, and the spec is honest about not fabricating them. No verdict possible on their internals.

---

## Verdict

**PASS** with one HIGH correction (RemoteTriggerTool ledger row), one MEDIUM addition (workerType cross-spec), and a few LOW citation cleanups. Spec 32 is one of the higher-quality entries in this batch — the verbatim regex/alphabet/blocklist, jitter constants, and channel envelope all reproduce exactly. The "missing-source" framing is intellectually honest, but the RemoteTriggerTool slip undermines that framing and should be fixed before Phase 10 close.
