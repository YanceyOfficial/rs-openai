//! Build assistants that can call models and use tools to perform tasks.
//!
//! [Get started with the Assistants API](https://platform.openai.com/docs/assistants)

use crate::client::OpenAI;
use crate::interfaces::assistants;
pub use crate::interfaces::assistants::*;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct Assistants<'a> {
    openai: &'a OpenAI,
}

impl<'a> Assistants<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Create an assistant with a model and instructions.
    pub async fn create_assistant(
        &self,
        req: &assistants::AssistantRequest,
    ) -> OpenAIResponse<assistants::AssistantResponse> {
        self.openai.post("/assistants", req).await
    }

    /// Returns a list of assistants.
    pub async fn list_assistants(
        &self,
        req: &assistants::ListAssistantRequest,
    ) -> OpenAIResponse<assistants::ListAssistantResponse> {
        self.openai.get("/assistants", req).await
    }

    /// Retrieves an assistant.
    pub async fn retrieve_assistant(
        &self,
        assistant_id: &str,
    ) -> OpenAIResponse<assistants::AssistantResponse> {
        self.openai
            .get(&format!("/assistants/{assistant_id}"), &())
            .await
    }

    /// Modifies an assistant.
    pub async fn modify_assistant(
        &self,
        assistant_id: &str,
        req: &assistants::AssistantRequest,
    ) -> OpenAIResponse<assistants::AssistantResponse> {
        self.openai
            .post(&format!("/assistants/{assistant_id}"), req)
            .await
    }

    /// Delete an assistant.
    pub async fn delete_assistant(
        &self,
        assistant_id: &str,
    ) -> OpenAIResponse<assistants::DeleteAssistantResponse> {
        self.openai
            .delete(&format!("/assistants/{assistant_id}"), &())
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::assistants::AssistantRequest;
    use crate::shared::response_wrapper::OpenAIError;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_assistant_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/assistants"))
            .and(body_json(serde_json::json!({"model": "gpt-4o-mini"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "asst_123", "object": "assistant", "created_at": 1700000000_u64,
                "name": null, "description": null, "model": "gpt-4o-mini", "instructions": null,
                "tools": [], "tool_resources": null, "metadata": {},
                "temperature": 1.0, "top_p": 1.0, "response_format": "auto"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = AssistantRequest::builder().model("gpt-4o-mini").build();

        let res = client.assistants().create_assistant(&req).await.unwrap();
        assert_eq!(res.id, "asst_123");
    }

    #[tokio::test]
    async fn delete_assistant_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/assistants/does-not-exist"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": {
                    "message": "No assistant found with id 'does-not-exist'",
                    "type": "invalid_request_error",
                    "param": null,
                    "code": null
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let err = client
            .assistants()
            .delete_assistant("does-not-exist")
            .await
            .unwrap_err();
        assert!(matches!(err, OpenAIError::ApiError(_)));
    }
}
