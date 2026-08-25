//! Stateful conversation context for the Responses API — replaces the Assistants API's
//! "thread" concept. Includes the Items sub-resource — see
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

    /// Scoped accessor for this conversation's items.
    pub fn items(&self) -> ConversationItems<'a> {
        ConversationItems::new(self.openai, self.id.clone())
    }
}

pub struct ConversationItems<'a> {
    openai: &'a OpenAI,
    conversation_id: String,
}

impl<'a> ConversationItems<'a> {
    pub(crate) fn new(openai: &'a OpenAI, conversation_id: String) -> Self {
        Self {
            openai,
            conversation_id,
        }
    }

    pub async fn create(
        &self,
        req: &conversations::AddConversationItemsRequest,
    ) -> OpenAIResponse<conversations::ConversationItemList> {
        self.openai
            .post(
                &format!("/conversations/{}/items", self.conversation_id),
                req,
            )
            .await
    }

    pub async fn list(
        &self,
        req: &conversations::ListConversationItemsRequest,
    ) -> OpenAIResponse<conversations::ConversationItemList> {
        self.openai
            .get_with_pairs(
                &format!("/conversations/{}/items", self.conversation_id),
                &req.to_query_pairs(),
            )
            .await
    }

    pub fn item(&self, id: impl Into<String>) -> ConversationItem<'a> {
        ConversationItem::new(self.openai, self.conversation_id.clone(), id.into())
    }
}

pub struct ConversationItem<'a> {
    openai: &'a OpenAI,
    conversation_id: String,
    id: String,
}

impl<'a> ConversationItem<'a> {
    pub(crate) fn new(openai: &'a OpenAI, conversation_id: String, id: String) -> Self {
        Self {
            openai,
            conversation_id,
            id,
        }
    }

    /// Confirmed via source: returns a raw JSON value, not a fully-typed struct --
    /// see `docs/superpowers/specs/2026-08-20-evals-runs-design.md`-style reasoning
    /// documented on `interfaces::conversations::ConversationItemList`.
    pub async fn retrieve(
        &self,
        req: &conversations::RetrieveConversationItemRequest,
    ) -> OpenAIResponse<serde_json::Value> {
        self.openai
            .get_with_pairs(
                &format!("/conversations/{}/items/{}", self.conversation_id, self.id),
                &req.to_query_pairs(),
            )
            .await
    }

    /// Confirmed via source: deleting an item returns the parent `Conversation`
    /// object, not a `{deleted, object, id}`-style confirmation -- a genuine
    /// asymmetry from every other delete operation in this crate.
    pub async fn delete(&self) -> OpenAIResponse<conversations::Conversation> {
        self.openai
            .delete(
                &format!("/conversations/{}/items/{}", self.conversation_id, self.id),
                &(),
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::conversations::{
        AddConversationItemsRequest, CreateConversationRequest, ListConversationItemsRequest,
        RetrieveConversationItemRequest, UpdateConversationRequest,
    };
    use crate::interfaces::responses::{ResponseInputItem, ResponseInputRole};
    use wiremock::matchers::{body_json, method, path, query_param};
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

    #[tokio::test]
    async fn items_create_serializes_request_and_deserializes_list() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/conversations/conv_123/items"))
            .and(body_json(serde_json::json!({
                "items": [{"type": "message", "role": "user", "content": "hi"}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{"id": "item_1", "type": "message", "role": "user", "content": "hi"}],
                "first_id": "item_1",
                "has_more": false,
                "last_id": "item_1",
                "object": "list"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = AddConversationItemsRequest::builder()
            .items(vec![ResponseInputItem::Message {
                role: ResponseInputRole::User,
                content: "hi".to_string(),
            }])
            .build();
        let res = client
            .conversation("conv_123")
            .items()
            .create(&req)
            .await
            .unwrap();

        assert_eq!(res.data.len(), 1);
        assert_eq!(res.data[0]["id"], "item_1");
    }

    #[tokio::test]
    async fn items_list_serializes_include_as_repeated_query_params() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/conversations/conv_123/items"))
            .and(query_param("limit", "5"))
            .and(query_param("include", "message.output_text.logprobs"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [],
                "first_id": "",
                "has_more": false,
                "last_id": "",
                "object": "list"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ListConversationItemsRequest::builder()
            .limit(5)
            .include(vec!["message.output_text.logprobs".to_string()])
            .build();
        let res = client
            .conversation("conv_123")
            .items()
            .list(&req)
            .await
            .unwrap();

        assert!(res.data.is_empty());
    }

    #[tokio::test]
    async fn item_retrieve_returns_raw_json_value() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/conversations/conv_123/items/item_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "item_1",
                "type": "local_shell_call",
                "action": {"type": "exec", "command": ["ls"], "env": {}}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = RetrieveConversationItemRequest::builder().build();
        let res = client
            .conversation("conv_123")
            .items()
            .item("item_1")
            .retrieve(&req)
            .await
            .unwrap();

        // A raw JSON value round-trips a variant this crate has never modeled
        // (local_shell_call) without needing to know its shape in advance.
        assert_eq!(res["type"], "local_shell_call");
        assert_eq!(res["action"]["command"][0], "ls");
    }

    #[tokio::test]
    async fn item_delete_returns_parent_conversation_not_a_confirmation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/conversations/conv_123/items/item_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "conv_123",
                "object": "conversation",
                "created_at": 1700000000_i64,
                "metadata": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .conversation("conv_123")
            .items()
            .item("item_1")
            .delete()
            .await
            .unwrap();

        // The response deserializes as a Conversation (has `created_at`/`metadata`),
        // not a delete-confirmation shape -- this test compiling and passing is
        // itself the proof the asymmetry is modeled correctly.
        assert_eq!(res.id, "conv_123");
    }
}
