# Realtime API — Phase G: Auto-Reconnect — Design

Status: Approved
Date: 2026-09-02

## Context

Seventh and final Realtime slice from Phase A's roadmap. Adds **opt-in transparent reconnection** to the WebSocket session: when a live connection drops with a recoverable close code, `RealtimeConnection` silently re-establishes it with exponential backoff and the caller's `recv()` loop continues uninterrupted.

Researched via `gh api` against `openai-python`'s `resources/realtime/realtime.py` and `types/websocket_reconnection.py` (fetched and read directly). The SDK's design:

- **Opt-in** via an `on_reconnecting` callback; the reconnect path is dormant otherwise.
- Triggers only *during iteration*, when `recv()` raises `ConnectionClosedError` whose close code is **recoverable**: `1001, 1005, 1006, 1011, 1012, 1013, 1015` (confirmed via `_RECOVERABLE_CLOSE_CODES`).
- Backoff `min(initial_delay * 2^(attempt-1), max_delay)` multiplied by a `0.75..1.0` jitter factor, up to `max_retries` (default 5). Sleeps before each attempt, attempt 1 included.
- `on_reconnecting(ReconnectingEvent)` returns `ReconnectingOverrides { extra_query?, extra_headers?, abort? }` before each attempt.
- A byte-bounded `SendQueue` (default 1 MiB): while reconnecting, `send()` enqueues; the queue is flushed after a successful reconnect and, on give-up, drained into a `WebSocketConnectionClosedError`.
- Does **not** resume session state -- the app re-sends `session.update` itself.

**What maps to this crate, and what does not.** `RealtimeConnection`'s `send` / `recv` / `close` all take `&mut self`, so send and recv cannot run concurrently -- which removes the entire reason `SendQueue` exists. A reconnect runs synchronously inside one `recv().await`; by the time the caller's next `send()` runs, the reconnect has already resolved (succeeded or exhausted). And the Realtime server emits `session.created` as the first event on *every* new connection (confirmed, Phase A) -- so after a transparent reconnect the caller's `recv()` loop naturally receives a fresh `session.created`, which is their signal to re-apply `session.update`. No callback is needed to carry that signal.

Per the approved scope: **minimal transparent reconnect** -- no `on_reconnecting` callback, no send queue, no `ReconnectingOverrides`. Those are deferred as genuinely unnecessary for a server-side binding using a long-lived API key, in the same spirit as every prior phase's deferrals.

## Goals

### New config type (`interfaces/realtime.rs`)

```rust
#[derive(Builder, Debug, Clone)]
pub struct RealtimeReconnectConfig {
    pub max_retries: u32,       // default 5
    pub initial_delay_ms: u64,  // default 500
    pub max_delay_ms: u64,      // default 8000
}
```

`Builder` with `#[builder(default = ...)]` on each field so `RealtimeReconnectConfig::builder().build()` yields the SDK defaults. Also a manual `Default` impl (or `#[derive(Default)]` won't give non-zero defaults -- use an explicit `impl Default`).

### `RealtimeConnectRequest` gains one opt-in field (`interfaces/realtime.rs`)

- `reconnect: Option<RealtimeReconnectConfig>` -- when `None` (the default), behavior is byte-identical to today: a dropped connection ends the stream exactly as it does now.

### `RealtimeConnection` reconnect state (`apis/realtime/connection.rs`)

`RealtimeConnection` currently discards `base_url` / `api_key` / `model` after connecting. It gains:

- `base_url: String`, `api_key: String`, `model: Option<String>` -- owned copies, needed to re-establish.
- `reconnect: Option<RealtimeReconnectConfig>`.
- `intentionally_closed: bool` -- set by `close()`; a user-initiated close never triggers reconnect.
- `reconnect_fut: Option<Pin<Box<dyn Future<Output = OpenAIResponse<WsStream>> + Send>>>` -- the in-progress reconnect, owning cloned inputs so it is `'static`.

`connect()`'s internal WebSocket setup is extracted into a private `async fn establish_ws(base_url: &str, api_key: &str, model: Option<&str>) -> OpenAIResponse<WsStream>` reused by both `connect()` and the reconnect loop.

### Reconnect loop (`apis/realtime/connection.rs`)

A private `async fn reconnect_loop(base_url: String, api_key: String, model: Option<String>, cfg: RealtimeReconnectConfig) -> OpenAIResponse<WsStream>`:

- `for attempt in 1..=cfg.max_retries`: compute `base = min(initial_delay_ms as f64 * 2^(attempt-1), max_delay_ms as f64)`, `jitter = 0.75 + rand::random::<f64>() * 0.25`, sleep `(base * jitter) as u64` ms via `tokio::time::sleep`, then `establish_ws(...)`. Return `Ok(ws)` on the first success; keep the last `Err`.
- After all attempts fail: `Err(OpenAIError::WebSocket(format!("reconnect failed after {max_retries} attempts: {last_err}")))`.

`rand` is already a crate dependency (`rand = "0.8.5"`). `tokio`'s `time` feature is already available (`features = ["full"]`).

### `poll_next` state machine (`apis/realtime/connection.rs`)

Replaces the current body:

1. If `reconnect_fut` is `Some`, poll it. `Pending` -> `Poll::Pending`. `Ready(Ok(ws))` -> `self.ws = ws`, `self.reconnect_fut = None`, `continue` (the next poll of the fresh socket yields the server's `session.created`). `Ready(Err(e))` -> `self.reconnect_fut = None`, return `Poll::Ready(Some(Err(e)))`, and the poll after that returns `Poll::Ready(None)`.
2. Otherwise poll `self.ws` as today, with two arms changed:
   - `Message::Text` / ping-pong / etc. -- unchanged.
   - `Message::Close(frame)` -- extract the code (`frame.map(|f| u16::from(f.code)).unwrap_or(1006)`, matching the SDK's default when no code is received); if `should_reconnect(code)` start `reconnect_fut` and `continue`, else `Poll::Ready(None)` as today.
   - `Poll::Ready(Some(Err(_)))` (transport error, e.g. abrupt network drop) -- treat as code `1006`; if `should_reconnect(1006)` start `reconnect_fut` and `continue`, else return the error as today.
   - `Poll::Ready(None)` (stream ended, no close frame) -- treat as `1006`, same handling.

`should_reconnect(code)` = `self.reconnect.is_some() && !self.intentionally_closed && RECOVERABLE_CLOSE_CODES.contains(&code)`, where `RECOVERABLE_CLOSE_CODES: [u16; 7] = [1001, 1005, 1006, 1011, 1012, 1013, 1015]`.

Once `reconnect_fut` yields a terminal `Err`, a `reconnect_exhausted: bool` latch is set so the subsequent poll returns `None` rather than re-entering the reconnect path.

### `close()` (`apis/realtime/connection.rs`)

Sets `self.intentionally_closed = true` before closing the socket, so an in-flight or subsequent recoverable close is not reconnected. Once `intentionally_closed`, a trailing transport error from the not-quite-completed close handshake is swallowed -- `poll_next` returns `None`, not `Err` -- since the caller asked to close.

### `apis/realtime/mod.rs`

`Realtime::connect` passes `req.reconnect.clone()` through to `RealtimeConnection::connect`, whose signature gains a `reconnect: Option<RealtimeReconnectConfig>` parameter.

## Non-goals

- An `on_reconnecting` callback and `ReconnectingOverrides` (per-attempt query/header mutation, programmatic abort) -- deferred; `max_retries` bounds the attempts, and credential refresh is not a concern for a long-lived API key.
- A send queue / `WebSocketConnectionClosedError` with unsent messages -- unnecessary given the `&mut self` API serializes send and recv (see Context).
- Automatic session-state resumption (replaying `session.update`, restoring conversation items) -- the caller does this on seeing the post-reconnect `session.created`, exactly as with the Python SDK.
- Reconnection for the plain-REST `calls` / `client_secrets` resources -- those are one-shot HTTP requests already covered by the crate's existing retry middleware.

## Testing

Uses Phase A's existing local-WebSocket harness (`spawn_test_server`), extended to accept more than one connection:

- **Transparent reconnect:** server accepts connection 1, sends an event, closes with code `1012`; accepts connection 2, sends `session.created`. Assert the caller's stream yields the pre-drop event, then `session.created`, with no error surfaced and no gap.
- **Non-recoverable close:** server closes connection 1 with `1008` -> stream ends (`None`), no second connection attempted.
- **Retry exhaustion:** `max_retries = 2`, server accepts connection 1 then closes `1011` and refuses all further connections -> stream yields one `Err(OpenAIError::WebSocket(...))` then `None`; total wall time bounded by the two short backoff sleeps (test uses `initial_delay_ms = 10`, `max_delay_ms = 40`).
- **Intentional close:** caller calls `close()`; a subsequent poll returns `None` with no reconnect attempt.
- **Opt-out unchanged:** with `reconnect: None`, a `1012` close ends the stream exactly as the existing `recv_returns_none_after_clean_close` test expects.

## Versioning

Purely additive -- a new opt-in `RealtimeConnectRequest` field, a new config struct, new private `RealtimeConnection` fields, and one new parameter on the `pub(crate)` `RealtimeConnection::connect`. With `reconnect` unset, every existing behavior is byte-identical.
