# Codex vs Claude Code: 시스템 프롬프트 톤 비교 연구

> 연구 질문: "사람들은 흔히 Claude Code는 인간처럼 따뜻하게 대화하고, Codex 에이전트는 기계적이고 딱딱하다고 말한다. 두 하네스의 **시스템 프롬프트**를 비교하면 이 체감 차이가 어디서 오는가? 그리고 Claude Code의 접근을 차용해 Codex를 더 편안·친근하게 만들 수 있는가?"
>
> 작성일: 2026-05-31 · 대상 리포: 이 포크 (`codex-rs/` = Codex 제품, `Claude Code Src/` = Claude Code TS 참조 하네스)

---

## 0. 한눈에 보는 결론

체감("Claude=인간적, Codex=기계적")은 **부분적으로 부정확하지만, 설계상 근거가 있다.**

1. **부정확한 부분** — Codex에는 Claude보다도 따뜻한 페르소나(`friendly`)가 내장돼 있다. 능력의 문제가 아니라 *기본값 선택*의 문제다.
2. **근거 있는 부분** — Codex의 사실상 기본 톤은 `concise, direct`(효율 지향)에 **엄격한 포맷 규칙**이 더해져 출력이 정형화된다. Claude Code 기본은 "콘솔이 아니라 사람에게 쓴다"는 프레임을 명시하고 구조 남용을 경계해 산문체·유연함을 유도한다.
3. **설계 철학 차이** — Codex는 톤을 *교체 가능한 모듈*(`{{ personality }}` 슬롯)로, Claude Code는 *제품 정체성에 녹인 단일 서술*로 다룬다.

---

## 1. 비교 대상 파일

| 진영 | 파일 | 역할 |
|------|------|------|
| Codex | `codex-rs/core/gpt_5_2_prompt.md` | GPT-5.2 기본 시스템 프롬프트 |
| Codex | `codex-rs/protocol/src/prompts/base_instructions/default.md` | 프로토콜 계층 기본 지시 |
| Codex | `codex-rs/core/templates/model_instructions/gpt-5.2-codex_instructions_template.md` | `{{ personality }}` 슬롯을 가진 템플릿 |
| Codex | `codex-rs/core/templates/personalities/gpt-5.2-codex_friendly.md` | "friendly" 페르소나 변형 |
| Codex | `codex-rs/core/templates/personalities/gpt-5.2-codex_pragmatic.md` | "pragmatic" 페르소나 변형 |
| Claude Code | `Claude Code Src/src/constants/prompts.ts` | 단일 시스템 프롬프트 (조건부 블록 포함) |

---

## 2. 핵심 발견 — "Codex도 따뜻할 수 있다"

가장 중요한 반전: Codex의 `friendly` 페르소나는 Claude Code의 어떤 톤 지시보다도 감정적으로 따뜻하다.

`codex-rs/core/templates/personalities/gpt-5.2-codex_friendly.md:11-16`:

> "Your voice is **warm, encouraging, and conversational. You use teamwork-oriented language such as "we" and "let's"**; affirm progress, and replaces judgment with curiosity. You use light enthusiasm and humor... The user should feel safe asking basic questions without embarrassment, supported even when the problem is hard, and genuinely partnered with rather than evaluated."
>
> "**You are NEVER curt or dismissive.**"

즉 "딱딱함"은 Codex의 한계가 아니라 **기본 페르소나 선택**의 결과다. 사람들이 체감하는 Codex는 `friendly`가 아니라 `pragmatic` 계열(또는 모델 기본 프롬프트의 `concise, direct`)이다.

---

## 3. 차이를 만드는 5개 축

### 3.1 기본 톤 지시의 분량과 프레이밍

**Codex 기본** — 한 문단으로 끝난다. `codex-rs/core/gpt_5_2_prompt.md:15`:

> "Your default personality and tone is **concise, direct, and friendly**. You communicate efficiently, always keeping the user clearly informed about ongoing actions without unnecessary detail... Unless explicitly asked, you avoid excessively verbose explanations about your work."

**Claude Code 기본** — 전용 섹션을 할애한다. `Claude Code Src/src/constants/prompts.ts:408-411`:

> "# Communicating with the user
> When sending user-facing text, **you're writing for a person, not logging to a console.**... When making updates, **assume the person has stepped away and lost the thread.** They don't know codenames, abbreviations, or shorthand... Write so they can pick back up cold: use complete, grammatically correct sentences without unexplained jargon. Expand technical terms... **Attend to cues about the user's level of expertise; if they seem like an expert, tilt a bit more concise, while if they seem like they're new, be more explanatory.**"

| 축 | Codex 기본 | Claude Code 기본 |
|----|-----------|-----------------|
| 톤 지시 분량 | 한 문단 | 전용 섹션 + 여러 문단 |
| 독자 프레이밍 | "the user"(작업 대상) | "a person" who "lost the thread"(사람) |
| 문체 지시 | "concise, efficient" | "flowing prose", "complete sentences", "expand jargon" |
| 전문성 적응 | 없음 | 전문가면 간결, 초보면 설명적 |

같은 "concise"라도 Claude는 *사람에게 글을 쓴다*는 프레임에서 출발하고, Codex 기본은 *효율적 정보 전달*에서 출발한다.

### 3.2 포맷팅 철학 — 기계적 인상의 1차 원인

Codex `model_instructions/gpt-5.2-codex_instructions_template.md:7,12-13`은 경직된 규칙을 명시한다:

> "Follow the formatting rules exactly... **Never use nested bullets.** Keep lists flat... Headers... short Title Case (1-3 words) wrapped in `**…**`."

같은 파일이 "Formatting should make results easy to scan, **but not feel mechanical**"(:7)이라고 덧붙이지만, 규칙 자체가 정형화를 강제한다. 추가로 `gpt_5_2_prompt.md:226-231`의 "Final answer compactness rules (enforced)"는 문장 수·스니펫 수까지 수치로 통제한다.

반대로 Claude Code는 구조 남용을 경계한다. `Claude Code Src/src/constants/prompts.ts:413` 및 톤 섹션:

> "Write user-facing text in **flowing prose**... **Only use tables when appropriate**... Match responses to the task: a simple question gets a direct answer **in prose, not headers and numbered sections.**"

→ Codex는 "정해진 틀로 스캔하기 좋게", Claude는 "사람이 읽기 좋게 유연하게". 이 시각적 1차 인상이 딱딱함 vs 인간미를 가른다.

### 3.3 협업자 vs 실행자 스탠스

둘 다 "단순 실행자가 아니다"라고 하지만 강조점이 다르다.

- **Claude Code** (`prompts.ts`, Doing tasks 섹션): "You're a **collaborator, not just an executor**—users benefit from your **judgment, not just your compliance**." (사용자 전제가 틀렸으면 지적)
- **Codex pragmatic** (`...codex_pragmatic.md:3`): "deeply pragmatic, effective software engineer... collaboration is a kind of **quiet joy**." (묵묵히 유능한 동료)

Claude는 "의견을 가진 동료", Codex 기본은 "조용히 잘하는 엔지니어"에 가깝다. 후자가 더 도구처럼 느껴진다.

### 3.4 아첨 금지 — 차이가 아닌 공통점

흔히 "Codex가 차갑다"의 원인으로 지목되지만, **Claude도 동일하게 금지**한다.

- Codex pragmatic(`...codex_pragmatic.md:15`): "avoiding **cheerleading, motivational language, or artificial reassurance**... no flattery, no hype."
- Claude Code(Tone and style): "Don't... use superlatives to **oversell small wins**."

둘 다 영혼 없는 칭찬을 금지한다. 체감 차이의 원인은 여기가 아니다.

### 3.5 아키텍처 — 모듈 주입 vs 단일 서술

- **Codex**: 페르소나가 `{{ personality }}` 슬롯으로 **교체 가능** (`...instructions_template.md:3`). friendly/pragmatic을 런타임 주입. 톤이 별도 파일로 분리·모듈화됨.
- **Claude Code**: 톤이 단일 시스템 프롬프트에 `[ANT-ONLY]`/`[3P-ONLY]` 조건부 블록으로 **녹아 있음**. 풍부한 "Communicating with the user" 섹션은 `[ANT-ONLY]`라 1st-party 환경에서만 전체 활성화되고, 3rd-party 빌드는 더 짧은 "Output efficiency"만 받는다.

### 보너스: Codex도 완전히 차갑진 않다

공정을 기하면, Codex 기본 프롬프트에도 따뜻한 지시가 있다. `gpt_5_2_prompt.md:162`:

> "For casual conversation, brainstorming tasks, or quick questions... **respond in a friendly, conversational tone.** You should ask questions, suggest ideas, and adapt to the user's style."

다만 이는 "casual" 상황으로 한정되고, 작업 결과 보고는 `:170`의 "Brevity is very important as a default... no more than 10 lines"와 엄격한 포맷 규칙에 지배된다. 그래서 *작업 중* 인상이 기계적으로 남는다.

---

## 4. 제안 — Claude Code 기반 Codex 톤 개선

목표: Codex가 더 편안·친근하게 느껴지도록, **Claude Code의 검증된 기법을 Codex 프롬프트 구조에 이식**한다. Codex의 모듈식 설계(`{{ personality }}` 슬롯, 별도 페르소나 파일)를 존중하므로, 침습적 재작성보다 **삽입형 개선**이 적합하다.

### 제안 1 — 기본 페르소나를 friendly 쪽으로 이동 (가장 저비용·고효과)

체감 문제의 핵심은 "능력 부재"가 아니라 "기본값 선택"이다. 기본 personality 주입을 `pragmatic` → `friendly`로 바꾸거나, 둘을 절충한 `balanced` 페르소나를 신설한다. 코드 변경 없이 프롬프트 파일 한 개로 톤이 크게 달라진다.

### 제안 2 — "사람에게 쓴다" 프레임 이식

Codex 기본 Personality(`gpt_5_2_prompt.md:15`)는 *무엇을* 전달할지만 말하고, *누구에게* 쓰는지는 말하지 않는다. Claude Code의 핵심 한 문장을 차용한다.

**Before** (`gpt_5_2_prompt.md:15`):
> Your default personality and tone is concise, direct, and friendly. You communicate efficiently...

**After (제안):**
> Your default personality and tone is concise, direct, and friendly. **When you write user-facing text, remember you're writing for a person, not logging to a console — they see only your words, not your tool calls or reasoning.** You communicate efficiently... **and you write in complete, readable sentences, expanding jargon and codenames the user may not have tracked.**

차용 근거: `Claude Code Src/src/constants/prompts.ts:409-411`.

### 제안 3 — 포맷 규칙에 "기계적으로 느껴지지 않기" 우선순위 명시

현재 템플릿은 "but not feel mechanical"을 한 번 언급하지만, 곧바로 "Follow the formatting rules exactly"가 이를 압도한다. Claude의 "task에 맞춰 형태를 고르라"는 원칙을 우선 규칙으로 끌어올린다.

**삽입 제안** (`...instructions_template.md`의 Final answer formatting rules 상단):
> **Before applying any structure rule below, ask whether structure actually helps this answer. A simple question deserves a direct answer in prose, not headers and bullets. Reach for headers/tables only when grouping genuinely aids scanning — readability for a human reader outranks conformance to these rules.**

차용 근거: Claude Code Tone and style — "a simple question gets a direct answer in prose, not headers and numbered sections."

### 제안 4 — "진행 중" 업데이트를 인간적으로

Claude Code는 작업 도중 "사람이 자리를 비웠다 돌아왔다고 가정하고" 핵심 순간마다 짧게 알린다(`prompts.ts:409`). Codex의 Autonomy 섹션은 "persist until done"에 치우쳐 *침묵 후 결과 덤프* 패턴을 유도한다. 다음을 추가한다:

> **While working, give brief human-readable updates at load-bearing moments — when you find a root cause, change direction, or hit a blocker — written so a user who stepped away can pick the thread back up cold. Don't narrate every step; do surface the few that matter.**

### 제안 5 — `friendly` 페르소나의 강점을 기본에도 한 스푼

`friendly` 전체를 기본화하지 않더라도, "we/let's" 같은 협업 언어와 "사용자가 기본적인 질문을 부끄럼 없이 하도록"이라는 심리적 안전감 한 줄을 기본 Personality에 더하면 친근함이 크게 오른다 (`...codex_friendly.md:12` 차용).

### 적용 시 주의 (이 리포의 컨벤션)

- 톤 변경은 **프롬프트 파일 수정**이므로 `codex-core` 로직 변경은 거의 없다. 그래도 프롬프트가 스냅샷 테스트에 묶여 있으면 `cargo insta` 갱신이 필요할 수 있다 (`AGENTS.md`).
- 기본 페르소나 전환은 사용자 체감을 직접 바꾸는 결정이므로, A/B 또는 옵트인 설정(`config.toml`)으로 도입하는 편이 안전하다.
- 제안 2~4는 비파괴적 "삽입"이라 위험이 낮고, 제안 1(기본값 전환)은 영향이 커 별도 합의가 필요하다.

---

## 5. 후속 연구 방향

- **정량 비교**: 두 기본 프롬프트의 톤 지시 토큰 수, "you/we/user" 빈도, 명령형 vs 서술형 비율 측정.
- **재현 실험**: friendly vs pragmatic vs Claude 기본에 동일 프롬프트를 줘 출력 차이를 `just exec`로 수집·대조.
- **사용자 평가**: 동일 작업의 두 톤 출력에 대한 편안함/신뢰도 블라인드 설문.

---

## 부록 — 인용 출처 색인

- `codex-rs/core/gpt_5_2_prompt.md` — :15 (Personality), :160-170 (Presenting your work), :172-243 (formatting rules), :226-231 (compactness enforced)
- `codex-rs/core/templates/model_instructions/gpt-5.2-codex_instructions_template.md` — :3 (personality slot), :7 (formatting exactly / not mechanical), :12-13 (no nested bullets, Title Case)
- `codex-rs/core/templates/personalities/gpt-5.2-codex_friendly.md` — :11-16 (warm/conversational, never curt)
- `codex-rs/core/templates/personalities/gpt-5.2-codex_pragmatic.md` — :3 (quiet joy), :15 (no cheerleading)
- `Claude Code Src/src/constants/prompts.ts` — :408-413 (Communicating with the user), :435 (emoji), :444 (Tone and style)
