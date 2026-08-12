# Responses API: `web_search` Tool Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the `web_search` tool type (request + `WebSearchCall` output item, including its 3-variant `action` union) to the Responses API — the third variant on `ResponseTool`/`ResponseOutputItem`, after `Function`/`FileSearch` and `Message`/`FunctionCall`/`FileSearchCall`.

**Architecture:** Two feature tasks in the existing `rs_openai/src/interfaces/responses.rs` (no new files): Task 1 adds the request-side types and `ResponseTool::WebSearch`; Task 2 adds the response-side types (including the `WebSearchAction` union) and `ResponseOutputItem::WebSearchCall`. Task 3 is a lightweight final cross-check.

**Tech Stack:** Same as the rest of the crate (Rust edition 2024, `bon` builders, `serde`, `strum::Display`, `wiremock` tests) — no new dependencies.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-12-responses-websearch-design.md`.
- **Deliberately out of scope this round — do not add:** the versioned `"web_search_2025_08_26"` tool-type discriminator (the tag stays the standard `"web_search"` string, produced automatically by the `WebSearch` variant name), any tool type beyond `function`/`file_search`/`web_search` (`code_interpreter`/`computer_use`/`image_generation`/`mcp` and their output items), streaming events for `web_search`, `return_token_budget`.
- `ResponseTool::WebSearch(ResponseWebSearchTool)` and `ResponseOutputItem::WebSearchCall(ResponseWebSearchCallItem)` are ADDITIONS to already-existing extensible tagged unions (both already have two variants each: `ResponseTool::Function`/`FileSearch`, `ResponseOutputItem::Message`/`FunctionCall`/`FileSearchCall`) — do not redefine either enum from scratch, add a third variant to what's there.
- `WebSearchUserLocation.kind` and `WebSearchActionSource.kind` both map to the wire key `"type"` via `#[serde(rename = "type")]` — the Rust field is named `kind`, matching the exact precedent already established by `ResponseToolChoice::Specific { kind: String, ... }` in the same file (avoids colliding with the Rust keyword `type`).
- `WebSearchContextSize` derives both `strum::Display` and `serde::Serialize`/`Deserialize` — needs matching rename coverage on both sides (`#[serde(rename_all = "snake_case")]` + `#[strum(serialize_all = "snake_case")]`), a standing crate-wide rule from prior slices.
- `ResponseWebSearchCallItem.status` stays a plain `String`, NOT a closed enum — same caution already applied to `file_search`'s `status` (only 4 of an unconfirmed-total value set were seen).
- `WebSearchAction` IS fully typed (unlike `file_search`'s `filters`, which stays raw JSON) — 3 variants: `Search`, `OpenPage`, `FindInPage`. `FindInPage`'s `pattern`/`url` fields are REQUIRED (no `Option`), distinct from `Search`'s and `OpenPage`'s all-optional fields — do not make them `Option` "to be safe," the design spec confirms they're required per OpenAI's docs.
- Every task's "done" check includes `cargo build --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace`, and `cargo fmt --all -- --check`.
- **This machine needs `NO_PROXY=127.0.0.1,localhost` prefixed on every `cargo test` invocation**, or every `wiremock`-based HTTP test fails with a misleading `JSONDeserialize(Error("EOF while parsing a value", ...))` (a local proxy-client environment quirk, not a code defect — pure `#[test]`s with no HTTP are unaffected). E.g. `NO_PROXY=127.0.0.1,localhost cargo test --workspace`.
- **Stage only the exact files you changed when committing — use explicit file paths in `git add`, never `git add -A` or `git add .`.** This repo has a known, harmless source of untracked binary files (`rs_openai/*.mp3`/`*.opus`, byproducts of an unrelated Audio module test) that a broad `git add` can accidentally sweep into a commit — this already happened once in a recent slice. These files are now gitignored as a safety net, but explicit paths are still the correct habit.
- Follow TDD: write the failing test first, confirm it fails for the expected reason, then implement.
- All changes land in the existing `rs_openai/src/interfaces/responses.rs` and `rs_openai/src/apis/responses.rs` (test module only) — no new files, no changes to `apis/mod.rs`, `interfaces/mod.rs`, or `client.rs`.

---

### Task 1: `web_search` tool (request side)

**Files:**
- Modify: `rs_openai/src/interfaces/responses.rs`
- Modify: `rs_openai/src/apis/responses.rs` (test module only)

**Interfaces:**
- Consumes: `ResponseTool` (already exists, from the Responses core + file_search slices).
- Produces: `WebSearchFilters`, `WebSearchContextSize`, `WebSearchUserLocation`, `ResponseWebSearchTool`, `ResponseTool::WebSearch` (new variant). Task 2 does not depend on this task's specifics.

- [ ] **Step 1: Write the failing tests**

In `rs_openai/src/apis/responses.rs`, extend the test module's imports. Find the block starting `use crate::interfaces::responses::{` and add `ResponseWebSearchTool`, `WebSearchContextSize`, `WebSearchUserLocation` to the list (keep it alphabetically sorted, matching the existing style) — replace:

```rust
    use crate::interfaces::responses::{
        CreateResponseRequest, FileSearchRankingOptions, ListInputItemsRequest,
        ResponseConversationParam, ResponseFileSearchTool, ResponseFunctionTool, ResponseInput,
        ResponseInputItem, ResponseInputRole, ResponseOutputItem, ResponseStreamEvent,
        ResponseTool, ResponseToolChoice, ResponseToolChoiceFunctionName, ResponseToolChoiceMode,
    };
```

with:

```rust
    use crate::interfaces::responses::{
        CreateResponseRequest, FileSearchRankingOptions, ListInputItemsRequest,
        ResponseConversationParam, ResponseFileSearchTool, ResponseFunctionTool, ResponseInput,
        ResponseInputItem, ResponseInputRole, ResponseOutputItem, ResponseStreamEvent,
        ResponseTool, ResponseToolChoice, ResponseToolChoiceFunctionName, ResponseToolChoiceMode,
        ResponseWebSearchTool, WebSearchContextSize, WebSearchUserLocation,
    };
```

Add this test to the end of the `mod tests` block:

```rust

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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::responses`
Expected: FAIL with compile errors — `ResponseWebSearchTool`, `WebSearchContextSize`, `WebSearchUserLocation`, and `ResponseTool::WebSearch` don't exist yet.

- [ ] **Step 3: Implement**

In `rs_openai/src/interfaces/responses.rs`, add the request-side types right before `ResponseTool`'s definition. Replace:

```rust
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

with:

```rust
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct WebSearchFilters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_domains: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum WebSearchContextSize {
    Low,
    Medium,
    High,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct WebSearchUserLocation {
    #[builder(into)]
    #[serde(rename = "type")]
    pub kind: String,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ResponseWebSearchTool {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<WebSearchFilters>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_context_size: Option<WebSearchContextSize>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_location: Option<WebSearchUserLocation>,
}

/// A tool the model may call. Extensible union — this slice implements `Function`,
/// `FileSearch`, and `WebSearch`; later slices add `CodeInterpreter`/`ComputerUse`/
/// `ImageGeneration`/`Mcp` as new non-breaking variants.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseTool {
    Function(ResponseFunctionTool),
    FileSearch(ResponseFileSearchTool),
    WebSearch(ResponseWebSearchTool),
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::responses`
Expected: PASS — all responses tests green, including the 1 new one.

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

Stage only the two files you changed by exact path:

```bash
git add rs_openai/src/interfaces/responses.rs rs_openai/src/apis/responses.rs
git commit -m "$(cat <<'EOF'
feat: add the web_search tool request type to Responses

ResponseTool::WebSearch is the third variant on the already-
extensible union, after Function/FileSearch. filters/
search_context_size/user_location match OpenAI's confirmed shape;
the versioned "web_search_2025_08_26" tag variant is not modeled.
See docs/superpowers/specs/2026-08-12-responses-websearch-design.md.
EOF
)"
```

---

### Task 2: `web_search_call` output item (response side)

**Files:**
- Modify: `rs_openai/src/interfaces/responses.rs`
- Modify: `rs_openai/src/apis/responses.rs` (test module only)

**Interfaces:**
- Consumes: `ResponseOutputItem` (already exists, from the Responses core + file_search slices).
- Produces: `WebSearchActionSource`, `WebSearchAction`, `ResponseWebSearchCallItem`, `ResponseOutputItem::WebSearchCall` (new variant). Nothing later in this plan depends on this task's specifics — Task 3 is a read-only cross-check.

- [ ] **Step 1: Write the failing tests**

In `rs_openai/src/apis/responses.rs`, extend the test module's imports. Replace:

```rust
    use crate::interfaces::responses::{
        CreateResponseRequest, FileSearchRankingOptions, ListInputItemsRequest,
        ResponseConversationParam, ResponseFileSearchTool, ResponseFunctionTool, ResponseInput,
        ResponseInputItem, ResponseInputRole, ResponseOutputItem, ResponseStreamEvent,
        ResponseTool, ResponseToolChoice, ResponseToolChoiceFunctionName, ResponseToolChoiceMode,
        ResponseWebSearchTool, WebSearchContextSize, WebSearchUserLocation,
    };
```

with:

```rust
    use crate::interfaces::responses::{
        CreateResponseRequest, FileSearchRankingOptions, ListInputItemsRequest,
        ResponseConversationParam, ResponseFileSearchTool, ResponseFunctionTool, ResponseInput,
        ResponseInputItem, ResponseInputRole, ResponseOutputItem, ResponseStreamEvent,
        ResponseTool, ResponseToolChoice, ResponseToolChoiceFunctionName, ResponseToolChoiceMode,
        ResponseWebSearchTool, WebSearchAction, WebSearchContextSize, WebSearchUserLocation,
    };
```

Add these tests to the end of the `mod tests` block:

```rust

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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::responses`
Expected: FAIL with compile errors — `WebSearchAction`, `ResponseOutputItem::WebSearchCall` don't exist yet.

- [ ] **Step 3: Implement**

In `rs_openai/src/interfaces/responses.rs`, add the response-side types right before `ResponseOutputItem`'s definition. Replace:

```rust
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

with:

```rust
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct WebSearchActionSource {
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WebSearchAction {
    Search {
        #[serde(skip_serializing_if = "Option::is_none")]
        query: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        queries: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        sources: Option<Vec<WebSearchActionSource>>,
    },
    OpenPage {
        #[serde(skip_serializing_if = "Option::is_none")]
        url: Option<String>,
    },
    FindInPage {
        pattern: String,
        url: String,
    },
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseWebSearchCallItem {
    pub id: String,
    pub action: WebSearchAction,
    pub status: String,
}

/// One item in a response's `output` array. Extensible union — this slice implements
/// `Message`/`FunctionCall`/`FileSearchCall`/`WebSearchCall`; later slices add `Reasoning`/
/// `ComputerCall`/`CodeInterpreterCall`/`ImageGenerationCall`/`McpCall`/etc. as new
/// non-breaking variants.
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseOutputItem {
    Message(ResponseOutputMessage),
    FunctionCall(ResponseFunctionCallItem),
    FileSearchCall(ResponseFileSearchCallItem),
    WebSearchCall(ResponseWebSearchCallItem),
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::responses`
Expected: PASS — all responses tests green, including the 4 new ones.

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

Stage only the two files you changed by exact path:

```bash
git add rs_openai/src/interfaces/responses.rs rs_openai/src/apis/responses.rs
git commit -m "$(cat <<'EOF'
feat: add the web_search_call output item to Responses

ResponseOutputItem::WebSearchCall is the fourth variant on the
already-extensible union. WebSearchAction is a fully-typed 3-variant
union (Search/OpenPage/FindInPage) -- unlike file_search's filters,
this is the central payload of a web search call, not optional
metadata, so it's modeled properly rather than deferred to raw JSON.
FindInPage's pattern/url are required, matching OpenAI's documented
shape exactly. See
docs/superpowers/specs/2026-08-12-responses-websearch-design.md.
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

Run: `grep -n "pub struct\|pub enum" rs_openai/src/interfaces/responses.rs | grep -i "websearch"`
Expected: exactly 7 new items — `WebSearchFilters`, `WebSearchContextSize`, `WebSearchUserLocation`, `ResponseWebSearchTool`, `WebSearchActionSource`, `WebSearchAction`, `ResponseWebSearchCallItem`.

Run: `grep -n "WebSearch(ResponseWebSearchTool)\|WebSearchCall(ResponseWebSearchCallItem)" rs_openai/src/interfaces/responses.rs`
Expected: exactly 2 matches — the two new enum variant declarations.

Run: `grep -rn "CodeInterpreter\|ComputerUse\|ImageGeneration\|Mcp\b\|return_token_budget\|web_search_2025" rs_openai/src/interfaces/responses.rs`
Expected: no output (or only doc-comment mentions explaining deferral — confirm any hits are doc-comment-only, not actual fields/variants).

- [ ] **Step 2: Run the full verification suite on the assembled branch**

Run: `cargo fmt --all -- --check`
Expected: clean.

Run: `cargo build --workspace --all-targets`
Expected: clean build.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0.

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace`
Expected: PASS — full suite green, with the responses module contributing 5 new tests total (1 from Task 1, 4 from Task 2) beyond the pre-existing baseline.

- [ ] **Step 3: Done**

If Steps 1-2 all passed clean, this task made no changes — no commit needed. If any check found a real gap, stop and report exactly what it found rather than silently patching it — the fix belongs in the task that owns the affected file, following that task's own conventions.

---
