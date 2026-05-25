# 33 — Mode Delta: Daemon (`feature('DAEMON')`)

> Mode-delta spec for the build-time `DAEMON` feature gate. Adjacent: 22 (api),
> 23 (mcp), 34 (bridge), 35 (remote-server). This spec documents ONLY the
> deltas the `DAEMON` flag introduces. Anthropic-internal (`USER_TYPE === 'ant'`)
> code is in scope per the master plan.
>
> **Source-coverage status: severely partial.** The supervisor implementation
> (`src/daemon/main.ts`, `src/daemon/workerRegistry.ts`) and the
> `remoteControlServer` command body (`src/commands/remoteControlServer/`) are
> referenced from the leaked tree but the directories themselves are absent.
> What survives: three callsites in `cli.tsx` / `commands.ts`, the
> `SessionKind` enum literals (`'daemon' | 'daemon-worker'`), one PID-file
> field, and one env-var contract. Nearly every behavioral section below is
> annotated *missing-source*. See §12.

---

## 1. Purpose & Scope

`feature('DAEMON')` is a build-time bundler flag (`bun:bundle`) that, when
enabled, compiles three additional control paths into the CLI:

1. A `claude daemon [subcommand]` subcommand fast-path that boots a long-running
   supervisor process via `daemonMain` from `../daemon/main.js`
   (cli.tsx:165-180).
2. A `--daemon-worker <kind>` internal fast-path used by the supervisor when it
   spawns child workers; it dispatches to `runDaemonWorker(kind)` from
   `../daemon/workerRegistry.js` (cli.tsx:95-106).
3. A `remoteControlServerCommand` slash-command, which is **further gated** by
   `feature('BRIDGE_MODE')` (commands.ts:76-79, 327). When co-gated with
   `BRIDGE_MODE`, this is the only DAEMON-attributable surface that reaches the
   in-REPL command system; in a pure-DAEMON build (DAEMON on, BRIDGE_MODE off)
   the command is omitted and DAEMON contributes **zero** in-REPL slash
   commands. It lives at `./commands/remoteControlServer/index.js`.

Indirectly, the `SessionKind` enum gains the values `'daemon'` and
`'daemon-worker'` so that `concurrentSessions` PID-file registration, `claude
ps` listing, and cleanup can identify supervisor + worker processes
(concurrentSessions.ts:18, 34).

### IN scope
- The three feature-flag callsites above and their immediate behavior.
- The `SessionKind` enum extension and the `CLAUDE_CODE_SESSION_KIND` env-var
  contract for daemon spawns.
- The PID-file lifecycle for daemon and daemon-worker processes (delegated to
  the shared `concurrentSessions.ts` machinery).
- The lazy-import / DCE pattern that gates daemon code out of non-daemon
  builds.

### OUT of scope (refer by spec #)
- Anthropic API client                                 → 22
- MCP service host / clients                           → 23
- Bridge IPC, IDE bridge protocol                      → 34
- Remote sessions / `src/remote/`, `src/server/`       → 35
- General slash-command registry shape                 → 20
- Bridge-mode command surface (incl. `remoteControl`
  REPL option, `remoteControlAtStartup` setting)      → 34
- The `--assistant` / "Agent SDK daemon" string in
  main.tsx (Kairos `assistant` subcommand, not
  `feature('DAEMON')`) — see master plan §3 / spec 32
- The `claude self-hosted-runner` fast-path
  (cli.tsx:235-243) gated on `feature('SELF_HOSTED_RUNNER')`
  with body in the absent `../self-hosted-runner/main.js`.
  Sibling structural shape (lazy import, fast-path, missing
  implementation directory) but a **distinct gate** — not a
  DAEMON variant. See spec 35.
- The legacy CLI fast-paths
  `claude remote-control|rc|remote|sync|bridge`
  (cli.tsx:108-162) are gated on `feature('BRIDGE_MODE')`
  alone (no DAEMON co-gate) and are unrelated to DAEMON.
  See spec 34.

The `--assistant` flag and "Agent SDK daemon" comments at main.tsx:1053-1074,
2643, 3288-3290, 3842 are part of the Kairos `assistant` subsystem (spec 32),
**not** `feature('DAEMON')`. They share the word "daemon" but use distinct
gates. Do not redocument here.

---

## 2. Source Map

**Present in leak (full):**

| Path | Lines | Role |
|---|---|---|
| `src/entrypoints/cli.tsx` | 95-106 | `--daemon-worker` fast-path |
| `src/entrypoints/cli.tsx` | 164-180 | `claude daemon` fast-path |
| `src/commands.ts` | 76-79 | `remoteControlServerCommand` lazy-require |
| `src/commands.ts` | 327 | Spread into command list |
| `src/utils/concurrentSessions.ts` | 18 | `SessionKind` enum incl. `'daemon'`, `'daemon-worker'` |
| `src/utils/concurrentSessions.ts` | 31-37 | `envSessionKind()` reads `CLAUDE_CODE_SESSION_KIND` |
| `src/utils/concurrentSessions.ts` | 59-109 | `registerSession()` PID-file write incl. `kind` field |

**Missing source (referenced but not present in leak):**

| Path | Referenced from | Exports expected |
|---|---|---|
| `src/daemon/main.ts` (or `.tsx`) | cli.tsx:177 | `daemonMain(args: string[]): Promise<void>` |
| `src/daemon/workerRegistry.ts` (or `.tsx`) | cli.tsx:103 | `runDaemonWorker(kind: string): Promise<void>` |
| `src/daemon/` directory | both above | entire daemon implementation |
| `src/commands/remoteControlServer/index.ts` | commands.ts:78 | default-exported `Command` |
| `src/commands/remoteControlServer/` directory | commands.ts | the command body, prompt, args |

**Adjacent context that informs but is not owned here:**

| Path | Owned by spec | Why relevant |
|---|---|---|
| `src/utils/concurrentSessions.ts` | 41 (session-state-history) / 19 | Hosts the `SessionKind`; daemon adds two values |
| `src/bridge/` | 34 | Daemon command is co-gated by `BRIDGE_MODE` |
| `src/services/api/` | 22 | Daemon may host an API surface (unverifiable) |
| `src/services/mcp/` | 23 | Daemon may host MCP servers (unverifiable) |

---

## 3. Public Interface (Contract)

### 3.1 CLI surface

Two CLI invocations are added under `feature('DAEMON')`:

```
claude daemon [subcommand...]                  — supervisor entry
claude --daemon-worker <kind>                   — internal worker entry
```

The leak reveals **no** subcommand list, **no** worker `<kind>` enumeration,
**no** flags, **no** stdout/stderr contract for either. Sub-spec consumers
**MUST** treat `daemon`'s subcommand surface as *missing-source*.

The internal worker form is fast-pathed before `enableConfigs()` /
`initSinks()` per cli.tsx:96-99 ("perf-sensitive. No `enableConfigs()`, no
analytics sinks at this layer — workers are lean. If a worker kind needs
configs/auth (assistant will), it calls them inside its run() fn."). The
public form invokes `enableConfigs()` then `initSinks()` then `daemonMain()`
(cli.tsx:166-178).

### 3.2 Slash-command surface

When **both** `feature('DAEMON')` and `feature('BRIDGE_MODE')` are true, the
default export of `./commands/remoteControlServer/index.js` is concatenated
into the final command list (commands.ts:76-79, 327). Command name, prompt,
args, hidden-help — *missing-source*.

### 3.3 Process / IPC contract (delta vs non-DAEMON build)

| Contract | Value | Source |
|---|---|---|
| `SessionKind` literal added | `'daemon'` | concurrentSessions.ts:18 |
| `SessionKind` literal added | `'daemon-worker'` | concurrentSessions.ts:18 |
| Env var consumed by spawned children | `CLAUDE_CODE_SESSION_KIND` ∈ {`'bg'`,`'daemon'`,`'daemon-worker'`} | concurrentSessions.ts:33-34 |
| Gate on env-var read | requires `feature('BG_SESSIONS')` at runtime | concurrentSessions.ts:32 |

Children spawned by the supervisor MUST set `CLAUDE_CODE_SESSION_KIND` so the
child registers itself with the correct `kind` in its PID file
(concurrentSessions.ts:25-37 comment is verbatim authoritative).

### 3.4 Daemon protocol message shapes — *missing-source*

The supervisor↔worker IPC framing, request/response envelope, and authentication
handshake are not present in the leak. See §12.

### 3.5 Hosted services

Whether the daemon hosts the api client (spec 22), MCP service (spec 23), or
LSP service (spec 24) — *missing-source*. The cli.tsx fast-path body invokes
only `enableConfigs()`, `initSinks()`, and `daemonMain(args.slice(1))`
(cli.tsx:166-178); any additional hosting happens inside `daemonMain` which is
not in the leak.

---

## 4. Data Model & State

### 4.1 PID file (delta)

The shared `registerSession()` writes one JSON record per process to
`<getClaudeConfigHomeDir()>/sessions/<pid>.json` with mode `0o700` on the
directory (concurrentSessions.ts:64, 75-76). The `kind` field carries the
`SessionKind` value. For DAEMON builds this can be `'daemon'` or
`'daemon-worker'` (concurrentSessions.ts:62, 84).

The write is wrapped in a `try { mkdir(sessionsDir, {recursive: true, mode:
0o700}); chmod(sessionsDir, 0o700); writeFile(...) }` block; failures are
swallowed and logged via `logForDebugging('[concurrentSessions] register
failed: ...')` and never re-thrown (concurrentSessions.ts:75-76, 105-108; see
§9). A daemon or daemon-worker that fails to write its PID record continues
running unregistered, which is a known visibility gap for `claude ps`.

The full record (verbatim, concurrentSessions.ts:78-96):

```ts
jsonStringify({
  pid: process.pid,
  sessionId: getSessionId(),
  cwd: getOriginalCwd(),
  startedAt: Date.now(),
  kind,
  entrypoint: process.env.CLAUDE_CODE_ENTRYPOINT,
  ...(feature('UDS_INBOX')
    ? { messagingSocketPath: process.env.CLAUDE_CODE_MESSAGING_SOCKET }
    : {}),
  ...(feature('BG_SESSIONS')
    ? {
        name: process.env.CLAUDE_CODE_SESSION_NAME,
        logPath: process.env.CLAUDE_CODE_SESSION_LOG,
        agent: process.env.CLAUDE_CODE_AGENT,
      }
    : {}),
})
```

### 4.2 Persistent daemon state (socket path, port, lockfile)

*Missing-source.* The leak gives no daemon-private socket/lockfile/port
constants. `messagingSocketPath` belongs to `UDS_INBOX` (spec 35/41), not
`DAEMON`.

---

## 5. Algorithm / Control Flow

Two pseudocode paths are recoverable; everything inside `daemonMain` /
`runDaemonWorker` is *missing-source*.

### 5.1 `claude daemon ...` dispatch (cli.tsx:165-180)

```
if feature('DAEMON') and args[0] == 'daemon':
    profileCheckpoint('cli_daemon_path')
    enableConfigs()                          # config.js
    initSinks()                              # sinks.js
    await daemonMain(args.slice(1))          # ../daemon/main.js  (MISSING)
    return
```

### 5.2 `--daemon-worker <kind>` dispatch (cli.tsx:95-106)

```
# Fast-path: must precede the daemon subcommand check.
# Spawned per-worker → perf-sensitive.
# No enableConfigs(), no analytics sinks at this layer; lean by design.
# If a worker kind needs configs/auth (e.g. assistant), it calls them
# inside its own run() fn.
if feature('DAEMON') and args[0] == '--daemon-worker':
    await runDaemonWorker(args[1])           # ../daemon/workerRegistry.js (MISSING)
    return
```

### 5.3 `remoteControlServerCommand` registration (commands.ts:76-79, 327)

```
const remoteControlServerCommand =
    (feature('DAEMON') && feature('BRIDGE_MODE'))
        ? require('./commands/remoteControlServer/index.js').default
        : null
...
[
  ...,
  ...(remoteControlServerCommand ? [remoteControlServerCommand] : []),
  ...,
]
```

The `require()` is at module top-level so the bundler can DCE the entire
module when either flag is false.

### 5.4 Worker registration into `concurrentSessions`

A daemon child invokes `registerSession()` (the existing shared path); kind
selection follows envSessionKind() ?? 'interactive' (concurrentSessions.ts:62)
which means the supervisor MUST set `CLAUDE_CODE_SESSION_KIND=daemon` (or
`daemon-worker`) before spawning, and the runtime gate `feature('BG_SESSIONS')`
must also be live for the env var to be honored (concurrentSessions.ts:31-37).

### 5.5 Lifecycle (start / stop / signal handling)

*Missing-source.* No daemonization step (`fork`, `setsid`,
`detachIO`), no `SIGTERM`/`SIGINT` handler, no graceful drain, no PID-file lock,
no idle timeout, no max-connection cap, and no log-rotation policy is
recoverable from the leaked tree. See §12.

---

## 6. Verbatim Assets

This is a delta spec; verbatim content is limited to what the leak preserves.

### 6.1 Verbatim callsites

`src/entrypoints/cli.tsx:95-106`:

```ts
  // Fast-path for `--daemon-worker=<kind>` (internal — supervisor spawns this).
  // Must come before the daemon subcommand check: spawned per-worker, so
  // perf-sensitive. No enableConfigs(), no analytics sinks at this layer —
  // workers are lean. If a worker kind needs configs/auth (assistant will),
  // it calls them inside its run() fn.
  if (feature('DAEMON') && args[0] === '--daemon-worker') {
    const {
      runDaemonWorker
    } = await import('../daemon/workerRegistry.js');
    await runDaemonWorker(args[1]);
    return;
  }
```

`src/entrypoints/cli.tsx:164-180`:

```ts
  // Fast-path for `claude daemon [subcommand]`: long-running supervisor.
  if (feature('DAEMON') && args[0] === 'daemon') {
    profileCheckpoint('cli_daemon_path');
    const {
      enableConfigs
    } = await import('../utils/config.js');
    enableConfigs();
    const {
      initSinks
    } = await import('../utils/sinks.js');
    initSinks();
    const {
      daemonMain
    } = await import('../daemon/main.js');
    await daemonMain(args.slice(1));
    return;
  }
```

`src/commands.ts:76-79`:

```ts
const remoteControlServerCommand =
  feature('DAEMON') && feature('BRIDGE_MODE')
    ? require('./commands/remoteControlServer/index.js').default
    : null
```

`src/commands.ts:327`:

```ts
  ...(remoteControlServerCommand ? [remoteControlServerCommand] : []),
```

`src/utils/concurrentSessions.ts:18`:

```ts
export type SessionKind = 'interactive' | 'bg' | 'daemon' | 'daemon-worker'
```

`src/utils/concurrentSessions.ts:31-37`:

```ts
function envSessionKind(): SessionKind | undefined {
  if (feature('BG_SESSIONS')) {
    const k = process.env.CLAUDE_CODE_SESSION_KIND
    if (k === 'bg' || k === 'daemon' || k === 'daemon-worker') return k
  }
  return undefined
}
```

### 6.2 Daemon protocol message shapes

*Missing-source* — the leak contains no daemon-protocol type, schema, or
serialization helper. See §12 Q1.

### 6.3 PID file path (delta-relevant constants)

| Constant | Value | Source |
|---|---|---|
| Sessions root dir | `join(getClaudeConfigHomeDir(), 'sessions')` | concurrentSessions.ts:21-23 |
| PID file path | `<sessionsDir>/<process.pid>.json` | concurrentSessions.ts:64 |
| Sessions dir mode | `0o700` (mkdir + chmod) | concurrentSessions.ts:75-76 |
| PID-file filename regex (sweep) | `/^\d+\.json$/` | concurrentSessions.ts:186 |

Daemon-specific socket path / lockfile path: *missing-source*.

### 6.4 Constants table (defaults)

| Constant | Value | Source |
|---|---|---|
| Default daemon port | *missing-source* | — |
| Idle timeout | *missing-source* | — |
| Max connections | *missing-source* | — |
| Worker spawn perf-discipline note (verbatim comment) | "perf-sensitive. No enableConfigs(), no analytics sinks at this layer — workers are lean. If a worker kind needs configs/auth (assistant will), it calls them inside its run() fn." | cli.tsx:97-99 |

### 6.5 Env vars

| Var | Domain | Source |
|---|---|---|
| `CLAUDE_CODE_SESSION_KIND` | `'bg' \| 'daemon' \| 'daemon-worker'` (read only when `feature('BG_SESSIONS')`) | concurrentSessions.ts:33-34 |
| `CLAUDE_CODE_ENTRYPOINT` | propagated into PID file | concurrentSessions.ts:85 |

### 6.6 User-facing strings

The DAEMON flag itself adds **no** user-facing string in the surviving source.
The strings under `remoteControlServer/index.js` (command help/prompt) are
*missing-source*. The "Agent SDK daemon" / `--assistant` strings in
`src/main.tsx` belong to spec 32 (Kairos), not here.

### 6.7 Profile checkpoint label

`'cli_daemon_path'` (cli.tsx:166).

### 6.8 Commander option (adjacent, NOT a DAEMON delta)

`src/main.tsx:1000` declares `--workload <tag>` with the help string referring
to "SDK daemon callers". This is a billing-attribution flag for `--print` mode
and is **not** gated by `feature('DAEMON')`; documented here only to disclaim
mis-attribution.

---

## 7. Side Effects & I/O

Verifiable deltas:

- Spawning a daemon supervisor (via `claude daemon`) calls
  `enableConfigs()` and `initSinks()` before `daemonMain()` (cli.tsx:166-178).
  All filesystem and network I/O performed by the supervisor is *missing-source*.
- Spawning a daemon worker (`--daemon-worker`) deliberately **skips**
  `enableConfigs()` and `initSinks()` (cli.tsx:97-99). Workers must call them
  inline if needed.
- Both supervisor and worker, on registering with `concurrentSessions`,
  create a 0o700-mode `sessions/` directory and write a JSON PID file
  (concurrentSessions.ts:64, 75-76, 77-97). Cleanup removes the file on exit
  (concurrentSessions.ts:66-72).
- IPC sockets, lockfiles, log files, and any port binding: *missing-source*.

---

## 8. Feature Flags & Variants

| Flag | Effect |
|---|---|
| `feature('DAEMON')` | Compiles in: `claude daemon` fast-path (cli.tsx:165), `--daemon-worker` fast-path (cli.tsx:100), and the `remoteControlServer` slash-command (when co-gated, commands.ts:77). Each callsite is a build-time literal so the bundler DCEs both fast-paths and the lazy-required command module when DAEMON is off. |
| `feature('DAEMON') && feature('BRIDGE_MODE')` | Additional co-gate for `remoteControlServerCommand` (commands.ts:76-79). When BRIDGE_MODE is off, the command is omitted regardless of DAEMON. |
| `feature('BG_SESSIONS')` (interaction) | Required at runtime for `CLAUDE_CODE_SESSION_KIND` env var to be honored by `envSessionKind()` (concurrentSessions.ts:32-37). Without BG_SESSIONS, daemon-spawned children fall back to `kind: 'interactive'` (concurrentSessions.ts:62). |
| `feature('UDS_INBOX')` (interaction) | If on, PID file additionally records `messagingSocketPath` (concurrentSessions.ts:86-88). Daemon does not own this; see spec 35/41. |

`USER_TYPE === 'ant'` interaction: none of the four DAEMON callsites condition
on USER_TYPE in the leaked source.

---

## 9. Error Handling & Edge Cases

Recoverable:

- `registerSession()` errors are caught and logged via
  `logForDebugging('[concurrentSessions] register failed: ...')`; never
  re-thrown (concurrentSessions.ts:105-108). A daemon-worker failing to
  register continues running.
- PID-file unlink swallows ENOENT (concurrentSessions.ts:66-72).
- WSL stale-PID sweep is skipped (concurrentSessions.ts:194-201, comment).

Daemon-internal error handling (signal handling, supervised-worker restart,
crash loops): *missing-source*.

---

## 10. Telemetry & Observability

- `profileCheckpoint('cli_daemon_path')` is emitted on the supervisor entry
  (cli.tsx:166). No checkpoint is emitted on the worker fast-path.
- Worker fast-path explicitly skips `initSinks()`, meaning analytics sinks are
  not initialized at the entrypoint (cli.tsx:97-99 comment). Per-worker
  analytics depend on whether `runDaemonWorker(kind)`'s implementation calls
  `initSinks()` itself — *missing-source*.
- PID file presence is the de-facto observability surface for `claude ps`
  (concurrentSessions.ts:168-204; full enumeration semantics owned by
  spec 19/41).

---

## 11. Reimplementation Checklist

To bit-exactly reimplement the DAEMON-mode delta you must:

1. Add the exact callsites in §6.1 — preserve literal flag expressions
   (`feature('DAEMON')`, `feature('DAEMON') && feature('BRIDGE_MODE')`) and
   inline import order; the bundler depends on them being module-top-level.
2. Preserve the ordering: `--daemon-worker` fast-path **before** the `daemon`
   subcommand fast-path (cli.tsx:96 comment is authoritative).
3. Worker fast-path: do **not** call `enableConfigs()` or `initSinks()` at the
   entrypoint layer (cli.tsx:97-99). Defer to `runDaemonWorker(kind)`.
4. Supervisor fast-path: call `profileCheckpoint('cli_daemon_path')`,
   `enableConfigs()`, `initSinks()`, then `daemonMain(args.slice(1))`
   (cli.tsx:166-178).
5. Extend `SessionKind` to include `'daemon'` and `'daemon-worker'`
   (concurrentSessions.ts:18). Honor them in `envSessionKind()` only when
   `feature('BG_SESSIONS')` (concurrentSessions.ts:31-37).
6. When spawning supervisor children, set
   `process.env.CLAUDE_CODE_SESSION_KIND` per child role.
7. Co-gate `remoteControlServerCommand` on `feature('BRIDGE_MODE')`
   (commands.ts:77).
8. Implement `daemonMain`, `runDaemonWorker`, and the
   `commands/remoteControlServer/` module — all of which are absent from the
   leak. See §12.
9. **Auth refresh wiring (cross-spec, see spec 34).** When the supervisor (or
   any daemon-hosted code path) opens a `bridgeApi` client, it MUST pass
   `onAuth401: handleOAuth401Error` (see `initReplBridge.ts:430,531`;
   `bridgeMain.ts:2351,2901`) — otherwise the no-handler branch at
   `bridgeApi.ts:117-120` returns the raw 401 to the caller, which
   `handleErrorStatus` converts into a `BridgeFatalError` even when refresh
   credentials exist on disk. Daemon implementers must either wire
   `handleOAuth401Error` or explicitly accept fatal-on-stale-token semantics.
   The previous Phase 9.6 wording "immediate `BridgeFatalError`" oversimplifies
   the actual flow: the 401 propagates to the caller and typically becomes
   fatal via `handleErrorStatus` downstream (bridgeApi.ts:104 comment is
   authoritative).

---

## 12. Open Questions / Unknowns

The DAEMON mode is the most sparsely-preserved subsystem in this leak. The
following items are *missing-source* and MUST be resolved by the verification
agent (Phase 9) or recovered from a different artifact before any consumer
spec can claim full coverage.

1. **Q1 — Supervisor body.** `src/daemon/main.ts` (referenced cli.tsx:177) is
   absent. Unknown: subcommand list (`start`/`stop`/`status`/`restart`?),
   foreground-vs-detached behavior, daemonization steps, signal handlers,
   PID-file lock, idle timeout, max-connection cap, graceful shutdown.
2. **Q2 — Worker registry.** `src/daemon/workerRegistry.ts` (referenced
   cli.tsx:103) is absent. Unknown: enumerated worker `<kind>` values,
   per-kind `run()` contract, restart policy.
3. **Q3 — IPC protocol.** No daemon-protocol message type, schema, framing
   (length-prefix? newline-delimited JSON? MCP-over-UDS?), or transport
   (UDS path? localhost TCP port? named pipe on Windows?) is preserved.
4. **Q4 — Authentication.** Whether daemon connections require a token, share
   the OAuth/JWT path of spec 25, or rely solely on UDS-mode-700 ambient
   authority is unverifiable from the leak.
5. **Q5 — Hosted services.** Whether the daemon hosts the api client
   (spec 22), MCP service (spec 23), or LSP service (spec 24) — and whether
   it acts as a multiplexer for them — is unverifiable. The supervisor body
   is absent.
6. **Q6 — `remoteControlServerCommand` body.** `src/commands/remoteControlServer/`
   is absent. Unknown: command name, prompt, arguments, hidden status, output.
   The directory name strongly suggests it is the in-REPL "host this machine
   as a remote-control server" command counterpart to the bridge client at
   `src/bridge/` (spec 34), but this is conjecture, not source.
7. **Q7 — Logging differences.** Whether daemon mode rotates logs, writes to
   `~/.claude/logs/daemon.log`, or reuses the `sinks.ts` machinery is
   unverifiable. Worker fast-path explicitly skips `initSinks()` at entry, but
   the inner call site in `runDaemonWorker` (if any) is absent.
8. **Q8 — Default port / socket path constants.** Not present in the leak.
   Adjacent `messagingSocketPath` (UDS_INBOX, spec 35) is a different surface.
9. **Q9 — `claude assistant` overlap.** The "Agent SDK daemon" comments in
   `src/main.tsx` (lines 1053-1074, 2643, 3288-3290, 3842) belong to the
   `--assistant` Kairos path (spec 32), not `feature('DAEMON')`. Confirmed by
   inspection of those lines (none reference `feature('DAEMON')`). Spec 32
   should explicitly disclaim the same overlap.
10. **Q10 — `BRIDGE_MODE` co-gate semantics.** Why `remoteControlServer` is
    only registered when both DAEMON and BRIDGE_MODE are live is implied by
    the directory name but not stated. Spec 34 should cross-reference the
    same callsite.
