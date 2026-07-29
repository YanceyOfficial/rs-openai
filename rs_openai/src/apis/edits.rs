//! Given a prompt and an instruction, the model will return an edited version of the prompt.

use crate::client::OpenAI;
use crate::interfaces::edits;
pub use crate::interfaces::edits::*;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct Edits<'a> {
    openai: &'a OpenAI,
}

impl<'a> Edits<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates a new edit for the provided input, instruction, and parameters.
    pub async fn create(
        &self,
        req: &edits::CreateEditRequest,
    ) -> OpenAIResponse<edits::EditResponse> {
        self.openai.post("/edits", req).await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::edits::CreateEditRequest;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/edits"))
            .and(body_json(serde_json::json!({
                "model": "text-davinci-edit-001",
                "input": "Today is Monkey.",
                "instruction": "Fix the grammar mistakes."
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "edit",
                "created": 1700000000_u32,
                "choices": [{"text": "Today is Monday.", "index": 0}],
                "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateEditRequest::builder()
            .model("text-davinci-edit-001")
            .input("Today is Monkey.")
            .instruction("Fix the grammar mistakes.")
            .build();

        let res = client.edits().create(&req).await.unwrap();
        assert_eq!(res.choices[0].text, "Today is Monday.");
    }

    #[tokio::test]
    async fn create_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/edits"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "message": "instruction is required",
                    "type": "invalid_request_error",
                    "param": "instruction",
                    "code": null
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateEditRequest::builder()
            .model("text-davinci-edit-001")
            .instruction("")
            .build();

        let err = client.edits().create(&req).await.unwrap_err();
        match err {
            crate::shared::response_wrapper::OpenAIError::ApiError(e) => {
                assert_eq!(e.error.param, Some(serde_json::json!("instruction")));
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }
}
