//! Model responses — OpenAI's now-recommended primary API, superseding Chat Completions and
//! (eventually) Assistants. Covers core CRUD/lifecycle, streaming, all 7 confirmed tool types
//! (function/file_search/web_search/image_generation/mcp/computer_use/code_interpreter), the
//! `conversation` param, multimodal (text/image/file) input, and the `include` param (on
//! create/retrieve/list_input_items) -- see docs/superpowers/specs/2026-08-07-responses-core-
//! design.md and later specs under docs/superpowers/specs/ prefixed `responses-`. Remaining
//! gaps: per-tool-type streaming events beyond text/function-call, `return_token_budget`, and
//! mcp's deferred `connector_id`/`tunnel_id`/`allowed_callers`/`McpToolFilter`.

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
    pub async fn retrieve(
        &self,
        req: &responses::RetrieveResponseRequest,
    ) -> OpenAIResponse<responses::Response> {
        self.openai
            .get_with_pairs(&format!("/responses/{}", self.id), &req.to_query_pairs())
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
            .get_with_pairs(
                &format!("/responses/{}/input_items", self.id),
                &req.to_query_pairs(),
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::chat::SortOrder;
    use crate::interfaces::containers::ContainerMemoryLimit;
    use crate::interfaces::images::{Background, ImageSize, Quality};
    use crate::interfaces::responses::{
        ComputerAction, ComputerActionButton, ComputerCallOutputScreenshot, ComputerEnvironment,
        ComputerSafetyCheck, CreateResponseRequest, FileSearchRankingOptions, ImageDetail,
        ImageGenerationAction, ListInputItemsRequest, McpApprovalFilter, McpApprovalMode,
        McpRequireApproval, McpToolNameFilter, ResponseCodeInterpreterAutoContainer,
        ResponseCodeInterpreterContainer, ResponseCodeInterpreterTool,
        ResponseComputerUsePreviewTool, ResponseConversationParam, ResponseFileSearchTool,
        ResponseFunctionTool, ResponseImageGenerationTool, ResponseIncludable, ResponseInput,
        ResponseInputContent, ResponseInputFile, ResponseInputImage, ResponseInputItem,
        ResponseInputRole, ResponseInputText, ResponseMcpTool, ResponseOutputItem,
        ResponseStreamEvent, ResponseTool, ResponseToolChoice, ResponseToolChoiceFunctionName,
        ResponseToolChoiceMode, ResponseWebSearchTool, RetrieveResponseRequest, WebSearchAction,
        WebSearchContextSize, WebSearchUserLocation,
    };
    use futures::StreamExt;
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn response_input_item_message_serializes_with_type_tag() {
        let item = ResponseInputItem::Message {
            role: ResponseInputRole::User,
            content: "hello".into(),
        };

        let json = serde_json::to_value(&item).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"type": "message", "role": "user", "content": "hello"})
        );
    }

    #[test]
    fn response_input_item_message_serializes_multimodal_content_list() {
        let item = ResponseInputItem::Message {
            role: ResponseInputRole::User,
            content: vec![
                ResponseInputContent::InputText(
                    ResponseInputText::builder()
                        .text("What's in this image?")
                        .build(),
                ),
                ResponseInputContent::InputImage(
                    ResponseInputImage::builder()
                        .detail(ImageDetail::Auto)
                        .image_url("https://example.com/cat.png")
                        .build(),
                ),
                ResponseInputContent::InputFile(
                    ResponseInputFile::builder()
                        .filename("report.pdf")
                        .file_data("base64stuff")
                        .build(),
                ),
            ]
            .into(),
        };

        let json = serde_json::to_value(&item).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "message",
                "role": "user",
                "content": [
                    {"type": "input_text", "text": "What's in this image?"},
                    {"type": "input_image", "detail": "auto", "image_url": "https://example.com/cat.png"},
                    {"type": "input_file", "filename": "report.pdf", "file_data": "base64stuff"}
                ]
            })
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
    fn response_input_item_mcp_approval_response_serializes() {
        let item = ResponseInputItem::McpApprovalResponse {
            approval_request_id: "mcpr_1".to_string(),
            approve: true,
        };

        let value = serde_json::to_value(&item).unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "type": "mcp_approval_response",
                "approval_request_id": "mcpr_1",
                "approve": true
            })
        );
    }

    #[test]
    fn response_input_item_computer_call_output_serializes_with_safety_checks() {
        let item = ResponseInputItem::ComputerCallOutput {
            id: None,
            call_id: "call_cu_1".to_string(),
            output: ComputerCallOutputScreenshot::builder()
                .kind("computer_screenshot")
                .image_url("data:image/png;base64,abc123")
                .detail("original")
                .build(),
            acknowledged_safety_checks: Some(vec![
                ComputerSafetyCheck::builder()
                    .id("sc_1")
                    .code("malicious_instructions")
                    .message("Detected a potentially malicious instruction.")
                    .build(),
            ]),
            status: None,
        };

        let value = serde_json::to_value(&item).unwrap();
        assert_eq!(
            value,
            serde_json::json!({
                "type": "computer_call_output",
                "call_id": "call_cu_1",
                "output": {
                    "type": "computer_screenshot",
                    "image_url": "data:image/png;base64,abc123",
                    "detail": "original"
                },
                "acknowledged_safety_checks": [
                    {"id": "sc_1", "code": "malicious_instructions", "message": "Detected a potentially malicious instruction."}
                ]
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
    async fn create_serializes_include_field() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/responses"))
            .and(body_json(serde_json::json!({
                "model": "gpt-5-mini",
                "input": "What's the capital of France?",
                "include": ["file_search_call.results", "reasoning.encrypted_content"]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "resp_123",
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
        let req = CreateResponseRequest::builder()
            .model("gpt-5-mini")
            .input(ResponseInput::Text(
                "What's the capital of France?".to_string(),
            ))
            .include(vec![
                ResponseIncludable::FileSearchCallResults,
                ResponseIncludable::ReasoningEncryptedContent,
            ])
            .build();

        let res = client.responses().create(&req).await.unwrap();
        assert_eq!(res.id, "resp_123");
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
                content: "What's the weather in Boston?".into(),
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
        let req = RetrieveResponseRequest::builder().build();
        let res = client.response("resp_789").retrieve(&req).await.unwrap();
        assert_eq!(res.id, "resp_789");
    }

    #[tokio::test]
    async fn retrieve_sends_include_query_params() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/responses/resp_789"))
            .and(query_param("include", "message.output_text.logprobs"))
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
        let req = RetrieveResponseRequest::builder()
            .include(vec![ResponseIncludable::MessageOutputTextLogprobs])
            .build();
        let res = client.response("resp_789").retrieve(&req).await.unwrap();
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

    #[tokio::test]
    async fn list_input_items_sends_include_query_param() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/responses/resp_789/input_items"))
            .and(query_param("include", "file_search_call.results"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [],
                "first_id": null,
                "last_id": null,
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ListInputItemsRequest::builder()
            .include(vec![
                ResponseIncludable::FileSearchCallResults,
                ResponseIncludable::ReasoningEncryptedContent,
            ])
            .build();
        let res = client
            .response("resp_789")
            .list_input_items(&req)
            .await
            .unwrap();
        assert!(res.data.is_empty());
    }

    #[tokio::test]
    async fn list_input_items_deserializes_multimodal_message_item() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/responses/resp_789/input_items"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{
                    "type": "message",
                    "role": "user",
                    "content": [
                        {"type": "input_text", "text": "Describe this image"},
                        {"type": "input_image", "detail": "high", "file_id": "file-abc123"}
                    ]
                }],
                "first_id": "item_1",
                "last_id": "item_1",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ListInputItemsRequest::builder().build();
        let res = client
            .response("resp_789")
            .list_input_items(&req)
            .await
            .unwrap();

        match &res.data[0] {
            ResponseInputItem::Message { content, .. } => match content {
                crate::interfaces::responses::ResponseInputMessageContent::Items(items) => {
                    assert_eq!(items.len(), 2);
                    match &items[1] {
                        ResponseInputContent::InputImage(img) => {
                            assert_eq!(img.file_id.as_deref(), Some("file-abc123"));
                        }
                        other => panic!("expected InputImage, got {other:?}"),
                    }
                }
                other => panic!("expected multimodal Items content, got {other:?}"),
            },
            other => panic!("expected Message, got {other:?}"),
        }
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
    fn response_stream_event_file_search_call_lifecycle_deserializes() {
        for (json_type, expected) in [
            (
                "response.file_search_call.in_progress",
                "FileSearchCallInProgress",
            ),
            (
                "response.file_search_call.searching",
                "FileSearchCallSearching",
            ),
            (
                "response.file_search_call.completed",
                "FileSearchCallCompleted",
            ),
        ] {
            let value = serde_json::json!({
                "type": json_type,
                "item_id": "fs_1",
                "output_index": 0,
                "sequence_number": 5
            });

            let event: ResponseStreamEvent = serde_json::from_value(value).unwrap();
            let debug = format!("{event:?}");
            assert!(
                debug.starts_with(expected),
                "expected {expected}, got {debug}"
            );
            match event {
                ResponseStreamEvent::FileSearchCallInProgress {
                    item_id,
                    sequence_number,
                    ..
                }
                | ResponseStreamEvent::FileSearchCallSearching {
                    item_id,
                    sequence_number,
                    ..
                }
                | ResponseStreamEvent::FileSearchCallCompleted {
                    item_id,
                    sequence_number,
                    ..
                } => {
                    assert_eq!(item_id, "fs_1");
                    assert_eq!(sequence_number, 5);
                }
                other => panic!("expected a FileSearchCall* variant, got {other:?}"),
            }
        }
    }

    #[test]
    fn response_stream_event_web_search_call_lifecycle_deserializes() {
        for (json_type, expected) in [
            (
                "response.web_search_call.in_progress",
                "WebSearchCallInProgress",
            ),
            (
                "response.web_search_call.searching",
                "WebSearchCallSearching",
            ),
            (
                "response.web_search_call.completed",
                "WebSearchCallCompleted",
            ),
        ] {
            let value = serde_json::json!({
                "type": json_type,
                "item_id": "ws_1",
                "output_index": 0,
                "sequence_number": 7
            });

            let event: ResponseStreamEvent = serde_json::from_value(value).unwrap();
            let debug = format!("{event:?}");
            assert!(
                debug.starts_with(expected),
                "expected {expected}, got {debug}"
            );
            match event {
                ResponseStreamEvent::WebSearchCallInProgress {
                    item_id,
                    sequence_number,
                    ..
                }
                | ResponseStreamEvent::WebSearchCallSearching {
                    item_id,
                    sequence_number,
                    ..
                }
                | ResponseStreamEvent::WebSearchCallCompleted {
                    item_id,
                    sequence_number,
                    ..
                } => {
                    assert_eq!(item_id, "ws_1");
                    assert_eq!(sequence_number, 7);
                }
                other => panic!("expected a WebSearchCall* variant, got {other:?}"),
            }
        }
    }

    #[test]
    fn response_stream_event_image_generation_call_lifecycle_deserializes() {
        for (json_type, expected) in [
            (
                "response.image_generation_call.in_progress",
                "ImageGenerationCallInProgress",
            ),
            (
                "response.image_generation_call.generating",
                "ImageGenerationCallGenerating",
            ),
            (
                "response.image_generation_call.completed",
                "ImageGenerationCallCompleted",
            ),
        ] {
            let value = serde_json::json!({
                "type": json_type,
                "item_id": "ig_1",
                "output_index": 0,
                "sequence_number": 9
            });

            let event: ResponseStreamEvent = serde_json::from_value(value).unwrap();
            let debug = format!("{event:?}");
            assert!(
                debug.starts_with(expected),
                "expected {expected}, got {debug}"
            );
            match event {
                ResponseStreamEvent::ImageGenerationCallInProgress {
                    item_id,
                    sequence_number,
                    ..
                }
                | ResponseStreamEvent::ImageGenerationCallGenerating {
                    item_id,
                    sequence_number,
                    ..
                }
                | ResponseStreamEvent::ImageGenerationCallCompleted {
                    item_id,
                    sequence_number,
                    ..
                } => {
                    assert_eq!(item_id, "ig_1");
                    assert_eq!(sequence_number, 9);
                }
                other => panic!("expected an ImageGenerationCall* variant, got {other:?}"),
            }
        }
    }

    #[test]
    fn response_stream_event_image_generation_call_partial_image_deserializes() {
        let value = serde_json::json!({
            "type": "response.image_generation_call.partial_image",
            "item_id": "ig_1",
            "output_index": 0,
            "partial_image_b64": "base64stuff",
            "partial_image_index": 0,
            "sequence_number": 10,
            "background": "transparent",
            "output_format": "png",
            "quality": "high",
            "size": "1024x1024"
        });

        let event: ResponseStreamEvent = serde_json::from_value(value).unwrap();
        match event {
            ResponseStreamEvent::ImageGenerationCallPartialImage {
                partial_image_b64,
                partial_image_index,
                background,
                output_format,
                ..
            } => {
                assert_eq!(partial_image_b64, "base64stuff");
                assert_eq!(partial_image_index, 0);
                assert_eq!(background.as_deref(), Some("transparent"));
                assert_eq!(output_format.as_deref(), Some("png"));
            }
            other => panic!("expected ImageGenerationCallPartialImage, got {other:?}"),
        }
    }

    #[test]
    fn response_stream_event_mcp_call_lifecycle_deserializes() {
        for (json_type, expected) in [
            ("response.mcp_call.in_progress", "McpCallInProgress"),
            ("response.mcp_call.completed", "McpCallCompleted"),
            ("response.mcp_call.failed", "McpCallFailed"),
            (
                "response.mcp_list_tools.in_progress",
                "McpListToolsInProgress",
            ),
            ("response.mcp_list_tools.completed", "McpListToolsCompleted"),
            ("response.mcp_list_tools.failed", "McpListToolsFailed"),
        ] {
            let value = serde_json::json!({
                "type": json_type,
                "item_id": "mcp_1",
                "output_index": 0,
                "sequence_number": 11
            });

            let event: ResponseStreamEvent = serde_json::from_value(value).unwrap();
            let debug = format!("{event:?}");
            assert!(
                debug.starts_with(expected),
                "expected {expected}, got {debug}"
            );
        }
    }

    #[test]
    fn response_stream_event_mcp_call_arguments_delta_and_done_deserialize() {
        let delta_value = serde_json::json!({
            "type": "response.mcp_call_arguments.delta",
            "item_id": "mcp_1",
            "output_index": 0,
            "delta": "{\"query\":",
            "sequence_number": 12
        });
        let event: ResponseStreamEvent = serde_json::from_value(delta_value).unwrap();
        match event {
            ResponseStreamEvent::McpCallArgumentsDelta { delta, .. } => {
                assert_eq!(delta, "{\"query\":");
            }
            other => panic!("expected McpCallArgumentsDelta, got {other:?}"),
        }

        let done_value = serde_json::json!({
            "type": "response.mcp_call_arguments.done",
            "item_id": "mcp_1",
            "output_index": 0,
            "arguments": "{\"query\":\"weather\"}",
            "sequence_number": 13
        });
        let event: ResponseStreamEvent = serde_json::from_value(done_value).unwrap();
        match event {
            ResponseStreamEvent::McpCallArgumentsDone { arguments, .. } => {
                assert_eq!(arguments, "{\"query\":\"weather\"}");
            }
            other => panic!("expected McpCallArgumentsDone, got {other:?}"),
        }
    }

    #[test]
    fn response_stream_event_code_interpreter_call_lifecycle_deserializes() {
        for (json_type, expected) in [
            (
                "response.code_interpreter_call.in_progress",
                "CodeInterpreterCallInProgress",
            ),
            (
                "response.code_interpreter_call.interpreting",
                "CodeInterpreterCallInterpreting",
            ),
            (
                "response.code_interpreter_call.completed",
                "CodeInterpreterCallCompleted",
            ),
        ] {
            let value = serde_json::json!({
                "type": json_type,
                "item_id": "ci_1",
                "output_index": 0,
                "sequence_number": 14
            });

            let event: ResponseStreamEvent = serde_json::from_value(value).unwrap();
            let debug = format!("{event:?}");
            assert!(
                debug.starts_with(expected),
                "expected {expected}, got {debug}"
            );
        }
    }

    #[test]
    fn response_stream_event_code_interpreter_call_code_delta_and_done_deserialize() {
        let delta_value = serde_json::json!({
            "type": "response.code_interpreter_call_code.delta",
            "item_id": "ci_1",
            "output_index": 0,
            "delta": "import pandas",
            "sequence_number": 15
        });
        let event: ResponseStreamEvent = serde_json::from_value(delta_value).unwrap();
        match event {
            ResponseStreamEvent::CodeInterpreterCallCodeDelta { delta, .. } => {
                assert_eq!(delta, "import pandas");
            }
            other => panic!("expected CodeInterpreterCallCodeDelta, got {other:?}"),
        }

        let done_value = serde_json::json!({
            "type": "response.code_interpreter_call_code.done",
            "item_id": "ci_1",
            "output_index": 0,
            "code": "import pandas as pd",
            "sequence_number": 16
        });
        let event: ResponseStreamEvent = serde_json::from_value(done_value).unwrap();
        match event {
            ResponseStreamEvent::CodeInterpreterCallCodeDone { code, .. } => {
                assert_eq!(code, "import pandas as pd");
            }
            other => panic!("expected CodeInterpreterCallCodeDone, got {other:?}"),
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

    #[test]
    fn response_conversation_param_serializes_bare_string_id() {
        let param = ResponseConversationParam::Id("conv_123".to_string());
        assert_eq!(
            serde_json::to_value(&param).unwrap(),
            serde_json::json!("conv_123")
        );
    }

    #[test]
    fn response_conversation_param_serializes_object_with_id() {
        let param = ResponseConversationParam::Object {
            id: "conv_123".to_string(),
        };
        assert_eq!(
            serde_json::to_value(&param).unwrap(),
            serde_json::json!({"id": "conv_123"})
        );
    }

    #[tokio::test]
    async fn create_serializes_conversation_as_bare_string() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/responses"))
            .and(body_json(serde_json::json!({
                "model": "gpt-5-mini",
                "input": "hi",
                "conversation": "conv_123"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "resp_1",
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
        let req = CreateResponseRequest::builder()
            .model("gpt-5-mini")
            .input(ResponseInput::Text("hi".to_string()))
            .conversation(ResponseConversationParam::Id("conv_123".to_string()))
            .build();

        let res = client.responses().create(&req).await.unwrap();
        assert_eq!(res.id, "resp_1");
    }

    #[tokio::test]
    async fn create_serializes_file_search_tool_and_deserializes_call_with_results() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/responses"))
            .and(body_json(serde_json::json!({
                "model": "gpt-5-mini",
                "input": "What does our refund policy say?",
                "tools": [{
                    "type": "file_search",
                    "vector_store_ids": ["vs_456"],
                    "max_num_results": 5,
                    "ranking_options": {
                        "ranker": "auto",
                        "score_threshold": 0.5
                    }
                }]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "resp_2",
                "object": "response",
                "created_at": 1700000000_i64,
                "status": "completed",
                "error": null,
                "incomplete_details": null,
                "model": "gpt-5-mini",
                "output": [{
                    "type": "file_search_call",
                    "id": "fs_1",
                    "queries": ["refund policy"],
                    "status": "completed",
                    "results": [{
                        "attributes": {"category": "billing"},
                        "file_id": "file-abc123",
                        "filename": "policy.md",
                        "score": 0.87,
                        "text": "Refunds are processed within 5 days."
                    }]
                }],
                "usage": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let tool = ResponseTool::FileSearch(
            ResponseFileSearchTool::builder()
                .vector_store_ids(vec!["vs_456".to_string()])
                .max_num_results(5_u32)
                .ranking_options(
                    FileSearchRankingOptions::builder()
                        .ranker("auto")
                        .score_threshold(0.5_f32)
                        .build(),
                )
                .build(),
        );
        let req = CreateResponseRequest::builder()
            .model("gpt-5-mini")
            .input(ResponseInput::Text(
                "What does our refund policy say?".to_string(),
            ))
            .tools(vec![tool])
            .build();

        let res = client.responses().create(&req).await.unwrap();
        match &res.output[0] {
            ResponseOutputItem::FileSearchCall(call) => {
                assert_eq!(call.id, "fs_1");
                assert_eq!(call.queries, vec!["refund policy".to_string()]);
                assert_eq!(call.status, "completed");
                let results = call.results.as_ref().expect("expected results");
                assert_eq!(results.len(), 1);
                assert_eq!(
                    results[0].text.as_deref(),
                    Some("Refunds are processed within 5 days.")
                );
            }
            other => panic!("expected FileSearchCall, got {other:?}"),
        }
    }

    #[test]
    fn response_file_search_call_deserializes_with_null_results() {
        let value = serde_json::json!({
            "type": "file_search_call",
            "id": "fs_2",
            "queries": ["deep research"],
            "status": "in_progress",
            "results": null
        });

        let item: ResponseOutputItem = serde_json::from_value(value).unwrap();
        match item {
            ResponseOutputItem::FileSearchCall(call) => {
                assert_eq!(call.status, "in_progress");
                assert!(call.results.is_none());
            }
            other => panic!("expected FileSearchCall, got {other:?}"),
        }
    }

    #[test]
    fn response_output_item_discriminates_message_and_file_search_call_in_same_array() {
        let value = serde_json::json!([
            {
                "type": "message",
                "id": "msg_1",
                "role": "assistant",
                "status": "completed",
                "content": [{"type": "output_text", "text": "Here's what I found.", "annotations": []}]
            },
            {
                "type": "file_search_call",
                "id": "fs_3",
                "queries": ["refund policy"],
                "status": "completed",
                "results": null
            }
        ]);

        let items: Vec<ResponseOutputItem> = serde_json::from_value(value).unwrap();
        assert_eq!(items.len(), 2);
        assert!(matches!(items[0], ResponseOutputItem::Message(_)));
        assert!(matches!(items[1], ResponseOutputItem::FileSearchCall(_)));
    }

    #[tokio::test]
    async fn create_serializes_web_search_tool_with_location_and_context_size() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/responses"))
            .and(body_json(serde_json::json!({
                "model": "gpt-5-mini",
                "input": "What's the weather in London right now?",
                "tools": [{
                    "type": "web_search",
                    "search_context_size": "low",
                    "user_location": {
                        "type": "approximate",
                        "city": "London",
                        "country": "GB"
                    }
                }]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "resp_3",
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
        let tool = ResponseTool::WebSearch(
            ResponseWebSearchTool::builder()
                .search_context_size(WebSearchContextSize::Low)
                .user_location(
                    WebSearchUserLocation::builder()
                        .kind("approximate")
                        .city("London")
                        .country("GB")
                        .build(),
                )
                .build(),
        );
        let req = CreateResponseRequest::builder()
            .model("gpt-5-mini")
            .input(ResponseInput::Text(
                "What's the weather in London right now?".to_string(),
            ))
            .tools(vec![tool])
            .build();

        let res = client.responses().create(&req).await.unwrap();
        assert_eq!(res.id, "resp_3");
    }

    #[tokio::test]
    async fn create_deserializes_web_search_call_with_search_action() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/responses"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "resp_4",
                "object": "response",
                "created_at": 1700000000_i64,
                "status": "completed",
                "error": null,
                "incomplete_details": null,
                "model": "gpt-5-mini",
                "output": [{
                    "type": "web_search_call",
                    "id": "ws_1",
                    "status": "completed",
                    "action": {
                        "type": "search",
                        "query": "latest news about AI"
                    }
                }],
                "usage": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateResponseRequest::builder()
            .model("gpt-5-mini")
            .input(ResponseInput::Text("Search for AI news".to_string()))
            .build();

        let res = client.responses().create(&req).await.unwrap();
        match &res.output[0] {
            ResponseOutputItem::WebSearchCall(call) => {
                assert_eq!(call.id, "ws_1");
                match &call.action {
                    WebSearchAction::Search { query, .. } => {
                        assert_eq!(query.as_deref(), Some("latest news about AI"));
                    }
                    other => panic!("expected Search action, got {other:?}"),
                }
            }
            other => panic!("expected WebSearchCall, got {other:?}"),
        }
    }

    #[test]
    fn web_search_action_open_page_deserializes() {
        let value = serde_json::json!({"type": "open_page", "url": "https://example.com"});
        let action: WebSearchAction = serde_json::from_value(value).unwrap();
        match action {
            WebSearchAction::OpenPage { url } => {
                assert_eq!(url.as_deref(), Some("https://example.com"));
            }
            other => panic!("expected OpenPage, got {other:?}"),
        }
    }

    #[test]
    fn web_search_action_find_in_page_deserializes_with_required_fields() {
        let value = serde_json::json!({
            "type": "find_in_page",
            "pattern": "climate change",
            "url": "https://example.com/article"
        });
        let action: WebSearchAction = serde_json::from_value(value).unwrap();
        match action {
            WebSearchAction::FindInPage { pattern, url } => {
                assert_eq!(pattern, "climate change");
                assert_eq!(url, "https://example.com/article");
            }
            other => panic!("expected FindInPage, got {other:?}"),
        }
    }

    #[test]
    fn response_output_item_discriminates_all_four_variants_in_same_array() {
        let value = serde_json::json!([
            {
                "type": "message",
                "id": "msg_1",
                "role": "assistant",
                "status": "completed",
                "content": [{"type": "output_text", "text": "Here's what I found.", "annotations": []}]
            },
            {
                "type": "function_call",
                "id": "fc_1",
                "call_id": "call_abc123",
                "name": "get_weather",
                "arguments": "{}",
                "status": "completed"
            },
            {
                "type": "file_search_call",
                "id": "fs_1",
                "queries": ["policy"],
                "status": "completed",
                "results": null
            },
            {
                "type": "web_search_call",
                "id": "ws_2",
                "status": "completed",
                "action": {"type": "search", "query": "weather"}
            }
        ]);

        let items: Vec<ResponseOutputItem> = serde_json::from_value(value).unwrap();
        assert_eq!(items.len(), 4);
        assert!(matches!(items[0], ResponseOutputItem::Message(_)));
        assert!(matches!(items[1], ResponseOutputItem::FunctionCall(_)));
        assert!(matches!(items[2], ResponseOutputItem::FileSearchCall(_)));
        assert!(matches!(items[3], ResponseOutputItem::WebSearchCall(_)));
    }

    #[tokio::test]
    async fn create_serializes_image_generation_tool_reusing_images_module_enums() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/responses"))
            .and(body_json(serde_json::json!({
                "model": "gpt-5-mini",
                "input": "Draw a gray tabby cat hugging an otter",
                "tools": [{
                    "type": "image_generation",
                    "action": "generate",
                    "background": "transparent",
                    "quality": "high",
                    "size": "1024x1024"
                }]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "resp_5",
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
        let tool = ResponseTool::ImageGeneration(
            ResponseImageGenerationTool::builder()
                .action(ImageGenerationAction::Generate)
                .background(Background::Transparent)
                .quality(Quality::High)
                .size(ImageSize::S1024x1024)
                .build(),
        );
        let req = CreateResponseRequest::builder()
            .model("gpt-5-mini")
            .input(ResponseInput::Text(
                "Draw a gray tabby cat hugging an otter".to_string(),
            ))
            .tools(vec![tool])
            .build();

        let res = client.responses().create(&req).await.unwrap();
        assert_eq!(res.id, "resp_5");
    }

    #[tokio::test]
    async fn create_serializes_mcp_tool_with_plain_require_approval() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/responses"))
            .and(body_json(serde_json::json!({
                "model": "gpt-5-mini",
                "input": "List the files in my repo",
                "tools": [{
                    "type": "mcp",
                    "server_label": "github",
                    "server_url": "https://mcp.example.com/github",
                    "headers": {"X-Api-Key": "secret"},
                    "require_approval": "never",
                    "allowed_tools": ["list_files", "read_file"]
                }]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "resp_7",
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
        let mut headers = std::collections::HashMap::new();
        headers.insert("X-Api-Key".to_string(), "secret".to_string());
        let tool = ResponseTool::Mcp(
            ResponseMcpTool::builder()
                .server_label("github")
                .server_url("https://mcp.example.com/github")
                .headers(headers)
                .require_approval(McpRequireApproval::Mode(McpApprovalMode::Never))
                .allowed_tools(vec!["list_files".to_string(), "read_file".to_string()])
                .build(),
        );
        let req = CreateResponseRequest::builder()
            .model("gpt-5-mini")
            .input(ResponseInput::Text("List the files in my repo".to_string()))
            .tools(vec![tool])
            .build();

        let res = client.responses().create(&req).await.unwrap();
        assert_eq!(res.id, "resp_7");
    }

    #[tokio::test]
    async fn create_serializes_mcp_tool_with_filter_require_approval() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/responses"))
            .and(body_json(serde_json::json!({
                "model": "gpt-5-mini",
                "input": "Read a file",
                "tools": [{
                    "type": "mcp",
                    "server_label": "github",
                    "server_url": "https://mcp.example.com/github",
                    "require_approval": {
                        "never": {"tool_names": ["read_file"]}
                    }
                }]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "resp_8",
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
        let tool = ResponseTool::Mcp(
            ResponseMcpTool::builder()
                .server_label("github")
                .server_url("https://mcp.example.com/github")
                .require_approval(McpRequireApproval::Filter(McpApprovalFilter {
                    never: Some(McpToolNameFilter {
                        tool_names: vec!["read_file".to_string()],
                    }),
                    always: None,
                }))
                .build(),
        );
        let req = CreateResponseRequest::builder()
            .model("gpt-5-mini")
            .input(ResponseInput::Text("Read a file".to_string()))
            .tools(vec![tool])
            .build();

        let res = client.responses().create(&req).await.unwrap();
        assert_eq!(res.id, "resp_8");
    }

    #[tokio::test]
    async fn create_serializes_computer_tool_as_bare_tag() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/responses"))
            .and(body_json(serde_json::json!({
                "model": "gpt-5-mini",
                "input": "Take a screenshot",
                "tools": [{"type": "computer"}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "resp_10",
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
        let req = CreateResponseRequest::builder()
            .model("gpt-5-mini")
            .input(ResponseInput::Text("Take a screenshot".to_string()))
            .tools(vec![ResponseTool::Computer])
            .build();

        let res = client.responses().create(&req).await.unwrap();
        assert_eq!(res.id, "resp_10");
    }

    #[tokio::test]
    async fn create_serializes_computer_use_preview_tool() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/responses"))
            .and(body_json(serde_json::json!({
                "model": "gpt-5-mini",
                "input": "Take a screenshot",
                "tools": [{
                    "type": "computer_use_preview",
                    "display_width": 1024,
                    "display_height": 768,
                    "environment": "browser"
                }]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "resp_11",
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
        let tool = ResponseTool::ComputerUsePreview(
            ResponseComputerUsePreviewTool::builder()
                .display_width(1024)
                .display_height(768)
                .environment(ComputerEnvironment::Browser)
                .build(),
        );
        let req = CreateResponseRequest::builder()
            .model("gpt-5-mini")
            .input(ResponseInput::Text("Take a screenshot".to_string()))
            .tools(vec![tool])
            .build();

        let res = client.responses().create(&req).await.unwrap();
        assert_eq!(res.id, "resp_11");
    }

    #[tokio::test]
    async fn create_serializes_code_interpreter_tool_with_container_id() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/responses"))
            .and(body_json(serde_json::json!({
                "model": "gpt-5-mini",
                "input": "Compute the mean of [1, 2, 3]",
                "tools": [{
                    "type": "code_interpreter",
                    "container": "cntr_abc123"
                }]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "resp_13",
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
        let tool = ResponseTool::CodeInterpreter(
            ResponseCodeInterpreterTool::builder()
                .container(ResponseCodeInterpreterContainer::Id(
                    "cntr_abc123".to_string(),
                ))
                .build(),
        );
        let req = CreateResponseRequest::builder()
            .model("gpt-5-mini")
            .input(ResponseInput::Text(
                "Compute the mean of [1, 2, 3]".to_string(),
            ))
            .tools(vec![tool])
            .build();

        let res = client.responses().create(&req).await.unwrap();
        assert_eq!(res.id, "resp_13");
    }

    #[tokio::test]
    async fn create_serializes_code_interpreter_tool_with_auto_container() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/responses"))
            .and(body_json(serde_json::json!({
                "model": "gpt-5-mini",
                "input": "Compute the mean of [1, 2, 3]",
                "tools": [{
                    "type": "code_interpreter",
                    "container": {
                        "type": "auto",
                        "memory_limit": "4g",
                        "file_ids": ["file_1", "file_2"]
                    }
                }]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "resp_14",
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
        let tool = ResponseTool::CodeInterpreter(
            ResponseCodeInterpreterTool::builder()
                .container(ResponseCodeInterpreterContainer::Auto(
                    ResponseCodeInterpreterAutoContainer::builder()
                        .kind("auto")
                        .memory_limit(ContainerMemoryLimit::FourGb)
                        .file_ids(vec!["file_1".to_string(), "file_2".to_string()])
                        .build(),
                ))
                .build(),
        );
        let req = CreateResponseRequest::builder()
            .model("gpt-5-mini")
            .input(ResponseInput::Text(
                "Compute the mean of [1, 2, 3]".to_string(),
            ))
            .tools(vec![tool])
            .build();

        let res = client.responses().create(&req).await.unwrap();
        assert_eq!(res.id, "resp_14");
    }

    #[tokio::test]
    async fn create_deserializes_code_interpreter_call() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/responses"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "resp_15",
                "object": "response",
                "created_at": 1700000000_i64,
                "status": "completed",
                "error": null,
                "incomplete_details": null,
                "model": "gpt-5-mini",
                "output": [{
                    "type": "code_interpreter_call",
                    "id": "ci_1",
                    "code": "print(sum([1, 2, 3]) / 3)",
                    "container_id": "cntr_abc123",
                    "status": "completed"
                }],
                "usage": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateResponseRequest::builder()
            .model("gpt-5-mini")
            .input(ResponseInput::Text(
                "Compute the mean of [1, 2, 3]".to_string(),
            ))
            .build();

        let res = client.responses().create(&req).await.unwrap();
        match &res.output[0] {
            ResponseOutputItem::CodeInterpreterCall(call) => {
                assert_eq!(call.id, "ci_1");
                assert_eq!(call.code, "print(sum([1, 2, 3]) / 3)");
                assert_eq!(call.container_id, "cntr_abc123");
                assert_eq!(call.status, "completed");
            }
            other => panic!("expected CodeInterpreterCall, got {other:?}"),
        }
    }

    #[test]
    fn response_output_item_discriminates_code_interpreter_call_in_same_array() {
        let value = serde_json::json!([
            {
                "type": "message",
                "id": "msg_3",
                "role": "assistant",
                "status": "completed",
                "content": [{"type": "output_text", "text": "Done.", "annotations": []}]
            },
            {
                "type": "code_interpreter_call",
                "id": "ci_2",
                "code": "1 + 1",
                "container_id": "cntr_xyz",
                "status": "completed"
            }
        ]);

        let items: Vec<ResponseOutputItem> = serde_json::from_value(value).unwrap();
        assert_eq!(items.len(), 2);
        assert!(matches!(items[0], ResponseOutputItem::Message(_)));
        assert!(matches!(
            items[1],
            ResponseOutputItem::CodeInterpreterCall(_)
        ));
    }

    #[test]
    fn response_code_interpreter_container_deserializes_both_untagged_forms() {
        let id_form: ResponseCodeInterpreterContainer =
            serde_json::from_value(serde_json::json!("cntr_abc123")).unwrap();
        assert!(
            matches!(id_form, ResponseCodeInterpreterContainer::Id(ref id) if id == "cntr_abc123")
        );

        let auto_form: ResponseCodeInterpreterContainer =
            serde_json::from_value(serde_json::json!({
                "type": "auto",
                "memory_limit": "4g",
                "file_ids": ["file_1"]
            }))
            .unwrap();
        match auto_form {
            ResponseCodeInterpreterContainer::Auto(container) => {
                assert_eq!(container.kind, "auto");
                assert!(matches!(
                    container.memory_limit,
                    Some(crate::interfaces::containers::ContainerMemoryLimit::FourGb)
                ));
                assert_eq!(container.file_ids, Some(vec!["file_1".to_string()]));
            }
            other => panic!("expected Auto, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn create_deserializes_completed_image_generation_call() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/responses"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "resp_6",
                "object": "response",
                "created_at": 1700000000_i64,
                "status": "completed",
                "error": null,
                "incomplete_details": null,
                "model": "gpt-5-mini",
                "output": [{
                    "type": "image_generation_call",
                    "id": "ig_1",
                    "status": "completed",
                    "revised_prompt": "A gray tabby cat hugging an otter, warm heartwarming style.",
                    "result": "base64imagedatahere"
                }],
                "usage": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateResponseRequest::builder()
            .model("gpt-5-mini")
            .input(ResponseInput::Text("Draw a cat".to_string()))
            .build();

        let res = client.responses().create(&req).await.unwrap();
        match &res.output[0] {
            ResponseOutputItem::ImageGenerationCall(call) => {
                assert_eq!(call.id, "ig_1");
                assert_eq!(call.status, "completed");
                assert_eq!(call.result.as_deref(), Some("base64imagedatahere"));
                assert_eq!(
                    call.revised_prompt.as_deref(),
                    Some("A gray tabby cat hugging an otter, warm heartwarming style.")
                );
            }
            other => panic!("expected ImageGenerationCall, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn create_deserializes_completed_mcp_call() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/responses"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "resp_9",
                "object": "response",
                "created_at": 1700000000_i64,
                "status": "completed",
                "error": null,
                "incomplete_details": null,
                "model": "gpt-5-mini",
                "output": [{
                    "type": "mcp_call",
                    "id": "mcp_1",
                    "server_label": "github",
                    "name": "list_files",
                    "arguments": "{\"path\":\"/\"}",
                    "output": "[\"README.md\", \"src/\"]",
                    "error": null,
                    "approval_request_id": null
                }],
                "usage": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateResponseRequest::builder()
            .model("gpt-5-mini")
            .input(ResponseInput::Text("List files".to_string()))
            .build();

        let res = client.responses().create(&req).await.unwrap();
        match &res.output[0] {
            ResponseOutputItem::McpCall(call) => {
                assert_eq!(call.id, "mcp_1");
                assert_eq!(call.server_label, "github");
                assert_eq!(call.name, "list_files");
                assert_eq!(call.arguments, "{\"path\":\"/\"}");
                assert_eq!(call.output.as_deref(), Some("[\"README.md\", \"src/\"]"));
                assert!(call.error.is_none());
                assert!(call.approval_request_id.is_none());
            }
            other => panic!("expected McpCall, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn create_deserializes_computer_call_with_actions_array() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/responses"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "resp_12",
                "object": "response",
                "created_at": 1700000000_i64,
                "status": "completed",
                "error": null,
                "incomplete_details": null,
                "model": "gpt-5-mini",
                "output": [{
                    "type": "computer_call",
                    "id": "cu_1",
                    "call_id": "call_cu_1",
                    "status": "completed",
                    "action": null,
                    "actions": [
                        {"type": "screenshot"},
                        {"type": "click", "x": 405, "y": 157, "button": "left"}
                    ],
                    "pending_safety_checks": []
                }],
                "usage": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateResponseRequest::builder()
            .model("gpt-5-mini")
            .input(ResponseInput::Text("Take a screenshot".to_string()))
            .build();

        let res = client.responses().create(&req).await.unwrap();
        match &res.output[0] {
            ResponseOutputItem::ComputerCall(call) => {
                assert_eq!(call.id, "cu_1");
                assert_eq!(call.call_id, "call_cu_1");
                assert_eq!(call.status, "completed");
                assert!(call.action.is_none());
                let actions = call.actions.as_ref().expect("actions array");
                assert_eq!(actions.len(), 2);
                assert!(matches!(actions[0], ComputerAction::Screenshot {}));
                match &actions[1] {
                    ComputerAction::Click { x, y, button, keys } => {
                        assert_eq!(*x, 405);
                        assert_eq!(*y, 157);
                        assert_eq!(*button, Some(ComputerActionButton::Left));
                        assert!(keys.is_none());
                    }
                    other => panic!("expected Click, got {other:?}"),
                }
                assert_eq!(call.pending_safety_checks.as_ref().unwrap().len(), 0);
            }
            other => panic!("expected ComputerCall, got {other:?}"),
        }
    }

    #[test]
    fn response_computer_call_deserializes_with_pending_safety_checks() {
        let value = serde_json::json!({
            "type": "computer_call",
            "id": "cu_2",
            "call_id": "call_cu_2",
            "status": "completed",
            "action": {"type": "type", "text": "hello"},
            "actions": null,
            "pending_safety_checks": [
                {"id": "sc_1", "code": "malicious_instructions", "message": "Detected a potentially malicious instruction."}
            ]
        });

        let item: ResponseOutputItem = serde_json::from_value(value).unwrap();
        match item {
            ResponseOutputItem::ComputerCall(call) => {
                assert_eq!(call.id, "cu_2");
                assert!(call.actions.is_none());
                match call.action.as_ref().unwrap() {
                    ComputerAction::Type { text } => assert_eq!(text, "hello"),
                    other => panic!("expected Type, got {other:?}"),
                }
                let checks = call.pending_safety_checks.as_ref().unwrap();
                assert_eq!(checks.len(), 1);
                assert_eq!(checks[0].id, "sc_1");
                assert_eq!(checks[0].code, "malicious_instructions");
            }
            other => panic!("expected ComputerCall, got {other:?}"),
        }
    }

    #[test]
    fn computer_action_drag_deserializes_path_as_coordinate_pairs() {
        let value = serde_json::json!({
            "type": "drag",
            "path": [[100, 200], [150, 250], [200, 300]],
            "keys": ["CTRL"]
        });

        let action: ComputerAction = serde_json::from_value(value).unwrap();
        match action {
            ComputerAction::Drag { path, keys } => {
                assert_eq!(path, vec![(100, 200), (150, 250), (200, 300)]);
                assert_eq!(keys, Some(vec!["CTRL".to_string()]));
            }
            other => panic!("expected Drag, got {other:?}"),
        }
    }

    #[test]
    fn response_mcp_list_tools_deserializes_with_two_tools() {
        let value = serde_json::json!({
            "type": "mcp_list_tools",
            "id": "mcpl_1",
            "server_label": "github",
            "tools": [
                {
                    "name": "list_files",
                    "description": "List files in a directory",
                    "input_schema": {"type": "object", "properties": {"path": {"type": "string"}}},
                    "annotations": null
                },
                {
                    "name": "read_file",
                    "description": null,
                    "input_schema": {"type": "object"},
                    "annotations": {"readOnly": true}
                }
            ]
        });

        let item: ResponseOutputItem = serde_json::from_value(value).unwrap();
        match item {
            ResponseOutputItem::McpListTools(list) => {
                assert_eq!(list.id, "mcpl_1");
                assert_eq!(list.server_label, "github");
                assert_eq!(list.tools.len(), 2);
                assert_eq!(list.tools[0].name, "list_files");
                assert_eq!(
                    list.tools[0].description.as_deref(),
                    Some("List files in a directory")
                );
                assert!(list.tools[1].description.is_none());
                assert_eq!(
                    list.tools[1].annotations,
                    Some(serde_json::json!({"readOnly": true}))
                );
            }
            other => panic!("expected McpListTools, got {other:?}"),
        }
    }

    #[test]
    fn response_mcp_approval_request_deserializes() {
        let value = serde_json::json!({
            "type": "mcp_approval_request",
            "id": "mcpr_1",
            "server_label": "github",
            "name": "delete_file",
            "arguments": "{\"path\":\"/tmp/x\"}"
        });

        let item: ResponseOutputItem = serde_json::from_value(value).unwrap();
        match item {
            ResponseOutputItem::McpApprovalRequest(req) => {
                assert_eq!(req.id, "mcpr_1");
                assert_eq!(req.server_label, "github");
                assert_eq!(req.name, "delete_file");
                assert_eq!(req.arguments, "{\"path\":\"/tmp/x\"}");
            }
            other => panic!("expected McpApprovalRequest, got {other:?}"),
        }
    }

    #[test]
    fn response_image_generation_call_deserializes_with_null_result_and_prompt() {
        let value = serde_json::json!({
            "type": "image_generation_call",
            "id": "ig_2",
            "status": "in_progress",
            "result": null,
            "revised_prompt": null
        });

        let item: ResponseOutputItem = serde_json::from_value(value).unwrap();
        match item {
            ResponseOutputItem::ImageGenerationCall(call) => {
                assert_eq!(call.status, "in_progress");
                assert!(call.result.is_none());
                assert!(call.revised_prompt.is_none());
            }
            other => panic!("expected ImageGenerationCall, got {other:?}"),
        }
    }

    #[test]
    fn response_output_item_discriminates_all_five_variants_in_same_array() {
        let value = serde_json::json!([
            {
                "type": "message",
                "id": "msg_1",
                "role": "assistant",
                "status": "completed",
                "content": [{"type": "output_text", "text": "Here you go.", "annotations": []}]
            },
            {
                "type": "function_call",
                "id": "fc_1",
                "call_id": "call_abc123",
                "name": "get_weather",
                "arguments": "{}",
                "status": "completed"
            },
            {
                "type": "file_search_call",
                "id": "fs_1",
                "queries": ["policy"],
                "status": "completed",
                "results": null
            },
            {
                "type": "web_search_call",
                "id": "ws_2",
                "status": "completed",
                "action": {"type": "search", "query": "weather"}
            },
            {
                "type": "image_generation_call",
                "id": "ig_3",
                "status": "completed",
                "result": "abc",
                "revised_prompt": null
            }
        ]);

        let items: Vec<ResponseOutputItem> = serde_json::from_value(value).unwrap();
        assert_eq!(items.len(), 5);
        assert!(matches!(items[0], ResponseOutputItem::Message(_)));
        assert!(matches!(items[1], ResponseOutputItem::FunctionCall(_)));
        assert!(matches!(items[2], ResponseOutputItem::FileSearchCall(_)));
        assert!(matches!(items[3], ResponseOutputItem::WebSearchCall(_)));
        assert!(matches!(
            items[4],
            ResponseOutputItem::ImageGenerationCall(_)
        ));
    }

    #[test]
    fn response_output_item_discriminates_mcp_variants_in_same_array() {
        let value = serde_json::json!([
            {
                "type": "mcp_list_tools",
                "id": "mcpl_2",
                "server_label": "github",
                "tools": []
            },
            {
                "type": "mcp_call",
                "id": "mcp_2",
                "server_label": "github",
                "name": "list_files",
                "arguments": "{}",
                "output": "[]",
                "error": null,
                "approval_request_id": null
            },
            {
                "type": "mcp_approval_request",
                "id": "mcpr_2",
                "server_label": "github",
                "name": "delete_file",
                "arguments": "{}"
            }
        ]);

        let items: Vec<ResponseOutputItem> = serde_json::from_value(value).unwrap();
        assert_eq!(items.len(), 3);
        assert!(matches!(items[0], ResponseOutputItem::McpListTools(_)));
        assert!(matches!(items[1], ResponseOutputItem::McpCall(_)));
        assert!(matches!(
            items[2],
            ResponseOutputItem::McpApprovalRequest(_)
        ));
    }

    #[test]
    fn response_output_item_discriminates_computer_call_in_same_array() {
        let value = serde_json::json!([
            {
                "type": "message",
                "id": "msg_2",
                "role": "assistant",
                "status": "completed",
                "content": [{"type": "output_text", "text": "Done.", "annotations": []}]
            },
            {
                "type": "computer_call",
                "id": "cu_3",
                "call_id": "call_cu_3",
                "status": "completed",
                "action": null,
                "actions": [{"type": "wait"}],
                "pending_safety_checks": null
            }
        ]);

        let items: Vec<ResponseOutputItem> = serde_json::from_value(value).unwrap();
        assert_eq!(items.len(), 2);
        assert!(matches!(items[0], ResponseOutputItem::Message(_)));
        assert!(matches!(items[1], ResponseOutputItem::ComputerCall(_)));
    }
}
