# Responses API: Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the Responses API's core surface — `create`/`retrieve`/`delete`/`cancel`/`list_input_items`, plain-text input/output, and function-calling — turning Phase 2a's zero-method `Responses`/`Response` scaffolds into real, working accessors.

**Architecture:** New file `rs_openai/src/interfaces/responses.rs` holds every request/response type; `rs_openai/src/apis/responses.rs`'s already-scaffolded `Responses<'a>`/`Response<'a>` structs gain real methods. Five tasks: (1) the type foundation nothing else depends on shipping first (input/tool/tool-choice types), (2) `CreateResponseRequest` + the `Response` object types + `create()` (the flagship operation, depends on Task 1's tool types), (3) `retrieve`/`delete`/`cancel`, (4) `list_input_items`, (5) final cross-check.

**Tech Stack:** Same as the rest of the crate (Rust edition 2024, `bon` builders, `serde`, `strum::Display`, `wiremock` tests) — no new dependencies.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-07-responses-core-design.md`.
- **Deliberately out of scope this round** — do not add: `stream`/streaming support, any tool type beyond `function` (`web_search`/`file_search`/`code_interpreter`/`computer_use`/`image_generation`/`mcp`), any output item type beyond `Message`/`FunctionCall`, multimodal input content, the `conversation` parameter, `background`/`context_management`/`include`/`reasoning`/`modalities` request fields, or a typed annotation union (`ResponseOutputText.annotations` stays `Vec<serde_json::Value>`). `ResponseTool` and `ResponseOutputItem` are deliberately designed as extensible tagged unions — future slices add variants, not a rework.
- Reuse existing Chat types directly, do not redefine: `crate::interfaces::chat::ChatResponseFormat`, `crate::interfaces::chat::JsonSchemaFormat`, `crate::interfaces::chat::ServiceTier`, `crate::interfaces::chat::SortOrder`.
- Naming collision, resolved and non-negotiable: the interface type is bare `Response` (matching OpenAI's own object name — `ResponseResponse` would be absurd). This collides with `apis::responses::Response<'a>` (the ID-scoped accessor struct, already fixed by Phase 2a's scaffold). Inside `apis/responses.rs`, refer to the body type via the qualified path `responses::Response` wherever both names are in scope — this is an ordinary Rust namespace disambiguation, not a new pattern.
- `ResponseUsageTokenDetails`'s sub-fields (`cached_tokens`, `reasoning_tokens`) are `Option<u32>`, not bare `u32` — applying the lesson from the existing-modules-field-fixes slice's `ChatUsage` finding (a required sub-field on an uncertain-presence nested object risks failing deserialization of the entire response).
- `Response` does not echo back request-side config fields (`temperature`, `tools`, etc.) — deliberately, per the spec's Design section. Do not add these without checking with the plan author first; if you believe one is genuinely needed, stop and ask rather than adding it.
- Every task's "done" check includes `cargo build --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace`, and `cargo fmt --all -- --check`.
- Follow TDD: write the failing test first, confirm it fails for the expected reason (compile error, since tests reference types/fields/methods that don't exist yet), then implement.
- `interfaces/responses.rs` does not exist yet — Task 1 creates it. `apis/mod.rs` and `client.rs` already have `responses` fully wired (Phase 2a) — do not touch either file in this plan. `interfaces/mod.rs` needs `pub mod responses;` added (Task 1 only).

---

### Task 1: Type foundation — input, tools, tool choice

**Files:**
- Create: `rs_openai/src/interfaces/responses.rs`
- Modify: `rs_openai/src/interfaces/mod.rs`
- Modify: `rs_openai/src/apis/responses.rs` (re-export line + tests)

**Interfaces:**
- Consumes: nothing from earlier tasks (first task).
- Produces: `ResponseInput`, `ResponseInputItem`, `ResponseInputRole`, `ResponseFunctionTool`, `ResponseTool`, `ResponseToolChoiceMode`, `ResponseToolChoiceFunctionName`, `ResponseToolChoice`, `TruncationStrategy`. Task 2 consumes `ResponseInput`/`ResponseTool`/`ResponseToolChoice`/`TruncationStrategy` directly in `CreateResponseRequest`.

This task adds no HTTP methods — it's pure type definitions, tested via direct `serde_json` serialize/deserialize assertions, the same pattern already used for the Grader union's own foundation task in the existing-modules-field-fixes plan.

- [ ] **Step 1: Write the failing tests**

Create `rs_openai/src/apis/responses.rs`'s test module. Replace the entire current file:

```rust
//! Model responses — OpenAI's now-recommended primary API, superseding Chat Completions and
//! (eventually) Assistants. Core surface only this round: plain-text input/output and
//! function-calling, non-streaming — see
//! docs/superpowers/specs/2026-08-07-responses-core-design.md.

use crate::client::OpenAI;
use crate::interfaces::responses;
pub use crate::interfaces::responses::*;

pub struct Responses<'a> {
    openai: &'a OpenAI,
}

impl<'a> Responses<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
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
}

#[cfg(test)]
mod tests {
    use crate::interfaces::responses::{
        ResponseFunctionTool, ResponseInputItem, ResponseInputRole, ResponseTool,
        ResponseToolChoice, ResponseToolChoiceFunctionName, ResponseToolChoiceMode,
    };

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
        assert_eq!(serde_json::to_value(&choice).unwrap(), serde_json::json!("auto"));
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
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --workspace --lib apis::responses`
Expected: FAIL with compile errors — `crate::interfaces::responses` doesn't exist yet (`interfaces/responses.rs` hasn't been created), so nothing in this file resolves.

- [ ] **Step 3: Implement — create `interfaces/responses.rs`**

Create `rs_openai/src/interfaces/responses.rs`:

```rust
use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ResponseInput {
    Text(String),
    Items(Vec<ResponseInputItem>),
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ResponseInputRole {
    User,
    Assistant,
    System,
    Developer,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseInputItem {
    Message {
        role: ResponseInputRole,
        content: String,
    },
    FunctionCallOutput {
        call_id: String,
        output: String,
    },
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ResponseFunctionTool {
    #[builder(into)]
    pub name: String,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// A JSON Schema object describing the function's parameters.
    pub parameters: serde_json::Value,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// A tool the model may call. Extensible union — this slice implements only `Function`;
/// later slices add `WebSearch`/`FileSearch`/`CodeInterpreter`/`ComputerUse`/
/// `ImageGeneration`/`Mcp` as new non-breaking variants.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseTool {
    Function(ResponseFunctionTool),
}

/// The plain string modes `tool_choice` can take.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ResponseToolChoiceMode {
    None,
    Auto,
    Required,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResponseToolChoiceFunctionName {
    pub name: String,
}

/// Controls which (if any) tool is called by the model. Either a plain mode string
/// (`"none"`/`"auto"`/`"required"`) or an object forcing one specific function. Mirrors
/// `crate::interfaces::chat::ToolChoice`'s shape exactly, but kept as a distinct type since
/// the two APIs document this parameter separately.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ResponseToolChoice {
    Mode(ResponseToolChoiceMode),
    Specific {
        #[serde(rename = "type")]
        kind: String,
        function: ResponseToolChoiceFunctionName,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum TruncationStrategy {
    Auto,
    Disabled,
}
```

- [ ] **Step 4: Wire `responses` into `interfaces/mod.rs`**

Replace the full contents of `rs_openai/src/interfaces/mod.rs` with:

```rust
pub mod assistants;
pub mod audio;
pub mod batch;
pub mod chat;
pub mod completions;
pub mod embeddings;
pub mod files;
pub mod fine_tuning;
pub mod images;
pub mod messages;
pub mod models;
pub mod moderations;
pub mod responses;
pub mod runs;
pub mod threads;
pub mod uploads;
```

(`apis/mod.rs` and `client.rs` already have `responses` fully wired from Phase 2a — do not touch either file.)

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --workspace --lib apis::responses`
Expected: PASS — all 5 new tests green.

Run: `cargo test --workspace`
Expected: PASS — full suite green (same count as before this task plus 5 new tests).

- [ ] **Step 6: Verify**

Run: `cargo build --workspace --all-targets`
Expected: clean build.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0.

Run: `cargo fmt --all -- --check`
Expected: clean (run `cargo fmt --all` first if needed).

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat: add Responses API input/tool/tool-choice type foundation

Creates interfaces/responses.rs (didn't exist — Phase 2a only
scaffolded apis/responses.rs). ResponseTool is an extensible tagged
union implementing only the function type this round; later slices
add web_search/file_search/code_interpreter/computer_use/
image_generation/mcp as new variants. ResponseToolChoice mirrors
Chat's ToolChoice shape. No HTTP methods yet — Task 2 wires create().
See docs/superpowers/specs/2026-08-07-responses-core-design.md.
EOF
)"
```

---

### Task 2: `CreateResponseRequest`, the `Response` object, `create()`

**Files:**
- Modify: `rs_openai/src/interfaces/responses.rs`
- Modify: `rs_openai/src/apis/responses.rs`

**Interfaces:**
- Consumes: `ResponseInput`, `ResponseTool`, `ResponseToolChoice`, `TruncationStrategy` from Task 1; `crate::interfaces::chat::{ChatResponseFormat, ServiceTier}` (already exist, from the earlier Chat slices).
- Produces: `CreateResponseRequest`, `ResponseTextConfig`, `ResponseStatus`, `ResponseError`, `ResponseIncompleteDetails`, `ResponseOutputContent`, `ResponseOutputMessage`, `ResponseFunctionCallItem`, `ResponseOutputItem`, `ResponseUsageTokenDetails`, `ResponseUsage`, `Response` (the response body type — see Global Constraints for the naming-collision resolution), and `Responses::create()`. Tasks 3/4 consume `responses::Response` (the retrieve/cancel return type) and `ResponseStatus`/`ResponseOutputItem` indirectly through it.

- [ ] **Step 1: Write the failing tests**

Extend the imports in `rs_openai/src/apis/responses.rs`'s test module — replace:

```rust
    use crate::interfaces::responses::{
        ResponseFunctionTool, ResponseInputItem, ResponseInputRole, ResponseTool,
        ResponseToolChoice, ResponseToolChoiceFunctionName, ResponseToolChoiceMode,
    };
```

with:

```rust
    use crate::client::OpenAI;
    use crate::interfaces::responses::{
        CreateResponseRequest, ResponseFunctionTool, ResponseInput, ResponseInputItem,
        ResponseInputRole, ResponseTool, ResponseToolChoice, ResponseToolChoiceFunctionName,
        ResponseToolChoiceMode,
    };
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
```

Add these tests (place them at the end of the `mod tests` block):

```rust

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
            .input(ResponseInput::Text("What's the capital of France?".to_string()))
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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --workspace --lib apis::responses`
Expected: FAIL with compile errors — `CreateResponseRequest`, `ResponseOutputItem`, `ResponseOutputContent`, and `Responses::create` don't exist yet.

- [ ] **Step 3: Implement**

At the end of `rs_openai/src/interfaces/responses.rs`, add:

```rust

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ResponseTextConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<crate::interfaces::chat::ChatResponseFormat>,
}

#[derive(Builder, Debug, Serialize, Clone)]
pub struct CreateResponseRequest {
    #[builder(into)]
    pub model: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<ResponseInput>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncation: Option<TruncationStrategy>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ResponseToolChoice>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ResponseTool>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<ResponseTextConfig>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_identifier: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<crate::interfaces::chat::ServiceTier>,
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ResponseStatus {
    Queued,
    InProgress,
    Completed,
    Incomplete,
    Failed,
    Cancelled,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseIncompleteDetails {
    pub reason: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseOutputContent {
    OutputText {
        text: String,
        annotations: Vec<serde_json::Value>,
    },
    Refusal {
        refusal: String,
    },
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseOutputMessage {
    pub id: String,
    pub role: String,
    pub content: Vec<ResponseOutputContent>,
    pub status: ResponseStatus,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseFunctionCallItem {
    pub id: String,
    pub call_id: String,
    pub name: String,
    pub arguments: String,
    pub status: ResponseStatus,
}

/// One item in a response's `output` array. Extensible union — this slice implements only
/// `Message`/`FunctionCall`; later slices add `Reasoning`/`WebSearchCall`/`FileSearchCall`/
/// `ComputerCall`/`CodeInterpreterCall`/`ImageGenerationCall`/`McpCall`/etc. as new
/// non-breaking variants.
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseOutputItem {
    Message(ResponseOutputMessage),
    FunctionCall(ResponseFunctionCallItem),
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseUsageTokenDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u32>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseUsage {
    pub input_tokens: u32,
    pub input_tokens_details: Option<ResponseUsageTokenDetails>,
    pub output_tokens: u32,
    pub output_tokens_details: Option<ResponseUsageTokenDetails>,
    pub total_tokens: u32,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Response {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub status: ResponseStatus,
    pub error: Option<ResponseError>,
    pub incomplete_details: Option<ResponseIncompleteDetails>,
    pub model: String,
    pub output: Vec<ResponseOutputItem>,
    pub usage: Option<ResponseUsage>,
    pub previous_response_id: Option<String>,
    pub instructions: Option<String>,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}
```

In `rs_openai/src/apis/responses.rs`, add the `create` method. Replace:

```rust
impl<'a> Responses<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}
```

with:

```rust
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
```

Add the necessary imports at the top of `rs_openai/src/apis/responses.rs` — replace:

```rust
use crate::client::OpenAI;
use crate::interfaces::responses;
pub use crate::interfaces::responses::*;
```

with:

```rust
use crate::client::OpenAI;
use crate::interfaces::responses;
pub use crate::interfaces::responses::*;
use crate::shared::response_wrapper::OpenAIResponse;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --workspace --lib apis::responses`
Expected: PASS — all 9 responses tests green (5 from Task 1, 4 new).

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
feat: add CreateResponseRequest, the Response object, and create()

The flagship Responses operation. ResponseOutputItem is an
extensible tagged union implementing only Message/FunctionCall this
round. ResponseUsageTokenDetails' sub-fields are Option<u32>, not
bare u32, applying the ChatUsage deserialization-fragility lesson
from the existing-modules-field-fixes slice. Reuses Chat's
ChatResponseFormat/ServiceTier directly. See
docs/superpowers/specs/2026-08-07-responses-core-design.md.
EOF
)"
```

---

### Task 3: `retrieve`, `delete`, `cancel`

**Files:**
- Modify: `rs_openai/src/interfaces/responses.rs`
- Modify: `rs_openai/src/apis/responses.rs`

**Interfaces:**
- Consumes: `responses::Response` (as the return type for `retrieve`/`cancel`) from Task 2.
- Produces: `DeleteResponseResponse`; `Response::retrieve()`, `Response::delete()`, `Response::cancel()` (the ID-scoped accessor's methods). Task 4 doesn't depend on this task's specific additions, only on the same `Response<'a>` accessor struct existing.

- [ ] **Step 1: Write the failing tests**

Add to `rs_openai/src/apis/responses.rs`'s `mod tests` block (place at the end):

```rust

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
        assert_eq!(
            res.status.to_string(),
            "cancelled"
        );
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --workspace --lib apis::responses`
Expected: FAIL with compile errors — `.retrieve()`, `.delete()`, `.cancel()` don't exist on `Response<'a>` yet.

- [ ] **Step 3: Implement**

At the end of `rs_openai/src/interfaces/responses.rs`, add:

```rust

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteResponseResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}
```

In `rs_openai/src/apis/responses.rs`, add the three methods to `impl<'a> Response<'a>`. Replace:

```rust
impl<'a> Response<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }
}
```

with:

```rust
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
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --workspace --lib apis::responses`
Expected: PASS — all responses tests green, including the 3 new ones.

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
feat: add Response::retrieve/delete/cancel

Fills out the ID-scoped accessor's lifecycle operations. cancel is
only meaningful for background responses (not yet supported by
create() this round, per the design's Non-goals) but the endpoint
itself is independent of how the response was created, so it's
implemented now. See
docs/superpowers/specs/2026-08-07-responses-core-design.md.
EOF
)"
```

---

### Task 4: `list_input_items`

**Files:**
- Modify: `rs_openai/src/interfaces/responses.rs`
- Modify: `rs_openai/src/apis/responses.rs`

**Interfaces:**
- Consumes: `ResponseInputItem` from Task 1; `crate::interfaces::chat::SortOrder` (already exists).
- Produces: `ListInputItemsRequest`, `ListInputItemsResponse`; `Response::list_input_items()`. Nothing later depends on this — it's the last feature task in this plan.

- [ ] **Step 1: Write the failing test**

Add to `rs_openai/src/apis/responses.rs`'s `mod tests` block (place at the end). Extend the imports — replace:

```rust
    use crate::interfaces::responses::{
        CreateResponseRequest, ResponseFunctionTool, ResponseInput, ResponseInputItem,
        ResponseInputRole, ResponseTool, ResponseToolChoice, ResponseToolChoiceFunctionName,
        ResponseToolChoiceMode,
    };
```

with:

```rust
    use crate::interfaces::responses::{
        CreateResponseRequest, ListInputItemsRequest, ResponseFunctionTool, ResponseInput,
        ResponseInputItem, ResponseInputRole, ResponseTool, ResponseToolChoice,
        ResponseToolChoiceFunctionName, ResponseToolChoiceMode,
    };
    use crate::interfaces::chat::SortOrder;
```

Add this test:

```rust

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
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --workspace --lib apis::responses::tests::list_input_items_deserializes_function_call_output_item`
Expected: FAIL with a compile error — `ListInputItemsRequest` doesn't exist, `.list_input_items(...)` doesn't exist on `Response<'a>`.

- [ ] **Step 3: Implement**

At the end of `rs_openai/src/interfaces/responses.rs`, add:

```rust

#[derive(Builder, Debug, Serialize, Clone)]
pub struct ListInputItemsRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<crate::interfaces::chat::SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListInputItemsResponse {
    pub object: String,
    pub data: Vec<ResponseInputItem>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}
```

In `rs_openai/src/apis/responses.rs`, add the method to `impl<'a> Response<'a>` (after `cancel`, before the closing `}`):

```rust

    /// Lists a response's input items.
    pub async fn list_input_items(
        &self,
        req: &responses::ListInputItemsRequest,
    ) -> OpenAIResponse<responses::ListInputItemsResponse> {
        self.openai
            .get(&format!("/responses/{}/input_items", self.id), req)
            .await
    }
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --workspace --lib apis::responses`
Expected: PASS — all responses tests green, including the new one.

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
feat: add Response::list_input_items

Reuses Chat's SortOrder for the order param, same shape/reasoning as
the other Chat-type reuses in this slice. Tests deserializing a
FunctionCallOutput input item specifically — the harder of the two
ResponseInputItem variants, since it never appears message-shaped.
See docs/superpowers/specs/2026-08-07-responses-core-design.md.
EOF
)"
```

---

### Task 5: Final cross-check and whole-workspace verification

**Files:**
- None modified (verification only, plus a possible trivial fix if the cross-check finds a gap).

**Interfaces:**
- Consumes: everything from Tasks 1-4.
- Produces: nothing further depends on this; it's the last task.

- [ ] **Step 1: Cross-check against the design spec**

Run: `grep -n "pub struct\|pub enum" rs_openai/src/interfaces/responses.rs`
Expected: every type named in the design spec's Design section is present — `ResponseInput`, `ResponseInputRole`, `ResponseInputItem`, `ResponseFunctionTool`, `ResponseTool`, `ResponseToolChoiceMode`, `ResponseToolChoiceFunctionName`, `ResponseToolChoice`, `TruncationStrategy`, `ResponseTextConfig`, `CreateResponseRequest`, `ResponseStatus`, `ResponseError`, `ResponseIncompleteDetails`, `ResponseOutputContent`, `ResponseOutputMessage`, `ResponseFunctionCallItem`, `ResponseOutputItem`, `ResponseUsageTokenDetails`, `ResponseUsage`, `Response`, `DeleteResponseResponse`, `ListInputItemsRequest`, `ListInputItemsResponse` — 23 types total.

Run: `grep -n "pub async fn" rs_openai/src/apis/responses.rs`
Expected: exactly 5 methods — `create` (on `Responses<'a>`), `retrieve`/`delete`/`cancel`/`list_input_items` (on `Response<'a>`).

Run: `grep -rn "stream\|web_search\|file_search\|code_interpreter\|computer_use\|image_generation\|mcp\|conversation:" rs_openai/src/interfaces/responses.rs rs_openai/src/apis/responses.rs`
Expected: no output (or only doc-comment mentions explaining deferral, e.g. inside the `ResponseTool`/`ResponseOutputItem` doc comments listing future variant names — confirm any hits are doc-comment-only, not actual fields/variants/methods).

- [ ] **Step 2: Run the full verification suite on the assembled branch**

Run: `cargo fmt --all -- --check`
Expected: clean.

Run: `cargo build --workspace --all-targets`
Expected: clean build.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0.

Run: `cargo test --workspace`
Expected: PASS — full suite green, with the responses module contributing 13 tests total (5 from Task 1, 4 from Task 2, 3 from Task 3, 1 from Task 4, plus whatever the pre-existing baseline was).

- [ ] **Step 3: Done**

If Steps 1-2 all passed clean, this task made no changes — no commit needed. If any check found a real gap, stop and report exactly what it found rather than silently patching it — the fix belongs in the task that owns the affected file, following that task's own conventions.

---
