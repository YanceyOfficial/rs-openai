# Chat Tool-Calling Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add OpenAI Chat Completions tool-calling support to `rs_openai` — request-side `tools`/`tool_choice`/`parallel_tool_calls`, message-side `tool_calls`/`tool_call_id` plus a new `Role::Tool`, and streaming `Delta.tool_calls` — with zero changes to any other Chat capability.

**Architecture:** Pure field-addition to `rs_openai/src/interfaces/chat.rs`'s existing types; no new endpoints, no new accessor methods on `apis/chat.rs`'s `Chat<'a>`. `ChatCompletionMessage.content` changes from `String` to `Option<String>` — required for correctness, not a style choice, since a real assistant message that only calls tools has `"content": null` on the wire.

**Tech Stack:** Same as the rest of the crate (Rust edition 2024, `bon` builders, `serde`, `wiremock` tests) — no new dependencies.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-04-chat-tool-calling-design.md`. Background research: `docs/superpowers/research/2026-07-31-field-diff-chat-embeddings-moderations-models.md`.
- Scope is tool-calling only. Do not add `response_format`, `reasoning_effort`, `service_tier`, `store`, stored-completion retrieve/list/delete, `stream_options`, `seed`/`logprobs`, multimodal `content` (`Vec<ContentPart>`), `refusal`, `audio`, a `Developer` role variant, or any tool-call-argument accumulation/reconstruction logic — all explicitly out of scope per the spec's Non-goals.
- Serialization convention (exact, verified against the spec): the three new `CreateChatRequest` fields (`tools`, `tool_choice`, `parallel_tool_calls`) get `#[serde(skip_serializing_if = "Option::is_none")]`, matching every other optional field already on that struct. The `ChatCompletionMessage` fields (`content`, `tool_calls`, `tool_call_id`) do **not** get `skip_serializing_if` — they always serialize explicitly (as `null` when absent), matching the existing `name` field's convention on the same struct. Do not "fix" this asymmetry — it's intentional and specified.
- Every existing test in `rs_openai/src/apis/chat.rs` (`create_serializes_request_and_deserializes_response`, `create_rejects_stream_true`, `create_with_stream_rejects_stream_false`, `create_maps_api_error`, `create_with_stream_yields_deltas_and_terminates_after_done`) must keep passing **unchanged** — no edits to their bodies. `content: Some("hi")` serializes identically to today's `content: "hi"`, so this should require no changes to them at all. (Note: the overall message JSON does gain two new explicit-null keys — `tool_calls`/`tool_call_id` — since those don't skip-serialize; only `content` itself is byte-identical.)
- Every task's "done" check includes `cargo build --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace`, and `cargo fmt --all -- --check`.
- Follow TDD: write the failing test first, confirm it fails for the expected reason (compile error, since the test references types/fields that don't exist yet), then implement.

---

### Task 1: Core tool-calling types — `Role::Tool`, `ChatCompletionMessage`, `ToolCall`/`FunctionCall`, `ChatCompletionTool`/`FunctionDefinition`, `ToolChoice`, `CreateChatRequest` fields

**Files:**
- Modify: `rs_openai/src/interfaces/chat.rs`
- Modify: `rs_openai/src/apis/chat.rs` (tests only)

**Interfaces:**
- Consumes: nothing from earlier tasks (this is the first task).
- Produces: `Role::Tool`; `ChatCompletionMessage { role, content: Option<String>, name: Option<String>, tool_calls: Option<Vec<ToolCall>>, tool_call_id: Option<String> }`; `ToolCall { id: String, kind: String, function: FunctionCall }`; `FunctionCall { name: String, arguments: String }`; `ChatCompletionTool { kind: String, function: FunctionDefinition }` (with `Builder`); `FunctionDefinition { name: String, description: Option<String>, parameters: Option<serde_json::Value>, strict: Option<bool> }` (with `Builder`); `ToolChoiceMode::{None, Auto, Required}`; `ToolChoiceFunctionName { name: String }`; `ToolChoice::{Mode(ToolChoiceMode), Specific { kind: String, function: ToolChoiceFunctionName }}`; `CreateChatRequest` gains `tools: Option<Vec<ChatCompletionTool>>`, `tool_choice: Option<ToolChoice>`, `parallel_tool_calls: Option<bool>`. Task 2 consumes `ToolCall` (reused as-is for the streaming `Delta`).

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)] mod tests` block in `rs_openai/src/apis/chat.rs`, alongside the existing tests (add the new imports to the existing `use crate::interfaces::chat::{ChatCompletionMessage, CreateChatRequest, Role};` line — extend it to `use crate::interfaces::chat::{ChatCompletionMessage, ChatCompletionTool, CreateChatRequest, FunctionCall, FunctionDefinition, Role, ToolCall, ToolChoice, ToolChoiceMode};`):

```rust
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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --workspace --lib apis::chat`
Expected: FAIL with compile errors — `ToolCall`, `ChatCompletionTool`, `FunctionCall`, `FunctionDefinition`, `ToolChoice`, `ToolChoiceMode` don't exist yet, `Role::Tool` doesn't exist, `ChatCompletionMessage` has no `tool_calls`/`tool_call_id` setters, `.content(...)` on an assistant-message builder without content doesn't compile against a required `String` field.

- [ ] **Step 3: Implement — `Role` gains `Tool`**

In `rs_openai/src/interfaces/chat.rs`, replace:

```rust
#[derive(Debug, Serialize, Deserialize, Clone, Default, strum::Display)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    #[strum(serialize = "system")]
    System,
    #[default]
    #[strum(serialize = "user")]
    User,
    #[strum(serialize = "assistant")]
    Assistant,
}
```

with:

```rust
#[derive(Debug, Serialize, Deserialize, Clone, Default, strum::Display)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    #[strum(serialize = "system")]
    System,
    #[default]
    #[strum(serialize = "user")]
    User,
    #[strum(serialize = "assistant")]
    Assistant,
    #[strum(serialize = "tool")]
    Tool,
}
```

- [ ] **Step 4: Implement — `ChatCompletionMessage`, new tool-call/tool-definition/tool-choice types**

Replace:

```rust
#[derive(Builder, Debug, Clone, Deserialize, Serialize)]
pub struct ChatCompletionMessage {
    /// The role of the author of this message. One of `system`, `user`, or `assistant`.
    pub role: Role,

    /// The contents of the message.
    #[builder(into)]
    pub content: String,

    /// The name of the author of this message. May contain a-z, A-Z, 0-9, and underscores, with a maximum length of 64 characters.
    #[builder(into)]
    pub name: Option<String>,
}
```

with:

```rust
#[derive(Builder, Debug, Clone, Deserialize, Serialize)]
pub struct ChatCompletionMessage {
    /// The role of the author of this message. One of `system`, `user`, `assistant`, or `tool`.
    pub role: Role,

    /// The contents of the message. `None` (serializes as `null`) when an assistant message
    /// contains only tool calls — real OpenAI responses do this.
    #[builder(into)]
    pub content: Option<String>,

    /// The name of the author of this message. May contain a-z, A-Z, 0-9, and underscores, with a maximum length of 64 characters.
    #[builder(into)]
    pub name: Option<String>,

    /// Populated on assistant messages that call one or more tools instead of (or alongside)
    /// producing text.
    pub tool_calls: Option<Vec<ToolCall>>,

    /// Set on `role: Tool` messages to link this message's `content` (the tool's result) back
    /// to the `ToolCall.id` it answers.
    #[builder(into)]
    pub tool_call_id: Option<String>,
}

/// A single function call requested by the model, as part of an assistant message's
/// `tool_calls`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionCall {
    pub name: String,

    /// Raw JSON-encoded arguments, deliberately left unparsed — decide how (and whether) to
    /// validate/deserialize this against your own tool's parameter schema.
    pub arguments: String,
}

/// One tool call the model made, inside an assistant message's `tool_calls` array.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCall {
    pub id: String,

    /// Always `"function"` as of this writing; OpenAI has never shipped a second tool-call type.
    #[serde(rename = "type")]
    pub kind: String,

    pub function: FunctionCall,
}

/// A function's definition, as advertised to the model via `CreateChatRequest.tools`.
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct FunctionDefinition {
    #[builder(into)]
    pub name: String,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// A JSON Schema object describing the function's parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// A tool the model may call, as an entry in `CreateChatRequest.tools`.
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ChatCompletionTool {
    /// Always `"function"` as of this writing.
    #[serde(rename = "type")]
    #[builder(default = "function".to_string())]
    pub kind: String,

    pub function: FunctionDefinition,
}

/// The plain string modes `tool_choice` can take.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoiceMode {
    None,
    Auto,
    Required,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolChoiceFunctionName {
    pub name: String,
}

/// Controls which (if any) tool the model calls. Either a plain mode string
/// (`"none"`/`"auto"`/`"required"`) or an object forcing one specific function.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ToolChoice {
    Mode(ToolChoiceMode),
    Specific {
        #[serde(rename = "type")]
        kind: String,
        function: ToolChoiceFunctionName,
    },
}
```

- [ ] **Step 5: Implement — `CreateChatRequest` gains `tools`/`tool_choice`/`parallel_tool_calls`**

In `rs_openai/src/interfaces/chat.rs`, in `CreateChatRequest`, immediately before the closing `}` (after the existing `user` field), add:

```rust

    /// A list of tools the model may call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ChatCompletionTool>>,

    /// Controls which (if any) tool is called by the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,

    /// Whether to enable parallel function calling during tool use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test --workspace --lib apis::chat`
Expected: PASS — all chat tests green, including the two new ones.

Run: `cargo test --workspace`
Expected: PASS — full suite green (same count as before this task plus 2 new tests; no other module's tests should be affected).

- [ ] **Step 7: Verify**

Run: `cargo build --workspace --all-targets`
Expected: clean build.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0.

Run: `cargo fmt --all -- --check`
Expected: clean (run `cargo fmt --all` first if needed).

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat: add Chat tool-calling support

Adds Role::Tool, ChatCompletionMessage.tool_calls/tool_call_id (and
makes content optional — real assistant tool-call messages have
content: null), ToolCall/FunctionCall, ChatCompletionTool/
FunctionDefinition, and ToolChoice, plus CreateChatRequest.tools/
tool_choice/parallel_tool_calls. Scoped to tool-calling only per
docs/superpowers/specs/2026-08-04-chat-tool-calling-design.md; the
other Chat gaps from the field-diff research are separate slices.
EOF
)"
```

---

### Task 2: Streaming — `Delta.tool_calls`

**Files:**
- Modify: `rs_openai/src/interfaces/chat.rs`
- Modify: `rs_openai/src/apis/chat.rs` (tests only)

**Interfaces:**
- Consumes: `ToolCall` from Task 1 (reused as-is — real streamed tool-call deltas are partial per-chunk, but this task does not model partial-field variants or build any accumulation logic; each chunk's `tool_calls` entries deserialize into the same full `ToolCall` shape, with whatever fields that chunk actually carries).
- Produces: `Delta { content: Option<String>, tool_calls: Option<Vec<ToolCall>> }`. Nothing later depends on this — it's the last task in this plan.

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` block in `rs_openai/src/apis/chat.rs`:

```rust
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
                    "content": null,
                    "tool_calls": [{
                        "id": "call_stream_1",
                        "type": "function",
                        "function": {"name": "get_weather", "arguments": "{\"location\":"}
                    }]
                },
                "finish_reason": null,
                "index": 0
            }]
        });
        let chunk2 = serde_json::json!({
            "id": "chatcmpl-789",
            "object": "chat.completion.chunk",
            "model": "gpt-4o-mini",
            "created": 1700000000_u32,
            "choices": [{
                "delta": {"content": null, "tool_calls": null},
                "finish_reason": "tool_calls",
                "index": 0
            }]
        });

        let sse_body =
            format!("data: {chunk1}\n\ndata: {chunk2}\n\ndata: [DONE]\n\n",);

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

        assert_eq!(received.len(), 2);
        let tool_calls = received[0].choices[0]
            .delta
            .tool_calls
            .as_ref()
            .expect("expected tool_calls in first chunk");
        assert_eq!(tool_calls[0].id, "call_stream_1");
        assert_eq!(tool_calls[0].function.arguments, "{\"location\":");
        assert!(received[1].choices[0].delta.tool_calls.is_none());
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --workspace --lib apis::chat::tests::create_with_stream_yields_tool_call_deltas`
Expected: FAIL with a compile error — `Delta` has no `tool_calls` field yet.

- [ ] **Step 3: Implement**

In `rs_openai/src/interfaces/chat.rs`, replace:

```rust
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Delta {
    pub content: Option<String>,
}
```

with:

```rust
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Delta {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --workspace --lib apis::chat`
Expected: PASS — all chat tests green, including the new streaming test.

Run: `cargo test --workspace`
Expected: PASS — full suite green.

- [ ] **Step 5: Verify**

Run: `cargo build --workspace --all-targets`
Expected: clean build.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0.

Run: `cargo fmt --all -- --check`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat: expose tool_calls on streaming Chat deltas

Delta.tool_calls carries each chunk's raw, partial tool-call data as
OpenAI sends it — no accumulation/reconstruction of multi-chunk
function-call arguments is built here, per the spec's explicit scope.
EOF
)"
```

---
