//! Stateful conversation context for the Responses API — replaces the Assistants API's
//! "thread" concept. Core CRUD only this round, no items sub-resource — see
//! docs/superpowers/specs/2026-08-12-conversations-design.md.

use crate::client::OpenAI;
use crate::interfaces::conversations;
pub use crate::interfaces::conversations::*;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct Conversations<'a> {
    openai: &'a OpenAI,
}

impl<'a> Conversations<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates a conversation.
    pub async fn create(
        &self,
        req: &conversations::CreateConversationRequest,
    ) -> OpenAIResponse<conversations::Conversation> {
        self.openai.post("/conversations", req).await
    }
}

#[allow(dead_code)]
pub struct Conversation<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> Conversation<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::conversations::CreateConversationRequest;
    use crate::interfaces::responses::{ResponseInputItem, ResponseInputRole};
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_items_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/conversations"))
            .and(body_json(serde_json::json!({
                "items": [{"type": "message", "role": "user", "content": "hi"}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "conv_123",
                "object": "conversation",
                "created_at": 1700000000_i64,
                "metadata": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateConversationRequest::builder()
            .items(vec![ResponseInputItem::Message {
                role: ResponseInputRole::User,
                content: "hi".to_string(),
            }])
            .build();

        let res = client.conversations().create(&req).await.unwrap();
        assert_eq!(res.id, "conv_123");
    }

    #[tokio::test]
    async fn create_without_items_omits_the_field() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/conversations"))
            .and(body_json(serde_json::json!({
                "metadata": {"project": "support"}
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "conv_456",
                "object": "conversation",
                "created_at": 1700000000_i64,
                "metadata": {"project": "support"}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateConversationRequest::builder()
            .metadata(std::collections::HashMap::from([(
                "project".to_string(),
                "support".to_string(),
            )]))
            .build();

        let res = client.conversations().create(&req).await.unwrap();
        assert_eq!(
            res.metadata.as_ref().and_then(|m| m.get("project")),
            Some(&"support".to_string())
        );
    }
}
