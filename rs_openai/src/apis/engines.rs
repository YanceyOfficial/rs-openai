//! These endpoints describe and provide access to the various engines available in the API.
//!
//! # Warning
//!
//! The Engines endpoints are deprecated.
//!
//! Please use their replacement, [Models](https://platform.openai.com/docs/api-reference/models), instead. [Learn more](https://help.openai.com/TODO).

use crate::client::OpenAI;
use crate::interfaces::engines;
pub use crate::interfaces::engines::*;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct Engines<'a> {
    openai: &'a OpenAI,
}

impl<'a> Engines<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Lists the currently available (non-finetuned) models,
    /// and provides basic information about each one such as the owner and availability.
    #[deprecated(
        note = "The Engines endpoints are deprecated. Please use their replacement, Models, instead."
    )]
    pub async fn list(&self) -> OpenAIResponse<engines::EngineListResponse> {
        self.openai.get("/engines", &()).await
    }

    /// Retrieves a model instance, providing basic information about it such as the owner and availability.
    ///
    /// # Path parameters
    ///
    /// - `engine_id` - The ID of the engine to use for this request
    #[deprecated(
        note = "The Engines endpoints are deprecated. Please use their replacement, Models, instead."
    )]
    pub async fn retrieve(&self, engine_id: &str) -> OpenAIResponse<engines::EngineResponse> {
        self.openai.get(&format!("/engines/{engine_id}"), &()).await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn mock_client() -> (MockServer, OpenAI) {
        let mock_server = MockServer::start().await;
        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        (mock_server, client)
    }

    #[allow(deprecated)]
    #[tokio::test]
    async fn list_deserializes_response() {
        let (mock_server, client) = mock_client().await;

        Mock::given(method("GET"))
            .and(path("/engines"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [
                    {"id": "davinci", "object": "engine", "owner": "openai", "ready": true}
                ]
            })))
            .mount(&mock_server)
            .await;

        let res = client.engines().list().await.unwrap();
        assert_eq!(res.data.len(), 1);
        assert_eq!(res.data[0].id, "davinci");
    }

    #[allow(deprecated)]
    #[tokio::test]
    async fn retrieve_deserializes_response() {
        let (mock_server, client) = mock_client().await;

        Mock::given(method("GET"))
            .and(path("/engines/davinci"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "davinci", "object": "engine", "owner": "openai", "ready": true
            })))
            .mount(&mock_server)
            .await;

        let res = client.engines().retrieve("davinci").await.unwrap();
        assert_eq!(res.id, "davinci");
    }
}
