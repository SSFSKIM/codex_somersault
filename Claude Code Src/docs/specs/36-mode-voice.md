# 36 — Mode: Voice (`VOICE_MODE`)

Hold-to-talk dictation. Records mic audio, streams 16 kHz / 16-bit / mono PCM
to Anthropic's `voice_stream` STT WebSocket endpoint, splices interim and
final transcripts into the prompt input. Anthropic-OAuth-only; not API
keys, Bedrock, Vertex, Foundry. Gated by `feature('VOICE_MODE')` plus the
`tengu_amber_quartz_disabled` GrowthBook kill-switch.

## §1 Scope and entry points

| File | Role |
|---|---|
| `src/voice/voiceModeEnabled.ts` | Flag/auth/kill-switch resolver |
| `src/services/voice.ts` | Audio capture (cpal NAPI / SoX / arecord) |
| `src/services/voiceStreamSTT.ts` | WebSocket STT client |
| `src/services/voiceKeyterms.ts` | Domain-vocabulary hint builder |
| `src/commands/voice/index.ts`, `…/voice.ts` | `/voice` toggle command |
| `src/hooks/useVoice.ts` | Hold-to-talk session FSM (REPL-side) |
| `src/hooks/useVoiceIntegration.tsx` | Prompt-input splicing + key handler |
| `src/hooks/useVoiceEnabled.ts` | Memoized renderer-side enabled gate |
| `src/context/voice.tsx` | `VoiceState` store (Provider + selectors) |
| `src/components/PromptInput/VoiceIndicator.tsx` | "listening…" / shimmer UI |
| `src/components/PromptInput/Notifications.tsx` | Indicator render site |
| `src/components/PromptInput/PromptInputFooterLeftSide.tsx` | Hint render |
| `src/components/LogoV2/VoiceModeNotice.tsx` | First-N-runs notice |
| `src/state/AppState.tsx` | Conditional `VoiceProvider` mount |
| `src/screens/REPL.tsx` | `useVoiceIntegration` call site (4022-4034); `VoiceKeybindingHandler` import (`screens/REPL.tsx:103`, gated independently of `useVoiceIntegration` at line 98) |
| `src/keybindings/{schema,defaultBindings,validate}.ts` | `voice:pushToTalk` |
| `src/utils/settings/types.ts` | `voiceEnabled` setting (Zod) |
| `src/tools/ConfigTool/{supportedSettings,prompt}.ts` | Config exposure |
| `src/commands.ts` | Conditional `/voice` registration |

Compile-time gate: `feature('VOICE_MODE')` — a Bun `bun:bundle` flag
that drives dead-code elimination at build time (call sites:
`commands.ts:80-81`, `commands.ts:328`, `state/AppState.tsx:14-16`,
`screens/REPL.tsx:98`, `screens/REPL.tsx:103`, `screens/REPL.tsx:4022`,
`Notifications.tsx:37`, `PromptInputFooterLeftSide.tsx:266-285`,
`defaultBindings.ts:96`, `settings/types.ts:864-871`,
`supportedSettings.ts:144-152`). This is **NOT** the same as the
`USER_TYPE === 'ant'` Anthropic-employee gate used elsewhere in the
codebase (cf. CLAUDE.md "Two Dominant Patterns"); a repo-wide grep of
`src/voice/`, `src/hooks/useVoice*`, `src/services/voice*`,
`src/context/voice.tsx`, and `src/components/.../Voice*` shows **zero**
`USER_TYPE` / `'ant'` branches. Voice mode ships in non-`ant` builds.

Runtime gates (all three must hold for the feature to be active):
1. GrowthBook kill-switch `tengu_amber_quartz_disabled` (default
   `false`, `voiceModeEnabled.ts:20-23`).
2. **Anthropic OAuth provider presence** via `isAnthropicAuthEnabled()`
   (`voiceModeEnabled.ts:32-44`). This checks the auth *provider*
   (Anthropic-OAuth-only; not API keys, Bedrock, Vertex, Foundry); it
   does **not** restrict to Anthropic employees and is independent of
   `USER_TYPE`. Any user authenticated via the Anthropic OAuth flow
   passes this gate.
3. User-set `voiceEnabled: true` in settings
   (`useVoiceEnabled.ts:20-24`).

## §2 Voice context wiring location (was unresolved per 00 §12.8)

**`context/voice/` does NOT exist; the actual file is `src/context/voice.tsx`.**
`VoiceProvider` is mounted by `src/state/AppState.tsx:14-16, 94` inside
`MailboxProvider` (`<MailboxProvider><VoiceProvider>{children}…`),
gated by `feature('VOICE_MODE')` — when the flag is off the import is
replaced with a pass-through (`feature('VOICE_MODE') ?
require('../context/voice.js').VoiceProvider : ({children}) => children`).
The store is constructed once via `createStore<VoiceState>(DEFAULT_STATE)`
in `useState`, never re-created (`context/voice.tsx:23-39`). Three
selectors are exported: `useVoiceState(selector)` (subscribes via
`useSyncExternalStore`), `useSetVoiceState()` (stable setter),
`useGetVoiceState()` (sync reader for callbacks).

The store has **no direct wiring into `QueryEngine`/`query.ts` (turn
pipeline)**. Voice state never reaches the LLM loop directly. The
integration is one-way: transcripts are spliced into the prompt-input
buffer (`useVoiceIntegration.tsx:281-310`) which is later submitted by
the normal Enter path. See §3.

`VoiceState` shape (`context/voice.tsx:4-17`):
- `voiceState: 'idle' | 'recording' | 'processing'`
- `voiceError: string | null`
- `voiceInterimTranscript: string`
- `voiceAudioLevels: number[]`
- `voiceWarmingUp: boolean`

DEFAULT_STATE: `{ voiceState: 'idle', voiceError: null,
voiceInterimTranscript: '', voiceAudioLevels: [], voiceWarmingUp: false }`.

## §3 End-to-end activation flow

1. **Bootstrap.** `feature('VOICE_MODE')` strips/keeps voice imports at
   bundle time (`commands.ts:80-81`, `state/AppState.tsx:14-16`).
2. **Visibility.** `/voice` registered iff `isVoiceGrowthBookEnabled()`;
   hidden iff `!isVoiceModeEnabled()` (`commands/voice/index.ts:11-13`).
3. **`/voice` toggle.** Pre-flight on enable
   (`commands/voice/voice.ts:34-145`):
   `isVoiceModeEnabled()` → `checkRecordingAvailability()` →
   `isVoiceStreamAvailable()` → `checkVoiceDependencies()` →
   `requestMicrophonePermission()`. On success writes
   `voiceEnabled: true` to user settings, fires
   `tengu_voice_toggled` analytics, returns
   `Voice mode enabled. Hold ${key} to record.${langNote}`.
4. **Provider mount.** `VoiceProvider` wraps the React tree
   (`state/AppState.tsx:94`).
5. **Hook integration.** `screens/REPL.tsx:4022-4034` calls
   `useVoiceIntegration({setInputValueRaw, inputValueRef, insertTextRef})`.
6. **Key activation.** `voice:pushToTalk` bound to `space` by default
   (`defaultBindings.ts:96`). `useVoiceKeybindingHandler`
   (`useVoiceIntegration.tsx:373-…`) detects bare-char vs modifier-combo:
   modifier combos activate on first press; bare chars require
   HOLD_THRESHOLD=5 rapid presses with WARMUP_THRESHOLD=2 flow-through.
7. **Session start.** `useVoice.startRecordingSession()`
   (`useVoice.ts:633-1011`): sync `updateState('recording')` BEFORE any
   await; lazy-load `voiceModule`; `checkRecordingAvailability()`; start
   audio capture with `silenceDetection:false`; gather keyterms;
   `connectVoiceStream(...)` with `{language, keyterms}`. Audio chunks
   buffer (`audioBuffer: Buffer[]`) until `onReady` flushes them as
   coalesced 32 KB (~1 s) frames.
8. **Transcripts.** Server emits `TranscriptText` (interim or
   refinement) and `TranscriptEndpoint` (utterance final).
   `useVoice.ts:357-461` accumulates finals separated by spaces; mirrors
   to `voiceInterimTranscript` for live preview.
9. **Splicing.** `useVoiceIntegration.tsx:253-280, 281-310` writes
   `prefix + (leadingSpace) + interim + (trailingSpace) + suffix` into
   the prompt input on each `voiceInterimTranscript` change; final
   transcripts overwrite via `handleVoiceTranscript`.
10. **Release.** Auto-repeat gap > `RELEASE_TIMEOUT_MS=200` (or
    `REPEAT_FALLBACK_MS=600` / `FIRST_PRESS_FALLBACK_MS=2000`) →
    `finishRecording()` → `connection.finalize()` → either
    `post_closestream_endpoint`, `no_data_timeout` (1.5 s), `ws_close`,
    `ws_already_closed`, or `safety_timeout` (5 s). On
    `no_data_timeout` with audio signal + wsConnected + non-focus +
    empty transcript, replay the buffered audio once on a fresh
    connection (250 ms backoff).
11. **Submission.** Final text is spliced into the input; the user
    presses Enter via the standard prompt-input path. There is no
    direct turn-pipeline call.

## §4 Subsystem internals (audio capture)

Backends, in order (`services/voice.ts:330-396`):
1. `audio-capture-napi` (cpal-backed) on macOS / Linux / Windows
   when `napi.isNativeAudioAvailable()` AND on Linux when
   `linuxHasAlsaCards()` (reads `/proc/asound/cards`).
2. Linux `arecord -f S16_LE -r 16000 -c 1 -t raw -q -` if
   `hasCommand('arecord') && (await probeArecord()).ok`. Probe spawns
   the same args, races a 150 ms timer (alive=ok), memoized
   (`services/voice.ts:75-118`).
3. SoX `rec` with arguments per §6 below.

`requestMicrophonePermission()` (`voice.ts:241-257`) fires
`startRecording` once with `silenceDetection:false`; on macOS this
triggers the TCC permission dialog. The probe trusts the spawn
result over TCC status APIs (unreliable for ad-hoc / cross-arch
binaries).

Native module load is lazy (`voiceModule` import on first
`useEffect` after `enabled`, `useVoice.ts:530-536`); cpal `dlopen`
blocks the event loop ~1 s warm / up to ~8 s cold (`services/voice.ts:14-36`).

Remote environments are blocked: `isRunningOnHomespace() ||
isEnvTruthy(process.env.CLAUDE_CODE_REMOTE)` returns
`available:false` (`voice.ts:259-268`).

`stopRecording()` calls `napi.stopNativeRecording()` if native is
active else `activeRecorder.kill('SIGTERM')` (`voice.ts:515-525`).

## §5 STT streaming protocol

Endpoint `wss://{api}/api/ws/speech_to_text/voice_stream` (default
host = `getOauthConfig().BASE_API_URL` swapped to ws/wss; override
via `VOICE_STREAM_BASE_URL` env). Headers: `Authorization: Bearer
${tokens.accessToken}`, `User-Agent: getUserAgent()`, `x-app: cli`.
TLS via `getWebSocketTLSOptions()`; proxy via Bun-vs-Node branch
(`voiceStreamSTT.ts:185-195`).

Query params (`voiceStreamSTT.ts:144-173`): `encoding=linear16`,
`sample_rate=16000`, `channels=1`, `endpointing_ms=300`,
`utterance_end_ms=1000`, `language=${options.language ?? 'en'}`. If
GrowthBook `tengu_cobalt_frost` is true, additionally
`use_conversation_engine=true` and `stt_provider=deepgram-nova3`
(Nova 3). For each keyterm in `options.keyterms`:
`params.append('keyterms', term)`.

Wire frames:
- Client → server: control text frames `KEEPALIVE_MSG`,
  `CLOSE_STREAM_MSG`; binary audio frames are
  `Buffer.from(audioChunk)` (defensive copy because NAPI buffers
  share pooled ArrayBuffers).
- Server → client: JSON `TranscriptText {data}`, `TranscriptEndpoint`,
  `TranscriptError {error_code?, description?}`, `error {message?}`.

KeepAlive every `KEEPALIVE_INTERVAL_MS = 8_000` ms; one
immediate KeepAlive sent on `open` (`voiceStreamSTT.ts:322-348`).

Finalize sources (`voiceStreamSTT.ts:60-66`):
`post_closestream_endpoint | no_data_timeout | safety_timeout |
ws_close | ws_already_closed`. Timers
`FINALIZE_TIMEOUTS_MS = { safety: 5_000, noData: 1_500 }`
(`voiceStreamSTT.ts:44-47`).

CloseStream is sent in a `setTimeout(0)` to flush queued
`onData` callbacks before the server is told to stop accepting
audio (`voiceStreamSTT.ts:297-304`). After CloseStream, further
`send()` chunks are dropped.

Auto-finalize for non-Nova-3 segments: when a new
`TranscriptText` is neither a prefix nor a suffix of the previous
one, the prior `lastTranscriptText` is emitted as final
(`voiceStreamSTT.ts:396-410`). Nova 3 disables this — its
interims are cumulative across segments and can revise earlier
text.

WebSocket close codes 1000/1005 are silent; others surface as
`Connection closed: code ${code} — ${reason}`. HTTP upgrade
non-101 → `unexpected-response` listener flags fatal=true for
4xx, fatal=false for 5xx; 101 spuriously firing is ignored
(Bun-on-Windows quirk, `voiceStreamSTT.ts:511-533`).

## §6 Inline contracts (verbatim where possible)

### 6.1 Audio capture defaults

```
RECORDING_SAMPLE_RATE = 16000
RECORDING_CHANNELS    = 1
encoding              = linear16 (S16_LE; 16-bit signed little-endian)
SILENCE_DURATION_SECS = '2.0'
SILENCE_THRESHOLD     = '3%'
```

(`services/voice.ts:40-46`)

### 6.2 SoX `rec` argv (push-to-talk path uses NO `silence` filter)

```
rec -q --buffer 1024 -t raw -r 16000 -e signed -b 16 -c 1 -
```

If `silenceDetection: true`, append:
`silence 1 0.1 3% 1 2.0 3%` (`services/voice.ts:410-439`).

### 6.3 `arecord` argv

```
arecord -f S16_LE -r 16000 -c 1 -t raw -q -
```

(`services/voice.ts:475-486`; probe variant writes to `/dev/null`,
`services/voice.ts:77-91`.)

### 6.4 voice_stream WebSocket request shape

```
URL  = ${VOICE_STREAM_BASE_URL || BASE_API_URL→ws(s)://}
       /api/ws/speech_to_text/voice_stream
       ?encoding=linear16
       &sample_rate=16000
       &channels=1
       &endpointing_ms=300
       &utterance_end_ms=1000
       &language=<bcp47>
       [&use_conversation_engine=true&stt_provider=deepgram-nova3]
       [&keyterms=<t1>&keyterms=<t2>…]
Headers:
  Authorization: Bearer <oauth.accessToken>
  User-Agent:    <getUserAgent()>
  x-app:         cli
Control msgs (text frames):
  '{"type":"KeepAlive"}'
  '{"type":"CloseStream"}'
Audio frames (binary): Buffer.from(rawPcm16le)
```

(`voiceStreamSTT.ts:29-30, 36-47, 132-195`.)

### 6.5 Server message types (verbatim TS)

```ts
type VoiceStreamTranscriptText     = { type: 'TranscriptText'; data: string }
type VoiceStreamTranscriptEndpoint = { type: 'TranscriptEndpoint' }
type VoiceStreamTranscriptError    = {
  type: 'TranscriptError'; error_code?: string; description?: string
}
type VoiceStreamMessage =
  | VoiceStreamTranscriptText
  | VoiceStreamTranscriptEndpoint
  | VoiceStreamTranscriptError
  | { type: 'error'; message?: string }
```

(`voiceStreamSTT.ts:75-94`.)

### 6.6 Static keyterm dictionary (verbatim)

```
GLOBAL_KEYTERMS = [
  'MCP', 'symlink', 'grep', 'regex', 'localhost', 'codebase',
  'TypeScript', 'JSON', 'OAuth', 'webhook', 'gRPC', 'dotfiles',
  'subagent', 'worktree',
]
MAX_KEYTERMS = 50
```

(`services/voiceKeyterms.ts:13-31, 55`. "Claude" and "Anthropic" are
already server-side base keyterms.) Dynamic additions (in order, dedup,
cap 50): `basename(getProjectRoot())` if 3-50 chars; words from
`splitIdentifier(getBranch())`; words from `splitIdentifier(basename)`
of each path in `recentFiles`. `splitIdentifier` splits camelCase,
PascalCase, kebab-case, snake_case, path separators; keeps fragments
of length 3-20 (`voiceKeyterms.ts:40-46`).

### 6.7 Push-to-talk timing constants

```
RELEASE_TIMEOUT_MS         = 200    // gap that signals key release
REPEAT_FALLBACK_MS         = 600    // arm release timer after first press
FIRST_PRESS_FALLBACK_MS    = 2000   // modifier-combo first-press cap
FOCUS_SILENCE_TIMEOUT_MS   = 5_000  // focus-mode idle teardown
AUDIO_LEVEL_BARS           = 16
KEEPALIVE_INTERVAL_MS      = 8_000
FINALIZE_TIMEOUTS_MS.safety = 5_000
FINALIZE_TIMEOUTS_MS.noData = 1_500
RAPID_KEY_GAP_MS           = 120    // useVoiceIntegration
HOLD_THRESHOLD             = 5      // bare-char activation
WARMUP_THRESHOLD           = 2      // bare-char flow-through
MODIFIER_FIRST_PRESS_FALLBACK_MS = 2000
```

(`useVoice.ts:160-183`, `voiceStreamSTT.ts:38-47`,
`useVoiceIntegration.tsx:38-54`.)

### 6.8 Push-to-talk command prompt (verbatim, command-time)

`/voice` enable success message
(`commands/voice/voice.ts:146-149`):

```
Voice mode enabled. Hold ${key} to record.${langNote}
```

`langNote` is one of:
- `''` (no hint),
- ` Note: "${stt.fellBackFrom}" is not a supported dictation language;
  using English. Change it via /config.`,
- ` Dictation language: ${stt.code} (/config to change).`
  (shown at most `LANG_HINT_MAX_SHOWS = 2` times per language).

VoiceModeNotice (first-N-runs, `MAX_SHOW_COUNT = 3`, gated by
`isVoiceModeEnabled() && !voiceEnabled && voiceNoticeSeenCount < 3 &&
!shouldShowOpus1mMergeNotice()`,
`components/LogoV2/VoiceModeNotice.tsx:11, 65-67`):

```
 Voice mode is now available · /voice to enable
```

VoiceIndicator strings (`VoiceIndicator.tsx:24-91`):
- recording: `listening…` (`<Text dimColor>`)
- processing: `Voice: processing…` (shimmer, or `color="warning"`
  with reduced motion)
- warmup hint: `keep holding…`

### 6.9 Settings / supported-config keys

`voiceEnabled: boolean | undefined` — "Enable voice mode (hold-to-talk
dictation)" (`utils/settings/types.ts:864-871`,
`tools/ConfigTool/supportedSettings.ts:144-152`,
`tools/ConfigTool/prompt.ts:25`).

`voiceLangHintShownCount`, `voiceLangHintLastLanguage`,
`voiceNoticeSeenCount` are global-config counters
(`commands/voice/voice.ts:130-145`,
`components/LogoV2/VoiceModeNotice.tsx:33-43`).

### 6.10 Keybinding

`voice:pushToTalk` registered in
`KEYBINDING_ACTIONS` (`keybindings/schema.ts:171`); default
`{ space: 'voice:pushToTalk' }` in `Chat` context only when
`feature('VOICE_MODE')` (`keybindings/defaultBindings.ts:96`).
Activation key resolved by forward-iterating `keybindingContext.bindings`
and last-wins; null-unbinding disables hold-to-talk
(`useVoiceIntegration.tsx:405-421`).

### 6.11 Supported BCP-47 STT languages (subset of server allowlist)

```
{ en, es, fr, ja, de, pt, it, ko, hi, id, ru, pl, tr, nl, uk, el,
  cs, da, sv, no }
```

(`hooks/useVoice.ts:91-114`.) `LANGUAGE_NAME_TO_CODE` maps
`english/spanish/español/espanol/french/français/francais/japanese/
日本語/german/deutsch/portuguese/português/portugues/italian/italiano/
korean/한국어/hindi/हिन्दी/हिंदी/indonesian/'bahasa indonesia'/bahasa/
russian/русский/polish/polski/turkish/türkçe/turkce/dutch/nederlands/
ukrainian/українська/greek/ελληνικά/czech/čeština/cestina/danish/dansk/
swedish/svenska/norwegian/norsk` to BCP-47 codes
(`hooks/useVoice.ts:42-89`). Default
`DEFAULT_STT_LANGUAGE = 'en'`.

### 6.12 Constants table

| Name | Value | File:line |
|---|---|---|
| RECORDING_SAMPLE_RATE | 16000 | services/voice.ts:40 |
| RECORDING_CHANNELS | 1 | services/voice.ts:41 |
| SILENCE_DURATION_SECS | '2.0' | services/voice.ts:44 |
| SILENCE_THRESHOLD | '3%' | services/voice.ts:45 |
| VOICE_STREAM_PATH | '/api/ws/speech_to_text/voice_stream' | voiceStreamSTT.ts:36 |
| KEEPALIVE_INTERVAL_MS | 8_000 | voiceStreamSTT.ts:38 |
| FINALIZE_TIMEOUTS_MS.safety | 5_000 | voiceStreamSTT.ts:44-47 |
| FINALIZE_TIMEOUTS_MS.noData | 1_500 | voiceStreamSTT.ts:44-47 |
| KEEPALIVE_MSG | '{"type":"KeepAlive"}' | voiceStreamSTT.ts:29 |
| CLOSE_STREAM_MSG | '{"type":"CloseStream"}' | voiceStreamSTT.ts:30 |
| RELEASE_TIMEOUT_MS | 200 | useVoice.ts:160 |
| REPEAT_FALLBACK_MS | 600 | useVoice.ts:171 |
| FIRST_PRESS_FALLBACK_MS | 2000 | useVoice.ts:172 |
| FOCUS_SILENCE_TIMEOUT_MS | 5_000 | useVoice.ts:177 |
| AUDIO_LEVEL_BARS | 16 | useVoice.ts:180 |
| DEFAULT_STT_LANGUAGE | 'en' | useVoice.ts:32 |
| MAX_KEYTERMS | 50 | voiceKeyterms.ts:55 |
| RAPID_KEY_GAP_MS | 120 | useVoiceIntegration.tsx:39 |
| MODIFIER_FIRST_PRESS_FALLBACK_MS | 2000 | useVoiceIntegration.tsx:46 |
| HOLD_THRESHOLD | 5 | useVoiceIntegration.tsx:51 |
| WARMUP_THRESHOLD | 2 | useVoiceIntegration.tsx:54 |
| LANG_HINT_MAX_SHOWS | 2 | commands/voice/voice.ts:14 |
| MAX_SHOW_COUNT (VoiceModeNotice) | 3 | LogoV2/VoiceModeNotice.tsx:11 |
| GrowthBook kill-switch flag | tengu_amber_quartz_disabled | voiceModeEnabled.ts:21 |
| GrowthBook Nova 3 flag | tengu_cobalt_frost | voiceStreamSTT.ts:157-160 |

## §7 Algorithms (pseudocode)

### 7.1 Push-to-talk session

```
on key event matching voice:pushToTalk in 'Chat' context:
  if !enabled or !isVoiceStreamAvailable(): return
  if focusTriggered: return                  // focus mode owns recording
  if focusMode and silenceTimedOut:          // re-arm after silence
    silenceTimedOut = false; focusTriggered = true
    startRecordingSession(); armFocusSilenceTimer(); return
  if state == 'processing': return
  if state == 'idle':
    startRecordingSession()                  // synchronous updateState('recording')
    schedule repeatFallbackTimer (fallbackMs)
  else if state == 'recording':
    seenRepeat = true; clear repeatFallbackTimer
  reset releaseTimer
  if state == 'recording' and seenRepeat:
    arm releaseTimer (RELEASE_TIMEOUT_MS) → finishRecording
```

### 7.2 finishRecording

```
attemptGen++; capture {durMs, hadAudio, retried, focusFlushed,
  wsConnected, myGen}; updateState('processing'); voiceModule.stopRecording()
finalizePromise = connectionRef ? connectionRef.finalize() : undefined
on finalize → finalizeSource:
  if isStale(myGen): return
  if (finalizeSource == 'no_data_timeout' && hadAudio && wsConnected
      && !focusTriggered && focusFlushed == 0
      && accumulated.trim() == '' && !silentDropRetried
      && fullAudio.length > 0):
    silentDropRetried = true
    sleep 250ms; reconnect; replay fullAudio in 32 KB slices; await finalize()
  log tengu_voice_recording_completed { transcriptChars, durMs, hadAudio,
       retried, silentDropRetried, wsConnected, focusTriggered }
  if text: onTranscript(text)
  else if focusFlushed == 0 && durMs > 2000:
    !wsConnected → 'Voice connection failed. …'
    !hadAudio   → 'No audio detected from microphone. …'
    else        → 'No speech detected.'
  updateState('idle')
```

### 7.3 connectVoiceStream early-error retry

```
if (!opts.fatal && !sawTranscript && state == 'recording'
    && !retryUsed):
  retryUsed = true
  emit tengu_voice_stream_early_retry
  connectionRef = null; attemptGen++
  setTimeout(250ms, () => if state=='recording': attemptConnect(keyterms))
else if surfacing:
  attemptGen++; logError; onError(...); audioBuffer.length=0
  cleanup(); updateState('idle')
```

### 7.4 RMS audio level

```
samples = chunk.length >> 1
sumSq = Σ ((b[i] | (b[i+1] << 8)) << 16 >> 16) ** 2  // s16le
rms = sqrt(sumSq / samples)
level = sqrt(min(rms / 2000, 1))
keep last AUDIO_LEVEL_BARS levels in audioLevelsRef
hasAudioSignal = hasAudioSignal || level > 0.01
```

(`useVoice.ts:185-197`.)

## §8 Privacy / network model

Audio NEVER stays local. Every hold-to-talk session opens an outbound
TLS WebSocket to Anthropic (`api.anthropic.com` by default; `claude.ai`
TLS-fingerprints non-browser clients per
`voiceStreamSTT.ts:124-131`). Audio is streamed as raw PCM, no
on-device transcription, no on-disk buffering except the in-memory
`fullAudioRef` replay buffer (≤ ~2 MB, cleared after finalize). No
logging of audio bytes; only chunk byte counts (`logForDebugging`,
`voiceStreamSTT.ts:223, 228`). Replay only fires once per session for
the silent-drop case (§7.2).

Remote / homespace environments are explicitly blocked
(`services/voice.ts:259-268`).

## §9 Activation modes

- **Push-to-talk** (default): hold `voice:pushToTalk` key. Release
  detected by auto-repeat gap. `silenceDetection: false` for SoX/native
  (`useVoice.ts:730-731`).
- **Focus mode**: REPL invokes `useVoiceIntegration` with
  `focusMode: false` hardwired (`useVoiceIntegration.tsx:323`). The
  hook itself supports focus-driven sessions (`useVoice.ts:572-630`,
  `armFocusSilenceTimer`, `silenceTimedOutRef`) — start on terminal
  focus, end on blur or 5 s silence, with each final transcript
  flushed immediately. In this snapshot, **focus mode and the
  associated terminal-blur auto-pause / teardown path are dead code**:
  `useVoice.ts` reads `useTerminalFocus` (line 268) and gates the
  focus-mode branches behind `if (!enabled || !focusMode) { … return }`
  (line 577) and `[enabled, focusMode, isFocused]` (line 630), and
  `focusMode` is constant `false` at the only call site. No CLI
  surface, setting, env var, or flag flips it on. Until that hardwired
  literal changes, blur/idle teardown cannot fire from push-to-talk.

## §10 Cross-references

- `01-entrypoint-bootstrap` — `feature()` evaluation, `VoiceProvider`
  mount.
- `26-service-analytics-flags` — GrowthBook `tengu_amber_quartz_disabled`,
  `tengu_cobalt_frost`; events `tengu_voice_toggled`,
  `tengu_voice_recording_started/_completed`,
  `tengu_voice_stream_early_retry`, `tengu_voice_silent_drop_replay`.
- `25-service-oauth-auth` — `getClaudeAIOAuthTokens`,
  `checkAndRefreshOAuthTokenIfNeeded`,
  `getWebSocketTLSOptions`, `getWebSocketProxyAgent/Url`.
- `02-settings-schemas-migrations` — `voiceEnabled` Zod schema entry.
- `37-ink-ui-shell` — `<VoiceIndicator>`, `<VoiceWarmupHint>`,
  `<VoiceModeNotice>`, footer hint.
- `39-vim-keybindings` — **confirmed gap**, not just overlap risk: a
  repo-wide grep of `src/vim/` for `voice|space|hold` returns zero
  voice-aware code paths, and the voice handler in
  `useVoiceIntegration.tsx` does not consult vim mode/state. Inside
  vim insert mode, holding `space` (the default `voice:pushToTalk`
  binding) will reach `HOLD_THRESHOLD = 5` rapid presses and activate
  push-to-talk *while also* inserting space characters into the
  buffer via the normal vim insert path — there is no mediation
  between the two consumers. Cross-spec to 39 for the vim-side
  description; the resolution (if any) lives outside this snapshot.
- `41-session-state-history` — transcript splices into prompt input
  buffer; submission goes through normal Enter path.

## §11 Adjacent (not redocumented)

01 (boot), 26 (flags/analytics), 37 (UI shell), 39 (vim), 41 (session).

## §12 Open / unresolved

- §12.8 of 00-overview ("voice context wiring location unresolved") —
  **resolved** in §2 above: provider lives in `src/context/voice.tsx`,
  mounted in `src/state/AppState.tsx:14-16, 94`, never wired into the
  turn pipeline. State is read by UI selectors and by
  `useVoiceIntegration` only.
- `audio-capture-napi` is an external NAPI dependency (vendored at
  `vendor/audio-capture-src/index.ts` per `services/voice.ts:30-31`);
  the source of that module is not in this leak (only consumer code).
- The Bun-vs-Node WebSocket-options branch (`voiceStreamSTT.ts:185-195`)
  references a Bun-specific `proxy` field; runtime selection between
  these is governed by `typeof Bun !== 'undefined'`.
- Focus mode is implemented end-to-end in `useVoice.ts` but is wired
  off (`focusMode: false`) in `useVoiceIntegration.tsx:323`. No flag
  or setting flips it on within scope; the terminal-blur/idle
  teardown branches (`useVoice.ts:577, 630, 1033`) are therefore
  unreachable from push-to-talk in this snapshot.
