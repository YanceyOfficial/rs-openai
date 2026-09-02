//! The live WebSocket connection to the Realtime API -- see
//! docs/superpowers/specs/2026-09-01-realtime-phase-a-design.md,
//! docs/superpowers/specs/2026-09-02-realtime-phase-b-audio-design.md, and
//! docs/superpowers/specs/2026-09-02-realtime-phase-g-auto-reconnect-design.md.

use crate::interfaces::realtime::{
    RealtimeClientEvent, RealtimeReconnectConfig, RealtimeServerEvent,
};
use crate::shared::response_wrapper::{OpenAIError, OpenAIResponse};
use futures::{SinkExt, Stream, StreamExt};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;
type ReconnectFut = Pin<Box<dyn Future<Output = OpenAIResponse<WsStream>> + Send>>;

/// WebSocket close codes worth retrying (RFC 6455 section 7.4.1), matching `openai-python`'s
/// `_RECOVERABLE_CLOSE_CODES`.
const RECOVERABLE_CLOSE_CODES: [u16; 7] = [1001, 1005, 1006, 1011, 1012, 1013, 1015];

/// A live WebSocket connection to the Realtime API. Obtained via `client.realtime().connect()`.
pub struct RealtimeConnection {
    ws: WsStream,
    /// Kept so a dropped connection can be re-established -- see the module design spec.
    base_url: String,
    api_key: String,
    model: Option<String>,
    reconnect: Option<RealtimeReconnectConfig>,
    intentionally_closed: bool,
    reconnect_exhausted: bool,
    reconnect_fut: Option<ReconnectFut>,
}

impl RealtimeConnection {
    pub(crate) async fn connect(
        base_url: &str,
        api_key: &str,
        model: Option<&str>,
        reconnect: Option<RealtimeReconnectConfig>,
    ) -> OpenAIResponse<Self> {
        let ws = establish_ws(base_url, api_key, model).await?;
        Ok(Self {
            ws,
            base_url: base_url.to_string(),
            api_key: api_key.to_string(),
            model: model.map(str::to_string),
            reconnect,
            intentionally_closed: false,
            reconnect_exhausted: false,
            reconnect_fut: None,
        })
    }

    /// Sends a single client event.
    pub async fn send(&mut self, event: &RealtimeClientEvent) -> OpenAIResponse<()> {
        let text = serde_json::to_string(event).map_err(OpenAIError::JSONDeserialize)?;
        self.ws
            .send(Message::Text(text))
            .await
            .map_err(|e| OpenAIError::WebSocket(e.to_string()))
    }

    /// Receives and parses the next server event. Returns `None` once the connection has
    /// closed cleanly -- mirrors `Stream::poll_next`'s `Option` wrapping (this method is
    /// implemented in terms of the `Stream` impl below, not a separate polling loop).
    ///
    /// When `reconnect` was configured on the [`crate::interfaces::realtime::RealtimeConnectRequest`],
    /// a drop with a recoverable close code is handled transparently here: the caller sees the
    /// stream continue, with the server's fresh `session.created` as the first event after
    /// reconnection.
    pub async fn recv(&mut self) -> Option<OpenAIResponse<RealtimeServerEvent>> {
        self.next().await
    }

    /// Closes the connection. A connection closed this way is never reconnected.
    pub async fn close(&mut self) -> OpenAIResponse<()> {
        self.intentionally_closed = true;
        self.ws
            .close(None)
            .await
            .map_err(|e| OpenAIError::WebSocket(e.to_string()))
    }

    /// If `reconnect` is configured, the connection was not closed intentionally, and `code`
    /// is recoverable, kicks off a reconnect future and returns `true`. Otherwise `false` --
    /// the caller should end (or error) the stream.
    fn start_reconnect_if_recoverable(&mut self, code: u16) -> bool {
        let Some(cfg) = self.reconnect.clone() else {
            return false;
        };
        if self.intentionally_closed || !RECOVERABLE_CLOSE_CODES.contains(&code) {
            return false;
        }
        self.reconnect_fut = Some(Box::pin(reconnect_loop(
            self.base_url.clone(),
            self.api_key.clone(),
            self.model.clone(),
            cfg,
        )));
        true
    }
}

impl Stream for RealtimeConnection {
    type Item = OpenAIResponse<RealtimeServerEvent>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            // 1. Drive an in-progress reconnect to completion before touching the socket.
            if self.reconnect_fut.is_some() {
                let poll = self
                    .reconnect_fut
                    .as_mut()
                    .expect("just checked is_some")
                    .as_mut()
                    .poll(cx);
                match poll {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(Ok(ws)) => {
                        self.ws = ws;
                        self.reconnect_fut = None;
                        // Loop again: the fresh socket's first event is `session.created`.
                        continue;
                    }
                    Poll::Ready(Err(e)) => {
                        self.reconnect_fut = None;
                        self.reconnect_exhausted = true;
                        return Poll::Ready(Some(Err(e)));
                    }
                }
            }
            if self.reconnect_exhausted {
                return Poll::Ready(None);
            }

            // 2. Poll the live socket.
            match Pin::new(&mut self.ws).poll_next(cx) {
                Poll::Ready(None) => {
                    if self.start_reconnect_if_recoverable(1006) {
                        continue;
                    }
                    return Poll::Ready(None);
                }
                Poll::Ready(Some(Err(e))) => {
                    if self.start_reconnect_if_recoverable(1006) {
                        continue;
                    }
                    // A socket that errors after the caller asked to close is just the close
                    // handshake not completing cleanly -- end the stream, don't surface it.
                    if self.intentionally_closed {
                        return Poll::Ready(None);
                    }
                    return Poll::Ready(Some(Err(OpenAIError::WebSocket(e.to_string()))));
                }
                Poll::Ready(Some(Ok(Message::Text(text)))) => {
                    return Poll::Ready(Some(
                        serde_json::from_str::<RealtimeServerEvent>(&text)
                            .map_err(OpenAIError::JSONDeserialize),
                    ));
                }
                Poll::Ready(Some(Ok(Message::Close(frame)))) => {
                    let code = frame.map(|f| u16::from(f.code)).unwrap_or(1006);
                    if self.start_reconnect_if_recoverable(code) {
                        continue;
                    }
                    return Poll::Ready(None);
                }
                // Ping/Pong/Binary/Frame carry no server event; keep polling.
                Poll::Ready(Some(Ok(_))) => continue,
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

/// Opens one WebSocket to the Realtime API with the `Authorization` header set.
async fn establish_ws(
    base_url: &str,
    api_key: &str,
    model: Option<&str>,
) -> OpenAIResponse<WsStream> {
    let url = build_ws_url(base_url, model);
    let mut request = url
        .into_client_request()
        .map_err(|e| OpenAIError::WebSocket(e.to_string()))?;
    let auth_value = HeaderValue::from_str(&format!("Bearer {api_key}"))
        .map_err(|e| OpenAIError::InvalidHeaderValue(e.to_string()))?;
    request.headers_mut().insert("Authorization", auth_value);

    let (ws, _response) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(|e| OpenAIError::WebSocket(e.to_string()))?;
    Ok(ws)
}

/// Retries `establish_ws` up to `cfg.max_retries` times with exponential backoff plus jitter,
/// sleeping before each attempt (attempt 1 included, matching `openai-python`).
async fn reconnect_loop(
    base_url: String,
    api_key: String,
    model: Option<String>,
    cfg: RealtimeReconnectConfig,
) -> OpenAIResponse<WsStream> {
    let mut last_err: Option<OpenAIError> = None;
    for attempt in 1..=cfg.max_retries {
        let factor = 2u64.saturating_pow(attempt - 1);
        let base = cfg
            .initial_delay_ms
            .saturating_mul(factor)
            .min(cfg.max_delay_ms) as f64;
        let jitter = 0.75 + rand::random::<f64>() * 0.25;
        tokio::time::sleep(Duration::from_millis((base * jitter) as u64)).await;

        match establish_ws(&base_url, &api_key, model.as_deref()).await {
            Ok(ws) => return Ok(ws),
            Err(e) => last_err = Some(e),
        }
    }
    Err(OpenAIError::WebSocket(format!(
        "reconnect failed after {} attempt(s){}",
        cfg.max_retries,
        last_err.map(|e| format!(": {e}")).unwrap_or_default()
    )))
}

/// Swaps `http`/`https` for `ws`/`wss` on the same base URL every other request already
/// uses (including in tests, which already override this to point at a mock server) and
/// appends `/realtime` plus `?model=...` if provided.
fn build_ws_url(base_url: &str, model: Option<&str>) -> String {
    let trimmed = base_url.trim_end_matches('/');
    let mut url = if let Some(rest) = trimmed.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = trimmed.strip_prefix("http://") {
        format!("ws://{rest}")
    } else {
        trimmed.to_string()
    };
    url.push_str("/realtime");
    if let Some(model) = model {
        url.push_str("?model=");
        url.push_str(model);
    }
    url
}

#[cfg(test)]
mod tests {
    use super::build_ws_url;
    use crate::client::OpenAI;
    use crate::interfaces::realtime::{
        RealtimeClientEvent, RealtimeConnectRequest, RealtimeServerEvent,
    };
    use crate::shared::response_wrapper::OpenAIError;
    use futures::{SinkExt, StreamExt};
    use std::future::Future;
    use std::pin::Pin;
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::Message;

    #[test]
    fn build_ws_url_swaps_https_and_appends_model() {
        let url = build_ws_url("https://api.openai.com/v1", Some("gpt-realtime-2.1"));
        assert_eq!(
            url,
            "wss://api.openai.com/v1/realtime?model=gpt-realtime-2.1"
        );
    }

    #[test]
    fn build_ws_url_swaps_http_for_local_test_servers() {
        let url = build_ws_url("http://127.0.0.1:4000", None);
        assert_eq!(url, "ws://127.0.0.1:4000/realtime");
    }

    /// Starts a local WS server for exactly one connection. `on_request` inspects the
    /// handshake request (used to assert the `Authorization` header); `on_accept` runs
    /// against the accepted server-side WebSocket stream once the handshake completes.
    async fn spawn_test_server<F>(
        on_request_header: std::sync::mpsc::Sender<Option<String>>,
        on_accept: F,
    ) -> String
    where
        F: FnOnce(
                tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
            ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
            + Send
            + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            // The callback's `Result` error type is a large struct owned by
            // `tokio-tungstenite` itself (this closure never actually returns `Err`).
            #[allow(clippy::result_large_err)]
            let callback =
                move |req: &tokio_tungstenite::tungstenite::handshake::server::Request, resp| {
                    let auth = req
                        .headers()
                        .get("Authorization")
                        .and_then(|v| v.to_str().ok())
                        .map(str::to_string);
                    let _ = on_request_header.send(auth);
                    Ok(resp)
                };
            let ws = tokio_tungstenite::accept_hdr_async(stream, callback)
                .await
                .unwrap();
            on_accept(ws).await;
        });

        format!("http://{addr}")
    }

    #[tokio::test]
    async fn connect_sends_authorization_bearer_header() {
        let (tx, rx) = std::sync::mpsc::channel();
        let base_url = spawn_test_server(tx, |mut ws| {
            Box::pin(async move {
                // Keep the connection open just long enough for the client to observe a
                // successful handshake, then close cleanly.
                let _ = ws.close(None).await;
            })
        })
        .await;

        let client = OpenAI::new("sk-test-realtime").with_base_url(base_url);
        let req = RealtimeConnectRequest::builder().build();
        let mut conn = client.realtime().connect(&req).await.unwrap();

        // Drain until the server's close arrives, confirming the handshake succeeded.
        let _ = conn.recv().await;

        let received_auth = rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        assert_eq!(received_auth.as_deref(), Some("Bearer sk-test-realtime"));
    }

    #[tokio::test]
    async fn send_transmits_bytes_the_server_receives() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let (msg_tx, msg_rx) = tokio::sync::oneshot::channel();
        let base_url = spawn_test_server(tx, move |mut ws| {
            Box::pin(async move {
                if let Some(Ok(Message::Text(text))) = ws.next().await {
                    let _ = msg_tx.send(text.to_string());
                }
            })
        })
        .await;

        let client = OpenAI::new("sk-test").with_base_url(base_url);
        let req = RealtimeConnectRequest::builder().build();
        let mut conn = client.realtime().connect(&req).await.unwrap();

        conn.send(&RealtimeClientEvent::ResponseCancel {
            event_id: None,
            response_id: Some("resp_42".to_string()),
        })
        .await
        .unwrap();

        let received = tokio::time::timeout(std::time::Duration::from_secs(5), msg_rx)
            .await
            .unwrap()
            .unwrap();
        let value: serde_json::Value = serde_json::from_str(&received).unwrap();
        assert_eq!(value["type"], "response.cancel");
        assert_eq!(value["response_id"], "resp_42");
    }

    #[tokio::test]
    async fn recv_parses_a_server_sent_event() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let base_url = spawn_test_server(tx, |mut ws| {
            Box::pin(async move {
                let payload = serde_json::json!({
                    "type": "session.created",
                    "event_id": "evt_1",
                    "session": {"type": "realtime", "model": "gpt-realtime-2.1"}
                })
                .to_string();
                let _ = ws.send(Message::Text(payload)).await;
                let _ = ws.close(None).await;
            })
        })
        .await;

        let client = OpenAI::new("sk-test").with_base_url(base_url);
        let req = RealtimeConnectRequest::builder().build();
        let mut conn = client.realtime().connect(&req).await.unwrap();

        let event = conn.recv().await.unwrap().unwrap();
        match event {
            crate::interfaces::realtime::RealtimeServerEvent::SessionCreated {
                event_id, ..
            } => assert_eq!(event_id, "evt_1"),
            other => panic!("expected SessionCreated, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn recv_returns_none_after_clean_close() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let base_url = spawn_test_server(tx, |mut ws| {
            Box::pin(async move {
                let _ = ws.close(None).await;
            })
        })
        .await;

        let client = OpenAI::new("sk-test").with_base_url(base_url);
        let req = RealtimeConnectRequest::builder().build();
        let mut conn = client.realtime().connect(&req).await.unwrap();

        let event = conn.recv().await;
        assert!(
            event.is_none(),
            "expected None after clean close, got {event:?}"
        );
    }

    #[tokio::test]
    async fn send_transmits_input_audio_buffer_append_over_real_socket() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let (msg_tx, msg_rx) = tokio::sync::oneshot::channel();
        let base_url = spawn_test_server(tx, move |mut ws| {
            Box::pin(async move {
                if let Some(Ok(Message::Text(text))) = ws.next().await {
                    let _ = msg_tx.send(text.to_string());
                }
            })
        })
        .await;

        let client = OpenAI::new("sk-test").with_base_url(base_url);
        let req = RealtimeConnectRequest::builder().build();
        let mut conn = client.realtime().connect(&req).await.unwrap();

        conn.send(&RealtimeClientEvent::InputAudioBufferAppend {
            event_id: None,
            audio: "YmFzZTY0YXVkaW9jaHVuaw==".to_string(),
        })
        .await
        .unwrap();

        let received = tokio::time::timeout(std::time::Duration::from_secs(5), msg_rx)
            .await
            .unwrap()
            .unwrap();
        let value: serde_json::Value = serde_json::from_str(&received).unwrap();
        assert_eq!(value["type"], "input_audio_buffer.append");
        assert_eq!(value["audio"], "YmFzZTY0YXVkaW9jaHVuaw==");
    }

    // ---- Phase G: auto-reconnect ----

    use crate::interfaces::realtime::RealtimeReconnectConfig;
    use tokio_tungstenite::tungstenite::protocol::CloseFrame;
    use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

    type ConnHandler =
        Box<dyn FnOnce(WsServerStream) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send>;
    type WsServerStream = tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>;

    /// Serves the given per-connection handlers in sequence, then drops the listener so any
    /// further connection attempt is refused. Returns the `http://` base URL.
    async fn spawn_seq_ws_server(handlers: Vec<ConnHandler>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            for handler in handlers {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                let Ok(ws) = tokio_tungstenite::accept_async(stream).await else {
                    continue;
                };
                handler(ws).await;
            }
        });
        format!("http://{addr}")
    }

    fn send_event_then_close(payload: serde_json::Value, code: u16) -> ConnHandler {
        Box::new(move |mut ws| {
            Box::pin(async move {
                let _ = ws.send(Message::Text(payload.to_string())).await;
                let _ = ws
                    .send(Message::Close(Some(CloseFrame {
                        code: CloseCode::from(code),
                        reason: "".into(),
                    })))
                    .await;
            })
        })
    }

    fn session_created_json(event_id: &str) -> serde_json::Value {
        serde_json::json!({
            "type": "session.created",
            "event_id": event_id,
            "session": {"type": "realtime", "model": "gpt-realtime-2.1"}
        })
    }

    fn rate_limits_json(event_id: &str) -> serde_json::Value {
        serde_json::json!({
            "type": "rate_limits.updated",
            "event_id": event_id,
            "rate_limits": [{"name": "requests", "limit": 100, "remaining": 99}]
        })
    }

    #[tokio::test]
    async fn transparent_reconnect_after_recoverable_close() {
        let base_url = spawn_seq_ws_server(vec![
            send_event_then_close(rate_limits_json("evt_before"), 1012),
            Box::new(|mut ws| {
                Box::pin(async move {
                    let _ = ws
                        .send(Message::Text(session_created_json("evt_after").to_string()))
                        .await;
                    // Keep the socket open so the client can keep reading.
                    let _ = ws.next().await;
                })
            }),
        ])
        .await;

        let client = OpenAI::new("sk-test").with_base_url(base_url);
        let req = RealtimeConnectRequest::builder()
            .reconnect(
                RealtimeReconnectConfig::builder()
                    .initial_delay_ms(10)
                    .max_delay_ms(40)
                    .build(),
            )
            .build();
        let mut conn = client.realtime().connect(&req).await.unwrap();

        let before = conn.recv().await.unwrap().unwrap();
        assert!(
            matches!(before, RealtimeServerEvent::RateLimitsUpdated { .. }),
            "expected pre-drop event, got {before:?}"
        );

        let after = tokio::time::timeout(std::time::Duration::from_secs(5), conn.recv())
            .await
            .expect("reconnect timed out")
            .unwrap()
            .unwrap();
        assert!(
            matches!(after, RealtimeServerEvent::SessionCreated { .. }),
            "expected post-reconnect session.created, got {after:?}"
        );
    }

    #[tokio::test]
    async fn non_recoverable_close_ends_stream_without_reconnect() {
        let base_url =
            spawn_seq_ws_server(vec![send_event_then_close(rate_limits_json("evt_1"), 1008)]).await;

        let client = OpenAI::new("sk-test").with_base_url(base_url);
        let req = RealtimeConnectRequest::builder()
            .reconnect(
                RealtimeReconnectConfig::builder()
                    .initial_delay_ms(10)
                    .build(),
            )
            .build();
        let mut conn = client.realtime().connect(&req).await.unwrap();

        let _ = conn.recv().await.unwrap().unwrap();
        assert!(conn.recv().await.is_none(), "1008 is not recoverable");
    }

    #[tokio::test]
    async fn reconnect_exhaustion_yields_error_then_none() {
        let base_url =
            spawn_seq_ws_server(vec![send_event_then_close(rate_limits_json("evt_1"), 1011)]).await;

        let client = OpenAI::new("sk-test").with_base_url(base_url);
        let req = RealtimeConnectRequest::builder()
            .reconnect(
                RealtimeReconnectConfig::builder()
                    .max_retries(2)
                    .initial_delay_ms(10)
                    .max_delay_ms(20)
                    .build(),
            )
            .build();
        let mut conn = client.realtime().connect(&req).await.unwrap();

        let _ = conn.recv().await.unwrap().unwrap();
        let err = tokio::time::timeout(std::time::Duration::from_secs(5), conn.recv())
            .await
            .expect("exhaustion timed out")
            .unwrap()
            .unwrap_err();
        assert!(matches!(err, OpenAIError::WebSocket(_)), "got {err:?}");
        assert!(conn.recv().await.is_none());
    }

    #[tokio::test]
    async fn intentional_close_does_not_reconnect() {
        let base_url = spawn_seq_ws_server(vec![Box::new(|mut ws| {
            Box::pin(async move {
                let _ = ws.next().await;
            })
        })])
        .await;

        let client = OpenAI::new("sk-test").with_base_url(base_url);
        let req = RealtimeConnectRequest::builder()
            .reconnect(
                RealtimeReconnectConfig::builder()
                    .initial_delay_ms(10)
                    .build(),
            )
            .build();
        let mut conn = client.realtime().connect(&req).await.unwrap();

        conn.close().await.unwrap();
        assert!(conn.recv().await.is_none());
    }

    #[tokio::test]
    async fn recoverable_close_ends_stream_when_reconnect_not_configured() {
        let base_url =
            spawn_seq_ws_server(vec![send_event_then_close(rate_limits_json("evt_1"), 1012)]).await;

        let client = OpenAI::new("sk-test").with_base_url(base_url);
        let req = RealtimeConnectRequest::builder().build();
        let mut conn = client.realtime().connect(&req).await.unwrap();

        let _ = conn.recv().await.unwrap().unwrap();
        assert!(
            conn.recv().await.is_none(),
            "no reconnect config -> stream just ends"
        );
    }
}
