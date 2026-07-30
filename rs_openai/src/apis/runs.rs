//! Create runs that assistants can interact with.
//!
//! Related guide: [Assistants](https://platform.openai.com/docs/assistants/overview)

use crate::client::OpenAI;
use crate::interfaces::runs;
pub use crate::interfaces::runs::*;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct Runs<'a> {
    openai: &'a OpenAI,
}

impl<'a> Runs<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Create a run.
    ///
    /// # Path parameters
    ///
    /// - `thread_id` - The ID of the thread to run.
    pub async fn create(
        &self,
        thread_id: &str,
        req: &runs::CreateRunRequest,
    ) -> OpenAIResponse<runs::RunResponse> {
        self.openai
            .post(&format!("/threads/{thread_id}/runs"), req)
            .await
    }

    /// Retrieves a thread.
    pub async fn retrieve(
        &self,
        thread_id: &str, // The thread object matching the specified ID.
    ) -> OpenAIResponse<runs::RunResponse> {
        self.openai.get(&format!("/runs/{thread_id}"), &()).await
    }

    /// Modifies a thread.
    pub async fn modify(
        &self,
        thread_id: &str, // The ID of the thread to modify. Only the `metadata` can be modified.
        req: &runs::ModifyRunRequest,
    ) -> OpenAIResponse<runs::RunResponse> {
        self.openai.post(&format!("/runs/{thread_id}"), req).await
    }

    /// Delete a thread.
    pub async fn delete(
        &self,
        thread_id: &str, // The ID of the thread to modify. Only the `metadata` can be modified.
    ) -> OpenAIResponse<runs::DeleteRunResponse> {
        self.openai.delete(&format!("/runs/{thread_id}"), &()).await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::runs::CreateRunRequest;
    use crate::shared::response_wrapper::OpenAIError;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/threads/thread_123/runs"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "run_123", "object": "thread.run", "created_at": 1700000000_u64,
                "metadata": {}, "tool_resources": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateRunRequest::builder().build();

        let res = client.runs().create("thread_123", &req).await.unwrap();
        assert_eq!(res.id, "run_123");
    }

    #[tokio::test]
    async fn delete_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/runs/run_missing"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": {"message": "No run found", "type": "invalid_request_error", "param": null, "code": null}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let err = client.runs().delete("run_missing").await.unwrap_err();
        assert!(matches!(err, OpenAIError::ApiError(_)));
    }
}
