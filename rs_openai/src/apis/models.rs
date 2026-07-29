//! List and describe the various models available in the API.
//! You can refer to the [Models](https://platform.openai.com/docs/models) documentation to understand what models are available and the differences between them.

use crate::client::OpenAI;
use crate::interfaces::models;
pub use crate::interfaces::models::*;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct Models<'a> {
    openai: &'a OpenAI,
}

impl<'a> Models<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Retrieves a model instance, providing basic information about the model such as the owner and permissioning.
    ///
    /// # Path parameters
    ///
    /// - `model` - The ID of the model to use for this request.
    pub async fn retrieve(&self, model: &str) -> OpenAIResponse<models::ModelResponse> {
        self.openai.get(&format!("/models/{model}"), &()).await
    }

    /// Lists the currently available models, and provides basic information about each one such as the owner and availability.
    pub async fn list(&self) -> OpenAIResponse<models::ListModelResponse> {
        self.openai.get("/models", &()).await
    }

    /// Delete a fine-tuned model. You must have the Owner role in your organization to delete a model.
    ///
    /// # Path parameters
    ///
    /// - `model` - The model to delete
    pub async fn delete(&self, model: &str) -> OpenAIResponse<models::DeleteModelResponse> {
        self.openai.delete(&format!("/models/{model}"), &()).await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::shared::response_wrapper::OpenAIError;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn mock_client() -> (MockServer, OpenAI) {
        let mock_server = MockServer::start().await;
        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        (mock_server, client)
    }

    #[tokio::test]
    async fn list_deserializes_response() {
        let (mock_server, client) = mock_client().await;

        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [
                    {"id": "gpt-4o-mini", "object": "model", "created": 1700000000_u32, "owned_by": "openai"}
                ]
            })))
            .mount(&mock_server)
            .await;

        let res = client.models().list().await.unwrap();
        assert_eq!(res.data.len(), 1);
        assert_eq!(res.data[0].id, "gpt-4o-mini");
    }

    #[tokio::test]
    async fn retrieve_deserializes_response() {
        let (mock_server, client) = mock_client().await;

        Mock::given(method("GET"))
            .and(path("/models/gpt-4o-mini"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "gpt-4o-mini", "object": "model", "created": 1700000000_u32, "owned_by": "openai"
            })))
            .mount(&mock_server)
            .await;

        let res = client.models().retrieve("gpt-4o-mini").await.unwrap();
        assert_eq!(res.id, "gpt-4o-mini");
    }

    #[tokio::test]
    async fn delete_maps_api_error_response() {
        let (mock_server, client) = mock_client().await;

        Mock::given(method("DELETE"))
            .and(path("/models/does-not-exist"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": {
                    "message": "The model 'does-not-exist' does not exist",
                    "type": "invalid_request_error",
                    "param": null,
                    "code": "model_not_found"
                }
            })))
            .mount(&mock_server)
            .await;

        let err = client.models().delete("does-not-exist").await.unwrap_err();
        match err {
            OpenAIError::ApiError(e) => {
                assert_eq!(e.error.code, Some(serde_json::json!("model_not_found")));
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }
}
