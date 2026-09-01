# Realtime API — Phase B: Audio Input/Output — Design

Status: Approved
Date: 2026-09-02

## Context

Second Realtime slice, building directly on Phase A's already-settled architecture (WebSocket connection, `RealtimeConnection`'s `send`/`recv`/`Stream` shape, the two-level `ConversationItem` tagging, the trim-aggressively-and-defer-richness discipline). No new architectural decisions needed here -- this is "apply the established pattern to the audio event/config surface," matching how the 5 Responses tool-streaming phases proceeded directly after the first (file_search) settled the pattern via `AskUserQuestion`.

## Goals

### Client events (3 new, confirmed via source; `output_audio_buffer.clear` deliberately excluded -- see Non-goals)

- `InputAudioBufferAppend { event_id: Option<String>, audio: String }` (`input_audio_buffer.append`) -- `audio` is base64-encoded bytes in the format configured by `session.audio.input.format`.
- `InputAudioBufferCommit { event_id: Option<String> }` (`input_audio_buffer.commit`)
- `InputAudioBufferClear { event_id: Option<String> }` (`input_audio_buffer.clear`)

### Server events (9 new)

- `InputAudioBufferCommitted { event_id, item_id, previous_item_id: Option<String> }` (`input_audio_buffer.committed`)
- `InputAudioBufferCleared { event_id }` (`input_audio_buffer.cleared`)
- `InputAudioBufferSpeechStarted { event_id, item_id, audio_start_ms: i64 }` (`input_audio_buffer.speech_started`)
- `InputAudioBufferSpeechStopped { event_id, item_id, audio_end_ms: i64 }` (`input_audio_buffer.speech_stopped`)
- `InputAudioBufferTimeoutTriggered { event_id, item_id, audio_start_ms: i64, audio_end_ms: i64 }` (`input_audio_buffer.timeout_triggered`)
- `ResponseAudioDelta { content_index, delta, event_id, item_id, output_index, response_id }` (`response.output_audio.delta` -- confirmed GA naming)
- `ResponseAudioDone { content_index, event_id, item_id, output_index, response_id }` (`response.output_audio.done`)
- `ResponseAudioTranscriptDelta { content_index, delta, event_id, item_id, output_index, response_id }` (`response.output_audio_transcript.delta`)
- `ResponseAudioTranscriptDone { content_index, event_id, item_id, output_index, response_id, transcript }` (`response.output_audio_transcript.done`)

### Audio config types (`interfaces/realtime.rs`)

- `RealtimeAudioFormat` -- a `type`-tagged 3-variant enum (`AudioPcm { rate: Option<u32> }` / `AudioPcmu` / `AudioPcma`, wire tags `audio/pcm`/`audio/pcmu`/`audio/pcma` -- explicit per-variant `#[serde(rename)]` since the values contain `/`, not expressible via `rename_all`).
- `RealtimeNoiseReduction { r#type: Option<String> }` (`"near_field"`/`"far_field"`).
- `RealtimeTurnDetection` -- a `type`-tagged 2-variant enum: `ServerVad { create_response, idle_timeout_ms, interrupt_response, prefix_padding_ms, silence_duration_ms, threshold }` / `SemanticVad { create_response, eagerness, interrupt_response }`. Modeled fully (not trimmed) -- VAD is the core mechanism that makes a voice conversation usable turn-by-turn; without it the client must manually commit every buffer chunk, which would make Phase B not meaningfully useful.
- `RealtimeAudioOutputConfig { format: Option<RealtimeAudioFormat>, speed: Option<f32>, voice: Option<serde_json::Value> }` -- one shared type reused across all three places the real API defines near-identical output-audio shapes (session-level `audio.output`, per-response `response.create`'s `audio.output` override, and the `RealtimeResponse.audio.output` resource field) -- confirmed via source that the only difference between them is whether `speed` is present, which is harmless as an always-optional field left unset where the real type doesn't have it. `voice` (a `string | one-of-10-named-values | {id: string}` union) kept as raw `serde_json::Value` -- an awkward small union not worth a dedicated type, matching `max_output_tokens`'s precedent from Phase A.
- `RealtimeAudioConfigInput { format: Option<RealtimeAudioFormat>, noise_reduction: Option<RealtimeNoiseReduction>, transcription: Option<serde_json::Value>, turn_detection: Option<RealtimeTurnDetection> }` -- `transcription`'s full config (delay/keywords/language/languages/model/prompt -- 6 sub-fields) kept as raw JSON, a deliberately deferred richness (secondary to "does audio work," matching Phase A's own established trimming discipline).
- `RealtimeAudioConfig { input: Option<RealtimeAudioConfigInput>, output: Option<RealtimeAudioOutputConfig> }` -- added as `RealtimeSessionConfig.audio` (previously omitted entirely in Phase A).
- `RealtimeAudioOutputConfigWrapper { output: Option<RealtimeAudioOutputConfig> }` -- the `{ output: ... }` nesting the real API uses for `response.create`'s per-call `audio` override and for the `RealtimeResponse.audio` resource field (confirmed via source: both wrap a single `output` key, distinct from the session-level `RealtimeAudioConfig` which has both `input` and `output`).

### Existing type updates

- `RealtimeSessionConfig` gains `audio: Option<RealtimeAudioConfig>`.
- `RealtimeResponseCreateParams` gains `audio: Option<RealtimeAudioOutputConfigWrapper>` where the wrapper is `{ output: Option<RealtimeAudioOutputConfig> }` (matches the real API's `RealtimeResponseCreateAudioOutput { output }` nesting).
- `RealtimeResponse` gains `audio: Option<RealtimeAudioOutputConfigWrapper>` (same wrapper, reused).
- `ConversationContentPart` needs no changes -- Phase A's flat, all-optional design already carries `audio`/`transcript` fields generically, confirming that design choice was forward-compatible.

## Non-goals

- `output_audio_buffer.clear` (client) and its three paired server events (`output_audio_buffer.started`/`.stopped`/`.cleared`) -- confirmed via source to be **WebRTC/SIP only** ("This event is emitted after an audio content part has been added... Learn more... for audio in WebRTC"). Genuinely inapplicable to a WebSocket-only connection, not merely deferred -- the client controls audio playback locally over WebSocket and has no need for a server-side "stop streaming audio to me" signal that only makes sense when the server itself is the one pushing continuous audio (WebRTC/SIP).
- Full `AudioTranscription` config modeling (kept as raw JSON -- see above).
- Everything already listed as out of scope for the whole Realtime effort in Phase A's spec: function calling, MCP tools, SIP calls, ephemeral tokens, transcription/translation session types, auto-reconnect.

## Testing

Same layered discipline as Phase A: plain `serde_json` round-trip tests for all 12 new event shapes and the new config types (including `RealtimeAudioFormat`'s and `RealtimeTurnDetection`'s tagged-union variants) -- no live socket needed for shape correctness. One live-WebSocket test confirming an `input_audio_buffer.append` client event with realistic base64 payload transmits correctly end-to-end (extends the existing local-test-server harness from `apis/realtime/connection.rs`, no new test infrastructure).

## Versioning

Purely additive.
