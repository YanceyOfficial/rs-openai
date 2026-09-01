//! The live WebSocket connection to the Realtime API -- see
//! docs/superpowers/specs/2026-09-01-realtime-phase-a-design.md and
//! docs/superpowers/specs/2026-09-02-realtime-phase-b-audio-design.md.

use crate::interfaces::realtime::{RealtimeClientEvent, RealtimeServerEvent};
use crate::shared::response_wrapper::{OpenAIError, OpenAIResponse};
use futures::{SinkExt, Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// A live WebSocket connection to the Realtime API. Obtained via `client.realtime().connect()`.
pub struct RealtimeConnection {
    ws: WsStream,
}

impl RealtimeConnection {
    pub(crate) async fn connect(
        base_url: &str,
        api_key: &str,
        model: Option<&str>,
    ) -> OpenAIResponse<Self> {
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

        Ok(Self { ws })
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
    pub async fn recv(&mut self) -> Option<OpenAIResponse<RealtimeServerEvent>> {
        self.next().await
    }

    /// Closes the connection.
    pub async fn close(&mut self) -> OpenAIResponse<()> {
        self.ws
            .close(None)
            .await
            .map_err(|e| OpenAIError::WebSocket(e.to_string()))
    }
}

impl Stream for RealtimeConnection {
    type Item = OpenAIResponse<RealtimeServerEvent>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            return match Pin::new(&mut self.ws).poll_next(cx) {
                Poll::Ready(None) => Poll::Ready(None),
                Poll::Ready(Some(Err(e))) => {
                    Poll::Ready(Some(Err(OpenAIError::WebSocket(e.to_string()))))
                }
                Poll::Ready(Some(Ok(Message::Text(text)))) => Poll::Ready(Some(
                    serde_json::from_str::<RealtimeServerEvent>(&text)
                        .map_err(OpenAIError::JSONDeserialize),
                )),
                Poll::Ready(Some(Ok(Message::Close(_)))) => Poll::Ready(None),
                // Ping/Pong/Binary/Frame carry no server event; keep polling.
                Poll::Ready(Some(Ok(_))) => continue,
                Poll::Pending => Poll::Pending,
            };
        }
    }
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
    use crate::interfaces::realtime::{RealtimeClientEvent, RealtimeConnectRequest};
    use futures::{SinkExt, StreamExt};
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
}
