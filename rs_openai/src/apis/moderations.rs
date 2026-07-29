//! Given a input text, outputs if the model classifies it as violating OpenAI's content policy. Related guide: [Moderations](https://platform.openai.com/docs/guides/moderation)

use crate::client::OpenAI;
use crate::interfaces::moderations;
pub use crate::interfaces::moderations::*;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct Moderations<'a> {
    openai: &'a OpenAI,
}

impl<'a> Moderations<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Classifies if text and/or image inputs are potentially harmful. Learn more in the [moderation guide](https://platform.openai.com/docs/guides/moderation).
    pub async fn create(
        &self,
        req: &moderations::CreateModerationRequest,
    ) -> OpenAIResponse<moderations::ModerationResponse> {
        self.openai.post("/moderations", req).await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::moderations::CreateModerationRequest;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn categories_fixture(flagged_violence: bool) -> serde_json::Value {
        serde_json::json!({
            "harassment": false, "harassment/threatening": false, "sexual": false,
            "hate": false, "hate/threatening": false, "illicit": false,
            "illicit/violent": false, "self-harm/intent": false,
            "self-harm/instructions": false, "self-harm": false,
            "sexual/minors": false, "violence": flagged_violence, "violence/graphic": false
        })
    }

    fn category_scores_fixture(violence_score: f64) -> serde_json::Value {
        serde_json::json!({
            "harassment": 0.0, "harassment/threatening": 0.0, "sexual": 0.0,
            "hate": 0.0, "hate/threatening": 0.0, "illicit": 0.0,
            "illicit/violent": 0.0, "self-harm/intent": 0.0,
            "self-harm/instructions": 0.0, "self-harm": 0.0,
            "sexual/minors": 0.0, "violence": violence_score, "violence/graphic": 0.0
        })
    }

    fn category_applied_input_types_fixture() -> serde_json::Value {
        serde_json::json!({
            "harassment": ["text"], "harassment/threatening": ["text"], "sexual": ["text"],
            "hate": ["text"], "hate/threatening": ["text"], "illicit": ["text"],
            "illicit/violent": ["text"], "self-harm/intent": ["text"],
            "self-harm/instructions": ["text"], "self-harm": ["text"],
            "sexual/minors": ["text"], "violence": ["text"], "violence/graphic": ["text"]
        })
    }

    #[tokio::test]
    async fn create_serializes_array_input_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/moderations"))
            .and(body_json(serde_json::json!({
                "input": ["Hello, how are you today?", "This is a plain test message."]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "modr-123",
                "model": "omni-moderation-latest",
                "results": [{
                    "flagged": true,
                    "categories": categories_fixture(true),
                    "category_scores": category_scores_fixture(0.9),
                    "category_applied_input_types": category_applied_input_types_fixture()
                }]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateModerationRequest::builder()
            .input(["Hello, how are you today?", "This is a plain test message."])
            .build();

        let res = client.moderations().create(&req).await.unwrap();
        assert!(res.results[0].flagged);
        assert!(res.results[0].categories.violence);
        assert_eq!(res.results[0].category_scores.violence, 0.9);
    }

    #[tokio::test]
    async fn create_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/moderations"))
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
        let req = CreateModerationRequest::builder().input("hi").build();

        let err = client.moderations().create(&req).await.unwrap_err();
        match err {
            crate::shared::response_wrapper::OpenAIError::ApiError(e) => {
                assert_eq!(e.error.param, Some(serde_json::json!("input")));
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }
}
