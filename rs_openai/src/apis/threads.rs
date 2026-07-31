//! Create threads that assistants can interact with.
//!
//! Related guide: [Assistants](https://platform.openai.com/docs/assistants/overview)

use crate::client::OpenAI;
use crate::interfaces::threads;
pub use crate::interfaces::threads::*;
use crate::shared::response_wrapper::OpenAIResponse;

/// The Assistants API is deprecated and sunsets **2026-08-26**. New integrations should use
/// the Responses API (`client.responses()`) combined with the Conversations API
/// (`client.conversations()`) instead — see OpenAI's Assistants migration guide. This module
/// is kept in its current flat structure and receives no further feature work.
pub struct Threads<'a> {
    openai: &'a OpenAI,
}

impl<'a> Threads<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Create a thread.
    pub async fn create(
        &self,
        req: &threads::CreateThreadRequest,
    ) -> OpenAIResponse<threads::ThreadResponse> {
        self.openai.post("/threads", req).await
    }

    /// Retrieves a thread.
    pub async fn retrieve(
        &self,
        thread_id: &str, // The thread object matching the specified ID.
    ) -> OpenAIResponse<threads::ThreadResponse> {
        self.openai.get(&format!("/threads/{thread_id}"), &()).await
    }

    /// Modifies a thread.
    pub async fn modify(
        &self,
        thread_id: &str, // The ID of the thread to modify. Only the `metadata` can be modified.
        req: &threads::ModifyThreadRequest,
    ) -> OpenAIResponse<threads::ThreadResponse> {
        self.openai
            .post(&format!("/threads/{thread_id}"), req)
            .await
    }

    /// Delete a thread.
    pub async fn delete(
        &self,
        thread_id: &str, // The ID of the thread to modify. Only the `metadata` can be modified.
    ) -> OpenAIResponse<threads::DeleteThreadResponse> {
        self.openai
            .delete(&format!("/threads/{thread_id}"), &())
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::threads::CreateThreadRequest;
    use crate::shared::response_wrapper::OpenAIError;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/threads"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "thread_123", "object": "thread", "created_at": 1700000000_u64,
                "metadata": {}, "tool_resources": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateThreadRequest::builder().build();

        let res = client.threads().create(&req).await.unwrap();
        assert_eq!(res.id, "thread_123");
    }

    #[tokio::test]
    async fn delete_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/threads/thread_missing"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": {"message": "No thread found", "type": "invalid_request_error", "param": null, "code": null}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let err = client.threads().delete("thread_missing").await.unwrap_err();
        assert!(matches!(err, OpenAIError::ApiError(_)));
    }
}
