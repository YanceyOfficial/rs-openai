//! Model responses — OpenAI's now-recommended primary API, superseding Chat Completions and
//! (eventually) Assistants. Core surface only this round: plain-text input/output and
//! function-calling, non-streaming — see
//! docs/superpowers/specs/2026-08-07-responses-core-design.md.

use crate::client::OpenAI;
use crate::interfaces::responses;
pub use crate::interfaces::responses::*;
use crate::shared::response_wrapper::OpenAIResponse;

#[allow(dead_code)]
pub struct Responses<'a> {
    openai: &'a OpenAI,
}

impl<'a> Responses<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates a model response.
    pub async fn create(
        &self,
        req: &responses::CreateResponseRequest,
    ) -> OpenAIResponse<responses::Response> {
        self.openai.post("/responses", req).await
    }
}

#[allow(dead_code)]
pub struct Response<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> Response<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::responses::{
        CreateResponseRequest, ResponseFunctionTool, ResponseInput, ResponseInputItem,
        ResponseInputRole, ResponseTool, ResponseToolChoice, ResponseToolChoiceFunctionName,
        ResponseToolChoiceMode,
    };
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn response_input_item_message_serializes_with_type_tag() {
        let item = ResponseInputItem::Message {
            role: ResponseInputRole::User,
            content: "hello".to_string(),
        };

        let json = serde_json::to_value(&item).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"type": "message", "role": "user", "content": "hello"})
        );
    }

    #[test]
    fn response_input_item_function_call_output_serializes_with_type_tag() {
        let item = ResponseInputItem::FunctionCallOutput {
            call_id: "call_123".to_string(),
            output: "72F and sunny".to_string(),
        };

        let json = serde_json::to_value(&item).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "function_call_output",
                "call_id": "call_123",
                "output": "72F and sunny"
            })
        );
    }

    #[test]
    fn response_tool_function_serializes_with_type_tag() {
        let tool = ResponseTool::Function(
            ResponseFunctionTool::builder()
                .name("get_weather")
                .parameters(serde_json::json!({"type": "object"}))
                .build(),
        );

        let json = serde_json::to_value(&tool).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "function",
                "name": "get_weather",
                "parameters": {"type": "object"}
            })
        );
    }

    #[test]
    fn response_tool_choice_serializes_mode_as_bare_string() {
        let choice = ResponseToolChoice::Mode(ResponseToolChoiceMode::Auto);
        assert_eq!(
            serde_json::to_value(&choice).unwrap(),
            serde_json::json!("auto")
        );
    }

    #[test]
    fn response_tool_choice_serializes_specific_function_as_object() {
        let choice = ResponseToolChoice::Specific {
            kind: "function".to_string(),
            function: ResponseToolChoiceFunctionName {
                name: "get_weather".to_string(),
            },
        };

        let json = serde_json::to_value(&choice).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"type": "function", "function": {"name": "get_weather"}})
        );
    }

    #[tokio::test]
    async fn create_serializes_text_input_and_deserializes_message_output() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/responses"))
            .and(body_json(serde_json::json!({
                "model": "gpt-5-mini",
                "input": "What's the capital of France?"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "resp_123",
                "object": "response",
                "created_at": 1700000000_i64,
                "status": "completed",
                "error": null,
                "incomplete_details": null,
                "model": "gpt-5-mini",
                "output": [{
                    "type": "message",
                    "id": "msg_1",
                    "role": "assistant",
                    "status": "completed",
                    "content": [{"type": "output_text", "text": "Paris.", "annotations": []}]
                }],
                "usage": {
                    "input_tokens": 10,
                    "output_tokens": 3,
                    "total_tokens": 13
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateResponseRequest::builder()
            .model("gpt-5-mini")
            .input(ResponseInput::Text(
                "What's the capital of France?".to_string(),
            ))
            .build();

        let res = client.responses().create(&req).await.unwrap();
        assert_eq!(res.id, "resp_123");
        match &res.output[0] {
            crate::interfaces::responses::ResponseOutputItem::Message(message) => {
                match &message.content[0] {
                    crate::interfaces::responses::ResponseOutputContent::OutputText {
                        text,
                        ..
                    } => assert_eq!(text, "Paris."),
                    other => panic!("expected OutputText, got {other:?}"),
                }
            }
            other => panic!("expected Message, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn create_serializes_message_array_input_with_tool_and_tool_choice() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/responses"))
            .and(body_json(serde_json::json!({
                "model": "gpt-5-mini",
                "input": [{"type": "message", "role": "user", "content": "What's the weather in Boston?"}],
                "tools": [{
                    "type": "function",
                    "name": "get_weather",
                    "parameters": {"type": "object", "properties": {"location": {"type": "string"}}}
                }],
                "tool_choice": "auto"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "resp_456",
                "object": "response",
                "created_at": 1700000000_i64,
                "status": "completed",
                "error": null,
                "incomplete_details": null,
                "model": "gpt-5-mini",
                "output": [{
                    "type": "function_call",
                    "id": "fc_1",
                    "call_id": "call_abc123",
                    "name": "get_weather",
                    "arguments": "{\"location\":\"Boston\"}",
                    "status": "completed"
                }],
                "usage": {
                    "input_tokens": 20,
                    "output_tokens": 8,
                    "total_tokens": 28
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let tool = ResponseTool::Function(
            ResponseFunctionTool::builder()
                .name("get_weather")
                .parameters(serde_json::json!({
                    "type": "object",
                    "properties": {"location": {"type": "string"}}
                }))
                .build(),
        );
        let req = CreateResponseRequest::builder()
            .model("gpt-5-mini")
            .input(ResponseInput::Items(vec![ResponseInputItem::Message {
                role: ResponseInputRole::User,
                content: "What's the weather in Boston?".to_string(),
            }]))
            .tools(vec![tool])
            .tool_choice(ResponseToolChoice::Mode(ResponseToolChoiceMode::Auto))
            .build();

        let res = client.responses().create(&req).await.unwrap();
        match &res.output[0] {
            crate::interfaces::responses::ResponseOutputItem::FunctionCall(call) => {
                assert_eq!(call.name, "get_weather");
                assert_eq!(call.call_id, "call_abc123");
                assert_eq!(call.arguments, r#"{"location":"Boston"}"#);
            }
            other => panic!("expected FunctionCall, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn create_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/responses"))
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
        let req = CreateResponseRequest::builder().model("").build();

        let err = client.responses().create(&req).await.unwrap_err();
        match err {
            crate::shared::response_wrapper::OpenAIError::ApiError(e) => {
                assert_eq!(e.error.param, Some(serde_json::json!("model")));
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }

    #[test]
    fn response_output_item_discriminates_message_and_function_call_in_same_array() {
        let value = serde_json::json!([
            {
                "type": "message",
                "id": "msg_1",
                "role": "assistant",
                "status": "completed",
                "content": [{"type": "output_text", "text": "Let me check the weather.", "annotations": []}]
            },
            {
                "type": "function_call",
                "id": "fc_1",
                "call_id": "call_abc123",
                "name": "get_weather",
                "arguments": "{\"location\":\"Boston\"}",
                "status": "completed"
            }
        ]);

        let items: Vec<crate::interfaces::responses::ResponseOutputItem> =
            serde_json::from_value(value).unwrap();

        assert_eq!(items.len(), 2);
        match &items[0] {
            crate::interfaces::responses::ResponseOutputItem::Message(message) => {
                match &message.content[0] {
                    crate::interfaces::responses::ResponseOutputContent::OutputText {
                        text,
                        ..
                    } => assert_eq!(text, "Let me check the weather."),
                    other => panic!("expected OutputText, got {other:?}"),
                }
            }
            other => panic!("expected Message, got {other:?}"),
        }
        match &items[1] {
            crate::interfaces::responses::ResponseOutputItem::FunctionCall(call) => {
                assert_eq!(call.name, "get_weather");
            }
            other => panic!("expected FunctionCall, got {other:?}"),
        }
    }
}
