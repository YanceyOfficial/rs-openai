# Realtime API — Phase D: SIP Call Management — Design

Status: Approved
Date: 2026-09-02

## Context

Fourth Realtime slice, same condensed spec-then-direct-implementation rhythm as Phases B/C. Unlike A/B/C, this slice is **architecturally ordinary for this crate**: confirmed via `gh api` against `openai-python`'s `resources/realtime/calls.py` that the `/v1/realtime/calls/*` surface is plain REST (POST + JSON, one POST + multipart), not WebSocket -- it fits this crate's standard `interfaces::*`/`apis::*` + `wiremock` pattern directly, the first Realtime slice able to reuse that existing testing discipline instead of the WebSocket-only harness Phase A built.

Confirmed via source: `Calls` has exactly 5 methods -- `create`, `accept`, `hangup`, `refer`, `reject`. `accept`/`hangup`/`refer`/`reject` return HTTP 204 No Content (`cast_to=NoneType` in the SDK). **This crate has never had to handle a 204 No Content response before** -- every existing endpoint, even `cancel`/`delete` actions, returns a JSON body (e.g. a `Deleted` object). The existing `post<T, F>`/`resolve_response` machinery unconditionally calls `serde_json::from_slice` on the response body, which fails on a genuinely empty body (not even valid JSON `null`). A new client primitive is needed.

**A real usability gap, closed as part of this slice's scope rather than left implicit**: `Calls` has no `list`/`retrieve` -- the only way to learn a `call_id` exists (for an inbound SIP call) is OpenAI's `realtime.call.incoming` webhook event. Confirmed via source (`types/webhooks/realtime_call_incoming_webhook_event.py`) that its `data` payload is `{ call_id: string, sip_headers: [{name, value}] }`. This crate's existing `WebhookEvent` (`interfaces/webhooks.rs`) is already a fully generic envelope (`kind: String`, `data: serde_json::Value`) -- it needs **zero code changes** to receive this event; a caller already gets it today. What's missing is a convenience type to deserialize `data` into once `kind == "realtime.call.incoming"`, without which Phase D's `accept`/`reject` methods would have no practical way to obtain a `call_id` in the first place. Added as a small, clearly-cross-referenced addition to `interfaces/realtime.rs` (not a change to `webhooks.rs` itself).

## Goals

### New `client.rs` primitive: 204 No Content POST

- `resolve_no_content_response(request) -> OpenAIResponse<()>` -- mirrors `resolve_file_response`'s existing shape (status-check, parse `ApiErrorResponse` from the body on failure, succeed with `()` without attempting to parse a body on success). Uses the retried client (`openai_request`, not `raw_request`) since these are plain JSON POSTs, matching `post<T, F>`'s existing client choice.
- `post_no_content<F>(&self, route: &str, json: &F) -> OpenAIResponse<()>` -- the public wrapper, same shape as `post<T, F>` but for endpoints with no response body.

### New request types (`interfaces/realtime.rs`)

- `CreateCallRequest { sdp: String, session: Option<RealtimeSessionConfig> }` -- NOT `Serialize`/`Deserialize` (consumed directly by a dedicated multipart-form builder, matching `RealtimeConnectRequest`'s existing precedent of a plain `Builder`-only request type for a non-JSON transport).
- `ReferCallRequest { target_uri: String }`.
- `RejectCallRequest { status_code: Option<i32> }` -- confirmed via source: defaults server-side to `603` (Decline) when omitted.
- `RealtimeCallSipHeader { name: String, value: String }` and `RealtimeCallIncomingEventData { call_id: String, sip_headers: Vec<RealtimeCallSipHeader> }` -- the convenience type described in Context, for deserializing `WebhookEvent.data` when `kind == "realtime.call.incoming"`. Doc comment cross-references `crate::interfaces::webhooks::WebhookEvent` explicitly so this connection isn't undiscoverable.
- **`accept` takes `&RealtimeSessionConfig` directly, no wrapper type** -- confirmed via source the real request body for `POST /realtime/calls/{call_id}/accept` is field-for-field the same `RealtimeSessionCreateRequestParam` shape already modeled as `RealtimeSessionConfig` (used today for `session.update` and `session.created`/`.updated`). Same type, three uses.

### New `apis::realtime::calls` module

Following this crate's established plural/singular accessor convention:

- `client.realtime().calls()` -> `RealtimeCalls<'a>` (collection-level):
  - `create(&self, req: &CreateCallRequest) -> OpenAIResponse<String>` -- `POST /realtime/calls`, multipart (`sdp` as an `application/sdp`-typed text part, `session` as an `application/json`-typed text part when present -- confirmed via source's `encode_multipart` call which sets exactly these two content-types), response is the raw SDP answer as text via the crate's existing `post_form_with_text_response` (already returns `String`, no client.rs changes needed for this method). WebRTC-specific -- the app is the one bootstrapping a new call and needs a server-side relay for the SDP exchange (browsers can't set an `Authorization` header directly against OpenAI). Distinct from the SIP flow below, but the same REST resource; implemented for completeness rather than split into a separate phase, since the added complexity (typed multipart parts, raw-text response) is small and already has direct precedent elsewhere in this crate (Skills' multipart, Container Files' raw-bytes fetch).
- `client.realtime().call(call_id)` -> `RealtimeCall<'a>` (ID-scoped, SIP-focused):
  - `accept(&self, session: &RealtimeSessionConfig) -> OpenAIResponse<()>` -- `POST /realtime/calls/{call_id}/accept`.
  - `hangup(&self) -> OpenAIResponse<()>` -- `POST /realtime/calls/{call_id}/hangup`, no body (matches this crate's existing body-less-action convention of sending `&()`, e.g. `Batches::cancel`).
  - `refer(&self, req: &ReferCallRequest) -> OpenAIResponse<()>` -- `POST /realtime/calls/{call_id}/refer`.
  - `reject(&self, req: &RejectCallRequest) -> OpenAIResponse<()>` -- `POST /realtime/calls/{call_id}/reject`.

## Non-goals

- Setting a custom `Accept` header (`application/sdp`, `*/*`) on any of these requests -- confirmed these are content-negotiation hints, not functional requirements (unlike ChatKit's `OpenAI-Beta` header, which the server actually requires); the server returns the documented content type regardless.
- Any change to `interfaces/webhooks.rs`/`apis/webhooks.rs` -- the existing generic envelope already receives `realtime.call.incoming` with zero code changes; only a convenience type for its `data` field is added, and it lives in `interfaces/realtime.rs`.
- `list`/`retrieve` on calls -- confirmed via source these don't exist on the real API at all, not a deferred gap.
- WebRTC session lifecycle beyond `create`'s SDP relay (ICE, media negotiation, etc.) -- entirely out of scope for a server-side binding crate, same reasoning already established in Phase A for choosing WebSocket over WebRTC as this crate's connection transport.

## Testing

First Realtime slice testable with this crate's standard `wiremock` HTTP-mock discipline (Phases A-C needed the WebSocket-specific local-test-server harness instead). Standard request/response round-trip tests for `accept`/`hangup`/`refer`/`reject` (asserting the request body and a 204 empty-body mock response resolves to `Ok(())`), plus one test for `create`'s multipart body and raw-text response. One dedicated test for `resolve_no_content_response`/`post_no_content` against a mock 204 response, and one against a mock error response (confirming the failure path still parses `ApiErrorResponse` correctly, matching `resolve_file_response`'s already-tested error path). One plain `serde_json` round-trip test for `RealtimeCallIncomingEventData`.

## Versioning

Purely additive.
