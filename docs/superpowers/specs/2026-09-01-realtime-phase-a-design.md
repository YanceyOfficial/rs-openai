# Realtime API — Phase A: Core Text-Only Voice Session — Design

Status: Approved
Date: 2026-09-01

## Context

`rs_openai`'s last unimplemented major surface. Explicitly excluded from every prior scaffolding pass because it's architecturally unlike everything else in the crate: every existing module (including Chat/Fine-Tuning/Responses' SSE streaming) is a single HTTP request with a streamed response body over the crate's existing `reqwest`-based client. Realtime is a persistent, stateful, full-duplex WebSocket connection exchanging a large taxonomy of events.

Research (via `gh api` against `openai-python`'s `resources/realtime/` and `types/realtime/`, cross-checked against the live guide at `developers.openai.com/api/docs/guides/realtime`) revealed Realtime is not one resource but a whole subsystem: 100+ dedicated types, three transports (WebSocket/WebRTC/SIP), three session types (voice agent/transcription/translation), SIP call management, ephemeral-token minting, and built-in auto-reconnect logic in the official SDK — comparable in scope to everything this project already did for the Responses API (~15 slices), not a single spec's worth of work.

Per `AskUserQuestion`, decomposed into phases (A: core text-only session; B: audio; C: function calling + MCP tools; D: SIP calls; E: ephemeral client secrets; F: transcription/translation session types; G: auto-reconnect). User chose Phase A first. This spec covers Phase A only.

## Goals

### Connection mechanics (confirmed via `openai-python`'s `resources/realtime/realtime.py`)

- URL: `wss://api.openai.com/v1/realtime` (the existing `base_url` with scheme swapped `http→ws`/`https→wss`, path suffixed `/realtime`), with `?model=...` if provided.
- Auth: the **same** `Authorization: Bearer <api_key>` header already used for every REST call, sent as a WebSocket handshake header. Confirmed no ephemeral token or `OpenAI-Beta` header is needed for a direct server-side connection — ephemeral tokens (`client_secrets`) are only for browser/WebRTC use (Phase E).
- Transport: WebSocket only (`tokio-tungstenite`, new dependency, `native-tls` feature to match `reqwest`'s existing default TLS backend and avoid a second TLS stack).

### File layout

- `interfaces/realtime.rs` (new, flat) — all event/type definitions.
- `apis/realtime/mod.rs` (new) — `Realtime<'a>` accessor (`client.realtime()`) with `connect(&RealtimeConnectRequest) -> OpenAIResponse<RealtimeConnection>`.
- `apis/realtime/connection.rs` (new) — `RealtimeConnection` (WebSocket wrapper), its `send`/`recv`/`close` methods, its `futures::Stream` impl, and the actual `tokio-tungstenite` connect logic. Kept separate from `client.rs` deliberately: `client.rs` stays HTTP-only; this file owns everything WebSocket-specific. Needs two new `pub(crate)` getters on `OpenAI` (`api_key()`, `base_url()`) since this file lives outside the `client` module.

### `RealtimeConnectRequest` (`interfaces/realtime.rs`)

```rust
pub struct RealtimeConnectRequest {
    pub model: Option<String>,
}
```

Minimal for Phase A — mirrors `openai-python`'s confirmed `connect(model=..., call_id=...)` signature with `call_id` (SIP, Phase D) and reconnection options (Phase G) deferred.

### `RealtimeConnection` API (`apis/realtime/connection.rs`)

Mirrors `openai-python`'s confirmed `AsyncRealtimeConnection` shape, adapted to Rust:

- `async fn send(&mut self, event: &RealtimeClientEvent) -> OpenAIResponse<()>`
- `async fn recv(&mut self) -> OpenAIResponse<RealtimeServerEvent>`
- `async fn close(&mut self) -> OpenAIResponse<()>`
- Implements `futures::Stream<Item = OpenAIResponse<RealtimeServerEvent>>` (delegating to the inner `tokio-tungstenite` stream's own `poll_next`, mapping `Message::Text` to a parsed `RealtimeServerEvent` and passing through end-of-stream on close) for `.next().await` ergonomics, matching this crate's existing SSE-streaming precedent.
- Deferred to later phases: auto-reconnect (Phase G), per-category sugar methods (`.session`/`.response`/`.conversation` convenience wrappers `openai-python` provides) -- Phase A callers construct `RealtimeClientEvent` variants directly and call `send()`.

### Client events (6, confirmed via `RealtimeClientEvent`'s real 11-variant union; 5 deferred to later phases)

One `RealtimeClientEvent` tagged enum (`#[serde(tag = "type")]`, explicit per-variant `#[serde(rename = "...")]` since wire values use dots):

- `SessionUpdate { event_id: Option<String>, session: RealtimeSessionConfig }` (`session.update`)
- `ConversationItemCreate { event_id: Option<String>, item: ConversationItem, previous_item_id: Option<String> }` (`conversation.item.create`)
- `ConversationItemDelete { event_id: Option<String>, item_id: String }` (`conversation.item.delete`)
- `ConversationItemRetrieve { event_id: Option<String>, item_id: String }` (`conversation.item.retrieve`)
- `ResponseCreate { event_id: Option<String>, response: Option<RealtimeResponseCreateParams> }` (`response.create`)
- `ResponseCancel { event_id: Option<String>, response_id: Option<String> }` (`response.cancel`)

Deferred: `session.update`'s audio/tools variants (covered by trimmed `RealtimeSessionConfig` below), `conversation.item.truncate` (audio-interruption-specific, zero utility without audio -- Phase B), `input_audio_buffer.*` (Phase B), `output_audio_buffer.clear` (Phase B).

### Server events (18, confirmed via `RealtimeServerEvent`'s real 41-variant union; ~23 deferred)

One `RealtimeServerEvent` tagged enum, same convention:

`SessionCreated`, `SessionUpdated`, `ConversationCreated`, `ConversationItemCreated`, `ConversationItemAdded`, `ConversationItemDone`, `ConversationItemDeleted`, `ConversationItemRetrieved`, `ResponseCreated`, `ResponseDone`, `ResponseOutputItemAdded`, `ResponseOutputItemDone`, `ResponseContentPartAdded`, `ResponseContentPartDone`, `ResponseTextDelta` (wire: `response.output_text.delta` -- confirmed GA naming, not the older `response.text.delta`), `ResponseTextDone` (`response.output_text.done`), `Error` (`error`), `RateLimitsUpdated` (`rate_limits.updated`).

Deferred: all `input_audio_buffer.*`/`output_audio_buffer.*` events, `conversation.item.truncated`, audio/audio_transcript delta+done, function_call_arguments delta+done, all `mcp_list_tools.*`/`response.mcp_call.*` events, DTMF, input-audio-transcription events -- Phases B/C.

### Supporting types

- `ConversationItem` -- a **two-level tagged union**, a genuinely new pattern for this crate: outer `#[serde(tag = "type")]` enum with one Phase-A variant, `Message(ConversationMessageItem)`; `ConversationMessageItem` is itself `#[serde(tag = "role")]`-tagged with `System`/`User`/`Assistant` variants. Required because the real API's system/user/assistant message shapes all share `type: "message"` and are only distinguished by `role` -- confirmed via source field-by-field. Deferred variants (function_call, function_call_output, the 4 MCP item types) belong to Phase C.
- `ConversationContentPart { r#type: Option<String>, text: Option<String>, audio: Option<String>, transcript: Option<String>, image_url: Option<String>, detail: Option<String> }` -- one flat, all-optional struct, matching the real API's actual per-role `Content` shape exactly (confirmed NOT a tagged enum in the wire format). Reused as-is for `ResponseContentPartAdded`/`Done`'s `part` field (confirmed structurally compatible, a strict subset).
- `RealtimeSessionConfig { r#type: String, model: Option<String>, instructions: Option<String>, output_modalities: Option<Vec<String>>, max_output_tokens: Option<serde_json::Value> }` -- deliberately trimmed from the real `RealtimeSessionCreateRequest`'s ~12 fields; `audio`/`tools`/`tool_choice`/`tracing`/`reasoning`/`prompt`/`truncation`/`include` deferred to their respective phases. Also used for `session.created`/`.updated`'s `session` field (the real API's `Union[RealtimeSessionCreateRequest, RealtimeTranscriptionSessionCreateRequest]` narrowed to just the `realtime`-type variant -- transcription sessions are Phase F).
- `RealtimeResponseCreateParams { instructions: Option<String>, output_modalities: Option<Vec<String>>, metadata: Option<HashMap<String,String>>, max_output_tokens: Option<serde_json::Value> }` -- similarly trimmed from the real type's ~11 fields.
- `RealtimeResponse { id, conversation_id, max_output_tokens: Option<serde_json::Value>, metadata: Option<HashMap<String,String>>, object, output: Option<Vec<ConversationItem>>, output_modalities, status, status_details: Option<serde_json::Value>, usage: Option<serde_json::Value> }` -- `status_details`/`usage` kept as raw JSON (established crate convention for deferring genuinely complex nested detail not core to Phase A).
- `RealtimeError { message: String, r#type: String, code: Option<String>, event_id: Option<String>, param: Option<String> }`.
- `max_output_tokens` (`int | "inf" | null` in the real API) modeled as raw `serde_json::Value` everywhere it appears -- an awkward small union not worth a dedicated type.

## Non-goals

Everything in Phases B-G (audio, function calling, MCP tools, SIP calls, ephemeral tokens, transcription/translation sessions, auto-reconnect) -- each gets its own future spec/slice if picked up.

## Testing

`wiremock` is HTTP-mock-only and can't mock WebSocket. Plan: a real local WebSocket server spun up via `tokio-tungstenite`'s own server-side API inside `#[cfg(test)]` (no new test dependency -- the client crate already provides both halves). Confirmed the existing `with_base_url()` override (already used everywhere else in this crate to point at a `wiremock` mock server) generalizes cleanly to WebSocket testing too, since `RealtimeConnection`'s URL construction just swaps `http→ws`/`https→wss` on the same `base_url`.

Tests: connection establishes and sends the `Authorization` header correctly; `send()` serializes each of the 6 client events correctly (one test per event, or parametrized where shapes are trivial); `recv()`/the `Stream` impl deserializes each of the 18 server events correctly, including the two-level `ConversationItem` tagging (a dedicated round-trip test for `System`/`User`/`Assistant` message variants, confirming the nested-tagged-enum serde pattern actually works); `close()` cleanly ends the stream.

## Versioning

Purely additive -- a wholly new top-level resource and a new dependency (`tokio-tungstenite`). No existing types touched.
