//! Given a chat conversation, the model will return a chat completion response.

use crate::client::OpenAI;
use crate::interfaces::chat;
pub use crate::interfaces::chat::*;
use crate::shared::response_wrapper::{OpenAIError, OpenAIResponse};
use crate::shared::utils::is_stream;
use futures::Stream;
use std::pin::Pin;

pub struct Chat<'a> {
    openai: &'a OpenAI,
}

impl<'a> Chat<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates a completion for the chat message.
    pub async fn create(
        &self,
        req: &chat::CreateChatRequest,
    ) -> OpenAIResponse<chat::ChatResponse> {
        if is_stream(req.stream) {
            return Err(OpenAIError::InvalidArgument(
                "When stream is true, use Chat::create_with_stream".into(),
            ));
        }

        self.openai.post("/chat/completions", req).await
    }

    /// Creates a completion for the chat message.
    pub async fn create_with_stream(
        &self,
        req: &chat::CreateChatRequest,
    ) -> Result<
        Pin<Box<dyn Stream<Item = OpenAIResponse<chat::ChatStreamResponse>> + Send>>,
        OpenAIError,
    > {
        if !is_stream(req.stream) {
            return Err(OpenAIError::InvalidArgument(
                "When stream is false, use Chat::create".into(),
            ));
        }

        self.openai.post_stream("/chat/completions", req).await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::chat::{
        ChatCompletionMessage, ChatCompletionTool, CreateChatRequest, FunctionCall,
        FunctionDefinition, Role, ToolCall, ToolChoice, ToolChoiceMode,
    };
    use crate::shared::response_wrapper::OpenAIError;
    use futures::StreamExt;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_json(serde_json::json!({
                "model": "gpt-4o-mini",
                "messages": [{"role": "user", "content": "hi", "name": null, "tool_calls": null, "tool_call_id": null}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-123",
                "object": "chat.completion",
                "created": 1700000000_u32,
                "choices": [{
                    "message": {"role": "assistant", "content": "Hello!", "name": null},
                    "finish_reason": "stop",
                    "index": 0
                }],
                "usage": {"prompt_tokens": 5, "completion_tokens": 2, "total_tokens": 7}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateChatRequest::builder()
            .model("gpt-4o-mini")
            .messages(vec![
                ChatCompletionMessage::builder()
                    .role(Role::User)
                    .content("hi")
                    .build(),
            ])
            .build();

        let res = client.chat().create(&req).await.unwrap();
        assert_eq!(res.choices[0].message.content.as_deref(), Some("Hello!"));
    }

    #[tokio::test]
    async fn create_rejects_stream_true() {
        let client = OpenAI::new("sk-test");
        let req = CreateChatRequest::builder()
            .model("gpt-4o-mini")
            .messages(vec![
                ChatCompletionMessage::builder()
                    .role(Role::User)
                    .content("hi")
                    .build(),
            ])
            .stream(true)
            .build();

        let err = client.chat().create(&req).await.unwrap_err();
        assert!(matches!(err, OpenAIError::InvalidArgument(_)));
    }

    #[tokio::test]
    async fn create_with_stream_rejects_stream_false() {
        let client = OpenAI::new("sk-test");
        let req = CreateChatRequest::builder()
            .model("gpt-4o-mini")
            .messages(vec![
                ChatCompletionMessage::builder()
                    .role(Role::User)
                    .content("hi")
                    .build(),
            ])
            .build();

        let err = client.chat().create_with_stream(&req).await.err().unwrap();
        assert!(matches!(err, OpenAIError::InvalidArgument(_)));
    }

    #[tokio::test]
    async fn create_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "error": {
                    "message": "Incorrect API key provided",
                    "type": "invalid_request_error",
                    "param": null,
                    "code": "invalid_api_key"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateChatRequest::builder()
            .model("gpt-4o-mini")
            .messages(vec![
                ChatCompletionMessage::builder()
                    .role(Role::User)
                    .content("hi")
                    .build(),
            ])
            .build();

        let err = client.chat().create(&req).await.unwrap_err();
        match err {
            OpenAIError::ApiError(e) => {
                assert_eq!(e.error.code, Some(serde_json::json!("invalid_api_key")));
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }

    /// End-to-end coverage of the SSE streaming happy path: mounts a `text/event-stream`
    /// response with several `data: {...}` chunks followed by `data: [DONE]`, drives the
    /// stream returned by `create_with_stream` to completion with `StreamExt::next`, and
    /// asserts the deltas and item count match, and that the stream ends cleanly (no
    /// trailing item, no error) right after `[DONE]`.
    #[tokio::test]
    async fn create_with_stream_yields_deltas_and_terminates_after_done() {
        let mock_server = MockServer::start().await;

        let chunk1 = serde_json::json!({
            "id": "chatcmpl-123",
            "object": "chat.completion.chunk",
            "model": "gpt-4o-mini",
            "created": 1700000000_u32,
            "choices": [{
                "delta": {"content": "Hello"},
                "finish_reason": null,
                "index": 0
            }]
        });
        let chunk2 = serde_json::json!({
            "id": "chatcmpl-123",
            "object": "chat.completion.chunk",
            "model": "gpt-4o-mini",
            "created": 1700000000_u32,
            "choices": [{
                "delta": {"content": " world"},
                "finish_reason": null,
                "index": 0
            }]
        });
        let chunk3 = serde_json::json!({
            "id": "chatcmpl-123",
            "object": "chat.completion.chunk",
            "model": "gpt-4o-mini",
            "created": 1700000000_u32,
            "choices": [{
                "delta": {"content": null},
                "finish_reason": "stop",
                "index": 0
            }]
        });

        let sse_body =
            format!("data: {chunk1}\n\ndata: {chunk2}\n\ndata: {chunk3}\n\ndata: [DONE]\n\n",);

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(sse_body.into_bytes(), "text/event-stream"),
            )
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateChatRequest::builder()
            .model("gpt-4o-mini")
            .messages(vec![
                ChatCompletionMessage::builder()
                    .role(Role::User)
                    .content("hi")
                    .build(),
            ])
            .stream(true)
            .build();

        let mut stream = client.chat().create_with_stream(&req).await.unwrap();

        let mut received = Vec::new();
        while let Some(item) = stream.next().await {
            received.push(item.unwrap());
        }

        assert_eq!(received.len(), 3, "expected exactly 3 items before [DONE]");
        assert_eq!(
            received[0].choices[0].delta.content.as_deref(),
            Some("Hello")
        );
        assert_eq!(
            received[1].choices[0].delta.content.as_deref(),
            Some(" world")
        );
        assert_eq!(
            received[2].choices[0].finish_reason.as_deref(),
            Some("stop")
        );

        // The stream must be exhausted (no trailing item after [DONE]).
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn create_serializes_tools_and_tool_result_message() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_json(serde_json::json!({
                "model": "gpt-4o-mini",
                "messages": [
                    {"role": "user", "content": "What's the weather in Boston?", "name": null, "tool_calls": null, "tool_call_id": null},
                    {
                        "role": "assistant",
                        "content": null,
                        "name": null,
                        "tool_calls": [{
                            "id": "call_abc123",
                            "type": "function",
                            "function": {"name": "get_weather", "arguments": "{\"location\":\"Boston\"}"}
                        }],
                        "tool_call_id": null
                    },
                    {"role": "tool", "content": "72F and sunny", "name": null, "tool_calls": null, "tool_call_id": "call_abc123"}
                ],
                "tools": [{
                    "type": "function",
                    "function": {
                        "name": "get_weather",
                        "description": "Get the current weather for a location",
                        "parameters": {"type": "object", "properties": {"location": {"type": "string"}}, "required": ["location"]}
                    }
                }],
                "tool_choice": "auto",
                "parallel_tool_calls": false
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-123",
                "object": "chat.completion",
                "created": 1700000000_u32,
                "choices": [{
                    "message": {"role": "assistant", "content": "It's 72F and sunny in Boston.", "name": null, "tool_calls": null, "tool_call_id": null},
                    "finish_reason": "stop",
                    "index": 0
                }],
                "usage": {"prompt_tokens": 20, "completion_tokens": 10, "total_tokens": 30}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());

        let tool = ChatCompletionTool::builder()
            .function(
                FunctionDefinition::builder()
                    .name("get_weather")
                    .description("Get the current weather for a location")
                    .parameters(serde_json::json!({
                        "type": "object",
                        "properties": {"location": {"type": "string"}},
                        "required": ["location"]
                    }))
                    .build(),
            )
            .build();

        let req = CreateChatRequest::builder()
            .model("gpt-4o-mini")
            .messages(vec![
                ChatCompletionMessage::builder()
                    .role(Role::User)
                    .content("What's the weather in Boston?")
                    .build(),
                ChatCompletionMessage::builder()
                    .role(Role::Assistant)
                    .tool_calls(vec![ToolCall {
                        id: "call_abc123".to_string(),
                        kind: "function".to_string(),
                        function: FunctionCall {
                            name: "get_weather".to_string(),
                            arguments: r#"{"location":"Boston"}"#.to_string(),
                        },
                    }])
                    .build(),
                ChatCompletionMessage::builder()
                    .role(Role::Tool)
                    .content("72F and sunny")
                    .tool_call_id("call_abc123")
                    .build(),
            ])
            .tools(vec![tool])
            .tool_choice(ToolChoice::Mode(ToolChoiceMode::Auto))
            .parallel_tool_calls(false)
            .build();

        let res = client.chat().create(&req).await.unwrap();
        assert_eq!(
            res.choices[0].message.content.as_deref(),
            Some("It's 72F and sunny in Boston.")
        );
    }

    #[tokio::test]
    async fn create_deserializes_response_with_null_content_and_tool_calls() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-456",
                "object": "chat.completion",
                "created": 1700000000_u32,
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": null,
                        "tool_calls": [{
                            "id": "call_xyz",
                            "type": "function",
                            "function": {"name": "get_weather", "arguments": "{\"location\":\"NYC\"}"}
                        }]
                    },
                    "finish_reason": "tool_calls",
                    "index": 0
                }],
                "usage": {"prompt_tokens": 12, "completion_tokens": 8, "total_tokens": 20}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateChatRequest::builder()
            .model("gpt-4o-mini")
            .messages(vec![
                ChatCompletionMessage::builder()
                    .role(Role::User)
                    .content("What's the weather in NYC?")
                    .build(),
            ])
            .build();

        let res = client.chat().create(&req).await.unwrap();
        let message = &res.choices[0].message;
        assert_eq!(message.content, None);
        let tool_calls = message
            .tool_calls
            .as_ref()
            .expect("expected tool_calls to be present");
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "call_xyz");
        assert_eq!(tool_calls[0].function.name, "get_weather");
        assert_eq!(tool_calls[0].function.arguments, r#"{"location":"NYC"}"#);
    }
}
