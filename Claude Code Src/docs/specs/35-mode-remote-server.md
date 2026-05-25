# 35 — Mode: Remote sessions / server mode

> Sub-agent: **sub-G5** · Adjacent: 01, 22, 25, 33, 34, 41

## §1. Scope & boundaries

Covers all forms of "remote" Claude Code execution where one process hosts the agent and a second process renders the UI or drives input. **Six distinct sub-modes** (corrected Phase 9.6 from prior "five" — `/ultrareview` was missing) share machinery (`SessionsWebSocket`, `SDKControlRequest`/`SDKControlResponse`, JWT-bearer auth):

1. **CCR** (Claude Code Remote) — the agent runs in an Anthropic-managed cloud container; the local CLI is a thin viewer/controller. Setup: `/web-setup` slash command (`CCR_REMOTE_SETUP`). Auto-attach: `CCR_AUTO_CONNECT`. Mirror (outbound-only fanout): `CCR_MIRROR`.
2. **Direct connect** — `claude server` exposes an HTTP+WebSocket session host; `claude open cc://…` or `claude <cc-url>` connects (`DIRECT_CONNECT`).
3. **SSH remote** — `claude ssh <host>` deploys the binary to a Linux host, tunnels API auth back via a unix-socket reverse-forward, and runs tools remotely (`SSH_REMOTE`).
4. **Self-hosted runner** — `claude self-hosted-runner` registers and polls a worker against `SelfHostedRunnerWorkerService` (`SELF_HOSTED_RUNNER`).
5. **UDS inbox / peers** — local Unix-domain-socket message bus for sibling tmux teammates (`UDS_INBOX`).
6. **`/ultrareview` (Teleported)** — `src/commands/review.ts:45-57` ships `/ultrareview` as the primary CCR-launch user-facing entrypoint. `reviewRemote.ts` ("Teleported `/ultrareview` execution. Creates a CCR session") creates a remote session that runs the review prompt server-side. Cataloged in spec 21d (command catalog plugin & misc) §2; this spec 35 owns the launch path / session-creation behavior.

The `CLAUDE_CODE_REMOTE` environment variable is the **server-side** flag: when CCR launches the CLI inside a container it sets `CLAUDE_CODE_REMOTE=true`, which reshapes ~25 unrelated subsystems (timeouts, allowed hooks, plugin sources, FS persistence, voice, etc.). It is *not* set on the client viewer.

OUT of scope: entrypoint argv parsing → 01; HTTP retry/auth refresh → 22; OAuth/JWT machinery → 25; Daemon shell → 33; Bridge/Remote Control → 34; transcript persistence → 41; **sandbox toggle / `SandboxManager` / `dangerouslySkipPermissions` × sandbox interaction → 42** (`/sandbox-toggle` is registered at `src/commands.ts:149`; `SandboxManager` is wired at `src/main.tsx:201, 314-316`; this spec does *not* re-derive sandbox semantics — direct-connect's `dangerouslySkipPermissions` flag in §4.2 / §5.5 is purely a per-session permission-bypass flag passed to the server, not a sandbox-state mutator). **Cross-references include 21d** for the `/ultrareview` command catalog entry (Phase 10c addition) **and 42** for sandbox semantics.

## §2. Source-coverage inventory

| File | Lines | Read |
|---|---|---|
| `src/remote/RemoteSessionManager.ts` | 343 | full |
| `src/remote/SessionsWebSocket.ts` | 404 | full |
| `src/remote/sdkMessageAdapter.ts` | 302 | full |
| `src/remote/remotePermissionBridge.ts` | 78 | full |
| `src/server/createDirectConnectSession.ts` | 88 | full |
| `src/server/directConnectManager.ts` | 213 | full |
| `src/server/types.ts` | 57 | full |
| `src/commands/remote-setup/index.ts` | 20 | full |
| `src/commands/remote-setup/api.ts` | 182 | full |
| `src/commands/remote-setup/remote-setup.tsx` | 186 | full |
| `src/commands.ts` (excerpt) | :91, :108, :619-686 | scoped |
| `src/main.tsx` (excerpt) | :548-642, :685-710, :1910-1951, :2916-2975, :3156-3220, :3835-3837, :3961-4070 | scoped |
| `src/setup.ts` | :86-102 | scoped |
| `src/entrypoints/cli.tsx` | :9-14, :238-245 | scoped |
| `src/upstreamproxy/upstreamproxy.ts` | :85-103 | scoped |
| `src/bridge/bridgeEnabled.ts` | :175-202 | scoped |
| `src/bridge/remoteBridgeCore.ts` | :720-760 | scoped |
| `src/utils/sessionActivity.ts` | :10-79 | scoped |
| `src/utils/permissions/permissionSetup.ts` | :749-955 | scoped |
| `src/services/api/withRetry.ts` | :713 | spot |
| `src/services/api/claude.ts` | :810 | spot |
| `src/screens/REPL.tsx` | :280, :1383 | spot |

Repo-wide grep enumerated 30+ `CLAUDE_CODE_REMOTE*` consumers (memdir, attribution, swarm, marketplaceManager, pluginLoader, FileEdit/FileWrite, voice, init, print, WebSocketTransport, queryHelpers, authFileDescriptor, hookEvents, filePersistence, agentMemory, reload-plugins, Notifications). Cited where behaviorally load-bearing.

`src/commands/peers/`, `src/utils/udsMessaging.ts`, `src/ssh/`, `src/self-hosted-runner/` are referenced (`commands.ts:108-112`, `setup.ts:95-101`, `main.tsx:3203`, `cli.tsx:241-243`) but not in the leak (gated branches with no companion source).

## §3. Public surface

### §3.1. Entrypoints / commands

- `claude /web-setup` (alias `web`) — slash command, `CCR_REMOTE_SETUP` + GrowthBook `tengu_cobalt_lantern` + policy `allow_remote_sessions` (`commands/remote-setup/index.ts:7-18`).
- `claude server [--port --host --auth-token --unix --workspace --idle-timeout --max-sessions]` — `DIRECT_CONNECT` (`main.tsx:3961-4037`). Defaults: port `0`, host `0.0.0.0`, idle `600000` ms, max sessions `32`. Auth token auto-generated `sk-ant-cc-${randomBytes(16).base64url}` if omitted (`:3999`).
- `claude open <cc-url>` — `DIRECT_CONNECT` headless connect (`main.tsx:4058-4069`).
- `claude <cc:// | cc+unix://>...` — argv rewrite to interactive TUI (`main.tsx:612-642`).
- `claude ssh <host> [dir] [--permission-mode <m>] [--dangerously-skip-permissions] [--local]` — `SSH_REMOTE` (`main.tsx:706-734`, `:4045-4052`).
- `claude self-hosted-runner ...` — `SELF_HOSTED_RUNNER` fast-path (`entrypoints/cli.tsx:238-245`).
- Always-registered hidden flags: `--teleport [session]`, `--remote [description]`, `--sdk-url <url>` (`main.tsx:3861-3865`). `BRIDGE_MODE`-only: `--remote-control [name]` / `--rc [name]` (`:3866-3868`). `UDS_INBOX`-only: `--messaging-socket-path <path>` (`:3835-3836`).

### §3.2. Inputs

- CCR session ID, org UUID, OAuth bearer token (refreshed each connect attempt).
- Direct-connect: `serverUrl`, `authToken?`, `cwd`, `dangerouslySkipPermissions?`.
- SSH: `host`, `cwd?`, `permissionMode?`, `--local` e2e flag, `extraCliArgs[]` for `--continue`/`-c`/`--resume`/`--model` forwarded to remote spawn (`main.tsx:735-738`).
- ENV: `CLAUDE_CODE_REMOTE`, `CLAUDE_CODE_REMOTE_SESSION_ID`, `CLAUDE_CODE_REMOTE_MEMORY_DIR`, `CLAUDE_CODE_REMOTE_SEND_KEEPALIVES`, `CCR_UPSTREAM_PROXY_ENABLED`, `CLAUDE_CODE_CCR_MIRROR`, `CCR_BYOC_BETA_HEADER` (request-side).

### §3.3. Outputs / side effects

- WebSocket subscription to remote session; HTTP POST for outbound user input (CCR uses `sendEventToRemoteSession`; direct-connect inlines `SDKUserMessage`-shaped JSON over WS).
- Permission resolution loop: server → `control_request{can_use_tool}` → client UI → `control_response{success, behavior: allow|deny}` (or cancel via `control_cancel_request`).
- Heartbeat: WS `ping` every 30 s.
- `claude server`: writes a server lock file (filename TBD per §12.5 — the only `server-*.json` filename observable in the leak is `~/.claude/server-sessions.json` at `server/types.ts:42-48`; the `lockfile.ts` module that owns the lock-file path is *not* in the leak, so the lock-file filename cannot be asserted from source). Sessions index persists to `~/.claude/server-sessions.json`. Probe-running-server / lock-write call-sites at `main.tsx:3993, 4018-4024`.
- CCR mirror (when `CCR_MIRROR` and not full Remote Control): outbound-only Remote Control fanout (`main.tsx:2917-2924, 2966-2968`).

## §4. Inputs (detailed)

### §4.1. CCR session config

`RemoteSessionConfig` (`remote/RemoteSessionManager.ts:50-62`):

```
{ sessionId: string,
  getAccessToken: () => string,    // closure → fresh token per connect attempt
  orgUuid: string,
  hasInitialPrompt?: boolean,
  viewerOnly?: boolean }            // `claude assistant` viewers; disables interrupt + reconnect timeout + title updates
```

### §4.2. Direct-connect session config

`DirectConnectConfig` (`server/directConnectManager.ts:13-18`):

```
{ serverUrl: string,
  sessionId: string,
  wsUrl: string,
  authToken?: string }
```

`/sessions` POST body: `{ cwd, dangerously_skip_permissions? }`. Response (`server/types.ts:5-11`, validated via `connectResponseSchema`): `{ session_id, ws_url, work_dir? }`.

### §4.3. cc:// URL grammar

`cc://` and `cc+unix://` recognized in argv (`main.tsx:614`). Parsed by `./server/parseConnectUrl.js` → `{ serverUrl, authToken }`. Headless (`-p|--print`) rewrites `argv` to internal `open` subcommand; interactive strips the URL and re-enters main command (`:622-639`).

## §5. Algorithms (pseudocode)

### §5.1. CCR connect lifecycle

```
RemoteSessionManager.connect():
  ws = new SessionsWebSocket(sessionId, orgUuid, getAccessToken, callbacks)
  ws.connect()  // async, fire-and-forget
SessionsWebSocket.connect():
  if state == 'connecting': return
  state = 'connecting'
  url = baseApiUrl.replace('https://','wss://') + '/v1/sessions/ws/{id}/subscribe?organization_uuid={org}'
  headers = { Authorization: 'Bearer ' + getAccessToken(),
              'anthropic-version': '2023-06-01' }
  if Bun: ws = new globalThis.WebSocket(url, {headers, proxy, tls})
  else:   ws = new (await import('ws'))(url, {headers, agent, ...tls})
  on open:    state='connected'; reconnectAttempts=0; sessionNotFoundRetries=0; startPingInterval(); cb.onConnected()
  on message: jsonParse → if hasStringType field → cb.onMessage(msg)
  on close(code): handleClose(code)
  on error: cb.onError()
```

### §5.2. Reconnect / backoff

```
handleClose(code):
  stopPingInterval(); ws=null
  if state=='closed' (already closed by client): return
  state = 'closed'
  if code in {4003}: cb.onClose(); return                  // permanent (unauth)
  if code == 4001:                                          // session-not-found, transient during compaction
    sessionNotFoundRetries++
    if sessionNotFoundRetries > 3: cb.onClose(); return
    scheduleReconnect(2000 * sessionNotFoundRetries, …)
    return
  if previousState=='connected' and reconnectAttempts < 5:
    reconnectAttempts++
    scheduleReconnect(2000, …)
  else:
    cb.onClose()

scheduleReconnect(delay, label):
  cb.onReconnecting()
  reconnectTimer = setTimeout(connect, delay)

reconnect()  // explicit force-reconnect
  reconnectAttempts=0; sessionNotFoundRetries=0
  close(); setTimeout(connect, 500)
```

### §5.3. Inbound message dispatch (CCR)

```
handleMessage(msg):
  if msg.type=='control_request': handleControlRequest(msg); return
  if msg.type=='control_cancel_request':
    pending = pendingPermissionRequests.get(msg.request_id)
    pendingPermissionRequests.delete(msg.request_id)
    cb.onPermissionCancelled(msg.request_id, pending?.tool_use_id)
    return
  if msg.type=='control_response': log+drop
  else if isSDKMessage(msg): cb.onMessage(msg)             // assistant / user / result / system / stream_event / tool_progress / ...

handleControlRequest(req):
  if req.request.subtype == 'can_use_tool':
    pendingPermissionRequests.set(req.request_id, req.request)
    cb.onPermissionRequest(req.request, req.request_id)
  else:
    ws.sendControlResponse({type:'control_response',
      response:{subtype:'error', request_id, error:'Unsupported control request subtype: '+sub}})
```

### §5.4. Permission response

```
respondToPermissionRequest(requestId, result):
  if !pendingPermissionRequests.has(requestId): logError; return
  pendingPermissionRequests.delete(requestId)
  ws.sendControlResponse({type:'control_response',
    response:{subtype:'success', request_id: requestId,
              response:{ behavior: result.behavior,
                         ...(allow ? {updatedInput} : {message}) }}})
```

### §5.5. Direct-connect session creation

```
createDirectConnectSession({serverUrl, authToken?, cwd, dangerouslySkipPermissions?}):
  resp = fetch(serverUrl + '/sessions',
                method='POST',
                headers={'content-type':'application/json',
                         ...(authToken && {'authorization':'Bearer '+authToken})},
                body=JSON({cwd, ...(dangerouslySkipPermissions && {dangerously_skip_permissions:true})}))
  if network err: throw DirectConnectError('Failed to connect to server at '+serverUrl+': '+msg)
  if !resp.ok:    throw DirectConnectError('Failed to create session: '+status+' '+statusText)
  parse = connectResponseSchema().safeParse(await resp.json())
  if !parse.success: throw DirectConnectError('Invalid session response: '+err.message)
  return { config:{serverUrl, sessionId, wsUrl, authToken}, workDir }
```

### §5.6. Direct-connect inbound dispatch

Direct-connect WS streams **newline-delimited JSON** (each frame may contain multiple `\n`-separated messages); CCR sends one JSON object per WS frame. Dispatch (`directConnectManager.ts:64-114`) drops these payload types: `control_response`, `keep_alive`, `control_cancel_request`, `streamlined_text`, `streamlined_tool_use_summary`, `system{subtype:post_turn_summary}`. Unsupported `control_request` subtypes emit `control_response{error}` so the server doesn't hang (`:88-99`).

### §5.7. Direct-connect outbound user message

Wire format **must** match SDKUserMessage (`directConnectManager.ts:130-141`):

```
{ "type":"user",
  "message": { "role":"user", "content": <RemoteMessageContent> },
  "parent_tool_use_id": null,
  "session_id": "" }
```

Interrupt (`:172-186`):

```
{ "type":"control_request",
  "request_id": <uuid>,
  "request": { "subtype":"interrupt" } }
```

### §5.8. CCR `/web-setup` flow

1. `tengu_remote_setup_started` event.
2. `checkLoginState()` (`remote-setup.tsx:23-61`):
   - `isSignedIn()` → `prepareApiRequest()` succeeds? else `not_signed_in`.
   - `getGhAuthStatus()` → `not_installed | not_authenticated | authenticated`.
   - If authenticated: `execa('gh', ['auth','token'], { stdout:'pipe', stderr:'ignore', timeout: 5000, reject:false })`; trim; if empty → `gh_not_authenticated`; else wrap in `RedactedGithubToken`.
3. Routing:
   - `not_signed_in` → done with message.
   - `gh_not_installed` / `gh_not_authenticated` → `openBrowser(getCodeWebUrl()+'/onboarding?step=alt-auth')`.
   - `has_gh_token` → confirm dialog → on Continue: `importGithubToken(token)` (POST to `${BASE_API_URL}/v1/code/github/import-token` with headers `OAuthHeaders + 'anthropic-beta':'ccr-byoc-2025-07-29' + 'x-organization-uuid':orgUUID`, timeout 15000, body `{ token: token.reveal() }`).
4. On 200: best-effort `createDefaultEnvironment()` (`POST /v1/environment_providers/cloud/create` with default `Default` env, python 3.11 + node 20, `cwd:'/home/user'`, timeout 15000); skipped when `fetchEnvironments().length > 0`.
5. `openBrowser(getCodeWebUrl())` → `${CLAUDE_AI_ORIGIN}/code`.

`RedactedGithubToken` redacts `String()`, `JSON.stringify()`, and `util.inspect()` to `'[REDACTED:gh-token]'`; only `.reveal()` exposes the value (`teleport/api.ts:16-33`).

### §5.9. CCR mirror gating

```
fullRemoteControl = remoteControl || getRemoteControlAtStartup() || kairosEnabled
ccrMirrorEnabled = false
if feature('CCR_MIRROR') && !fullRemoteControl:
  ccrMirrorEnabled = isCcrMirrorEnabled()
                   = isEnvTruthy(CLAUDE_CODE_CCR_MIRROR) || GrowthBook('tengu_ccr_mirror', false)
initialState.replBridgeEnabled       = fullRemoteControl || ccrMirrorEnabled
initialState.replBridgeOutboundOnly  = ccrMirrorEnabled
```

(`main.tsx:2916-2968`, `bridge/bridgeEnabled.ts:197-202`.)

### §5.10. CCR auto-connect default

`getCcrAutoConnectDefault()` returns `true` iff `feature('CCR_AUTO_CONNECT')` AND GrowthBook `tengu_cobalt_harbor` evaluates true (`bridge/bridgeEnabled.ts:185-189`). Explicit user `remoteControlAtStartup` setting always wins.

## §6. Behavior (mandatory inlines)

### §6.1. Remote-session protocol envelope (verbatim)

Inbound subscription URL (`SessionsWebSocket.ts:108-109`):

```
${BASE_API_URL with https→wss}/v1/sessions/ws/${sessionId}/subscribe?organization_uuid=${orgUuid}
```

Connect headers (`:115-118`):

```
Authorization: Bearer ${accessToken}
anthropic-version: 2023-06-01
```

Sessions message union — any object with a `string` `type` field is forwarded; downstream classifies (`:46-55`):

```
SessionsMessage = SDKMessage | SDKControlRequest | SDKControlResponse | SDKControlCancelRequest
```

Control request envelope (CCR send-side, `:347-356`):

```
{ "type": "control_request",
  "request_id": <crypto.randomUUID>,
  "request": <SDKControlRequestInner> }
```

Control response envelope (`RemoteSessionManager.ts:263-275`):

```
{ "type": "control_response",
  "response": {
    "subtype": "success",
    "request_id": <id>,
    "response": {
      "behavior": "allow"|"deny",
      ...(allow ? { "updatedInput": <Record> } : { "message": <string> })
    }
  }
}
```

Error response envelope (`:204-211`):

```
{ "type": "control_response",
  "response": {
    "subtype": "error",
    "request_id": <id>,
    "error": <string> } }
```

Interrupt request (`SessionsWebSocket.ts:347-356` + `RemoteSessionManager.ts:294-297`):

```
{ "type": "control_request",
  "request_id": <uuid>,
  "request": { "subtype": "interrupt" } }
```

Direct-connect outbound user-message envelope (verbatim, `directConnectManager.ts:131-140`):

```
{ "type": "user",
  "message": { "role": "user", "content": <content> },
  "parent_tool_use_id": null,
  "session_id": "" }
```

### §6.2. UDS socket path layout

UDS messaging server starts iff `feature('UDS_INBOX')` AND not `--bare` mode (or `--messaging-socket-path` explicitly set) (`setup.ts:86-101`):

```
startUdsMessaging(
  messagingSocketPath ?? getDefaultUdsSocketPath(),
  { isExplicit: messagingSocketPath !== undefined }
)
```

The CLI flag is registered as (`main.tsx:3835-3836`):

```
--messaging-socket-path <path>
  Unix domain socket path for the UDS messaging server (defaults to a tmp path)
```

After bind, the server exports `$CLAUDE_CODE_MESSAGING_SOCKET` for child processes (`setup.ts:91-94` comment); SessionStart hooks see it.

Direct-connect server unix-socket: registered via `--unix <path>` option (`main.tsx:3962`); when set, the server-lock `httpUrl` field is `unix:${path}` instead of `http://${host}:${port}` (`:4022`).

(Note: `src/utils/udsMessaging.ts` and `src/commands/peers/` are referenced from `setup.ts:96-100` and `commands.ts:108-112` but not present in the leak; the consumer-side path/contract above is fully captured.)

### §6.3. Setup flow user-facing strings (verbatim)

From `commands/remote-setup/remote-setup.tsx`:

- `"Checking login status…"` (`:153`).
- `"Connecting GitHub to Claude…"` (`:156`).
- `"Connect Claude on the web to GitHub?"` — dialog title (`:159`).
- `"Claude on the web requires connecting to your GitHub account to clone and push code on your behalf."` (`:162-164`).
- `"Your local credentials are used to authenticate with GitHub"` (dim, `:166`).
- Select options: `"Continue"` / `"Cancel"` (`:170, :173`).
- On `not_signed_in`: `"Not signed in to Claude. Run /login first."` (`:98`).
- On `gh_not_installed`: ``GitHub CLI not found. Install it via https://cli.github.com/, then run `gh auth login`, or connect GitHub on the web: ${url}`` (`:108`).
- On `gh_not_authenticated`: ``GitHub CLI not authenticated. Run `gh auth login` and try again, or connect GitHub on the web: ${url}`` (`:108`).
- On success: `` `Connected as ${result.result.github_username}. Opened ${url}` `` (`:150`).

`errorMessage()` returns (`:62-72`):

- `not_signed_in` → `` `Login failed. Please visit ${codeUrl} and login using the GitHub App` ``.
- `invalid_token` → `` 'GitHub rejected that token. Run `gh auth login` and try again.' ``.
- `server` → `` `Server error (${err.status}). Try again in a moment.` ``.
- `network` → `` "Couldn't reach the server. Check your connection." ``.

Slash command description (`commands/remote-setup/index.ts:9`): `"Setup Claude Code on the web (requires connecting your GitHub account)"`. `availability: ['claude-ai']`.

`server` banner is printed by `./server/serverBanner.js` `printBanner(config, authToken, actualPort)` (`main.tsx:3984-4017`); banner content not in the leak.

### §6.4. Constants tables

**IMPORTANT — two parallel WebSocket stacks.** The CLI ships *two distinct* WebSocket implementations with overlapping but **non-identical** reconnect/close-code semantics. Do not conflate them:

- **Stack A — `src/remote/SessionsWebSocket.ts`** (powers `RemoteSessionManager`, the viewer-side CCR bridge described in §3-§5 / §6.1). Subscribes to `wss://…/v1/sessions/ws/{id}/subscribe`. Uses an *attempt-count* budget (5 retries) and treats only `{4003}` as permanent.
- **Stack B — `src/cli/transports/WebSocketTransport.ts`** (powers `getTransportForUrl`-selected transport for SDK / CCR-v2 worker session ingress; selected by `transportUtils.ts` when neither `CLAUDE_CODE_USE_CCR_V2` nor `CLAUDE_CODE_POST_FOR_SESSION_INGRESS_V2` are set — see §13). Uses a *time-budget* reconnect ceiling (10 min wall clock), permanent codes `{1002, 4001, 4003}`, has 4003-with-`refreshHeaders()` recovery (one-shot per disconnect), buffered-message replay via `lastSentId` and `X-Last-Request-Id` request header, 5-minute keepalive frames (suppressed under `CLAUDE_CODE_REMOTE`).

These stacks never share a connection. Stack A carries the CCR control-plane (permission prompts, interrupt, session events); stack B carries the SDK session-ingress data-plane.

#### §6.4.A. Stack A constants — `src/remote/SessionsWebSocket.ts`

| Constant | Value | Source |
|---|---|---|
| `RECONNECT_DELAY_MS` | 2000 | `SessionsWebSocket.ts:17` |
| `MAX_RECONNECT_ATTEMPTS` | 5 (attempt-count budget) | `:18` |
| `PING_INTERVAL_MS` | 30000 | `:19` |
| `MAX_SESSION_NOT_FOUND_RETRIES` | 3 | `:26` |
| `PERMANENT_CLOSE_CODES` | `{ 4003 }` (unauthorized only) | `:34-36` |
| 4001 close code | session-not-found, transient (×3 retries, delay scaled by attempt) | `:258-272` |
| Force-reconnect bounce delay | 500 ms | `:399-402` |
| Sessions WS path | `/v1/sessions/ws/{sessionId}/subscribe?organization_uuid={org}` | `:108-109` |
| `anthropic-version` header | `2023-06-01` | `:117` |
| Token-refresh on close | none — 4003 is permanent (refresh handled out-of-band by `getAccessToken()` closure on the *next* connect attempt) | `:113-118` |
| Buffered replay | none | — |

#### §6.4.B. Stack B constants — `src/cli/transports/WebSocketTransport.ts`

| Constant | Value | Source |
|---|---|---|
| `DEFAULT_RECONNECT_GIVE_UP_MS` | 600_000 (10 min, time-budget) | `WebSocketTransport.ts:26` |
| `DEFAULT_KEEPALIVE_INTERVAL` | 300_000 (5 min) — suppressed when `CLAUDE_CODE_REMOTE` truthy | `:28, :771-791` |
| `PERMANENT_CLOSE_CODES` | `{ 1002, 4001, 4003 }` | `:42-46` |
| 4003 token-refresh recovery | one-shot per disconnect: `refreshHeaders()` → re-attempt; survives the otherwise-permanent 4003 | `:424-438, :500-503` |
| Buffered replay | `lastSentId` re-sent as `X-Last-Request-Id` header on reconnect | `:76, :152-155, :205, :592, :663` |
| Reconnect base/max delay | (see §13 SSETransport row for SSE; WS uses internal exponential — call-site only) | `:474-503` |

#### §6.4.C. Setup / direct-connect / `CLAUDE_CODE_REMOTE` constants

| Constant | Value | Source |
|---|---|---|
| `import-token` URL | `${BASE_API_URL}/v1/code/github/import-token` | `remote-setup/api.ts:64` |
| `CCR_BYOC_BETA_HEADER` | `ccr-byoc-2025-07-29` | `:7` |
| `import-token` HTTP timeout | 15000 ms | `:75` |
| `gh auth token` exec timeout | 5000 ms | `remote-setup.tsx:48` |
| `claude server` default port | `0` (random) | `main.tsx:3962` |
| `claude server` default host | `0.0.0.0` | `:3962` |
| `claude server` default idle timeout | `600000` ms | `:3962` |
| `claude server` default max sessions | `32` | `:3962` |
| Auto-generated server token format | `sk-ant-cc-${randomBytes(16).toString('base64url')}` | `:3999` |
| Direct-connect session schema | `{ session_id, ws_url, work_dir? }` (zod, `lazySchema`) | `server/types.ts:5-11` |
| Direct-connect drop list | `control_response, keep_alive, control_cancel_request, streamlined_text, streamlined_tool_use_summary, system{post_turn_summary}` | `directConnectManager.ts:104-110` |
| `CLAUDE_CODE_REMOTE` heap bump | adds `--max-old-space-size=8192` to `NODE_OPTIONS` at process start | `entrypoints/cli.tsx:9-14` |
| API timeout under `CLAUDE_CODE_REMOTE` | 120 000 ms (else 300 000 ms) | `services/api/claude.ts:810` |
| Permission modes allowed under `CLAUDE_CODE_REMOTE` settings | `acceptEdits`, `plan` only | `utils/permissions/permissionSetup.ts:749-755` |

### §6.5. `REMOTE_SAFE_COMMANDS` allowlist (verbatim)

From `src/commands.ts:619-637`:

```
session, exit, clear, help, theme, color, vim, cost, usage,
copy, btw, feedback, plan, keybindings, statusline, stickers, mobile
```

Comment annotations:

- `session` — Shows QR code / URL for remote session
- `exit` — Exit the TUI
- `clear` — Clear screen
- `help` — Show help
- `theme` — Change terminal theme
- `color` — Change agent color
- `vim` — Toggle vim mode
- `cost` — Show session cost (local cost tracking)
- `usage` — Show usage info
- `copy` — Copy last message
- `btw` — Quick note
- `feedback` — Send feedback
- `plan` — Plan mode toggle
- `keybindings` — Keybinding management
- `statusline` — Status line toggle
- `stickers` — Stickers
- `mobile` — Mobile QR code

Two consumers:

1. `filterCommandsForRemoteMode(commands)` — pre-filters before REPL renders, preventing local-only commands from flickering before the CCR `init` message arrives (`:684-686`).
2. REPL `handleRemoteInit` post-CCR-filter merge — preserves these even when CCR sends a stricter set (`screens/REPL.tsx:1383`).

`BRIDGE_SAFE_COMMANDS` (a separate set: `compact, clear, cost, summary, releaseNotes, files`) governs Bridge inbound, **not** CCR — see spec 34.

### §6.6. `CLAUDE_CODE_REMOTE` env effects (cross-cuts)

When the CLI executes inside CCR, the host sets `CLAUDE_CODE_REMOTE=true` and (when applicable) `CLAUDE_CODE_REMOTE_SESSION_ID`, `CLAUDE_CODE_REMOTE_MEMORY_DIR`. Behavioral effects (each cited; see also adjacent specs):

- Adds `--max-old-space-size=8192` to `NODE_OPTIONS` before any module loads (`entrypoints/cli.tsx:9-14`).
- Always-includes hook events in transcript (`main.tsx:1229-1231`); session ID derived from `CLAUDE_CODE_REMOTE_SESSION_ID || getSessionId()` for file persistence (`:1317`).
- API timeout drops 300 s → 120 s (`services/api/claude.ts:810`); retry policy adjusted (`services/api/withRetry.ts:713`); 401/403 retryable (per spec 22).
- File persistence path shifts (`utils/filePersistence/filePersistence.ts:65-69, 274-283`) — requires `CLAUDE_CODE_REMOTE_SESSION_ID`.
- `memdir` resolution: `CLAUDE_CODE_REMOTE_MEMORY_DIR` is **explicit override**; if `CLAUDE_CODE_REMOTE` is set but the memory dir isn't, MEMORY.md is **OFF** (`memdir/paths.ts:26, 45-46, 82-90`).
- `agentMemory.ts:30-33, 86-90` namespaces under `${CLAUDE_CODE_REMOTE_MEMORY_DIR}/projects/`.
- Plugin loader rewrites git URLs (`utils/plugins/pluginLoader.ts:674, 683-688`) and skips marketplace install (`marketplaceManager.ts:2476`).
- Voice disabled (`services/voice.ts:261`).
- Skips auto-FD auth (`utils/authFileDescriptor.ts:35`).
- Banned/limited features in REPL: `print.ts:512, 1711, 3069`, `Notifications.tsx:308`, `reload-plugins.ts:26` use `getIsRemoteMode()` to alter behavior.
- Settings `defaultMode` constrained to `acceptEdits` or `plan` (`utils/permissions/permissionSetup.ts:749-755`); rejection error: ``settings defaultMode "${settingsMode}" is not supported in CLAUDE_CODE_REMOTE — only acceptEdits and plan are allowed``.
- WebSocketTransport (Stack B, §6.4.B) keepalive **suppression** under `CLAUDE_CODE_REMOTE`: `cli/transports/WebSocketTransport.ts:771-791` — when running inside CCR the 5-min `DEFAULT_KEEPALIVE_INTERVAL` data frames are skipped (the surrounding CCR infrastructure provides liveness via Stack A's `PING_INTERVAL_MS=30000`). This is *only* a keepalive branch; it is **not** the gating point that selects WebSocketTransport vs HybridTransport vs SSETransport — that selection is in `cli/transports/transportUtils.ts` (see §13).
- Spawn-utils inheritance: `utils/swarm/spawnUtils.ts:109,113` propagates `CLAUDE_CODE_REMOTE` and `CLAUDE_CODE_REMOTE_MEMORY_DIR` to spawned children.
- Session keep-alives: `utils/sessionActivity.ts:36, 79` send keep-alives only when `CLAUDE_CODE_REMOTE_SEND_KEEPALIVES` is truthy.
- Init: `entrypoints/init.ts:163-167` runs an extra remote-only init step.
- Auth: `utils/auth.ts:93` and `init.ts` adjust login flow.

The above are *consumers* — the env var itself is set by the CCR controller out-of-band; the CLI never sets it on a viewer client.

### §6.7. Upstream-proxy gating

`initUpstreamProxy()` no-ops unless **all three** are present (`upstreamproxy/upstreamproxy.ts:85-110`):

1. `CLAUDE_CODE_REMOTE` truthy.
2. `CCR_UPSTREAM_PROXY_ENABLED` truthy (server-side GrowthBook → injected env).
3. `CLAUDE_CODE_REMOTE_SESSION_ID` set.
4. Session-token file at `SESSION_TOKEN_PATH` exists and is non-empty.

Then `setNonDumpable()` is called and `ANTHROPIC_BASE_URL` is read (CCR injects via StartupContext).

### §6.8. SDK message → REPL message conversion

`convertSDKMessage(msg, opts?)` (`remote/sdkMessageAdapter.ts:168-278`) returns one of `{type:'message', message}`, `{type:'stream_event', event}`, `{type:'ignored'}`. Cases:

- `assistant` → `AssistantMessage` (preserves `uuid`, `error`, sets `requestId:undefined`, fresh ISO timestamp).
- `user` → ignored unless `opts.convertToolResults` and content has `tool_result` blocks (creates `UserMessage` with `toolUseResult`), or `opts.convertUserTextMessages` for non-tool-result historical text.
- `stream_event` → `StreamEvent` `{type:'stream_event', event}`.
- `result` — only renders if `subtype !== 'success'` (errors only); content is `errors?.join(', ') || 'Unknown error'`, level `warning`. Success → ignored (REPL relies on `isLoading=false`).
- `system{init}` → SystemMessage with `Remote session initialized (model: ${msg.model})`.
- `system{status}` → if `status=='compacting'` → `Compacting conversation…`; else `Status: ${status}`; null `status` → ignored.
- `system{compact_boundary}` → `Conversation compacted` with `compactMetadata` mapped via `fromSDKCompactMetadata`.
- `system{hook_response}` and others → ignored + log.
- `tool_progress` → `Tool ${tool_name} running for ${elapsed_time_seconds}s…` with `toolUseID`.
- `auth_status`, `tool_use_summary`, `rate_limit_event`, unknown → ignored + log (graceful forward-compat).

Ancillary helpers: `isSessionEndMessage` (true iff `result`), `isSuccessResult`, `getResultText`.

### §6.9. Synthetic remote tool-use rendering

For permission prompts on tools the local CLI doesn't have loaded (e.g. remote MCP tools), `createSyntheticAssistantMessage(request, requestId)` produces a fake `AssistantMessage` with a single `tool_use` content block, `id: request.tool_use_id`, `name: request.tool_name`, `input: request.input`, model `''`, zero usage (`remote/remotePermissionBridge.ts:12-46`). `createToolStub(toolName)` returns a minimal `Tool` that always `needsPermissions() === true`, `isReadOnly() === false`, `isMcp: false`, `userFacingName === toolName`, and renders the first 3 input entries as `key: value, …` (`:53-78`).

### §6.10. SSH remote (`SSH_REMOTE`) flow sketch

Source not in leak; behavior captured from the call-sites:

1. argv-rewrite (`main.tsx:706-734`) extracts `--local`, `--dangerously-skip-permissions`, `--permission-mode`, plus the host positional.
2. `--continue`/`-c`/`--resume <uuid>`/`--model` are extracted into `_pendingSSH.extraCliArgs` and forwarded to the remote spawn (`:735-738`); these operate on the **remote** session history under the remote `~/.claude/projects/<cwd>/`.
3. Main command branch (`:3193-3220`): imports `./ssh/createSSHSession.js` (not present in leak), prints `Connecting to ${host}…\n` to stderr (with `\r`+EL0 progress when TTY), then `createSSHSession()` or `createLocalSSHSession()` (when `--local`).
4. Local proxy: ssh `-R unix-socket:` reverse-forwards to a local auth proxy so the remote uses the local user's API token without re-login (per `:4046-4052` description). Tools execute remotely; UI renders locally.
5. Headless `-p` not supported v1 (per comment at `:704-705`).

### §6.11. Self-hosted runner

Fast-path before main CLI loads (`entrypoints/cli.tsx:238-245`):

```
if feature('SELF_HOSTED_RUNNER') and args[0] == 'self-hosted-runner':
  await import('../self-hosted-runner/main.js').selfHostedRunnerMain(args.slice(1))
  return
```

Per the comment, the runner targets `SelfHostedRunnerWorkerService` with a register-then-poll loop where the poll *is* the heartbeat. Runtime source not in leak.

## §7. Failure modes

- WS open fails → `cb.onError(Error('[SessionsWebSocket] WebSocket error'))`; no immediate reconnect; close handler (when it fires) drives the schedule.
- Permanent 4003 close → `cb.onClose()`; no further reconnects.
- 4001 (session-not-found) > 3 retries → `cb.onClose()` with log `4001 retry budget exhausted (3), not reconnecting`.
- Generic close with `reconnectAttempts >= 5` → `cb.onClose()` with log `Not reconnecting`.
- `sendControlResponse` / `sendControlRequest` while not connected → `logError('[SessionsWebSocket] Cannot send: not connected')`, no throw.
- `respondToPermissionRequest` for unknown `requestId` → `logError`, no send.
- Unsupported `control_request.subtype` → server gets `control_response{error}` to avoid hang.
- Unknown SDK message `type` → logged and dropped (forward-compat).
- Direct-connect `/sessions` POST: network error → `DirectConnectError('Failed to connect to server at ${url}: ${msg}')`; HTTP non-OK → `DirectConnectError('Failed to create session: ${status} ${statusText}')`; schema mismatch → `DirectConnectError('Invalid session response: ${err.message}')`. Caller (`main.tsx:3172-3173`) calls `exitWithError → gracefulShutdown(1)`.
- Direct-connect WS not OPEN on send → silent return false / `void` (no throw).
- `claude server` already running (`probeRunningServer`) → stderr `A claude server is already running (pid ${existing.pid}) at ${existing.httpUrl}` and `process.exit(1)` (`main.tsx:3994-3998`).
- `/web-setup`:
  - Not signed in → message `"Not signed in to Claude. Run /login first."`.
  - GH not installed/authenticated → opens `${codeUrl}/onboarding?step=alt-auth` and returns informational message.
  - 200 success → opens `${codeUrl}` and reports `Connected as ${username}`.
  - 400 → `invalid_token`; 401 → `not_signed_in`; other status → `server{status}`; thrown → `network`. Errors logged via `logForDebugging` excluding the request body (which contains the raw token).
  - `createDefaultEnvironment()` is best-effort; failure is non-fatal — web onboarding state machine routes to `env-setup` on landing.
- `viewerOnly` mode (e.g. `claude assistant`): Ctrl+C/Esc do **not** send `interrupt`; reconnect doesn't enforce a 60 s timeout; session title never updated (`RemoteSessionManager.ts:55-61`).

## §8. Observability

Analytics events (logEvent):

- `tengu_remote_setup_started` — `/web-setup` invoked.
- `tengu_remote_setup_result` — fields: `result ∈ { not_signed_in, gh_not_installed, gh_not_authenticated, cancelled, import_failed, success }`; on `import_failed` also `error_kind ∈ {invalid_token, not_signed_in, server, network}`.
- `tengu_ccr_mirror_started` / `tengu_ccr_mirror_teardown` (when `CCR_MIRROR` outbound-only path) — fields `v2:true, expires_in_s, archive_*` (`bridge/remoteBridgeCore.ts:732-743, 748-752`).
- `tengu_bridge_repl_started` / `tengu_bridge_repl_teardown` (regular Remote Control teardown).

Debug log prefixes: `[RemoteSessionManager]`, `[SessionsWebSocket]`, `[DirectConnect]`, `[sdkMessageAdapter]`, `[upstreamproxy]`. All routed via `logForDebugging` (`utils/debug.ts`); errors via `logError`.

GrowthBook gates referenced: `tengu_cobalt_lantern` (gates `/web-setup` visibility), `tengu_cobalt_harbor` (auto-connect), `tengu_ccr_mirror` (mirror rollout).

Policy: `allow_remote_sessions` — when false, `/web-setup` `isHidden` and `isEnabled=false`.

## §9. Security & permissions

- Bearer JWT on every Sessions WS connect; `getAccessToken` is a **closure** so each reconnect attempt re-reads the current token (handles refresh between retries) (`SessionsWebSocket.ts:113-118`).
- TLS: `getWebSocketTLSOptions()` applied (mTLS-aware); proxy: `getWebSocketProxyUrl(url)` / `getWebSocketProxyAgent(url)` (`:120-170`).
- Permanent-close 4003 is the unauth signal; client stops reconnecting rather than retrying with a stale token (cross-ref 22 §retry).
- `RedactedGithubToken` masks the GitHub token in all common stringifications; raw value only via `.reveal()`. Axios error logs intentionally exclude `err.config.data` because that body contains the raw token (`teleport/api.ts:91-97`).
- `import-token` request includes `anthropic-beta: ccr-byoc-2025-07-29` and `x-organization-uuid: ${orgUUID}`.
- Direct-connect: `--auth-token` accepted on server CLI; auto-generated `sk-ant-cc-...base64url(16)` if absent. Token printed via `printBanner` (banner source not in leak). `cc://` URL scheme conveys the token to the client.
- `setNonDumpable()` invoked after upstream-proxy auth init to prevent core-dump leakage of session token.
- `CLAUDE_CODE_REMOTE` settings constraint: `defaultMode` must be `acceptEdits` or `plan`; other modes rejected with the verbatim error in §6.4.
- Permission resolution: tool inputs are revalidated locally; `result.behavior=='allow'` carries `updatedInput` so the client can rewrite tool input before remote execution (e.g. user-edited path arguments).

## §10. Testing & fixtures

No test files in the leak. Notable test/dev affordances captured from production code:

- `--local` flag for `claude ssh` (`main.tsx:715-718`) skips probe/deploy/SSH and spawns the child CLI directly with the same env — described in-source as "e2e test of the proxy/auth plumbing".
- `createDirectConnectSession` accepts `dangerouslySkipPermissions` purely as a session-init flag; the server side enforces.
- `createLocalSSHSession` (imported from `./ssh/createSSHSession.js` at `main.tsx:3199-3203`).

## §11. Cross-references

- 01 — argv parse / fast-paths / boot order for `cc://`, `ssh`, `self-hosted-runner`, `--handle-uri`.
- 22 — `withRetry`, 401/403 retryable for `CLAUDE_CODE_REMOTE`, base API URL.
- 25 — OAuth/JWT acquisition; the `getAccessToken` closure passed to `RemoteSessionManager`.
- 26 — GrowthBook gates `tengu_cobalt_lantern` / `tengu_cobalt_harbor` / `tengu_ccr_mirror`.
- 27 — `allow_remote_sessions` policy.
- 33 — Daemon (process model overlap with self-hosted-runner).
- 34 — Bridge / Remote Control; `CCR_MIRROR` is the outbound-only sibling.
- 41 — `--continue`/`--resume` semantics for SSH remote operate on **remote** transcripts.
- 42 — sandbox toggle / `SandboxManager` / `/sandbox-toggle` command (`src/commands.ts:149`, `src/main.tsx:201,314-316`). Spec 35's `dangerouslySkipPermissions` plumbing (§4.2, §5.5, §9) is independent of sandbox state.

## §12. Open questions (deferred)

> **Phase 9.7 sweep (2026-05-09)**: items below re-audited. Most are missing-leaked-source items that remain DEFERRED.

1. ~~**`src/utils/udsMessaging.ts` source**~~ — **DEFERRED (missing-leaked-source)**: server-side bind/protocol absent from leak; consumer-side captured. Recorded in spec 00 §13 as known-unfalsifiable for `UDS_INBOX` feature flag.
2. ~~**`src/commands/peers/index.ts`**~~ — **DEFERRED (missing-leaked-source)**: file referenced from `commands.ts:108-112` but not present in leak. Spec 21d (plugin-and-misc commands) records as DCE'd / missing.
3. ~~**`src/ssh/createSSHSession.ts`**~~ — **DEFERRED (missing-leaked-source)**: full SSH mechanics absent; only `main.tsx:3193-3220` call-site captured. Recorded.
4. ~~**`src/self-hosted-runner/main.ts`**~~ — **DEFERRED (missing-leaked-source)**: register/poll loop, worker-service API, heartbeat semantics absent from leak. Recorded.
5. ~~**`src/server/{server,sessionManager,...}.ts` direct-connect server-side surface**~~ — **DEFERRED (missing-leaked-source)**: imported but not present at cited paths. Lock-file format, `/sessions` schema, banner, log redaction — all server-side, missing from leak. Spec 00 §2.5 missing-source ledger covers.
6. ~~**CCR `sendEventToRemoteSession` HTTP envelope**~~ — **NOTE Phase 9.7**: spec 22 (service-api) §3 documents the HTTP envelope at the consumer interface; body shape and retry semantics owned there.
7. ~~**Close-code 4001 server semantics during compaction**~~ — **DEFERRED**: backend behavior not visible in leak. The 3×retry constant from comment is the only client-side observable; server-side close-code semantics are server-policy.

## §13. `src/cli/transports/` catalog (Phase 10 cleanup)

`WebSocketTransport.ts` is enumerated above (§3, gating at line 771). The
remaining files in `src/cli/transports/` implement transport-selection
logic and the v2 (POST/SSE) transport variants used for remote sessions.

| File | Purpose |
|---|---|
| `src/cli/transports/HybridTransport.ts` | WS-for-reads + HTTP-POST-for-writes hybrid. Activated when `CLAUDE_CODE_POST_FOR_SESSION_INGRESS_V2` is truthy. Uses `SerialBatchEventUploader` (100ms flush) + per-POST `POST_TIMEOUT_MS=15s`; `CLOSE_GRACE_MS=3s` drain budget on shutdown (best-effort, exceeds the 2s `gracefulShutdown` cleanup budget but the process lives ~2s longer for hooks/analytics). |
| `src/cli/transports/SSETransport.ts` | SSE-for-reads + HTTP-POST-for-writes. Activated when `CLAUDE_CODE_USE_CCR_V2` is truthy (v2 CCR mirror). Reconnection: `RECONNECT_BASE_DELAY_MS=1s` exponential up to `RECONNECT_MAX_DELAY_MS=30s`, give up after `RECONNECT_GIVE_UP_MS=10min`; liveness timeout `LIVENESS_TIMEOUT_MS=45s` (server keepalive every 15s, dead after 45s of silence). **POST-write retry contract (inline `sleep+retry`, *not* via `SerialBatchEventUploader`):** `POST_MAX_RETRIES=10` (`SSETransport.ts:30`), `POST_BASE_DELAY_MS=500` (`:31`), `POST_MAX_DELAY_MS=8000` (`:32`); per-attempt delay = `min(POST_BASE_DELAY_MS * 2^(attempt-1), POST_MAX_DELAY_MS)` with ±25% jitter (`:639-649`); 4xx (non-429) treated as permanent and dropped; loop continues after exhaustion with a warning log (`:639-641`). Compare HybridTransport's row: HybridTransport's POSTs flow through `SerialBatchEventUploader`'s coalescing+backpressure path; SSETransport's POSTs are direct, fire-and-retry per call. |
| `src/cli/transports/SerialBatchEventUploader.ts` | Generic serial ordered batch uploader: ≤1 in-flight POST, batched draining, exponential backoff with jitter (clamped), indefinite retry until `close()` or `maxConsecutiveFailures` (drops batch). `enqueue()` blocks for backpressure when `maxQueueSize` reached. Supports `retryAfterMs` for server 429 with `Retry-After`. |
| `src/cli/transports/WorkerStateUploader.ts` | Coalescing PUT `/worker` uploader (session state + metadata): ≤1 in-flight PUT + ≤1 pending patch (naturally bounded). Coalescing rules: top-level keys = last-value-wins; inside `external_metadata`/`internal_metadata` = RFC 7396 merge (null preserves server-side delete). |
| `src/cli/transports/ccrClient.ts` | CCR (Claude Code Remote) client. Wires `decodeJwtExpiry`, `createAxiosInstance` (proxy-aware), `registerSessionActivityCallback`, `getSessionIngressAuthHeaders/Token`, `RequiresActionDetails` / `SessionState`. Handles SDK partial-assistant streaming via `SDKPartialAssistantMessage`/`StdoutMessage`. |
| `src/cli/transports/transportUtils.ts` | `getTransportForUrl(url, headers, sessionId, refreshHeaders)` — selection priority: (1) `SSETransport` if `CLAUDE_CODE_USE_CCR_V2`; (2) `HybridTransport` if `CLAUDE_CODE_POST_FOR_SESSION_INGRESS_V2`; (3) `WebSocketTransport` (default). Derives the SSE stream URL by appending `/worker/events/stream` to the session URL. |
