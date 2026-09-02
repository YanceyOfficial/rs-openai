# Realtime API — Phase E: Ephemeral Client Secrets — Design

Status: Approved
Date: 2026-09-02

## Context

Fifth Realtime slice, following the same condensed spec-then-direct-implementation rhythm as Phases B/C/D. Like Phase D (SIP calls), this slice is **architecturally ordinary for this crate**: confirmed via `gh api` against `openai-python`'s `resources/realtime/client_secrets.py` that `POST /v1/realtime/client_secrets` is plain REST (one POST + JSON body, JSON response) -- it fits the crate's standard `interfaces::*`/`apis::*` + `wiremock` pattern directly, exactly as Phase D's `calls` module already established with `client.post::<T, F>`.

Confirmed via source: `ClientSecrets` has exactly one method, `create`. It mints a short-lived token (the response `value`, a string that looks like `ek_1234`) that a browser / mobile / WebRTC client can use to open a Realtime session without exposing the main API key. A single client secret can start multiple sessions until it expires. No `OpenAI-Beta` header and no ephemeral-token-of-its-own is required -- the endpoint uses the same `Authorization: Bearer <api_key>` every other REST call in this crate already uses (`security={"bearer_auth": True}` in the SDK, no extra headers).

Confirmed via source that both the request `session` and the response `session` are a **union of a `realtime` session object and a `transcription` session object** (`Union[RealtimeSessionCreateRequestParam, RealtimeTranscriptionSessionCreateRequestParam]` on the way in; `Annotated[Union[RealtimeSessionCreateResponse, RealtimeTranscriptionSessionCreateResponse], discriminator="type"]` on the way out). Transcription sessions are Phase F. This slice models the `realtime` side only, by **reusing the crate's existing `RealtimeSessionConfig`** -- the same deliberate reuse the crate already makes for the `session.created` / `session.updated` server events, whose `session` field is really `RealtimeSessionCreateResponse` but is modeled as `RealtimeSessionConfig` today. Phase F widens all three sites (this endpoint plus the two events) to a `#[serde(tag = "type")]` enum in one coherent pass.

## Goals

### New accessor (`apis/realtime/client_secrets.rs`, new file)

Following this crate's established plural-accessor convention (mirrors Phase D's `calls()` / `RealtimeCalls`):

- `client.realtime().client_secrets()` -> `RealtimeClientSecrets<'a>`:
  - `create(&self, req: &CreateClientSecretRequest) -> OpenAIResponse<RealtimeClientSecret>` -- `POST /realtime/client_secrets`, plain JSON via the existing `post<T, F>` (retried client, matching every other JSON POST in the crate; no `client.rs` changes needed).

Wired into `apis/realtime/mod.rs` (`pub mod client_secrets;`, `pub use client_secrets::RealtimeClientSecrets;`, and the `client_secrets()` method on `Realtime<'a>`).

### New request types (`interfaces/realtime.rs`)

- `CreateClientSecretRequest { expires_after: Option<ClientSecretExpiresAfter>, session: Option<RealtimeSessionConfig> }` -- `Builder, Debug, Serialize, Clone`; both fields `#[serde(skip_serializing_if = "Option::is_none")]` so an empty request serializes to `{}` (confirmed valid -- both params are optional, `TypedDict(total=False)`).
- `ClientSecretExpiresAfter { anchor: Option<String>, seconds: Option<i64> }` -- `Builder, Debug, Serialize, Deserialize, Clone`.
  - `anchor`: only `"created_at"` is currently supported; modeled as `Option<String>` (not an enum) to stay forward-compatible and match the crate's existing treatment of `RealtimeSessionConfig.r#type`.
  - `seconds`: number of seconds from the anchor to expiration. Server default 600 (10 minutes); valid range 10..7200 (2 hours) -- not enforced client-side, matching the crate's convention of letting the API validate ranges.

### New response type (`interfaces/realtime.rs`)

- `RealtimeClientSecret { value: String, expires_at: i64, session: RealtimeSessionConfig }` -- `Debug, Deserialize, Clone` (plus `Serialize`, the crate convention for response types so they round-trip in tests). Confirmed via source these are exactly the three fields of `ClientSecretCreateResponse`; it has no top-level `object` discriminator of its own.
  - `value`: the generated `ek_...` client secret.
  - `expires_at`: expiration timestamp for the client secret, unix seconds (`i64`, matching every other timestamp in the crate).

### `RealtimeSessionConfig` gains three additive optional fields

The response `session` is really `RealtimeSessionCreateResponse`, which carries `id` / `object` / `expires_at` that `RealtimeSessionConfig` does not model, so today they would be silently dropped on deserialize. Added as `Option` fields with `#[serde(skip_serializing_if = "Option::is_none")]` so the type round-trips the response losslessly and continues to serialize cleanly as a request body (all three omitted):

- `id: Option<String>` -- session id, looks like `sess_...`.
- `object: Option<String>` -- `"realtime.session"` for realtime sessions.
- `expires_at: Option<i64>` -- session expiration timestamp, unix seconds.

This also retroactively improves the `session.created` / `session.updated` server events, whose `session: RealtimeSessionConfig` field previously discarded the same three values.

## Non-goals

- The `transcription` session variant of `session` (request and response) -- Phase F, which introduces the `realtime | transcription` enum and applies it here and to the two session events together.
- Richer `RealtimeSessionCreateResponse` modeling beyond the three fields above (`tracing` / `reasoning` / `prompt` / `truncation` / `include` are still deferred on `RealtimeSessionConfig` itself, unchanged from Phase A).
- Any WebRTC / SIP connection logic that consumes the minted secret -- out of scope for a server-side binding crate, same boundary Phase A/D already set. The crate mints the secret; the client app uses it.
- A `retrieve` / `list` / `delete` on client secrets -- confirmed via source these do not exist on the real API.

## Testing

Standard `wiremock` HTTP-mock discipline, matching `calls.rs`:

- Full round-trip: `create` with `expires_after` set (`{anchor, seconds}`) and a `session` (`{type: "realtime", model, ...}`) -- asserts the exact request body, and that the mock response parses into `RealtimeClientSecret` with the right `value` / `expires_at` / `session` (including the new `session.id` / `session.expires_at`).
- Minimal request: `create` with an empty `CreateClientSecretRequest` serializes the body as `{}` (no null keys).
- Error path: a 4xx mock response resolves to `OpenAIError::ApiError`, matching `calls.rs`'s existing error-path coverage.
- One plain `serde_json` round-trip test for `RealtimeClientSecret` on its own, exercising the three new `RealtimeSessionConfig` response fields.

## Versioning

Purely additive -- a new accessor, two new request types, one new response type, and three new `Option` fields on an existing type (each omitted when serializing, so no wire-format change to existing request bodies).
