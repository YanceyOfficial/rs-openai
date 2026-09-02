# Realtime API — Phase F: Transcription Session Type — Design

Status: Approved
Date: 2026-09-02

## Context

Sixth Realtime slice, same condensed spec-then-direct-implementation rhythm as Phases B/C/D/E. Adds the Realtime API's **transcription session type** (`type: "transcription"`) -- a session that runs speech-to-text on the input audio buffer and emits transcription events, without the model generating any conversational response. Confirmed via `gh api` against `openai-python`'s `types/realtime/` (every relevant file fetched and read directly).

**Phase E's spec projected that this slice would "widen the `session` sites to a `#[serde(tag = "type")]` enum." Reading the actual transcription types changes that call:**

- A transcription session object is just `{ type: "transcription", audio: { input: {...} }, include: [...] }` (plus `id` / `object` / `expires_at` on responses -- already covered by Phase E's additive fields on `RealtimeSessionConfig`).
- Its `audio.input` shape -- `{ format, noise_reduction, transcription, turn_detection }` -- is confirmed **field-for-field identical** to the crate's existing Phase B `RealtimeAudioConfigInput`. `RealtimeAudioConfig { input, output }` with `output` unset serializes as `{"input":{...}}`, matching the transcription session's audio block exactly. Turn detection is the same `ServerVad | SemanticVad` already modeled as `RealtimeTurnDetection`.
- The **only** field `RealtimeSessionConfig` lacks for a transcription session is `include`.

An enum would buy nominal type-safety (can't set `tools` on a transcription session) that this crate deliberately does not provide elsewhere -- `RealtimeSessionConfig` is already a bag of optional fields where `r#type` is a free discriminator string, and request-only / response-only fields are already conflated (Phase E's `id` / `object` / `expires_at`). Its cost would be a new type, serde-ordering hazards (untagged) or a tag/`type`-field collision (internally tagged), and breaking field-type changes at five sites. Not worth it. This slice extends `RealtimeSessionConfig` instead -- consistent with the established design, and purely additive.

Confirmed via source: the server-event union gains exactly four transcription entries (`conversation.item.input_audio_transcription.completed` / `.delta` / `.failed` / `.segment`) not yet modeled. No new client events -- a transcription session is configured through the existing `session.update` client event. No `transcription_session.created` / `.updated` events exist; `session.created` / `session.updated` cover both session types (their `session` field is `Union[RealtimeSessionCreateRequest, RealtimeTranscriptionSessionCreateRequest]`, which the extended `RealtimeSessionConfig` receives losslessly).

Confirmed via source that `calls.create` / `calls.accept` (Phase D) take a **realtime-only** session param, not the union -- SIP/WebRTC calls are voice conversations. Those signatures are unchanged.

## Goals

### `RealtimeSessionConfig` gains one additive field (`interfaces/realtime.rs`)

- `include: Option<Vec<String>>` -- `#[serde(skip_serializing_if = "Option::is_none")]`. The only documented value is `"item.input_audio_transcription.logprobs"`; modeled as `Vec<String>` (not an enum) matching the crate's treatment of every other small string-set on this type. Confirmed via source it is also valid on `realtime` sessions (both `RealtimeSessionCreateRequest` and `RealtimeTranscriptionSessionCreateRequest` carry it), so this is not transcription-specific.

After this, `RealtimeSessionConfig::builder().r#type("transcription").audio(...).include(["item.input_audio_transcription.logprobs"]).build()` is a complete transcription session, flowing through the existing `session.update` client event and `session.created` / `.updated` / client-secret responses with no type changes anywhere.

### `RealtimeAudioConfigInput.transcription` becomes typed (`interfaces/realtime.rs`)

Phase B parked this as raw `serde_json::Value` ("deferred richness, secondary to 'does audio work'"). Phase F is when it becomes primary. Replaced with:

- `transcription: Option<RealtimeAudioTranscription>` where `RealtimeAudioTranscription { delay: Option<String>, keywords: Option<Vec<String>>, language: Option<String>, languages: Option<Vec<String>>, model: Option<String>, prompt: Option<String> }` -- `Builder, Debug, Serialize, Deserialize, Clone, Default`, every field `#[serde(skip_serializing_if = "Option::is_none")]`. Confirmed via source the request (`AudioTranscriptionParam`) and response (`AudioTranscription`) shapes are identical, so one type serves both. `delay` (`"minimal"|"low"|"medium"|"high"|"xhigh"`) and `model` (8 named values or a free string) kept as `Option<String>`, matching `RealtimeTurnDetection::SemanticVad.eagerness` and the crate's model-name convention.

This is technically breaking for any caller currently passing a `serde_json::Value` there, but pre-1.0 and the ergonomic payoff is the core of this phase. The Phase B "deferred richness" doc comment on `RealtimeAudioConfigInput` is updated.

### Four new `RealtimeServerEvent` variants (`interfaces/realtime.rs`)

All confirmed via source. Flat single-level `type`-tagged variants:

- `ConversationItemInputAudioTranscriptionCompleted { content_index: u32, event_id: String, item_id: String, transcript: String, usage: serde_json::Value, languages: Option<Vec<RealtimeTranscriptionLanguage>>, logprobs: Option<Vec<RealtimeLogProb>> }` -- tag `conversation.item.input_audio_transcription.completed`. `usage` is a two-variant union (token-billed vs. duration-billed) kept as raw JSON, matching `RealtimeResponse.usage`'s existing precedent -- genuinely secondary nested detail. `RealtimeTranscriptionLanguage { code: String }` (confirmed via source: an object, not a bare string, unlike `RealtimeAudioTranscription.languages`).
- `ConversationItemInputAudioTranscriptionDelta { event_id: String, item_id: String, content_index: Option<u32>, delta: Option<String>, logprobs: Option<Vec<RealtimeLogProb>> }` -- tag `...transcription.delta`.
- `ConversationItemInputAudioTranscriptionFailed { content_index: u32, event_id: String, item_id: String, error: RealtimeTranscriptionError }` -- tag `...transcription.failed`. `RealtimeTranscriptionError { code: Option<String>, message: Option<String>, param: Option<String>, r#type: Option<String> }` (all optional, confirmed via source).
- `ConversationItemInputAudioTranscriptionSegment { id: String, content_index: u32, end: f64, event_id: String, item_id: String, speaker: String, start: f64, text: String }` -- tag `...transcription.segment`.

Plus `RealtimeLogProb { token: String, bytes: Vec<u8>, logprob: f64 }` -- `Debug, Serialize, Deserialize, Clone`, confirmed via source (`LogProbProperties`). Small and central to the `logprobs` fields; modeled fully.

### `session` boxed in the three session-holding event variants

Typing `transcription` grows `RealtimeSessionConfig` by ~170 bytes, which tips `clippy::large_enum_variant` (CI runs `-D warnings`) on `RealtimeClientEvent`. Fixed the crate-consistent way (as `RealtimeTool::Mcp` already is): `session` becomes `Box<RealtimeSessionConfig>` in `RealtimeClientEvent::SessionUpdate` and, for uniformity, in `RealtimeServerEvent::SessionCreated` / `SessionUpdated`. Callers write `session: Box::new(config)`; matchers that bind `session` add a deref. `serde` handles `Box<T>` transparently, so no wire-format change.

### No client-event, `apis/`, or `client.rs` changes

Confirmed via source: `realtime_client_event.py`'s union is unchanged. A transcription session is configured with the existing `session.update` event carrying a `type: "transcription"` `RealtimeSessionConfig`. No new REST endpoint. `apis/realtime/mod.rs` needs no new accessor -- only its and `interfaces/realtime.rs`'s doc comments updated for Phase F, and the `Phase E → F` spec cross-references extended.

## Non-goals

- A `realtime | transcription` session enum -- see Context for why `RealtimeSessionConfig` is extended instead.
- Typed `usage` on the completed event -- the token/duration billing union is secondary nested detail, raw JSON per the crate's established escape-hatch convention.
- `RealtimeSessionConfig`'s other still-deferred fields (`tracing` / `reasoning` / `prompt` / `truncation`) -- untouched, unchanged from Phase A.
- Any change to `calls.create` / `calls.accept` -- confirmed realtime-only on the real API.
- `conversation.item.truncate` / `.truncated`, DTMF -- still deferred, unrelated to transcription.

## Testing

Same layered discipline as Phases B/C -- plain `serde_json` round-trip tests, no live socket needed for shape correctness:

- The four new server events added to `apis/realtime/mod.rs`'s existing `server_events_deserialize` table (one row each), plus a dedicated round-trip test for `...segment` and `...failed` (the two with the most structure -- nested `error`, `f64` timing fields).
- A `session.update` serialization test for a `type: "transcription"` session (`r#type` + `audio.input.transcription` typed fields + `include`), asserting the exact wire body.
- A round-trip test for `RealtimeAudioTranscription` on its own (every field populated), and one confirming `RealtimeSessionConfig` with `include` set round-trips.
- No new live-WebSocket test -- Phase A/B's `send`/`recv` tests already prove the transport handles arbitrary event payloads; this slice adds only new shapes.

## Versioning

Two breaking changes, both small and pre-1.0:

- `RealtimeAudioConfigInput.transcription`: `serde_json::Value` -> `Option<RealtimeAudioTranscription>` -- breaking only for callers who passed a raw JSON value there (a field Phase B shipped days ago; no example uses it).
- `session` in `RealtimeClientEvent::SessionUpdate` / `RealtimeServerEvent::SessionCreated` / `SessionUpdated`: `RealtimeSessionConfig` -> `Box<RealtimeSessionConfig>` (see the boxing section above).

The new `include` field on `RealtimeSessionConfig` and the four new server-event variants are purely additive. No wire-format change.
