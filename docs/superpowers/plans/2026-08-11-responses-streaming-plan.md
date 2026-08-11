# Responses API: Streaming Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add streaming support to the Responses API — `stream: Option<bool>` on `CreateResponseRequest`, a new `Responses::create_with_stream()` method, and a `ResponseStreamEvent` tagged union covering the lifecycle, output-item, text-delta, and function-call-argument-delta events relevant to the crate's existing (text + function-calling only) type surface.

**Architecture:** Two tasks. Task 1 adds the pure types (`CreateResponseRequest.stream`, `ResponseStreamEvent`) with no HTTP methods yet, TDD'd via direct `serde_json` round-trip assertions. Task 2 wires `create`/`create_with_stream` (mirroring Chat's/Completions' existing `is_stream`-gated split byte-for-byte) and adds the end-to-end `wiremock` SSE tests proving the full pipeline. Task 3 is a lightweight final cross-check, no subagent needed.

**Tech Stack:** Same as the rest of the crate (Rust edition 2024, `bon` builders, `serde`, `wiremock`, `futures`/`tokio-stream` for the SSE plumbing already built for Chat) — no new dependencies.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-11-responses-streaming-design.md`.
- **Scope, do not add:** `response.content_part.added`/`.done`, `response.output_text_annotation_added`, `response.refusal.delta`/`.done`, any event tied to `web_search`/`file_search`/`code_interpreter`/`image_generation`/`mcp`/`computer_use`, `response.reasoning_summary_part.*`/`response.reasoning_text.*`, or background-mode stream resumption (`starting_after`). `ResponseStreamEvent` covers exactly 12 variants: `Created`, `InProgress`, `Completed`, `Failed`, `Incomplete`, `OutputItemAdded`, `OutputItemDone`, `OutputTextDelta`, `OutputTextDone`, `FunctionCallArgumentsDelta`, `FunctionCallArgumentsDone`, `Error`.
- `ResponseStreamEvent`'s `type` strings are dot-separated literals (e.g. `response.output_item.added`) — NOT expressible via `#[serde(rename_all = "snake_case")]` (which would produce `output_item_added`, no `response.` prefix, underscores not dots). Every variant needs an explicit per-variant `#[serde(rename = "...")]`, following the exact precedent already in this crate: `BatchEndpoint` in `rs_openai/src/interfaces/batch.rs` (e.g. `#[serde(rename = "/v1/responses")]`).
- Every `sequence_number` field on every `ResponseStreamEvent` variant is `Option<u64>`, never bare `u64` — applying the same deserialization-fragility lesson already applied to `ResponseUsageTokenDetails` in the core slice (an uncertain-presence field must not be required, or one real event omitting it fails the whole stream item).
- Lifecycle variants (`Created`/`InProgress`/`Completed`/`Failed`/`Incomplete`) wrap the existing `Response` type directly (`rs_openai/src/interfaces/responses.rs`) — do not duplicate a smaller/partial response-snapshot type. `OutputItemAdded`/`OutputItemDone` wrap the existing `ResponseOutputItem` directly. `Error`'s fields (`message: String`, `param: Option<serde_json::Value>`, `code: Option<serde_json::Value>`) match `ApiErrorDetail` (`rs_openai/src/shared/response_wrapper.rs`) exactly, minus `r#type` (subsumed by the enum's own `"error"` tag) — do not invent a different error shape.
- `create`/`create_with_stream` must structurally mirror `Chat::create`/`create_with_stream` (`rs_openai/src/apis/chat.rs`) and `Completions::create`/`create_with_stream` exactly: same `is_stream(req.stream)` gate from `crate::shared::utils::is_stream`, same `OpenAIError::InvalidArgument` rejection wording pattern ("When stream is true/false, use Responses::create_with_stream"/"...::create"), same `self.openai.post_stream(...)` call. No new client.rs changes — `post_stream`/`stream_sse<T>` are already fully generic over the deserialized type.
- **Do not include a literal `data: [DONE]\n\n` sentinel in any mocked SSE test body.** Unlike Chat (which is confirmed to send `[DONE]`), whether Responses sends a `[DONE]` sentinel was not confirmed this session (see spec Context). The existing `stream_sse` implementation already handles a stream that simply closes after its last real event with no sentinel (the loop ends naturally when `event_source.next()` returns `None`) — so every test's mocked SSE body should end right after its last `data: {...}\n\n` line, proving that code path specifically.
- Every task's "done" check includes `cargo build --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace`, and `cargo fmt --all -- --check`.
- **This machine has `http_proxy`/`https_proxy`/`all_proxy` set to a local proxy client.** `reqwest` (used by both the crate itself and every `wiremock`-based test) honors these by default and routes even `127.0.0.1`-bound test traffic through that proxy, which breaks every HTTP-mocked test with a misleading `JSONDeserialize(Error("EOF while parsing a value", ...))` — this is an environment quirk, not a code defect, confirmed by the fact that the crate's 6 pure `#[test]`s (no HTTP) are unaffected while every `#[tokio::test]` that hits a `wiremock::MockServer` fails identically. **Always run `cargo test`/`cargo build`/`cargo clippy` with `NO_PROXY=127.0.0.1,localhost` set** (e.g. `NO_PROXY=127.0.0.1,localhost cargo test --workspace`) to bypass it. If a task's tests fail with this exact `EOF while parsing a value` symptom on every HTTP test and none of the pure unit tests, re-run with `NO_PROXY` set before concluding the implementation is broken.
- Follow TDD: write the failing test first, confirm it fails for the expected reason, then implement.
- `rs_openai/src/apis/mod.rs`, `rs_openai/src/interfaces/mod.rs`, and `rs_openai/src/client.rs` are already fully wired for `responses` from the core slice — do not touch any of them in this plan.

---

### Task 1: `CreateResponseRequest.stream` and `ResponseStreamEvent`

**Files:**
- Modify: `rs_openai/src/interfaces/responses.rs`
- Modify: `rs_openai/src/apis/responses.rs` (test module only — imports + new tests)

**Interfaces:**
- Consumes: `Response`, `ResponseOutputItem`, `CreateResponseRequest` (all already exist in `interfaces/responses.rs` from the core slice).
- Produces: `CreateResponseRequest.stream: Option<bool>` (new field); `ResponseStreamEvent` (new 12-variant tagged enum). Task 2 consumes both directly — `create`/`create_with_stream` read `req.stream`, and `create_with_stream` returns a stream of `ResponseStreamEvent`.

This task adds no HTTP methods — pure type definitions and a new request field, tested via direct `serde_json` serialize/deserialize assertions, the same pattern the core slice's own Task 1 used for its foundational types.

- [ ] **Step 1: Write the failing tests**

In `rs_openai/src/apis/responses.rs`, extend the test module's imports. Replace:

```rust
    use crate::interfaces::responses::{
        CreateResponseRequest, ListInputItemsRequest, ResponseFunctionTool, ResponseInput,
        ResponseInputItem, ResponseInputRole, ResponseTool, ResponseToolChoice,
        ResponseToolChoiceFunctionName, ResponseToolChoiceMode,
    };
```

with:

```rust
    use crate::interfaces::responses::{
        CreateResponseRequest, ListInputItemsRequest, ResponseFunctionTool, ResponseInput,
        ResponseInputItem, ResponseInputRole, ResponseStreamEvent, ResponseTool,
        ResponseToolChoice, ResponseToolChoiceFunctionName, ResponseToolChoiceMode,
    };
```

Add these tests to the end of the `mod tests` block:

```rust

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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::responses`
Expected: FAIL with compile errors — `ResponseStreamEvent` doesn't exist yet, and `CreateResponseRequest` has no `stream` field/builder method.

- [ ] **Step 3: Implement**

In `rs_openai/src/interfaces/responses.rs`, add the `stream` field to `CreateResponseRequest`. Replace:

```rust
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<crate::interfaces::chat::ServiceTier>,
}
```

with:

```rust
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<crate::interfaces::chat::ServiceTier>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}
```

At the end of `rs_openai/src/interfaces/responses.rs`, add:

```rust

/// A single Server-Sent Event from a streamed `Responses::create_with_stream` call. Covers
/// only the events relevant to this crate's current (text + function-calling only) type
/// surface — see docs/superpowers/specs/2026-08-11-responses-streaming-design.md for the
/// full list of deferred event families (content_part, refusal, other tool types, reasoning).
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ResponseStreamEvent {
    #[serde(rename = "response.created")]
    Created {
        response: Response,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.in_progress")]
    InProgress {
        response: Response,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.completed")]
    Completed {
        response: Response,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.failed")]
    Failed {
        response: Response,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.incomplete")]
    Incomplete {
        response: Response,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.output_item.added")]
    OutputItemAdded {
        output_index: u32,
        item: ResponseOutputItem,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.output_item.done")]
    OutputItemDone {
        output_index: u32,
        item: ResponseOutputItem,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.output_text.delta")]
    OutputTextDelta {
        item_id: String,
        output_index: u32,
        content_index: u32,
        delta: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.output_text.done")]
    OutputTextDone {
        item_id: String,
        output_index: u32,
        content_index: u32,
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.function_call_arguments.delta")]
    FunctionCallArgumentsDelta {
        item_id: String,
        output_index: u32,
        delta: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.function_call_arguments.done")]
    FunctionCallArgumentsDone {
        item_id: String,
        output_index: u32,
        arguments: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "error")]
    Error {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        param: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::responses`
Expected: PASS — all 18 responses tests green (13 existing from the core slice + 5 new).

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace`
Expected: PASS — full suite green.

- [ ] **Step 5: Verify**

Run: `cargo build --workspace --all-targets`
Expected: clean build.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0.

Run: `cargo fmt --all -- --check`
Expected: clean (run `cargo fmt --all` first if needed).

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat: add CreateResponseRequest.stream and ResponseStreamEvent

12-variant tagged union covering lifecycle, output-item, text-delta,
and function-call-argument-delta events -- the events relevant to
this crate's current text + function-calling type surface. Dotted
type strings use the crate's existing per-variant literal-rename
precedent (BatchEndpoint). Lifecycle events wrap the existing
Response type directly; Error mirrors ApiErrorDetail's shape. No
HTTP methods yet -- Task 2 wires create_with_stream(). See
docs/superpowers/specs/2026-08-11-responses-streaming-design.md.
EOF
)"
```

---

### Task 2: `create`/`create_with_stream` wiring and end-to-end streaming tests

**Files:**
- Modify: `rs_openai/src/apis/responses.rs`

**Interfaces:**
- Consumes: `ResponseStreamEvent`, `CreateResponseRequest.stream` from Task 1; `crate::shared::utils::is_stream`, `crate::shared::response_wrapper::OpenAIError`, `OpenAI::post_stream` (all pre-existing crate infrastructure, unchanged).
- Produces: `Responses::create_with_stream(&self, req: &responses::CreateResponseRequest) -> OpenAIResponse<Pin<Box<dyn Stream<Item = OpenAIResponse<responses::ResponseStreamEvent>> + Send>>>`; `Responses::create` now rejects `stream: true`. Nothing later in this plan depends on this task's specifics beyond it existing and working — Task 3 is a read-only cross-check.

- [ ] **Step 1: Write the failing tests**

In `rs_openai/src/apis/responses.rs`, update the top-of-file imports. Replace:

```rust
use crate::client::OpenAI;
use crate::interfaces::responses;
pub use crate::interfaces::responses::*;
use crate::shared::response_wrapper::OpenAIResponse;
```

with:

```rust
use crate::client::OpenAI;
use crate::interfaces::responses;
pub use crate::interfaces::responses::*;
use crate::shared::response_wrapper::{OpenAIError, OpenAIResponse};
use crate::shared::utils::is_stream;
use futures::Stream;
use std::pin::Pin;
```

Update the test module's imports — add `use futures::StreamExt;` alongside the existing `use wiremock::...` lines. Replace:

```rust
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
```

with:

```rust
    use futures::StreamExt;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
```

Add these tests to the end of the `mod tests` block:

```rust

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

        let err = client.responses().create_with_stream(&req).await.unwrap_err();
        assert!(matches!(
            err,
            crate::shared::response_wrapper::OpenAIError::InvalidArgument(_)
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

        let events = vec![
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
            .collect::<String>();

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
        assert!(matches!(received[1], ResponseStreamEvent::InProgress { .. }));
        assert!(matches!(received[2], ResponseStreamEvent::OutputItemAdded { .. }));
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
        assert!(matches!(received[6], ResponseStreamEvent::OutputItemDone { .. }));
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

        let events = vec![
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
            .collect::<String>();

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
            .input(ResponseInput::Text("What's the weather in Boston?".to_string()))
            .stream(true)
            .build();

        let mut stream = client.responses().create_with_stream(&req).await.unwrap();

        let mut received = Vec::new();
        while let Some(item) = stream.next().await {
            received.push(item.unwrap());
        }

        assert_eq!(received.len(), 5);
        assert!(matches!(received[0], ResponseStreamEvent::OutputItemAdded { .. }));
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
        assert!(matches!(received[4], ResponseStreamEvent::OutputItemDone { .. }));
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

        let events = vec![
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
            .collect::<String>();

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
                    response.incomplete_details.as_ref().map(|d| d.reason.as_str()),
                    Some("max_output_tokens")
                );
            }
            other => panic!("expected Incomplete, got {other:?}"),
        }
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::responses`
Expected: FAIL with compile errors — `Responses::create_with_stream` doesn't exist yet, and `create`'s current implementation doesn't check `req.stream` (the `create_rejects_stream_true` test would otherwise get a network-mock-mismatch failure rather than the expected `InvalidArgument`, since no mock server is set up for it).

- [ ] **Step 3: Implement**

In `rs_openai/src/apis/responses.rs`, replace the `create` method and add `create_with_stream` immediately after it. Replace:

```rust
    /// Creates a model response.
    pub async fn create(
        &self,
        req: &responses::CreateResponseRequest,
    ) -> OpenAIResponse<responses::Response> {
        self.openai.post("/responses", req).await
    }
```

with:

```rust
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
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::responses`
Expected: PASS — all 25 responses tests green (18 from Task 1 + 7 new).

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace`
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
feat: add Responses::create_with_stream

Structurally mirrors Chat::create/create_with_stream and
Completions::create/create_with_stream exactly -- same is_stream
gate, same rejection wording, same post_stream call. No client.rs
changes needed; post_stream/stream_sse were already fully generic.
Tests cover the happy-path text-delta stream, function-call-argument
deltas, a top-level error event, and failed/incomplete terminal
events -- none include a `[DONE]` sentinel, since that wasn't
confirmed to exist for this API and the existing stream_sse already
handles a stream that simply closes after its last event. See
docs/superpowers/specs/2026-08-11-responses-streaming-design.md.
EOF
)"
```

---

### Task 3: Final cross-check and whole-workspace verification

**Files:**
- None modified (verification only, plus a possible trivial fix if the cross-check finds a gap).

**Interfaces:**
- Consumes: everything from Tasks 1-2.
- Produces: nothing further depends on this; it's the last task.

- [ ] **Step 1: Cross-check against the design spec**

Run: `grep -n "pub enum ResponseStreamEvent" -A 90 rs_openai/src/interfaces/responses.rs | grep '#\[serde(rename'`
Expected: exactly 12 lines, one per variant, matching the 12 dotted/literal type strings listed in this plan's Global Constraints.

Run: `grep -n "pub async fn create" rs_openai/src/apis/responses.rs`
Expected: exactly 2 matches — `create` and `create_with_stream`.

Run: `grep -rn "content_part\|refusal\|web_search\|file_search\|code_interpreter\|image_generation\|mcp\|computer_use\|reasoning_summary\|reasoning_text\|starting_after" rs_openai/src/interfaces/responses.rs rs_openai/src/apis/responses.rs`
Expected: no output (or only doc-comment mentions explaining deferral — confirm any hits are doc-comment-only, not actual fields/variants).

- [ ] **Step 2: Run the full verification suite on the assembled branch**

Run: `cargo fmt --all -- --check`
Expected: clean.

Run: `cargo build --workspace --all-targets`
Expected: clean build.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0.

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace`
Expected: PASS — full suite green, with the responses module contributing 25 tests total (13 pre-existing from the core slice + 5 from Task 1 + 7 from Task 2).

- [ ] **Step 3: Done**

If Steps 1-2 all passed clean, this task made no changes — no commit needed. If any check found a real gap, stop and report exactly what it found rather than silently patching it — the fix belongs in the task that owns the affected file, following that task's own conventions.

---
