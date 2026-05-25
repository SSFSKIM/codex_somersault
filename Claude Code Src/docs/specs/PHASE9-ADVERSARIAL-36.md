# Phase 9.5b Adversarial Review — Spec 36 (VOICE_MODE)

Reviewer role: skeptic. Read-only verification against
`src/{voice,services/voice.ts,services/voiceKeyterms.ts,services/voiceStreamSTT.ts,
hooks/useVoice*,context/voice.tsx,state/AppState.tsx,screens/REPL.tsx,
keybindings/{schema,validate,defaultBindings}.ts,commands/voice/*,commands.ts}`.

## Severity counts

| Severity | Count |
|---|---|
| Critical (would mislead implementer) | 0 |
| Major (incorrect/misleading factual claim) | 2 |
| Minor (line drift / phrasing / coverage gap) | 5 |
| Nit | 2 |

## Top 5 findings

### F1 (Major) — Spec misattributes the ANT-only gate
Spec §1 lede says voice is "ant-only" via `feature('VOICE_MODE')`, and §0
overview noted ANT-only branches as a thing to verify. Verified: there is
NO `process.env.USER_TYPE === 'ant'` branch on any voice-touching file
(`grep -n "USER_TYPE\|'ant'" src/voice/* src/services/voice* src/hooks/useVoice* src/commands/voice/*`
returns zero hits). The only gate is `feature('VOICE_MODE')` (compile-time)
+ `isAnthropicAuthEnabled()` (runtime OAuth provider). However, the comment
in `voiceStreamSTT.ts:3` says *"Only reachable in ant builds (gated by
feature('VOICE_MODE'))"* — i.e. ANT-ness is implicit in the bundle flag
chosen at build time, not a separate runtime branch. Spec §1 conflates
"Anthropic-OAuth-only" with "ant-only" — those are different gates and
the spec should disambiguate (Anthropic OAuth ≠ Anthropic employee).

### F2 (Major) — "Voice mode auto-pause on terminal blur" is unsubstantiated for push-to-talk
Phase 9.5 prompt asks to verify "auto-pause on terminal blur". Verified:
`useVoice.ts` imports `useTerminalFocus` (line 11) and uses
`isFocused = useTerminalFocus()` (line 268) ONLY for **focus-mode**
sessions (`useVoice.ts:1028` "In focus mode, recording is driven by
terminal focus, not keypresses"). Push-to-talk path does NOT auto-pause
on blur — `useVoiceIntegration.tsx:323` hard-wires `focusMode: false`,
so the focus-driven teardown is dead code in REPL. Spec §9 documents
this correctly ("Wired off in REPL.tsx") but the Phase 9.5 prompt
phrasing implies it's an active feature. **Spec is right; the prompt
would be misleading.** Clarify in §9 that "auto-pause on blur" applies
only to focus mode (not push-to-talk) and is dormant in this snapshot.

### F3 (Minor) — Vim mode interaction is asserted but unverified
Spec §10 cross-refs to `39-vim-keybindings` "overlap risk on bare-char
hold key (space) inside vim insert/normal modes". Verified:
`grep -rn "voice" src/vim/` returns ZERO matches. Voice integration
neither knows about vim mode nor checks vim state before activating.
The `useVoiceKeybindingHandler` only checks `useIsModalOverlayActive`
(line ~388) and the `Chat` keybinding context. **The "overlap risk"
claim is correct but undertested**: in vim insert mode, holding space
WILL still trigger HOLD_THRESHOLD=5 and consume input. Spec should
upgrade §10 from "overlap risk" to "**confirmed** behavioral overlap;
no vim-mode awareness in the voice handler". Cross-spec 39 should
record this as an open issue, not a benign cross-ref.

### F4 (Minor) — `commands.ts:80-81, 328` cited; verified, BUT REPL line ref drifts
Spec §1 cites `screens/REPL.tsx:98-103` for `useVoiceIntegration` and
`§3.5` cites `screens/REPL.tsx:4022-4033` for the call site. Verified:
- Line 98: `const useVoiceIntegration` import gate ✓
- Line 103: `VoiceKeybindingHandler` import gate ✓
- Line 4024: `useVoiceIntegration({...})` call ✓
All three correct to the line. `commands.ts:80-81, 328` ✓.
**However** spec §3.5 says "REPL.tsx:4022-4033" — actual call spans
4022-4034. Off-by-one cosmetic.

### F5 (Minor) — Spec omits `VoiceKeybindingHandler` from §1 file table
§1 lists `useVoiceIntegration.tsx` but not the separately-exported
`VoiceKeybindingHandler` component (gated independently at
`screens/REPL.tsx:103`). It's mentioned in §6.10 prose and §7 pseudocode,
but a reader scanning §1 would miss that the `<VoiceKeybindingHandler>`
React component is mounted in REPL alongside the hook. Add a row.

## Verdict

**ACCEPT WITH MINOR REVISIONS.** Spec 36 is high-fidelity and §2
successfully resolves the Phase 0 "voice context wiring location is
unresolved" gap with specific line refs (`context/voice.tsx`,
`state/AppState.tsx:14-16, 94`) all of which I verified line-exact
(VoiceProvider passthrough at `AppState.tsx:14-16`; mount at line 94
inside `<MailboxProvider>`). Constants table (§6.12) is verified
against source — every value matches. WebSocket protocol (§5, §6.4),
keyterm dictionary (§6.6), language allowlist (§6.11), and pseudocode
(§7) all match the implementation. Recommended changes:
1. F1: split "ant-only" from "Anthropic-OAuth-only" in §1 lede
2. F2: clarify §9 that blur-teardown is focus-mode-only, dormant in REPL
3. F3: strengthen §10 vim cross-ref from "risk" to "confirmed gap"
4. F5: add `VoiceKeybindingHandler` to §1 file table
5. F4: cosmetic line range fix

## Cross-spec impact

- **21c (`/voice` command)** — spec 36 §3 fully covers `/voice` semantics
  (pre-flight chain, lang note, toggle off path). Spec 21c can defer to
  §3 and §6.8; no duplicated content needed. ✓
- **39 (vim keybindings)** — spec 39 should add an "open issue: voice
  push-to-talk has no vim-mode awareness; bare-char binding (space)
  consumes input in vim insert mode after HOLD_THRESHOLD". Currently
  spec 36 only says "overlap risk".
- **37 (Ink UI shell)** — `<VoiceIndicator>`, `<VoiceWarmupHint>`,
  `<VoiceModeNotice>` cross-ref correct.
- **41 (session/state)** — §3.11 correctly notes voice never reaches
  `QueryEngine` directly (one-way splice into prompt buffer).
- **26 (analytics flags)** — events list (`tengu_voice_toggled`,
  `tengu_voice_recording_started/_completed`,
  `tengu_voice_stream_early_retry`, `tengu_voice_silent_drop_replay`,
  `tengu_amber_quartz_disabled`, `tengu_cobalt_frost`) — first three
  verified in source; spec adds two events I did not bottom-verify.

## Hardest-to-verify claim

**§7.2 finishRecording's silent-drop replay path** — the spec asserts a
specific 8-condition guard (`finalizeSource == 'no_data_timeout' &&
hadAudio && wsConnected && !focusTriggered && focusFlushed == 0 &&
accumulated.trim() == '' && !silentDropRetried && fullAudio.length > 0`)
plus a 250 ms backoff replay in 32 KB slices on a fresh connection.
I could not exhaustively verify this in the budget — `useVoice.ts` is
1144 LOC and the replay logic spans roughly lines 633-1011 with state
captured across multiple closures (`silentDropRetriedRef`, `attemptGen`,
`fullAudioRef`, `focusFlushedRef`). The constants and high-level shape
match what I sampled, but a true line-by-line audit of the replay
guard would require ~5 more reads of useVoice.ts windows. The spec's
references (§3.10, §7.2) are internally consistent and the 8 conditions
appear plausible from the imports/refs, but this is the section most
likely to harbor an off-by-one or a missing guard not represented in
the spec.
