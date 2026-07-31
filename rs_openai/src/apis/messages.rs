//! Create messages that assistants can interact with.
//!
//! Related guide: [Assistants](https://platform.openai.com/docs/assistants/overview)

use crate::client::OpenAI;
use crate::interfaces::messages;
pub use crate::interfaces::messages::*;
use crate::shared::response_wrapper::OpenAIResponse;

/// The Assistants API is deprecated and sunsets **2026-08-26**. New integrations should use
/// the Responses API (`client.responses()`) combined with the Conversations API
/// (`client.conversations()`) instead — see OpenAI's Assistants migration guide. This module
/// is kept in its current flat structure and receives no further feature work.
pub struct Messages<'a> {
    openai: &'a OpenAI,
}

impl<'a> Messages<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Create a message.
    ///     
    /// # Path parameters
    ///
    /// - `thread_id` - The ID of the [thread](https://platform.openai.com/docs/api-reference/threads) to create a message for.
    pub async fn create(
        &self,
        thread_id: &str,
        req: &messages::CreateMessageRequest,
    ) -> OpenAIResponse<messages::MessageResponse> {
        self.openai
            .post(&format!("/threads/{thread_id}/messages"), req)
            .await
    }

    /// Returns a list of messages for a given thread.
    ///    
    /// # Path parameters
    ///
    /// - `thread_id` - The ID of the [thread](https://platform.openai.com/docs/api-reference/threads) to create a message for.
    pub async fn list(
        &self,
        thread_id: &str,
        req: &messages::ListMessageRequest,
    ) -> OpenAIResponse<messages::ListMessageResponse> {
        self.openai
            .get(&format!("/threads/{thread_id}/messages"), req)
            .await
    }

    /// Retrieves a message.
    ///   
    /// # Path parameters
    ///
    /// - `thread_id` - The ID of the [thread](https://platform.openai.com/docs/api-reference/threads) to create a message for.
    /// - `message_id` - The ID of the message to retrieve.
    pub async fn retrieve(
        &self,
        thread_id: &str,
        message_id: &str,
    ) -> OpenAIResponse<messages::MessageResponse> {
        self.openai
            .get(&format!("/threads/{thread_id}/messages/{message_id}"), &())
            .await
    }

    /// Modifies a message.
    ///
    /// # Path parameters
    ///
    /// - `thread_id` - The ID of the [thread](https://platform.openai.com/docs/api-reference/threads) to create a message for.
    /// - `message_id` - The ID of the message to retrieve.
    pub async fn modify(
        &self,
        thread_id: &str,
        message_id: &str,
        req: &messages::ModifyMessageRequest,
    ) -> OpenAIResponse<messages::MessageResponse> {
        self.openai
            .post(&format!("/threads/{thread_id}/messages/{message_id}"), req)
            .await
    }

    /// Delete a message.
    ///
    /// # Path parameters
    ///
    /// - `thread_id` - The ID of the [thread](https://platform.openai.com/docs/api-reference/threads) to create a message for.
    /// - `message_id` - The ID of the message to retrieve.
    pub async fn delete(
        &self,
        thread_id: &str,
        message_id: &str,
    ) -> OpenAIResponse<messages::DeleteMessageResponse> {
        self.openai
            .delete(&format!("/threads/{thread_id}/messages/{message_id}"), &())
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::messages::{Content, CreateMessageRequest};
    use crate::shared::response_wrapper::OpenAIError;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/threads/thread_123/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "msg_123", "object": "thread.message", "created_at": 1700000000_u64,
                "thread_id": "thread_123", "status": "completed", "incomplete_details": null,
                "completed_at": 1700000001_u64, "incomplete_at": 0, "role": "user",
                "content": "hello", "assistant_id": null, "run_id": null,
                "attachments": null, "metadata": {}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateMessageRequest::builder()
            .role("user")
            .content(Content::TextContent("hello".to_string()))
            .build();

        let res = client.messages().create("thread_123", &req).await.unwrap();
        assert_eq!(res.id, "msg_123");
    }

    #[tokio::test]
    async fn delete_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/threads/thread_123/messages/msg_missing"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": {"message": "No message found", "type": "invalid_request_error", "param": null, "code": null}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let err = client
            .messages()
            .delete("thread_123", "msg_missing")
            .await
            .unwrap_err();
        assert!(matches!(err, OpenAIError::ApiError(_)));
    }
}
