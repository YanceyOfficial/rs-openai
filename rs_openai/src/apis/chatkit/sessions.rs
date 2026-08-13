//! ChatKit sessions: `POST /chatkit/sessions` create,
//! `POST /chatkit/sessions/{session_id}/cancel` cancel — see
//! docs/superpowers/specs/2026-08-14-chatkit-design.md.

use crate::client::OpenAI;
use crate::interfaces::chatkit;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct ChatKitSessions<'a> {
    openai: &'a OpenAI,
}

impl<'a> ChatKitSessions<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates a ChatKit session, returning a `client_secret` to hand off to the ChatKit
    /// client library.
    pub async fn create(
        &self,
        req: &chatkit::CreateChatKitSessionRequest,
    ) -> OpenAIResponse<chatkit::ChatKitSession> {
        self.openai.chatkit_post("/chatkit/sessions", req).await
    }
}

pub struct ChatKitSession<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> ChatKitSession<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Cancels this ChatKit session.
    pub async fn cancel(&self) -> OpenAIResponse<chatkit::ChatKitSession> {
        self.openai
            .chatkit_post(&format!("/chatkit/sessions/{}/cancel", self.id), &())
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::chatkit::{
        ChatKitConfiguration, ChatKitWorkflowParam, ChatKitWorkflowTracing,
        CreateChatKitSessionRequest,
    };
    use crate::shared::types::{ExpiresAfter, ExpiresAfterAnchor};
    use std::collections::HashMap;
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_sends_beta_header_and_serializes_full_request() {
        let mock_server = MockServer::start().await;

        let mut state_variables = HashMap::new();
        state_variables.insert("tier".to_string(), serde_json::json!("gold"));

        Mock::given(method("POST"))
            .and(path("/chatkit/sessions"))
            .and(header("OpenAI-Beta", "chatkit_beta=v1"))
            .and(body_json(serde_json::json!({
                "workflow": {
                    "id": "wf_123",
                    "state_variables": {"tier": "gold"},
                    "tracing": {"enabled": false},
                    "version": "2"
                },
                "user": "user_abc",
                "chatkit_configuration": {
                    "automatic_thread_titling": {"enabled": true}
                },
                "expires_after": {"anchor": "created_at", "seconds": 3600},
                "rate_limits": {"max_requests_per_1_minute": 20}
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cksess_1",
                "object": "chatkit.session",
                "client_secret": "cksec_abc",
                "expires_at": 1700003600_i64,
                "status": "active",
                "user": "user_abc",
                "workflow": {"id": "wf_123"},
                "chatkit_configuration": {},
                "rate_limits": {"max_requests_per_1_minute": 20},
                "max_requests_per_1_minute": 20
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateChatKitSessionRequest::builder()
            .workflow(
                ChatKitWorkflowParam::builder()
                    .id("wf_123")
                    .state_variables(state_variables)
                    .tracing(ChatKitWorkflowTracing::builder().enabled(false).build())
                    .version("2")
                    .build(),
            )
            .user("user_abc")
            .chatkit_configuration(
                ChatKitConfiguration::builder()
                    .automatic_thread_titling(
                        crate::interfaces::chatkit::ChatKitAutomaticThreadTitling::builder()
                            .enabled(true)
                            .build(),
                    )
                    .build(),
            )
            .expires_after(
                ExpiresAfter::builder()
                    .anchor(ExpiresAfterAnchor::CreatedAt)
                    .seconds(3600)
                    .build(),
            )
            .rate_limits(
                crate::interfaces::chatkit::ChatKitRateLimits::builder()
                    .max_requests_per_1_minute(20)
                    .build(),
            )
            .build();

        let res = client.chatkit().sessions().create(&req).await.unwrap();
        assert_eq!(res.id, "cksess_1");
        assert_eq!(res.client_secret, "cksec_abc");
        assert_eq!(res.status, "active");
    }

    #[tokio::test]
    async fn cancel_deserializes_cancelled_session() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chatkit/sessions/cksess_2/cancel"))
            .and(header("OpenAI-Beta", "chatkit_beta=v1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cksess_2",
                "object": "chatkit.session",
                "client_secret": "cksec_xyz",
                "expires_at": 1700003600_i64,
                "status": "cancelled",
                "user": "user_abc",
                "workflow": {"id": "wf_123"},
                "chatkit_configuration": {},
                "rate_limits": {"max_requests_per_1_minute": 10},
                "max_requests_per_1_minute": 10
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client.chatkit().session("cksess_2").cancel().await.unwrap();
        assert_eq!(res.status, "cancelled");
    }

    #[tokio::test]
    async fn create_with_only_required_fields_omits_optional_ones() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chatkit/sessions"))
            .and(body_json(serde_json::json!({
                "workflow": {"id": "wf_456"},
                "user": "user_def"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cksess_3",
                "object": "chatkit.session",
                "client_secret": "cksec_def",
                "expires_at": 1700003600_i64,
                "status": "active",
                "user": "user_def",
                "workflow": {"id": "wf_456"},
                "chatkit_configuration": {},
                "rate_limits": {},
                "max_requests_per_1_minute": 10
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateChatKitSessionRequest::builder()
            .workflow(ChatKitWorkflowParam::builder().id("wf_456").build())
            .user("user_def")
            .build();

        let res = client.chatkit().sessions().create(&req).await.unwrap();
        assert_eq!(res.id, "cksess_3");
    }
}
