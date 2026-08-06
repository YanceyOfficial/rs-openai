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

    /// Retrieves a stored chat completion. Only completions created with `store: true`
    /// are retrievable.
    pub async fn retrieve(&self, completion_id: &str) -> OpenAIResponse<chat::ChatResponse> {
        self.openai
            .get(&format!("/chat/completions/{completion_id}"), &())
            .await
    }

    /// Lists stored chat completions.
    pub async fn list(
        &self,
        req: &chat::ListChatCompletionsRequest,
    ) -> OpenAIResponse<chat::ListChatCompletionsResponse> {
        self.openai.get("/chat/completions", req).await
    }

    /// Deletes a stored chat completion.
    pub async fn delete(
        &self,
        completion_id: &str,
    ) -> OpenAIResponse<chat::DeleteChatCompletionResponse> {
        self.openai
            .delete(&format!("/chat/completions/{completion_id}"), &())
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::chat::{
        ChatCompletionMessage, ChatCompletionTool, ChatReasoningEffort, ChatResponseFormat,
        CreateChatRequest, FunctionCall, FunctionDefinition, JsonSchemaFormat,
        ListChatCompletionsRequest, Role, ServiceTier, SortOrder, ToolCall, ToolChoice,
        ToolChoiceMode,
    };
    use crate::shared::response_wrapper::OpenAIError;
    use crate::shared::types::StreamOptions;
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
                "messages": [{"role": "user", "content": "hi", "name": null, "tool_calls": null, "tool_call_id": null, "refusal": null}]
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
                    {"role": "user", "content": "What's the weather in Boston?", "name": null, "tool_calls": null, "tool_call_id": null, "refusal": null},
                    {
                        "role": "assistant",
                        "content": null,
                        "name": null,
                        "tool_calls": [{
                            "id": "call_abc123",
                            "type": "function",
                            "function": {"name": "get_weather", "arguments": "{\"location\":\"Boston\"}"}
                        }],
                        "tool_call_id": null,
                        "refusal": null
                    },
                    {"role": "tool", "content": "72F and sunny", "name": null, "tool_calls": null, "tool_call_id": "call_abc123", "refusal": null}
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

    #[tokio::test]
    async fn create_with_stream_yields_tool_call_deltas() {
        let mock_server = MockServer::start().await;

        let chunk1 = serde_json::json!({
            "id": "chatcmpl-789",
            "object": "chat.completion.chunk",
            "model": "gpt-4o-mini",
            "created": 1700000000_u32,
            "choices": [{
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "id": "call_stream_1",
                        "type": "function",
                        "function": {"name": "get_weather", "arguments": ""}
                    }]
                },
                "finish_reason": null,
                "index": 0
            }]
        });
        // Realistic continuation chunks: only `index` + an `arguments` fragment, no
        // id/type/function.name — this is the shape that was previously broken.
        let chunk2 = serde_json::json!({
            "id": "chatcmpl-789",
            "object": "chat.completion.chunk",
            "model": "gpt-4o-mini",
            "created": 1700000000_u32,
            "choices": [{
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "function": {"arguments": "{\"location\":"}
                    }]
                },
                "finish_reason": null,
                "index": 0
            }]
        });
        let chunk3 = serde_json::json!({
            "id": "chatcmpl-789",
            "object": "chat.completion.chunk",
            "model": "gpt-4o-mini",
            "created": 1700000000_u32,
            "choices": [{
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "function": {"arguments": "\"Boston\"}"}
                    }]
                },
                "finish_reason": null,
                "index": 0
            }]
        });
        let chunk4 = serde_json::json!({
            "id": "chatcmpl-789",
            "object": "chat.completion.chunk",
            "model": "gpt-4o-mini",
            "created": 1700000000_u32,
            "choices": [{
                "delta": {},
                "finish_reason": "tool_calls",
                "index": 0
            }]
        });

        let sse_body = format!(
            "data: {chunk1}\n\ndata: {chunk2}\n\ndata: {chunk3}\n\ndata: {chunk4}\n\ndata: [DONE]\n\n",
        );

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
                    .content("weather?")
                    .build(),
            ])
            .stream(true)
            .build();

        let mut stream = client.chat().create_with_stream(&req).await.unwrap();

        let mut received = Vec::new();
        while let Some(item) = stream.next().await {
            received.push(item.unwrap());
        }

        assert_eq!(received.len(), 4);

        let first = &received[0].choices[0].delta.tool_calls.as_ref().unwrap()[0];
        assert_eq!(first.index, 0);
        assert_eq!(first.id.as_deref(), Some("call_stream_1"));
        assert_eq!(first.kind.as_deref(), Some("function"));
        let first_fn = first.function.as_ref().unwrap();
        assert_eq!(first_fn.name.as_deref(), Some("get_weather"));
        assert_eq!(first_fn.arguments, "");

        // The realistic, previously-broken continuation shape: no id/type/name, only a
        // function.arguments fragment.
        let second = &received[1].choices[0].delta.tool_calls.as_ref().unwrap()[0];
        assert_eq!(second.index, 0);
        assert_eq!(second.id, None);
        assert_eq!(second.kind, None);
        let second_fn = second.function.as_ref().unwrap();
        assert_eq!(second_fn.name, None);
        assert_eq!(second_fn.arguments, "{\"location\":");

        let third = &received[2].choices[0].delta.tool_calls.as_ref().unwrap()[0];
        assert_eq!(third.function.as_ref().unwrap().arguments, "\"Boston\"}");

        // Terminal chunk: empty delta, finish_reason set, no tool_calls.
        assert!(received[3].choices[0].delta.tool_calls.is_none());
        assert_eq!(
            received[3].choices[0].finish_reason.as_deref(),
            Some("tool_calls")
        );
    }

    #[tokio::test]
    async fn create_serializes_json_schema_response_format_and_reasoning_fields() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(body_json(serde_json::json!({
                "model": "gpt-5-mini",
                "messages": [{"role": "user", "content": "hi", "name": null, "tool_calls": null, "tool_call_id": null, "refusal": null}],
                "response_format": {
                    "type": "json_schema",
                    "json_schema": {"name": "answer", "schema": {"type": "object"}, "strict": true}
                },
                "reasoning_effort": "low",
                "service_tier": "flex",
                "seed": 7,
                "logprobs": true,
                "top_logprobs": 3,
                "store": true,
                "max_completion_tokens": 100,
                "metadata": {"team": "eval"},
                "safety_identifier": "user-hash-123",
                "prompt_cache_key": "cache-key-abc",
                "stream_options": {"include_usage": true}
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-999",
                "object": "chat.completion",
                "created": 1700000000_u32,
                "choices": [{
                    "message": {"role": "assistant", "content": "{}", "name": null, "tool_calls": null, "tool_call_id": null},
                    "finish_reason": "stop",
                    "index": 0
                }],
                "usage": {"prompt_tokens": 5, "completion_tokens": 2, "total_tokens": 7}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("team".to_string(), "eval".to_string());

        let req = CreateChatRequest::builder()
            .model("gpt-5-mini")
            .messages(vec![
                ChatCompletionMessage::builder()
                    .role(Role::User)
                    .content("hi")
                    .build(),
            ])
            .response_format(ChatResponseFormat::JsonSchema {
                json_schema: JsonSchemaFormat::builder()
                    .name("answer")
                    .schema(serde_json::json!({"type": "object"}))
                    .strict(true)
                    .build(),
            })
            .reasoning_effort(ChatReasoningEffort::Low)
            .service_tier(ServiceTier::Flex)
            .seed(7_i64)
            .logprobs(true)
            .top_logprobs(3_u8)
            .store(true)
            .max_completion_tokens(100_u32)
            .metadata(metadata)
            .safety_identifier("user-hash-123")
            .prompt_cache_key("cache-key-abc")
            .stream_options(StreamOptions::builder().include_usage(true).build())
            .build();

        let res = client.chat().create(&req).await.unwrap();
        assert_eq!(res.id, "chatcmpl-999");
    }

    #[tokio::test]
    async fn create_serializes_text_and_json_object_response_format_variants() {
        assert_eq!(
            serde_json::to_value(ChatResponseFormat::Text).unwrap(),
            serde_json::json!({"type": "text"})
        );
        assert_eq!(
            serde_json::to_value(ChatResponseFormat::JsonObject).unwrap(),
            serde_json::json!({"type": "json_object"})
        );
    }

    #[tokio::test]
    async fn create_deserializes_refusal_and_usage_token_details() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-refusal",
                "object": "chat.completion",
                "created": 1700000000_u32,
                "choices": [{
                    "message": {
                        "role": "assistant", "content": null, "refusal": "I can't help with that.",
                        "name": null, "tool_calls": null, "tool_call_id": null
                    },
                    "finish_reason": "stop",
                    "index": 0
                }],
                "usage": {
                    "prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15,
                    "completion_tokens_details": {
                        "reasoning_tokens": 2, "audio_tokens": 0,
                        "accepted_prediction_tokens": 0, "rejected_prediction_tokens": 0
                    },
                    "prompt_tokens_details": {
                        "audio_tokens": 0, "cached_tokens": 8, "cache_write_tokens": 0
                    }
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
                    .content("say something you can't")
                    .build(),
            ])
            .build();

        let res = client.chat().create(&req).await.unwrap();
        assert_eq!(
            res.choices[0].message.refusal.as_deref(),
            Some("I can't help with that.")
        );

        let completion_details = res
            .usage
            .completion_tokens_details
            .as_ref()
            .expect("expected completion_tokens_details");
        assert_eq!(completion_details.reasoning_tokens, Some(2));

        let prompt_details = res
            .usage
            .prompt_tokens_details
            .as_ref()
            .expect("expected prompt_tokens_details");
        assert_eq!(prompt_details.cached_tokens, Some(8));
    }

    /// Real OpenAI responses don't always populate every token-detail sub-field. The
    /// sub-fields on `CompletionTokensDetails`/`PromptTokensDetails` must be `Option<u32>`
    /// (not bare `u32`) so a response omitting one doesn't fail deserialization of the
    /// entire `ChatResponse`.
    #[tokio::test]
    async fn create_tolerates_missing_token_detail_subfields() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-partial-usage",
                "object": "chat.completion",
                "created": 1700000000_u32,
                "choices": [{
                    "message": {
                        "role": "assistant", "content": "hi", "refusal": null,
                        "name": null, "tool_calls": null, "tool_call_id": null
                    },
                    "finish_reason": "stop",
                    "index": 0
                }],
                "usage": {
                    "prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15,
                    "completion_tokens_details": {
                        "reasoning_tokens": 2, "audio_tokens": 0,
                        "accepted_prediction_tokens": 0
                        // rejected_prediction_tokens deliberately omitted
                    },
                    "prompt_tokens_details": {
                        "audio_tokens": 0, "cached_tokens": 8
                        // cache_write_tokens deliberately omitted
                    }
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
                    .content("hello")
                    .build(),
            ])
            .build();

        let res = client
            .chat()
            .create(&req)
            .await
            .expect("response with partial token details should still deserialize");

        let completion_details = res
            .usage
            .completion_tokens_details
            .as_ref()
            .expect("expected completion_tokens_details");
        assert_eq!(completion_details.reasoning_tokens, Some(2));
        assert_eq!(completion_details.rejected_prediction_tokens, None);

        let prompt_details = res
            .usage
            .prompt_tokens_details
            .as_ref()
            .expect("expected prompt_tokens_details");
        assert_eq!(prompt_details.cached_tokens, Some(8));
        assert_eq!(prompt_details.cache_write_tokens, None);
    }

    #[tokio::test]
    async fn retrieve_deserializes_stored_completion() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/chat/completions/chatcmpl-stored"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "chatcmpl-stored",
                "object": "chat.completion",
                "created": 1700000000_u32,
                "choices": [{
                    "message": {"role": "assistant", "content": "hi", "name": null, "tool_calls": null, "tool_call_id": null, "refusal": null},
                    "finish_reason": "stop",
                    "index": 0
                }],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client.chat().retrieve("chatcmpl-stored").await.unwrap();
        assert_eq!(res.id, "chatcmpl-stored");
    }

    #[tokio::test]
    async fn list_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/chat/completions"))
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
        let req = ListChatCompletionsRequest::builder()
            .limit(10_u32)
            .order(SortOrder::Desc)
            .build();
        let res = client.chat().list(&req).await.unwrap();
        assert!(!res.has_more);
    }

    #[tokio::test]
    async fn delete_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/chat/completions/does-not-exist"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": {
                    "message": "No completion found",
                    "type": "invalid_request_error",
                    "param": null,
                    "code": "not_found"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let err = client.chat().delete("does-not-exist").await.unwrap_err();
        assert!(matches!(err, OpenAIError::ApiError(_)));
    }
}
