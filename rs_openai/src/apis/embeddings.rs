//! Get a vector representation of a given input that can be easily consumed by machine learning models and algorithms.
//!
//! Related guide: [Embeddings](https://platform.openai.com/docs/guides/embeddings)

use crate::client::OpenAI;
use crate::interfaces::embeddings;
pub use crate::interfaces::embeddings::*;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct Embeddings<'a> {
    openai: &'a OpenAI,
}

impl<'a> Embeddings<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates an embedding vector representing the input text.
    pub async fn create(
        &self,
        req: &embeddings::CreateEmbeddingRequest,
    ) -> OpenAIResponse<embeddings::EmbeddingResponse> {
        self.openai.post("/embeddings", req).await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::embeddings::{CreateEmbeddingRequest, EncodingFormat};
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_array_input_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/embeddings"))
            .and(body_json(serde_json::json!({
                "model": "text-embedding-ada-002",
                "input": ["hello", "world"]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [
                    {"object": "embedding", "embedding": [0.1, 0.2, 0.3], "index": 0},
                    {"object": "embedding", "embedding": [0.4, 0.5, 0.6], "index": 1}
                ],
                "model": "text-embedding-ada-002",
                "usage": {"prompt_tokens": 2, "total_tokens": 2}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateEmbeddingRequest::builder()
            .model("text-embedding-ada-002")
            .input(["hello", "world"])
            .build();

        let res = client.embeddings().create(&req).await.unwrap();
        assert_eq!(res.data.len(), 2);
        assert_eq!(res.data[1].embedding, vec![0.4, 0.5, 0.6]);
    }

    #[tokio::test]
    async fn create_serializes_dimensions_and_encoding_format() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/embeddings"))
            .and(body_json(serde_json::json!({
                "model": "text-embedding-3-small",
                "input": "hello",
                "dimensions": 256,
                "encoding_format": "base64"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{"object": "embedding", "embedding": [0.1], "index": 0}],
                "model": "text-embedding-3-small",
                "usage": {"prompt_tokens": 1, "total_tokens": 1}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateEmbeddingRequest::builder()
            .model("text-embedding-3-small")
            .input("hello")
            .dimensions(256_u32)
            .encoding_format(EncodingFormat::Base64)
            .build();

        let res = client.embeddings().create(&req).await.unwrap();
        assert_eq!(res.data.len(), 1);
    }

    #[tokio::test]
    async fn create_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/embeddings"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "message": "input is required",
                    "type": "invalid_request_error",
                    "param": "input",
                    "code": null
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateEmbeddingRequest::builder()
            .model("text-embedding-ada-002")
            .input("x")
            .build();

        let err = client.embeddings().create(&req).await.unwrap_err();
        match err {
            crate::shared::response_wrapper::OpenAIError::ApiError(e) => {
                assert_eq!(e.error.param, Some(serde_json::json!("input")));
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }
}
