# Responses API: `conversation` Param + `file_search` Tool Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `CreateResponseRequest.conversation` and the `file_search` tool type (request + `FileSearchCall` output item) to the Responses API — both now unblocked by the already-shipped Conversations and Vector Stores slices.

**Architecture:** Two feature tasks in the existing `rs_openai/src/interfaces/responses.rs` (no new files): Task 1 adds `ResponseConversationParam` and the `conversation` field; Task 2 adds the `file_search` tool's full request/response type set and wires it into the already-extensible `ResponseTool`/`ResponseOutputItem` unions. Task 3 is a lightweight final cross-check.

**Tech Stack:** Same as the rest of the crate (Rust edition 2024, `bon` builders, `serde`, `wiremock` tests) — no new dependencies.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-12-responses-conversation-filesearch-design.md`.
- **Deliberately out of scope this round — do not add:** any tool type beyond `function`/`file_search` (`web_search`/`code_interpreter`/`computer_use`/`image_generation`/`mcp` and their output items), a typed `filters` grammar (`file_search`'s `filters` stays `Option<serde_json::Value>`, same deferral already established for Vector Stores' own `search.filters`), closed enums for `status`/`ranker` (both stay plain `String`), streaming events for `file_search` (`ResponseStreamEvent` stays scoped to text + function-calling events), the `include` request parameter.
- `ResponseConversationParam` is `#[serde(untagged)]` with `Id(String)`/`Object { id: String }` — mirrors `ResponseToolChoice`'s existing string-or-object pattern in the same file.
- `ResponseTool::FileSearch(ResponseFileSearchTool)` and `ResponseOutputItem::FileSearchCall(ResponseFileSearchCallItem)` are ADDITIONS to already-existing extensible tagged unions (both already have other variants: `ResponseTool::Function`, `ResponseOutputItem::Message`/`FunctionCall`) — do not redefine either enum from scratch, add a variant to what's there.
- Every task's "done" check includes `cargo build --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace`, and `cargo fmt --all -- --check`.
- **This machine needs `NO_PROXY=127.0.0.1,localhost` prefixed on every `cargo test` invocation**, or every `wiremock`-based HTTP test fails with a misleading `JSONDeserialize(Error("EOF while parsing a value", ...))` (a local proxy-client environment quirk, not a code defect — pure `#[test]`s with no HTTP are unaffected). E.g. `NO_PROXY=127.0.0.1,localhost cargo test --workspace`.
- Follow TDD: write the failing test first, confirm it fails for the expected reason, then implement.
- All changes land in the existing `rs_openai/src/interfaces/responses.rs` and `rs_openai/src/apis/responses.rs` (test module only) — no new files, no changes to `apis/mod.rs`, `interfaces/mod.rs`, or `client.rs`.

---

### Task 1: `conversation` param

**Files:**
- Modify: `rs_openai/src/interfaces/responses.rs`
- Modify: `rs_openai/src/apis/responses.rs` (test module only)

**Interfaces:**
- Consumes: `CreateResponseRequest` (already exists, from the Responses core slice).
- Produces: `ResponseConversationParam`; `CreateResponseRequest.conversation: Option<ResponseConversationParam>`. Task 2 does not depend on this task's specifics.

- [ ] **Step 1: Write the failing tests**

In `rs_openai/src/apis/responses.rs`, add a test. Find the test module's imports (the block starting `use crate::interfaces::responses::{`) and add `ResponseConversationParam` to the import list — replace:

```rust
    use crate::interfaces::responses::{
        CreateResponseRequest, ListInputItemsRequest, ResponseFunctionTool, ResponseInput,
        ResponseInputItem, ResponseInputRole, ResponseStreamEvent, ResponseTool,
        ResponseToolChoice, ResponseToolChoiceFunctionName, ResponseToolChoiceMode,
    };
```

with:

```rust
    use crate::interfaces::responses::{
        CreateResponseRequest, ListInputItemsRequest, ResponseConversationParam,
        ResponseFunctionTool, ResponseInput, ResponseInputItem, ResponseInputRole,
        ResponseStreamEvent, ResponseTool, ResponseToolChoice, ResponseToolChoiceFunctionName,
        ResponseToolChoiceMode,
    };
```

Add these tests to the end of the `mod tests` block:

```rust

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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::responses`
Expected: FAIL with compile errors — `ResponseConversationParam` doesn't exist yet, and `CreateResponseRequest` has no `.conversation(...)` builder method.

- [ ] **Step 3: Implement**

In `rs_openai/src/interfaces/responses.rs`, add `ResponseConversationParam` right after `ResponseToolChoice`'s definition (the pattern it mirrors) — find:

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum TruncationStrategy {
    Auto,
    Disabled,
}
```

and insert immediately before it:

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ResponseConversationParam {
    Id(String),
    Object { id: String },
}

```

Then add the `conversation` field to `CreateResponseRequest`. Replace:

```rust
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}
```

with:

```rust
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation: Option<ResponseConversationParam>,
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::responses`
Expected: PASS — all 3 new tests green.

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
feat: add CreateResponseRequest.conversation

ResponseConversationParam is an untagged string-or-{id} union,
mirroring ResponseToolChoice's existing pattern in the same file.
Unblocked now that the Conversations resource has shipped. See
docs/superpowers/specs/2026-08-12-responses-conversation-filesearch-design.md.
EOF
)"
```

---

### Task 2: `file_search` tool

**Files:**
- Modify: `rs_openai/src/interfaces/responses.rs`
- Modify: `rs_openai/src/apis/responses.rs` (test module only)

**Interfaces:**
- Consumes: `ResponseTool`, `ResponseOutputItem` (both already exist as extensible tagged unions, from the Responses core slice).
- Produces: `FileSearchRankingOptionsHybridSearch`, `FileSearchRankingOptions`, `ResponseFileSearchTool`, `ResponseTool::FileSearch` (new variant); `FileSearchResult`, `ResponseFileSearchCallItem`, `ResponseOutputItem::FileSearchCall` (new variant). Nothing later in this plan depends on this task's specifics — Task 3 is a read-only cross-check.

- [ ] **Step 1: Write the failing tests**

In `rs_openai/src/apis/responses.rs`, extend the test module's imports. Replace:

```rust
    use crate::interfaces::responses::{
        CreateResponseRequest, ListInputItemsRequest, ResponseConversationParam,
        ResponseFunctionTool, ResponseInput, ResponseInputItem, ResponseInputRole,
        ResponseStreamEvent, ResponseTool, ResponseToolChoice, ResponseToolChoiceFunctionName,
        ResponseToolChoiceMode,
    };
```

with:

```rust
    use crate::interfaces::responses::{
        CreateResponseRequest, FileSearchRankingOptions, ListInputItemsRequest,
        ResponseConversationParam, ResponseFileSearchTool, ResponseFunctionTool, ResponseInput,
        ResponseInputItem, ResponseInputRole, ResponseOutputItem, ResponseStreamEvent,
        ResponseTool, ResponseToolChoice, ResponseToolChoiceFunctionName, ResponseToolChoiceMode,
    };
```

Add these tests to the end of the `mod tests` block:

```rust

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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::responses`
Expected: FAIL with compile errors — `ResponseFileSearchTool`, `FileSearchRankingOptions`, `ResponseTool::FileSearch`, and `ResponseOutputItem::FileSearchCall` don't exist yet.

- [ ] **Step 3: Implement**

In `rs_openai/src/interfaces/responses.rs`, add the request-side types right before `ResponseTool`'s definition. Replace:

```rust
/// A tool the model may call. Extensible union — this slice implements only `Function`;
/// later slices add `WebSearch`/`FileSearch`/`CodeInterpreter`/`ComputerUse`/
/// `ImageGeneration`/`Mcp` as new non-breaking variants.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseTool {
    Function(ResponseFunctionTool),
}
```

with:

```rust
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct FileSearchRankingOptionsHybridSearch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_weight: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_weight: Option<f32>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct FileSearchRankingOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hybrid_search: Option<FileSearchRankingOptionsHybridSearch>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranker: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_threshold: Option<f32>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ResponseFileSearchTool {
    pub vector_store_ids: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_num_results: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranking_options: Option<FileSearchRankingOptions>,
}

/// A tool the model may call. Extensible union — this slice implements `Function` and
/// `FileSearch`; later slices add `WebSearch`/`CodeInterpreter`/`ComputerUse`/
/// `ImageGeneration`/`Mcp` as new non-breaking variants.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseTool {
    Function(ResponseFunctionTool),
    FileSearch(ResponseFileSearchTool),
}
```

Then add the response-side types right before `ResponseOutputItem`'s definition. Replace:

```rust
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
```

with:

```rust
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct FileSearchResult {
    pub attributes: Option<std::collections::HashMap<String, serde_json::Value>>,
    pub file_id: Option<String>,
    pub filename: Option<String>,
    pub score: Option<f32>,
    pub text: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseFileSearchCallItem {
    pub id: String,
    pub queries: Vec<String>,
    pub status: String,
    pub results: Option<Vec<FileSearchResult>>,
}

/// One item in a response's `output` array. Extensible union — this slice implements
/// `Message`/`FunctionCall`/`FileSearchCall`; later slices add `Reasoning`/`WebSearchCall`/
/// `ComputerCall`/`CodeInterpreterCall`/`ImageGenerationCall`/`McpCall`/etc. as new
/// non-breaking variants.
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseOutputItem {
    Message(ResponseOutputMessage),
    FunctionCall(ResponseFunctionCallItem),
    FileSearchCall(ResponseFileSearchCallItem),
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::responses`
Expected: PASS — all responses tests green, including the 3 new ones.

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
feat: add the file_search tool type to Responses

ResponseTool::FileSearch and ResponseOutputItem::FileSearchCall are
additions to the already-extensible tagged unions from the Responses
core slice -- exactly the non-breaking growth their own doc comments
anticipated. filters/status/ranker stay raw JSON/String pending
fuller confirmation of their real shapes. Unblocked now that Vector
Stores has shipped. See
docs/superpowers/specs/2026-08-12-responses-conversation-filesearch-design.md.
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

Run: `grep -n "pub struct\|pub enum" rs_openai/src/interfaces/responses.rs | grep -i "conversation\|filesearch"`
Expected: exactly 6 new items — `ResponseConversationParam`, `FileSearchRankingOptionsHybridSearch`, `FileSearchRankingOptions`, `ResponseFileSearchTool`, `FileSearchResult`, `ResponseFileSearchCallItem`.

Run: `grep -n "FileSearch(" rs_openai/src/interfaces/responses.rs`
Expected: exactly 2 matches — `ResponseTool::FileSearch(...)` and `ResponseOutputItem::FileSearchCall(...)` variant declarations (not counting their usages in tests).

Run: `grep -rn "WebSearch\|CodeInterpreter\|ComputerUse\|ImageGeneration\|Mcp\b\|ComparisonFilter\|CompoundFilter" rs_openai/src/interfaces/responses.rs`
Expected: no output (or only doc-comment mentions explaining deferral — confirm any hits are doc-comment-only, not actual fields/variants).

- [ ] **Step 2: Run the full verification suite on the assembled branch**

Run: `cargo fmt --all -- --check`
Expected: clean.

Run: `cargo build --workspace --all-targets`
Expected: clean build.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0.

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace`
Expected: PASS — full suite green, with the responses module contributing 6 new tests total (3 from Task 1, 3 from Task 2) beyond the pre-existing baseline.

- [ ] **Step 3: Done**

If Steps 1-2 all passed clean, this task made no changes — no commit needed. If any check found a real gap, stop and report exactly what it found rather than silently patching it — the fix belongs in the task that owns the affected file, following that task's own conventions.

---
