//! Model responses — OpenAI's now-recommended primary API, superseding Chat Completions and
//! (eventually) Assistants. Core surface only this round: plain-text input/output and
//! function-calling, non-streaming — see
//! docs/superpowers/specs/2026-08-07-responses-core-design.md.

use crate::client::OpenAI;
use crate::interfaces::responses;
pub use crate::interfaces::responses::*;
use crate::shared::response_wrapper::{OpenAIError, OpenAIResponse};
use crate::shared::utils::is_stream;
use futures::Stream;
use std::pin::Pin;

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
        if is_stream(req.stream) {
            return Err(OpenAIError::InvalidArgument(
                "When stream is true, use Responses::create_with_stream".into(),
            ));
        }

        self.openai.post("/responses", req).await
    }

    /// Creates a model response and streams the result as a sequence of `ResponseStreamEvent`s.
    pub async fn create_with_stream(
        &self,
        req: &responses::CreateResponseRequest,
    ) -> OpenAIResponse<
        Pin<Box<dyn Stream<Item = OpenAIResponse<responses::ResponseStreamEvent>> + Send>>,
    > {
        if !is_stream(req.stream) {
            return Err(OpenAIError::InvalidArgument(
                "When stream is false, use Responses::create".into(),
            ));
        }

        self.openai.post_stream("/responses", req).await
    }
}

pub struct Response<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> Response<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Retrieves a model response by ID.
    pub async fn retrieve(&self) -> OpenAIResponse<responses::Response> {
        self.openai
            .get(&format!("/responses/{}", self.id), &())
            .await
    }

    /// Deletes a model response.
    pub async fn delete(&self) -> OpenAIResponse<responses::DeleteResponseResponse> {
        self.openai
            .delete(&format!("/responses/{}", self.id), &())
            .await
    }

    /// Cancels a background response. Errors if the response isn't in a cancellable state.
    pub async fn cancel(&self) -> OpenAIResponse<responses::Response> {
        self.openai
            .post(&format!("/responses/{}/cancel", self.id), &())
            .await
    }

    /// Lists a response's input items.
    pub async fn list_input_items(
        &self,
        req: &responses::ListInputItemsRequest,
    ) -> OpenAIResponse<responses::ListInputItemsResponse> {
        self.openai
            .get(&format!("/responses/{}/input_items", self.id), req)
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::chat::SortOrder;
    use crate::interfaces::responses::{
        CreateResponseRequest, ListInputItemsRequest, ResponseFunctionTool, ResponseInput,
        ResponseInputItem, ResponseInputRole, ResponseStreamEvent, ResponseTool,
        ResponseToolChoice, ResponseToolChoiceFunctionName, ResponseToolChoiceMode,
    };
    use futures::StreamExt;
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

    #[tokio::test]
    async fn retrieve_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/responses/resp_789"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "resp_789",
                "object": "response",
                "created_at": 1700000000_i64,
                "status": "completed",
                "error": null,
                "incomplete_details": null,
                "model": "gpt-5-mini",
                "output": [],
                "usage": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client.response("resp_789").retrieve().await.unwrap();
        assert_eq!(res.id, "resp_789");
    }

    #[tokio::test]
    async fn delete_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/responses/does-not-exist"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": {
                    "message": "No response found",
                    "type": "invalid_request_error",
                    "param": null,
                    "code": "not_found"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let err = client
            .response("does-not-exist")
            .delete()
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            crate::shared::response_wrapper::OpenAIError::ApiError(_)
        ));
    }

    #[tokio::test]
    async fn cancel_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/responses/resp_789/cancel"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "resp_789",
                "object": "response",
                "created_at": 1700000000_i64,
                "status": "cancelled",
                "error": null,
                "incomplete_details": null,
                "model": "gpt-5-mini",
                "output": [],
                "usage": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client.response("resp_789").cancel().await.unwrap();
        assert_eq!(res.status.to_string(), "cancelled");
    }

    #[tokio::test]
    async fn list_input_items_deserializes_function_call_output_item() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/responses/resp_789/input_items"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{
                    "type": "function_call_output",
                    "call_id": "call_abc123",
                    "output": "72F and sunny"
                }],
                "first_id": "item_1",
                "last_id": "item_1",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ListInputItemsRequest::builder()
            .limit(10_u32)
            .order(SortOrder::Desc)
            .build();
        let res = client
            .response("resp_789")
            .list_input_items(&req)
            .await
            .unwrap();

        assert_eq!(res.data.len(), 1);
        match &res.data[0] {
            ResponseInputItem::FunctionCallOutput { call_id, output } => {
                assert_eq!(call_id, "call_abc123");
                assert_eq!(output, "72F and sunny");
            }
            other => panic!("expected FunctionCallOutput, got {other:?}"),
        }
        assert!(!res.has_more);
    }

    #[test]
    fn response_stream_event_created_deserializes_with_wrapped_response() {
        let value = serde_json::json!({
            "type": "response.created",
            "sequence_number": 1,
            "response": {
                "id": "resp_1",
                "object": "response",
                "created_at": 1700000000_i64,
                "status": "in_progress",
                "error": null,
                "incomplete_details": null,
                "model": "gpt-5-mini",
                "output": [],
                "usage": null,
                "previous_response_id": null,
                "instructions": null,
                "metadata": null
            }
        });

        let event: ResponseStreamEvent = serde_json::from_value(value).unwrap();
        match event {
            ResponseStreamEvent::Created {
                response,
                sequence_number,
            } => {
                assert_eq!(response.id, "resp_1");
                assert_eq!(sequence_number, Some(1));
            }
            other => panic!("expected Created, got {other:?}"),
        }
    }

    #[test]
    fn response_stream_event_output_text_delta_deserializes_without_sequence_number() {
        let value = serde_json::json!({
            "type": "response.output_text.delta",
            "item_id": "msg_1",
            "output_index": 0,
            "content_index": 0,
            "delta": "Hello"
        });

        let event: ResponseStreamEvent = serde_json::from_value(value).unwrap();
        match event {
            ResponseStreamEvent::OutputTextDelta {
                item_id,
                delta,
                sequence_number,
                ..
            } => {
                assert_eq!(item_id, "msg_1");
                assert_eq!(delta, "Hello");
                assert_eq!(sequence_number, None);
            }
            other => panic!("expected OutputTextDelta, got {other:?}"),
        }
    }

    #[test]
    fn response_stream_event_function_call_arguments_delta_deserializes() {
        let value = serde_json::json!({
            "type": "response.function_call_arguments.delta",
            "item_id": "fc_1",
            "output_index": 0,
            "delta": "{\"location\":"
        });

        let event: ResponseStreamEvent = serde_json::from_value(value).unwrap();
        match event {
            ResponseStreamEvent::FunctionCallArgumentsDelta { item_id, delta, .. } => {
                assert_eq!(item_id, "fc_1");
                assert_eq!(delta, "{\"location\":");
            }
            other => panic!("expected FunctionCallArgumentsDelta, got {other:?}"),
        }
    }

    #[test]
    fn response_stream_event_error_deserializes() {
        let value = serde_json::json!({
            "type": "error",
            "message": "rate limit exceeded",
            "param": null,
            "code": "rate_limit_exceeded"
        });

        let event: ResponseStreamEvent = serde_json::from_value(value).unwrap();
        match event {
            ResponseStreamEvent::Error { message, code, .. } => {
                assert_eq!(message, "rate limit exceeded");
                assert_eq!(code, Some(serde_json::json!("rate_limit_exceeded")));
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn create_response_request_serializes_stream_field_only_when_set() {
        let req = CreateResponseRequest::builder()
            .model("gpt-5-mini")
            .stream(true)
            .build();
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["stream"], serde_json::json!(true));

        let req_without_stream = CreateResponseRequest::builder().model("gpt-5-mini").build();
        let json_without_stream = serde_json::to_value(&req_without_stream).unwrap();
        assert!(json_without_stream.get("stream").is_none());
    }

    #[tokio::test]
    async fn create_rejects_stream_true() {
        let client = OpenAI::new("sk-test");
        let req = CreateResponseRequest::builder()
            .model("gpt-5-mini")
            .stream(true)
            .build();

        let err = client.responses().create(&req).await.unwrap_err();
        assert!(matches!(
            err,
            crate::shared::response_wrapper::OpenAIError::InvalidArgument(_)
        ));
    }

    #[tokio::test]
    async fn create_with_stream_rejects_stream_false() {
        let client = OpenAI::new("sk-test");
        let req = CreateResponseRequest::builder().model("gpt-5-mini").build();

        let result = client.responses().create_with_stream(&req).await;
        assert!(matches!(
            result,
            Err(crate::shared::response_wrapper::OpenAIError::InvalidArgument(_))
        ));
    }

    /// Happy-path text streaming: created -> in_progress -> output_item.added (Message) ->
    /// two output_text.delta chunks -> output_text.done -> output_item.done -> completed.
    /// No `data: [DONE]` sentinel -- see this plan's Global Constraints on why the mocked
    /// body ends right after the last real event.
    #[tokio::test]
    async fn create_with_stream_yields_text_deltas_and_terminates_after_completed() {
        let mock_server = MockServer::start().await;

        let response_snapshot = |status: &str| {
            serde_json::json!({
                "id": "resp_1",
                "object": "response",
                "created_at": 1700000000_i64,
                "status": status,
                "error": null,
                "incomplete_details": null,
                "model": "gpt-5-mini",
                "output": [],
                "usage": null,
                "previous_response_id": null,
                "instructions": null,
                "metadata": null
            })
        };

        let events = [
            serde_json::json!({"type": "response.created", "response": response_snapshot("in_progress")}),
            serde_json::json!({"type": "response.in_progress", "response": response_snapshot("in_progress")}),
            serde_json::json!({
                "type": "response.output_item.added",
                "output_index": 0,
                "item": {
                    "type": "message", "id": "msg_1", "role": "assistant",
                    "status": "in_progress", "content": []
                }
            }),
            serde_json::json!({
                "type": "response.output_text.delta",
                "item_id": "msg_1", "output_index": 0, "content_index": 0, "delta": "Hello"
            }),
            serde_json::json!({
                "type": "response.output_text.delta",
                "item_id": "msg_1", "output_index": 0, "content_index": 0, "delta": " world"
            }),
            serde_json::json!({
                "type": "response.output_text.done",
                "item_id": "msg_1", "output_index": 0, "content_index": 0, "text": "Hello world"
            }),
            serde_json::json!({
                "type": "response.output_item.done",
                "output_index": 0,
                "item": {
                    "type": "message", "id": "msg_1", "role": "assistant",
                    "status": "completed",
                    "content": [{"type": "output_text", "text": "Hello world", "annotations": []}]
                }
            }),
            serde_json::json!({"type": "response.completed", "response": response_snapshot("completed")}),
        ];

        let sse_body = events
            .iter()
            .map(|e| format!("data: {e}\n\n"))
            .collect::<Vec<String>>()
            .join("");

        Mock::given(method("POST"))
            .and(path("/responses"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(sse_body.into_bytes(), "text/event-stream"),
            )
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateResponseRequest::builder()
            .model("gpt-5-mini")
            .input(ResponseInput::Text("Say hello".to_string()))
            .stream(true)
            .build();

        let mut stream = client.responses().create_with_stream(&req).await.unwrap();

        let mut received = Vec::new();
        while let Some(item) = stream.next().await {
            received.push(item.unwrap());
        }

        assert_eq!(received.len(), 8);
        assert!(matches!(received[0], ResponseStreamEvent::Created { .. }));
        assert!(matches!(
            received[1],
            ResponseStreamEvent::InProgress { .. }
        ));
        assert!(matches!(
            received[2],
            ResponseStreamEvent::OutputItemAdded { .. }
        ));
        match &received[3] {
            ResponseStreamEvent::OutputTextDelta { delta, .. } => assert_eq!(delta, "Hello"),
            other => panic!("expected OutputTextDelta, got {other:?}"),
        }
        match &received[4] {
            ResponseStreamEvent::OutputTextDelta { delta, .. } => assert_eq!(delta, " world"),
            other => panic!("expected OutputTextDelta, got {other:?}"),
        }
        match &received[5] {
            ResponseStreamEvent::OutputTextDone { text, .. } => assert_eq!(text, "Hello world"),
            other => panic!("expected OutputTextDone, got {other:?}"),
        }
        assert!(matches!(
            received[6],
            ResponseStreamEvent::OutputItemDone { .. }
        ));
        match &received[7] {
            ResponseStreamEvent::Completed { response, .. } => {
                assert_eq!(response.status.to_string(), "completed");
            }
            other => panic!("expected Completed, got {other:?}"),
        }

        // The stream must be exhausted (no trailing item, no [DONE] needed).
        assert!(stream.next().await.is_none());
    }

    /// Function-call argument streaming: output_item.added (FunctionCall) -> two
    /// function_call_arguments.delta chunks -> function_call_arguments.done ->
    /// output_item.done. Proves the tagged union discriminates this event family from the
    /// text-delta family within the same stream.
    #[tokio::test]
    async fn create_with_stream_yields_function_call_argument_deltas() {
        let mock_server = MockServer::start().await;

        let events = [
            serde_json::json!({
                "type": "response.output_item.added",
                "output_index": 0,
                "item": {
                    "type": "function_call", "id": "fc_1", "call_id": "call_abc123",
                    "name": "get_weather", "arguments": "", "status": "in_progress"
                }
            }),
            serde_json::json!({
                "type": "response.function_call_arguments.delta",
                "item_id": "fc_1", "output_index": 0, "delta": "{\"location\":"
            }),
            serde_json::json!({
                "type": "response.function_call_arguments.delta",
                "item_id": "fc_1", "output_index": 0, "delta": "\"Boston\"}"
            }),
            serde_json::json!({
                "type": "response.function_call_arguments.done",
                "item_id": "fc_1", "output_index": 0, "arguments": "{\"location\":\"Boston\"}"
            }),
            serde_json::json!({
                "type": "response.output_item.done",
                "output_index": 0,
                "item": {
                    "type": "function_call", "id": "fc_1", "call_id": "call_abc123",
                    "name": "get_weather", "arguments": "{\"location\":\"Boston\"}",
                    "status": "completed"
                }
            }),
        ];

        let sse_body = events
            .iter()
            .map(|e| format!("data: {e}\n\n"))
            .collect::<Vec<String>>()
            .join("");

        Mock::given(method("POST"))
            .and(path("/responses"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(sse_body.into_bytes(), "text/event-stream"),
            )
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateResponseRequest::builder()
            .model("gpt-5-mini")
            .input(ResponseInput::Text(
                "What's the weather in Boston?".to_string(),
            ))
            .stream(true)
            .build();

        let mut stream = client.responses().create_with_stream(&req).await.unwrap();

        let mut received = Vec::new();
        while let Some(item) = stream.next().await {
            received.push(item.unwrap());
        }

        assert_eq!(received.len(), 5);
        assert!(matches!(
            received[0],
            ResponseStreamEvent::OutputItemAdded { .. }
        ));
        match &received[1] {
            ResponseStreamEvent::FunctionCallArgumentsDelta { delta, .. } => {
                assert_eq!(delta, "{\"location\":")
            }
            other => panic!("expected FunctionCallArgumentsDelta, got {other:?}"),
        }
        match &received[3] {
            ResponseStreamEvent::FunctionCallArgumentsDone { arguments, .. } => {
                assert_eq!(arguments, "{\"location\":\"Boston\"}")
            }
            other => panic!("expected FunctionCallArgumentsDone, got {other:?}"),
        }
        assert!(matches!(
            received[4],
            ResponseStreamEvent::OutputItemDone { .. }
        ));
    }

    #[tokio::test]
    async fn create_with_stream_yields_error_event() {
        let mock_server = MockServer::start().await;

        let sse_body = format!(
            "data: {}\n\n",
            serde_json::json!({
                "type": "error",
                "message": "the model is overloaded",
                "param": null,
                "code": "server_error"
            })
        );

        Mock::given(method("POST"))
            .and(path("/responses"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(sse_body.into_bytes(), "text/event-stream"),
            )
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateResponseRequest::builder()
            .model("gpt-5-mini")
            .input(ResponseInput::Text("hi".to_string()))
            .stream(true)
            .build();

        let mut stream = client.responses().create_with_stream(&req).await.unwrap();
        let first = stream.next().await.unwrap().unwrap();
        match first {
            ResponseStreamEvent::Error { message, .. } => {
                assert_eq!(message, "the model is overloaded");
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn create_with_stream_yields_failed_terminal_event() {
        let mock_server = MockServer::start().await;

        let events = [
            serde_json::json!({
                "type": "response.created",
                "response": {
                    "id": "resp_2", "object": "response", "created_at": 1700000000_i64,
                    "status": "in_progress", "error": null, "incomplete_details": null,
                    "model": "gpt-5-mini", "output": [], "usage": null,
                    "previous_response_id": null, "instructions": null, "metadata": null
                }
            }),
            serde_json::json!({
                "type": "response.failed",
                "response": {
                    "id": "resp_2", "object": "response", "created_at": 1700000000_i64,
                    "status": "failed",
                    "error": {"code": "server_error", "message": "internal error"},
                    "incomplete_details": null, "model": "gpt-5-mini", "output": [],
                    "usage": null, "previous_response_id": null, "instructions": null,
                    "metadata": null
                }
            }),
        ];

        let sse_body = events
            .iter()
            .map(|e| format!("data: {e}\n\n"))
            .collect::<Vec<String>>()
            .join("");

        Mock::given(method("POST"))
            .and(path("/responses"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(sse_body.into_bytes(), "text/event-stream"),
            )
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateResponseRequest::builder()
            .model("gpt-5-mini")
            .input(ResponseInput::Text("hi".to_string()))
            .stream(true)
            .build();

        let mut stream = client.responses().create_with_stream(&req).await.unwrap();
        let mut received = Vec::new();
        while let Some(item) = stream.next().await {
            received.push(item.unwrap());
        }

        assert_eq!(received.len(), 2);
        match &received[1] {
            ResponseStreamEvent::Failed { response, .. } => {
                assert_eq!(response.status.to_string(), "failed");
                assert_eq!(
                    response.error.as_ref().map(|e| e.message.as_str()),
                    Some("internal error")
                );
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn create_with_stream_yields_incomplete_terminal_event() {
        let mock_server = MockServer::start().await;

        let sse_body = format!(
            "data: {}\n\n",
            serde_json::json!({
                "type": "response.incomplete",
                "response": {
                    "id": "resp_3", "object": "response", "created_at": 1700000000_i64,
                    "status": "incomplete", "error": null,
                    "incomplete_details": {"reason": "max_output_tokens"},
                    "model": "gpt-5-mini", "output": [], "usage": null,
                    "previous_response_id": null, "instructions": null, "metadata": null
                }
            })
        );

        Mock::given(method("POST"))
            .and(path("/responses"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(sse_body.into_bytes(), "text/event-stream"),
            )
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateResponseRequest::builder()
            .model("gpt-5-mini")
            .input(ResponseInput::Text("hi".to_string()))
            .stream(true)
            .build();

        let mut stream = client.responses().create_with_stream(&req).await.unwrap();
        let first = stream.next().await.unwrap().unwrap();
        match first {
            ResponseStreamEvent::Incomplete { response, .. } => {
                assert_eq!(
                    response
                        .incomplete_details
                        .as_ref()
                        .map(|d| d.reason.as_str()),
                    Some("max_output_tokens")
                );
            }
            other => panic!("expected Incomplete, got {other:?}"),
        }
    }
}
