# Responses `code_interpreter` Tool Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the Responses API's `code_interpreter` tool type — the sixth and final Responses tool type, completing full coverage of every Responses tool OpenAI documents.

**Architecture:** All changes land in the existing `rs_openai/src/interfaces/responses.rs` — no new files, no client method changes (dispatches through the existing `Responses::create()`). This is a small slice (one tool config union + one output item) relative to prior tool-type slices, so it's a single feature task plus a lightweight final cross-check, rather than the 3-task split used for larger tool types.

**Tech Stack:** Rust, `serde` (`#[serde(tag = "type", rename_all = "snake_case")]` for the two extensible unions, `#[serde(untagged)]` for `ResponseCodeInterpreterContainer`), `bon::Builder`, `wiremock`.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-14-responses-code-interpreter-design.md`.
- The execution-results field (`outputs`/`results` — name unconfirmed by research) is NOT modeled at all — do not add it, do not guess at its name.
- `ResponseCodeInterpreterAutoContainer.memory_limit` reuses `crate::interfaces::containers::ContainerMemoryLimit` directly — do NOT define a new memory-limit enum for this tool.
- `ResponseCodeInterpreterCallItem.code` is REQUIRED (`String`, not `Option`) — analogous to `ResponseFunctionCallItem.arguments`, present as soon as the call object exists.
- `ResponseCodeInterpreterCallItem.status` stays a plain `String`, not a closed enum — matches every other status field in this crate.
- `ResponseCodeInterpreterAutoContainer.kind` uses `#[serde(rename = "type")] pub kind: String` — the established `kind`-field-for-wire-`"type"`-collision convention (same as `WebSearchUserLocation`/`ComputerCallOutputScreenshot`). The caller sets it explicitly via the builder (e.g. `.kind("auto")`) — do not hardcode the literal in the struct definition.
- Any doc comment on `ResponseTool`/`ResponseOutputItem` that lists "this slice implements X; only Y remains" must be updated in the same task that adds the new variant — a recurring, previously-caught gap in this project.
- Stage commits with **explicit file paths** (`git add path/to/file.rs`), never `git add -A`.
- Run `cargo fmt` and `cargo clippy -p rs_openai --all-targets -- -D warnings` before every commit; both must be clean.
- Environment: this machine's shell routes HTTP through a local proxy that breaks `wiremock` tests. Prefix every `cargo test`/`build`/`clippy` invocation with `NO_PROXY=127.0.0.1,localhost`. This is a local-shell-only concern — never write it into crate code, `Cargo.toml`, or CI config.

---

### Task 1: `code_interpreter` tool + output item

**Files:**
- Modify: `rs_openai/src/interfaces/responses.rs` (add new types immediately before the `ResponseTool` enum at line 290, right after `ResponseComputerUsePreviewTool`'s closing brace; add one variant to `ResponseTool`; update `ResponseTool`'s doc comment; add new types immediately before the `ResponseOutputItem` enum at line 643, right after `ResponseComputerCallItem`'s closing brace; add one variant to `ResponseOutputItem`; update `ResponseOutputItem`'s doc comment)
- Test: `rs_openai/src/apis/responses.rs` (add tests to the existing `#[cfg(test)] mod tests` block)

**Interfaces:**
- Consumes: existing `ResponseTool` enum (currently 7 variants: `Function`/`FileSearch`/`WebSearch`/`ImageGeneration`/`Mcp`/`Computer`/`ComputerUsePreview`), existing `ResponseOutputItem` enum (currently 9 variants), existing `crate::interfaces::containers::ContainerMemoryLimit` (reused directly, unmodified).
- Produces: `ResponseCodeInterpreterAutoContainer`, `ResponseCodeInterpreterContainer` (untagged 2-variant union), `ResponseCodeInterpreterTool`, `ResponseTool::CodeInterpreter(ResponseCodeInterpreterTool)` (8th variant), `ResponseCodeInterpreterCallItem`, `ResponseOutputItem::CodeInterpreterCall(ResponseCodeInterpreterCallItem)` (10th variant) — no later task in this plan depends on these; the final cross-check does.

- [ ] **Step 1: Write the failing test — tool serializes with `container` as a plain string ID**

Add to `rs_openai/src/apis/responses.rs` inside `mod tests`, after the last `create_serializes_computer_*`/`create_serializes_computer_use_preview_tool` test (search for `create_serializes_computer_use_preview_tool` — it's the last `computer`-related `create_*` test in the file):

```rust
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
                .container(ResponseCodeInterpreterContainer::Id("cntr_abc123".to_string()))
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
```

Add `ResponseCodeInterpreterContainer`, `ResponseCodeInterpreterTool` to the existing `use crate::interfaces::responses::{...}` import block at the top of `mod tests` — insert them alphabetically among the existing names.

- [ ] **Step 2: Run test to verify it fails**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai create_serializes_code_interpreter_tool_with_container_id 2>&1 | tail -30`
Expected: FAIL to compile — `ResponseTool::CodeInterpreter`, `ResponseCodeInterpreterContainer`, `ResponseCodeInterpreterTool` don't exist.

- [ ] **Step 3: Write the failing test — tool serializes with `container` as the `Auto` object form**

Add immediately after:

```rust
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
```

Add `ResponseCodeInterpreterAutoContainer` to the same import block. Also add `use crate::interfaces::containers::ContainerMemoryLimit;` as a new top-level `use` statement in `mod tests` (alongside the existing `use crate::interfaces::chat::SortOrder;` and `use crate::interfaces::images::{...};` lines) — `ContainerMemoryLimit` lives in a different module (`interfaces::containers`, from the just-shipped Containers slice), not `interfaces::responses`.

- [ ] **Step 4: Run tests to verify they fail**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai create_serializes_code_interpreter 2>&1 | tail -30`
Expected: FAIL to compile.

- [ ] **Step 5: Implement the request-side types**

In `rs_openai/src/interfaces/responses.rs`, insert immediately before the `ResponseTool` enum definition (currently at line 290, right after `ResponseComputerUsePreviewTool`'s closing brace):

```rust
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ResponseCodeInterpreterAutoContainer {
    #[builder(into)]
    #[serde(rename = "type")]
    pub kind: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_limit: Option<crate::interfaces::containers::ContainerMemoryLimit>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_ids: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ResponseCodeInterpreterContainer {
    Id(String),
    Auto(ResponseCodeInterpreterAutoContainer),
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ResponseCodeInterpreterTool {
    pub container: ResponseCodeInterpreterContainer,
}
```

Then modify the `ResponseTool` enum to add the 8th variant (also updating its doc comment):

```rust
/// A tool the model may call. This is the complete, confirmed set of Responses tool types --
/// `Function`, `FileSearch`, `WebSearch`, `ImageGeneration`, `Mcp`, `Computer`,
/// `ComputerUsePreview`, and `CodeInterpreter`.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseTool {
    Function(ResponseFunctionTool),
    FileSearch(ResponseFileSearchTool),
    WebSearch(ResponseWebSearchTool),
    ImageGeneration(ResponseImageGenerationTool),
    Mcp(ResponseMcpTool),
    Computer,
    ComputerUsePreview(ResponseComputerUsePreviewTool),
    CodeInterpreter(ResponseCodeInterpreterTool),
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai create_serializes_code_interpreter 2>&1 | tail -20`
Expected: both tests PASS.

- [ ] **Step 7: Write the failing test — deserializing a `code_interpreter_call` output item**

Add after `create_serializes_code_interpreter_tool_with_auto_container`:

```rust
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
```

- [ ] **Step 8: Run tests to verify they fail**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai code_interpreter_call 2>&1 | tail -30`
Expected: FAIL to compile — `ResponseOutputItem::CodeInterpreterCall` doesn't exist.

- [ ] **Step 9: Implement the response-side type**

In `rs_openai/src/interfaces/responses.rs`, insert immediately before the `ResponseOutputItem` enum definition (currently at line 643, right after `ResponseComputerCallItem`'s closing brace):

```rust
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseCodeInterpreterCallItem {
    pub id: String,
    pub code: String,
    pub container_id: String,
    pub status: String,
}
```

Then modify the `ResponseOutputItem` enum to add the 10th variant (also updating its doc comment):

```rust
/// One item in a response's `output` array. This is the complete, confirmed set of output
/// item types for the tools this crate implements -- `Message`, `FunctionCall`,
/// `FileSearchCall`, `WebSearchCall`, `ImageGenerationCall`, `McpListTools`, `McpCall`,
/// `McpApprovalRequest`, `ComputerCall`, and `CodeInterpreterCall`.
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseOutputItem {
    Message(ResponseOutputMessage),
    FunctionCall(ResponseFunctionCallItem),
    FileSearchCall(ResponseFileSearchCallItem),
    WebSearchCall(ResponseWebSearchCallItem),
    ImageGenerationCall(ResponseImageGenerationCallItem),
    McpListTools(ResponseMcpListToolsItem),
    McpCall(ResponseMcpCallItem),
    McpApprovalRequest(ResponseMcpApprovalRequestItem),
    ComputerCall(ResponseComputerCallItem),
    CodeInterpreterCall(ResponseCodeInterpreterCallItem),
}
```

- [ ] **Step 10: Run tests to verify they pass**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai code_interpreter_call 2>&1 | tail -20`
Expected: both PASS.

- [ ] **Step 11: Format, lint, full test run (also serves as this plan's final cross-check — no separate task needed given the small size of this slice)**

Run:
```bash
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo build -p rs_openai
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai 2>&1 | tail -10
```
Expected: clippy clean, build clean, all tests pass (167 + 4 new = 171).

Also manually verify (read the file, don't just trust the count):
- `ResponseTool` has exactly 8 variants: `Function`, `FileSearch`, `WebSearch`, `ImageGeneration`, `Mcp`, `Computer`, `ComputerUsePreview`, `CodeInterpreter`.
- `ResponseOutputItem` has exactly 10 variants: `Message`, `FunctionCall`, `FileSearchCall`, `WebSearchCall`, `ImageGenerationCall`, `McpListTools`, `McpCall`, `McpApprovalRequest`, `ComputerCall`, `CodeInterpreterCall`.
- No `outputs`/`results` field was added to `ResponseCodeInterpreterCallItem` (scope check — this was deliberately deferred).
- `ResponseCodeInterpreterAutoContainer.memory_limit` uses `crate::interfaces::containers::ContainerMemoryLimit`, not a newly-defined type (scope check).

- [ ] **Step 12: Commit**

```bash
git add rs_openai/src/interfaces/responses.rs rs_openai/src/apis/responses.rs
git commit -m "feat(responses): add code_interpreter tool, completing all Responses tool types"
```
