//! Realtime `client_secrets` -- see
//! docs/superpowers/specs/2026-09-02-realtime-phase-e-client-secrets-design.md. Plain REST,
//! like `super::calls` -- mints a short-lived `ek_...` token a browser / mobile / WebRTC
//! client can use to open a Realtime session without the main API key.

use crate::client::OpenAI;
use crate::interfaces::realtime;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct RealtimeClientSecrets<'a> {
    openai: &'a OpenAI,
}

impl<'a> RealtimeClientSecrets<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Mints a Realtime client secret with an associated session configuration. A single
    /// secret can start multiple sessions until it expires.
    pub async fn create(
        &self,
        req: &realtime::CreateClientSecretRequest,
    ) -> OpenAIResponse<realtime::RealtimeClientSecret> {
        self.openai.post("/realtime/client_secrets", req).await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::realtime::{
        ClientSecretExpiresAfter, CreateClientSecretRequest, RealtimeClientSecret,
        RealtimeSessionConfig,
    };
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn client_secret_round_trips_with_session_response_fields() {
        let json = serde_json::json!({
            "value": "ek_abc123",
            "expires_at": 1_700_000_600_i64,
            "session": {
                "type": "realtime",
                "id": "sess_abc",
                "object": "realtime.session",
                "model": "gpt-realtime-2.1",
                "expires_at": 1_700_003_600_i64
            }
        });

        let secret: RealtimeClientSecret = serde_json::from_value(json).unwrap();
        assert_eq!(secret.value, "ek_abc123");
        assert_eq!(secret.expires_at, 1_700_000_600);
        assert_eq!(secret.session.id.as_deref(), Some("sess_abc"));
        assert_eq!(secret.session.object.as_deref(), Some("realtime.session"));
        assert_eq!(secret.session.expires_at, Some(1_700_003_600));

        let round_tripped = serde_json::to_value(&secret).unwrap();
        assert_eq!(round_tripped["session"]["id"], "sess_abc");
        assert_eq!(round_tripped["session"]["expires_at"], 1_700_003_600_i64);
    }

    #[tokio::test]
    async fn create_sends_expires_after_and_session_and_parses_response() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/realtime/client_secrets"))
            .and(body_json(serde_json::json!({
                "expires_after": {"anchor": "created_at", "seconds": 3600},
                "session": {"type": "realtime", "model": "gpt-realtime-2.1"}
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "value": "ek_test",
                "expires_at": 1_700_000_600_i64,
                "session": {
                    "type": "realtime",
                    "id": "sess_1",
                    "object": "realtime.session",
                    "model": "gpt-realtime-2.1"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateClientSecretRequest::builder()
            .expires_after(
                ClientSecretExpiresAfter::builder()
                    .anchor("created_at")
                    .seconds(3600)
                    .build(),
            )
            .session(
                RealtimeSessionConfig::builder()
                    .r#type("realtime")
                    .model("gpt-realtime-2.1")
                    .build(),
            )
            .build();

        let secret = client
            .realtime()
            .client_secrets()
            .create(&req)
            .await
            .unwrap();
        assert_eq!(secret.value, "ek_test");
        assert_eq!(secret.expires_at, 1_700_000_600);
        assert_eq!(secret.session.id.as_deref(), Some("sess_1"));
    }

    #[tokio::test]
    async fn create_with_empty_request_sends_empty_body() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/realtime/client_secrets"))
            .and(body_json(serde_json::json!({})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "value": "ek_test",
                "expires_at": 1_700_000_600_i64,
                "session": {"type": "realtime"}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateClientSecretRequest::builder().build();
        client
            .realtime()
            .client_secrets()
            .create(&req)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn create_maps_api_error_on_failure() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/realtime/client_secrets"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "message": "Invalid session configuration",
                    "type": "invalid_request_error",
                    "code": null
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateClientSecretRequest::builder().build();
        let err = client
            .realtime()
            .client_secrets()
            .create(&req)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            crate::shared::response_wrapper::OpenAIError::ApiError(_)
        ));
    }
}
