# Responses `mcp` Tool Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the Responses API's `mcp` tool type (fifth variant on `ResponseTool`), its three output items (`mcp_list_tools`/`mcp_call`/`mcp_approval_request`, sixth/seventh/eighth on `ResponseOutputItem`), and its input-only approval-response item (fourth variant on `ResponseInputItem`) — scoped to `server_url`-based remote MCP servers only.

**Architecture:** All changes land in the existing `rs_openai/src/interfaces/responses.rs` (types) and `rs_openai/src/apis/responses.rs` (tests only — no new client methods needed, `mcp` is just a new `ResponseTool`/`ResponseOutputItem`/`ResponseInputItem` variant, dispatched through the existing `create()` method). Follows the exact extensible-tagged-union pattern already used for `file_search`/`web_search`/`image_generation`.

**Tech Stack:** Rust, `serde` (`#[serde(tag = "type", rename_all = "snake_case")]` for the three extensible unions, `#[serde(untagged)]` for `McpRequireApproval`), `bon::Builder`, `wiremock` for HTTP-mocked tests.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-13-responses-mcp-design.md`. Scope: `server_url`-based remote MCP servers only — do NOT add `connector_id`, `tunnel_id`, or `allowed_callers` fields to `ResponseMcpTool` (deferred, per `AskUserQuestion` "Core: server_url only (Recommended)").
- `allowed_tools` stays `Option<Vec<String>>` only — do NOT model the alternate `McpToolFilter` object form.
- `require_approval` IS fully typed (not raw JSON) — it is the tool's central safety-relevant field, per this project's "central payload gets full typing" precedent (`web_search`'s `action`).
- `mcp_list_tools`/`mcp_call`/`mcp_approval_request` output items do NOT get a `status` field — confirmed absent from OpenAI's own example JSON, a deliberate contrast with `file_search_call`/`web_search_call`/`image_generation_call`. Do not add one.
- `input_schema`/`annotations` on `McpListedTool` stay `serde_json::Value` — genuinely variable per-server shapes, not worth typing.
- Every enum deriving both `strum::Display` and `Serialize`/`Deserialize` needs matching rename coverage on both sides (this plan's only enum, `McpApprovalMode`, does NOT derive `strum::Display` — it's plain `Serialize`/`Deserialize` only, so this rule doesn't apply here, but don't add `strum::Display` without also adding `#[strum(serialize_all = "snake_case")]`).
- Environment: this machine's shell routes HTTP through a local proxy that breaks `wiremock` tests. Prefix every `cargo test`/`build`/`clippy` invocation with `NO_PROXY=127.0.0.1,localhost`. This is a local-shell-only concern — never write it into crate code, `Cargo.toml`, or CI config.
- Stage commits with **explicit file paths** (`git add path/to/file.rs`), never `git add -A` — this repo has a known source of untracked byproduct files from unrelated Audio tests.
- Run `cargo fmt` and `cargo clippy -p rs_openai --all-targets -- -D warnings` before every commit; both must be clean.

---

### Task 1: `mcp` tool type (request side)

**Files:**
- Modify: `rs_openai/src/interfaces/responses.rs` (add new types near the end of the tool-type section, after `ResponseImageGenerationTool`/before the `ResponseTool` enum at line ~182; add the `Mcp` variant to `ResponseTool`)
- Test: `rs_openai/src/apis/responses.rs` (add tests to the existing `#[cfg(test)] mod tests` block)

**Interfaces:**
- Consumes: existing `ResponseTool` enum (`#[serde(tag = "type", rename_all = "snake_case")]`, currently `Function`/`FileSearch`/`WebSearch`/`ImageGeneration`), existing `CreateResponseRequest`/`Responses::create()` (unchanged), existing `bon::Builder` derive pattern used throughout this file.
- Produces: `McpApprovalMode` (enum: `Never`/`Always`), `McpToolNameFilter` (struct: `tool_names: Vec<String>`), `McpApprovalFilter` (struct: `never: Option<McpToolNameFilter>`, `always: Option<McpToolNameFilter>`), `McpRequireApproval` (untagged enum: `Mode(McpApprovalMode)` | `Filter(McpApprovalFilter)`), `ResponseMcpTool` (struct — this is what later tasks and callers construct via `ResponseMcpTool::builder()`), `ResponseTool::Mcp(ResponseMcpTool)` variant.

- [ ] **Step 1: Write the failing test — plain-mode `require_approval` serializes correctly**

Add to `rs_openai/src/apis/responses.rs` inside `mod tests`, after `create_serializes_image_generation_tool_reusing_images_module_enums` (around line 1437):

```rust
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
```

Add `McpApprovalMode`, `McpRequireApproval`, `ResponseMcpTool` to the existing `use crate::interfaces::responses::{...}` import block at the top of `mod tests` (around line 101-108) — insert them alphabetically among the existing names.

- [ ] **Step 2: Run test to verify it fails**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai create_serializes_mcp_tool_with_plain_require_approval 2>&1 | tail -30`
Expected: FAIL to compile — `McpApprovalMode`, `McpRequireApproval`, `ResponseMcpTool` not found in `crate::interfaces::responses`.

- [ ] **Step 3: Write the failing test — object-filter `require_approval` serializes correctly**

Add immediately after the previous test:

```rust
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
```

Add `McpApprovalFilter`, `McpToolNameFilter` to the same import block.

- [ ] **Step 4: Run tests to verify both fail**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai create_serializes_mcp_tool 2>&1 | tail -30`
Expected: FAIL to compile — types still don't exist.

- [ ] **Step 5: Implement the types**

In `rs_openai/src/interfaces/responses.rs`, insert immediately before the `ResponseTool` enum definition (currently at line 182, right after `ResponseImageGenerationTool`'s closing brace):

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum McpApprovalMode {
    Never,
    Always,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct McpToolNameFilter {
    pub tool_names: Vec<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct McpApprovalFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub never: Option<McpToolNameFilter>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub always: Option<McpToolNameFilter>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum McpRequireApproval {
    Mode(McpApprovalMode),
    Filter(McpApprovalFilter),
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ResponseMcpTool {
    #[builder(into)]
    pub server_label: String,

    #[builder(into)]
    pub server_url: String,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<std::collections::HashMap<String, String>>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_approval: Option<McpRequireApproval>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub defer_loading: Option<bool>,
}
```

Then modify the `ResponseTool` enum to add the fifth variant:

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseTool {
    Function(ResponseFunctionTool),
    FileSearch(ResponseFileSearchTool),
    WebSearch(ResponseWebSearchTool),
    ImageGeneration(ResponseImageGenerationTool),
    Mcp(ResponseMcpTool),
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai create_serializes_mcp_tool 2>&1 | tail -20`
Expected: both tests PASS.

- [ ] **Step 7: Format, lint, full test run**

Run:
```bash
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai 2>&1 | tail -10
```
Expected: clippy clean, all tests pass (138 + 2 new = 140).

- [ ] **Step 8: Commit**

```bash
git add rs_openai/src/interfaces/responses.rs rs_openai/src/apis/responses.rs
git commit -m "feat(responses): add mcp tool type (request side)"
```

---

### Task 2: `mcp_list_tools` / `mcp_call` / `mcp_approval_request` output items (response side)

**Files:**
- Modify: `rs_openai/src/interfaces/responses.rs` (add new types near the end of the output-item section, before the `ResponseOutputItem` enum; add three variants to `ResponseOutputItem`)
- Test: `rs_openai/src/apis/responses.rs`

**Interfaces:**
- Consumes: `ResponseTool::Mcp`/`ResponseMcpTool` from Task 1 (used to build the request in the `create` test), existing `ResponseOutputItem` enum (currently `Message`/`FunctionCall`/`FileSearchCall`/`WebSearchCall`/`ImageGenerationCall`).
- Produces: `McpListedTool` (struct: `name: String`, `description: Option<String>`, `input_schema: serde_json::Value`, `annotations: Option<serde_json::Value>`), `ResponseMcpListToolsItem` (struct: `id`, `server_label`, `tools: Vec<McpListedTool>`), `ResponseMcpCallItem` (struct: `id`, `server_label`, `name`, `arguments: String`, `output: Option<String>`, `error: Option<String>`, `approval_request_id: Option<String>`), `ResponseMcpApprovalRequestItem` (struct: `id`, `server_label`, `name`, `arguments: String`), `ResponseOutputItem::McpListTools(ResponseMcpListToolsItem)` / `::McpCall(ResponseMcpCallItem)` / `::McpApprovalRequest(ResponseMcpApprovalRequestItem)` variants — later tasks and the final cross-check rely on these exact three variant names.

- [ ] **Step 1: Write the failing test — deserializing a completed `mcp_call`**

Add to `rs_openai/src/apis/responses.rs`, after `create_deserializes_completed_image_generation_call` (around line 1485):

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai create_deserializes_completed_mcp_call 2>&1 | tail -30`
Expected: FAIL to compile — `ResponseOutputItem::McpCall` variant doesn't exist.

- [ ] **Step 3: Write the failing test — `mcp_list_tools` with a 2-tool array**

Add immediately after:

```rust
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
```

- [ ] **Step 4: Run tests to verify they fail**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai response_mcp 2>&1 | tail -30`
Expected: FAIL to compile.

- [ ] **Step 5: Implement the types**

In `rs_openai/src/interfaces/responses.rs`, insert immediately before the `ResponseOutputItem` enum definition (currently at line 420, right after `ResponseImageGenerationCallItem`'s closing brace):

```rust
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct McpListedTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
    pub annotations: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseMcpListToolsItem {
    pub id: String,
    pub server_label: String,
    pub tools: Vec<McpListedTool>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseMcpCallItem {
    pub id: String,
    pub server_label: String,
    pub name: String,
    pub arguments: String,
    pub output: Option<String>,
    pub error: Option<String>,
    pub approval_request_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseMcpApprovalRequestItem {
    pub id: String,
    pub server_label: String,
    pub name: String,
    pub arguments: String,
}
```

Then modify the `ResponseOutputItem` enum to add the three new variants:

```rust
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
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai response_mcp create_deserializes_completed_mcp_call 2>&1 | tail -20`
Expected: all PASS.

- [ ] **Step 7: Format, lint, full test run**

Run:
```bash
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai 2>&1 | tail -10
```
Expected: clippy clean, all tests pass (140 + 3 new = 143).

- [ ] **Step 8: Commit**

```bash
git add rs_openai/src/interfaces/responses.rs rs_openai/src/apis/responses.rs
git commit -m "feat(responses): add mcp output items (list_tools, call, approval_request)"
```

---

### Task 3: `mcp_approval_response` input item + mixed-array discrimination test + final cross-check

**Files:**
- Modify: `rs_openai/src/interfaces/responses.rs` (add `McpApprovalResponse` variant to `ResponseInputItem`)
- Test: `rs_openai/src/apis/responses.rs`

**Interfaces:**
- Consumes: `ResponseInputItem` (currently `Message { role, content }` / `FunctionCallOutput { call_id, output }`), `ResponseOutputItem::McpListTools`/`::McpCall`/`::McpApprovalRequest` from Task 2 (used in the mixed-array test).
- Produces: `ResponseInputItem::McpApprovalResponse { approval_request_id: String, approve: bool }` — no other task depends on this; this is the last new type in the slice.

- [ ] **Step 1: Write the failing test — `McpApprovalResponse` serializes correctly as part of `input`**

Add to `rs_openai/src/apis/responses.rs`, after `web_search_action_find_in_page_deserializes_with_required_fields` or near the other `ResponseInputItem`-related tests (search the file for existing `ResponseInputItem::FunctionCallOutput` usage to place this nearby for locality):

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai response_input_item_mcp_approval_response_serializes 2>&1 | tail -30`
Expected: FAIL to compile — `ResponseInputItem::McpApprovalResponse` variant doesn't exist.

- [ ] **Step 3: Implement the variant**

In `rs_openai/src/interfaces/responses.rs`, modify the `ResponseInputItem` enum (currently at line 23):

```rust
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
    McpApprovalResponse {
        approval_request_id: String,
        approve: bool,
    },
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai response_input_item_mcp_approval_response_serializes 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 5: Write the mixed-array discrimination test**

`ResponseOutputItem` now has 8 variants. Add a new test (do not edit the existing 5-variant test from the image_generation slice — leave it as a valid regression check for that narrower array shape) to `rs_openai/src/apis/responses.rs`, after `response_output_item_discriminates_all_five_variants_in_same_array`:

```rust
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
        assert!(matches!(items[2], ResponseOutputItem::McpApprovalRequest(_)));
    }
```

- [ ] **Step 6: Run test to verify it passes**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai response_output_item_discriminates_mcp_variants_in_same_array 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 7: Format, lint, full test run (final cross-check)**

Run:
```bash
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo build -p rs_openai
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai 2>&1 | tail -10
```
Expected: clippy clean, build clean, all tests pass (143 + 2 new = 145).

Also manually verify (read the file, don't just trust the count):
- `ResponseTool` has exactly 5 variants: `Function`, `FileSearch`, `WebSearch`, `ImageGeneration`, `Mcp`.
- `ResponseOutputItem` has exactly 8 variants: `Message`, `FunctionCall`, `FileSearchCall`, `WebSearchCall`, `ImageGenerationCall`, `McpListTools`, `McpCall`, `McpApprovalRequest`.
- `ResponseInputItem` has exactly 3 variants: `Message`, `FunctionCallOutput`, `McpApprovalResponse`.
- No `connector_id`/`tunnel_id`/`allowed_callers` fields were added to `ResponseMcpTool` (scope check).
- No `status` field was added to any of the three new output item structs (scope check).

- [ ] **Step 8: Commit**

```bash
git add rs_openai/src/interfaces/responses.rs rs_openai/src/apis/responses.rs
git commit -m "feat(responses): add mcp_approval_response input item and cross-array test"
```
