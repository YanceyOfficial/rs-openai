//! Given a prompt, the model will return one or more predicted completions, and can also return the probabilities of alternative tokens at each position.

use crate::client::OpenAI;
use crate::interfaces::completions;
pub use crate::interfaces::completions::*;
use crate::shared::response_wrapper::{OpenAIError, OpenAIResponse};
use crate::shared::utils::is_stream;
use futures::Stream;
use std::pin::Pin;

pub struct Completions<'a> {
    openai: &'a OpenAI,
}

impl<'a> Completions<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates a completion for the provided prompt and parameters.
    pub async fn create(
        &self,
        req: &completions::CreateCompletionRequest,
    ) -> OpenAIResponse<completions::CompletionResponse> {
        if is_stream(req.stream) {
            return Err(OpenAIError::InvalidArgument(
                "When stream is true, use Completions::create_with_stream".into(),
            ));
        }

        self.openai.post("/completions", req).await
    }

    /// Creates a completion for the provided prompt and parameters.
    pub async fn create_with_stream(
        &self,
        req: &completions::CreateCompletionRequest,
    ) -> Result<
        Pin<Box<dyn Stream<Item = OpenAIResponse<completions::CompletionStreamResponse>> + Send>>,
        OpenAIError,
    > {
        if !is_stream(req.stream) {
            return Err(OpenAIError::InvalidArgument(
                "When stream is false, use Completions::create".into(),
            ));
        }

        self.openai.post_stream("/completions", req).await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::completions::CreateCompletionRequest;
    use crate::shared::response_wrapper::OpenAIError;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/completions"))
            .and(body_json(serde_json::json!({
                "model": "text-davinci-003",
                "prompt": "What's your name?",
                "max_tokens": 40
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cmpl-123",
                "object": "text_completion",
                "created": 1700000000_u32,
                "model": "text-davinci-003",
                "choices": [{"text": "I'm a language model.", "index": 0, "logprobs": null, "finish_reason": "stop"}],
                "usage": {"prompt_tokens": 5, "completion_tokens": 6, "total_tokens": 11}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateCompletionRequest::builder()
            .model("text-davinci-003")
            .prompt("What's your name?")
            .max_tokens(40_u16)
            .build();

        let res = client.completions().create(&req).await.unwrap();
        assert_eq!(res.choices[0].text, "I'm a language model.");
    }

    #[tokio::test]
    async fn create_rejects_stream_true() {
        let client = OpenAI::new("sk-test");
        let req = CreateCompletionRequest::builder()
            .model("text-davinci-003")
            .stream(true)
            .build();

        let err = client.completions().create(&req).await.unwrap_err();
        assert!(matches!(err, OpenAIError::InvalidArgument(_)));
    }

    #[tokio::test]
    async fn create_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/completions"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "message": "model is required",
                    "type": "invalid_request_error",
                    "param": "model",
                    "code": null
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateCompletionRequest::builder().model("").build();

        let err = client.completions().create(&req).await.unwrap_err();
        match err {
            OpenAIError::ApiError(e) => {
                assert_eq!(e.error.param, Some(serde_json::json!("model")));
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }
}
