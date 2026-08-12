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

pub struct Conversation<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> Conversation<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Retrieves a conversation by ID.
    pub async fn retrieve(&self) -> OpenAIResponse<conversations::Conversation> {
        self.openai
            .get(&format!("/conversations/{}", self.id), &())
            .await
    }

    /// Updates a conversation's metadata.
    pub async fn update(
        &self,
        req: &conversations::UpdateConversationRequest,
    ) -> OpenAIResponse<conversations::Conversation> {
        self.openai
            .post(&format!("/conversations/{}", self.id), req)
            .await
    }

    /// Deletes a conversation.
    pub async fn delete(&self) -> OpenAIResponse<conversations::DeleteConversationResponse> {
        self.openai
            .delete(&format!("/conversations/{}", self.id), &())
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::conversations::{CreateConversationRequest, UpdateConversationRequest};
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

    #[tokio::test]
    async fn retrieve_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/conversations/conv_789"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "conv_789",
                "object": "conversation",
                "created_at": 1700000000_i64,
                "metadata": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client.conversation("conv_789").retrieve().await.unwrap();
        assert_eq!(res.id, "conv_789");
    }

    #[tokio::test]
    async fn update_serializes_only_metadata() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/conversations/conv_789"))
            .and(body_json(serde_json::json!({
                "metadata": {"status": "archived"}
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "conv_789",
                "object": "conversation",
                "created_at": 1700000000_i64,
                "metadata": {"status": "archived"}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = UpdateConversationRequest::builder()
            .metadata(std::collections::HashMap::from([(
                "status".to_string(),
                "archived".to_string(),
            )]))
            .build();
        let res = client.conversation("conv_789").update(&req).await.unwrap();
        assert_eq!(
            res.metadata.as_ref().and_then(|m| m.get("status")),
            Some(&"archived".to_string())
        );
    }

    #[tokio::test]
    async fn delete_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/conversations/does-not-exist"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": {
                    "message": "No conversation found",
                    "type": "invalid_request_error",
                    "param": null,
                    "code": "not_found"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let err = client
            .conversation("does-not-exist")
            .delete()
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            crate::shared::response_wrapper::OpenAIError::ApiError(_)
        ));
    }
}
