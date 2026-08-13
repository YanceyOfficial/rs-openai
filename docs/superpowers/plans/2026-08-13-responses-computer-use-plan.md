# Responses `computer_use` Tool Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the Responses API's `computer_use` tool surface — both coexisting tool config variants (`computer` GA and `computer_use_preview`), the `computer_call` output item with its 9-action union and safety checks, and the `computer_call_output` input item used to answer it.

**Architecture:** All changes land in the existing `rs_openai/src/interfaces/responses.rs` (types) and `rs_openai/src/apis/responses.rs` (tests only). Follows the same extensible-tagged-union pattern already used for `file_search`/`web_search`/`image_generation`/`mcp`. This is the last Responses tool type besides `code_interpreter` (still gated on the not-yet-implemented Containers resource).

**Tech Stack:** Rust, `serde` (`#[serde(tag = "type", rename_all = "snake_case")]` for the extensible unions, including a unit variant on `ResponseTool` for the fieldless `computer` tool), `bon::Builder`, `wiremock` for HTTP-mocked tests.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-13-responses-computer-use-design.md`. Full scope confirmed via `AskUserQuestion`: both tool config variants, the complete 9-action union, both `action`(singular)/`actions`(plural) output fields, and safety checks.
- `ResponseTool::Computer` is a **unit variant** (no associated struct/data) — under `#[serde(tag = "type", rename_all = "snake_case")]` this serializes to exactly `{"type": "computer"}`. Do not create an empty `ResponseComputerTool` struct for it.
- `ComputerSafetyCheck.code` stays a plain `String`, not a closed enum — the known safety-check categories were never confirmed with their actual wire-value strings.
- `ResponseComputerCallItem.status` stays a plain `String`, not a closed enum — deliberate, for uniformity with `ResponseFileSearchCallItem`/`ResponseWebSearchCallItem`/`ResponseImageGenerationCallItem`/`ResponseMcpCallItem`, none of which use a closed enum for `status` either, even where the value set is confirmed.
- `x`/`y`/`scroll_x`/`scroll_y` on `ComputerAction` variants are `i64` (signed) — do not use `u32`/`u64`, multi-monitor coordinates can be negative.
- `Keypress.keys: Vec<String>` is REQUIRED (not `Option`) — it's the actual key combo being pressed. This is semantically distinct from the optional `keys` modifier field on `Click`/`DoubleClick`/`Drag`/`Move`/`Scroll` (held-down modifier keys during a pointer action) — do not conflate the two or make `Keypress.keys` optional.
- `ComputerCallOutputScreenshot.kind` uses `#[serde(rename = "type")] pub kind: String` — the established `kind`-field-for-wire-`"type"`-collision convention (same as `WebSearchUserLocation`/`WebSearchActionSource`/`ResponseToolChoice::Specific`). The caller sets it explicitly via the builder (e.g. `.kind("computer_screenshot")`) — do not hardcode the literal in the struct definition.
- Stage commits with **explicit file paths** (`git add path/to/file.rs`), never `git add -A`.
- Run `cargo fmt` and `cargo clippy -p rs_openai --all-targets -- -D warnings` before every commit; both must be clean.
- Environment: this machine's shell routes HTTP through a local proxy that breaks `wiremock` tests. Prefix every `cargo test`/`build`/`clippy` invocation with `NO_PROXY=127.0.0.1,localhost`. This is a local-shell-only concern — never write it into crate code, `Cargo.toml`, or CI config.
- Any doc comment on `ResponseTool`/`ResponseOutputItem`/`ResponseInputItem` that lists "this slice implements X; later slices add Y" must be updated in the SAME task that adds the corresponding variant(s) — this is a recurring, previously-caught gap in this project. Do it as part of each task's own commit, not deferred.

---

### Task 1: `computer`/`computer_use_preview` tools (request side)

**Files:**
- Modify: `rs_openai/src/interfaces/responses.rs` (add new types immediately before the `ResponseTool` enum at line 243, right after `ResponseMcpTool`'s closing brace; add two variants to `ResponseTool`; update `ResponseTool`'s doc comment)
- Test: `rs_openai/src/apis/responses.rs`

**Interfaces:**
- Consumes: existing `ResponseTool` enum (`#[serde(tag = "type", rename_all = "snake_case")]`, currently `Function`/`FileSearch`/`WebSearch`/`ImageGeneration`/`Mcp`), existing `CreateResponseRequest`/`Responses::create()` (unchanged), existing `bon::Builder` pattern.
- Produces: `ComputerEnvironment` (enum: `Windows`/`Mac`/`Linux`/`Ubuntu`/`Browser`), `ResponseComputerUsePreviewTool` (struct: `display_width: u32`, `display_height: u32`, `environment: ComputerEnvironment`, all required), `ResponseTool::Computer` (unit variant) and `ResponseTool::ComputerUsePreview(ResponseComputerUsePreviewTool)` variant — later tasks rely on `ResponseTool` now having 7 variants total.

- [ ] **Step 1: Write the failing test — GA `computer` tool serializes as a bare tag**

Add to `rs_openai/src/apis/responses.rs` inside `mod tests`, after `create_serializes_mcp_tool_with_filter_require_approval` (search for it — it's the last `mcp`-related `create_*` test in the file):

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai create_serializes_computer_tool_as_bare_tag 2>&1 | tail -30`
Expected: FAIL to compile — `ResponseTool::Computer` variant doesn't exist.

- [ ] **Step 3: Write the failing test — `computer_use_preview` tool serializes with its fields**

Add immediately after:

```rust
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
```

Add `ComputerEnvironment`, `ResponseComputerUsePreviewTool` to the existing `use crate::interfaces::responses::{...}` import block at the top of `mod tests` — insert them alphabetically among the existing names.

- [ ] **Step 4: Run tests to verify both fail**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai create_serializes_computer 2>&1 | tail -30`
Expected: FAIL to compile.

- [ ] **Step 5: Implement the types**

In `rs_openai/src/interfaces/responses.rs`, insert immediately before the `ResponseTool` enum definition (currently at line 243, right after `ResponseMcpTool`'s closing brace):

```rust
#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ComputerEnvironment {
    Windows,
    Mac,
    Linux,
    Ubuntu,
    Browser,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ResponseComputerUsePreviewTool {
    pub display_width: u32,
    pub display_height: u32,
    pub environment: ComputerEnvironment,
}
```

Then modify the `ResponseTool` enum (also updating its doc comment):

```rust
/// A tool the model may call. Extensible union — this slice implements `Function`,
/// `FileSearch`, `WebSearch`, `ImageGeneration`, `Mcp`, `Computer`, and `ComputerUsePreview`;
/// only `CodeInterpreter` remains as a future non-breaking variant (gated on the Containers
/// resource).
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
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai create_serializes_computer 2>&1 | tail -20`
Expected: both tests PASS.

- [ ] **Step 7: Format, lint, full test run**

Run:
```bash
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai 2>&1 | tail -10
```
Expected: clippy clean, all tests pass (145 + 2 new = 147).

- [ ] **Step 8: Commit**

```bash
git add rs_openai/src/interfaces/responses.rs rs_openai/src/apis/responses.rs
git commit -m "feat(responses): add computer and computer_use_preview tool types (request side)"
```

---

### Task 2: `computer_call` output item with action union and safety checks (response side)

**Files:**
- Modify: `rs_openai/src/interfaces/responses.rs` (add new types immediately before the `ResponseOutputItem` enum, currently at line 516, right after `ResponseMcpApprovalRequestItem`'s closing brace; add one variant to `ResponseOutputItem`; update `ResponseOutputItem`'s doc comment)
- Test: `rs_openai/src/apis/responses.rs`

**Interfaces:**
- Consumes: `ResponseTool::Computer`/`ResponseTool::ComputerUsePreview` from Task 1 (not directly used in this task's tests, but establishes that `ResponseTool` now has 7 variants), existing `ResponseOutputItem` enum (currently `Message`/`FunctionCall`/`FileSearchCall`/`WebSearchCall`/`ImageGenerationCall`/`McpListTools`/`McpCall`/`McpApprovalRequest`, 8 variants).
- Produces: `ComputerActionButton` (enum: `Left`/`Right`/`Wheel`/`Back`/`Forward`), `ComputerAction` (9-variant tagged union: `Click`/`DoubleClick`/`Drag`/`Keypress`/`Move`/`Screenshot`/`Scroll`/`Type`/`Wait` — exact field shapes below), `ComputerSafetyCheck` (struct: `id: String`, `code: String`, `message: String`), `ResponseComputerCallItem` (struct: `id`, `call_id`, `status: String`, `action: Option<ComputerAction>`, `actions: Option<Vec<ComputerAction>>`, `pending_safety_checks: Option<Vec<ComputerSafetyCheck>>`), `ResponseOutputItem::ComputerCall(ResponseComputerCallItem)` variant — later tasks and the final cross-check rely on `ResponseOutputItem` now having 9 variants, and on `ComputerSafetyCheck` being reused by Task 3's `ResponseInputItem::ComputerCallOutput.acknowledged_safety_checks`.

- [ ] **Step 1: Write the failing test — deserializing a GA-shape `computer_call` with `actions` array**

Add to `rs_openai/src/apis/responses.rs`, after `create_deserializes_completed_mcp_call` (search for it):

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai create_deserializes_computer_call_with_actions_array 2>&1 | tail -30`
Expected: FAIL to compile — `ResponseOutputItem::ComputerCall` doesn't exist.

- [ ] **Step 3: Write the failing test — `computer_call` with `pending_safety_checks` populated (preview shape, single `action`)**

Add immediately after:

```rust
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
```

Add `ComputerAction`, `ComputerActionButton` to the existing `use crate::interfaces::responses::{...}` import block, alphabetically.

- [ ] **Step 4: Run tests to verify they fail**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai computer 2>&1 | tail -30`
Expected: FAIL to compile.

- [ ] **Step 5: Implement the types**

In `rs_openai/src/interfaces/responses.rs`, insert immediately before the `ResponseOutputItem` enum definition (currently at line 516, right after `ResponseMcpApprovalRequestItem`'s closing brace):

```rust
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ComputerActionButton {
    Left,
    Right,
    Wheel,
    Back,
    Forward,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ComputerAction {
    Click {
        x: i64,
        y: i64,
        #[serde(skip_serializing_if = "Option::is_none")]
        button: Option<ComputerActionButton>,
        #[serde(skip_serializing_if = "Option::is_none")]
        keys: Option<Vec<String>>,
    },
    DoubleClick {
        x: i64,
        y: i64,
        #[serde(skip_serializing_if = "Option::is_none")]
        keys: Option<Vec<String>>,
    },
    Drag {
        path: Vec<(i64, i64)>,
        #[serde(skip_serializing_if = "Option::is_none")]
        keys: Option<Vec<String>>,
    },
    Keypress {
        keys: Vec<String>,
    },
    Move {
        x: i64,
        y: i64,
        #[serde(skip_serializing_if = "Option::is_none")]
        keys: Option<Vec<String>>,
    },
    Screenshot {},
    Scroll {
        x: i64,
        y: i64,
        scroll_x: i64,
        scroll_y: i64,
        #[serde(skip_serializing_if = "Option::is_none")]
        keys: Option<Vec<String>>,
    },
    Type {
        text: String,
    },
    Wait {},
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ComputerSafetyCheck {
    #[builder(into)]
    pub id: String,

    #[builder(into)]
    pub code: String,

    #[builder(into)]
    pub message: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseComputerCallItem {
    pub id: String,
    pub call_id: String,
    pub status: String,
    pub action: Option<ComputerAction>,
    pub actions: Option<Vec<ComputerAction>>,
    pub pending_safety_checks: Option<Vec<ComputerSafetyCheck>>,
}
```

Note: `ComputerActionButton` derives `PartialEq` (added to its derive list above, alongside the pattern used by sibling enums in this file) because Task 2's own test (`create_deserializes_computer_call_with_actions_array`) asserts `*button == Some(ComputerActionButton::Left)` via `assert_eq!`, which requires `PartialEq` on `ComputerActionButton` (and transitively on `Option<ComputerActionButton>`). Keep this derive — it is required for the test to compile, not optional.

Then modify the `ResponseOutputItem` enum (also updating its doc comment):

```rust
/// One item in a response's `output` array. Extensible union — this slice implements
/// `Message`/`FunctionCall`/`FileSearchCall`/`WebSearchCall`/`ImageGenerationCall`/`McpListTools`/`McpCall`/`McpApprovalRequest`/`ComputerCall`;
/// only `Reasoning`/`CodeInterpreterCall`/etc. remain as future non-breaking variants.
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
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai computer 2>&1 | tail -20`
Expected: all PASS.

- [ ] **Step 7: Format, lint, full test run**

Run:
```bash
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai 2>&1 | tail -10
```
Expected: clippy clean, all tests pass (147 + 3 new = 150).

- [ ] **Step 8: Commit**

```bash
git add rs_openai/src/interfaces/responses.rs rs_openai/src/apis/responses.rs
git commit -m "feat(responses): add computer_call output item with action union and safety checks"
```

---

### Task 3: `computer_call_output` input item + mixed-array discrimination test + final cross-check

**Files:**
- Modify: `rs_openai/src/interfaces/responses.rs` (add `ComputerCallOutputScreenshot` struct; add `ComputerCallOutput` variant to `ResponseInputItem`)
- Test: `rs_openai/src/apis/responses.rs`

**Interfaces:**
- Consumes: `ComputerSafetyCheck` from Task 2 (reused for `acknowledged_safety_checks`), `ResponseInputItem` (currently `Message`/`FunctionCallOutput`/`McpApprovalResponse`, 3 variants), `ResponseOutputItem::ComputerCall` from Task 2 (used in the mixed-array test).
- Produces: `ComputerCallOutputScreenshot` (struct: `kind: String` via `#[serde(rename = "type")]`, `image_url: String`, `detail: Option<String>`), `ResponseInputItem::ComputerCallOutput { id: Option<String>, call_id: String, output: ComputerCallOutputScreenshot, acknowledged_safety_checks: Option<Vec<ComputerSafetyCheck>>, status: Option<String> }` — no other task depends on this; this is the last new type in the slice, and this is the LAST task in the plan (the only remaining unimplemented Responses tool type after this is `code_interpreter`, gated on Containers).

- [ ] **Step 1: Write the failing test — `ComputerCallOutput` serializes correctly as part of `input`**

Add to `rs_openai/src/apis/responses.rs`, near the other `ResponseInputItem`-related tests (search the file for `response_input_item_mcp_approval_response_serializes` and place this immediately after it):

```rust
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
```

Add `ComputerCallOutputScreenshot`, `ComputerSafetyCheck` to the existing `use crate::interfaces::responses::{...}` import block, alphabetically.

- [ ] **Step 2: Run test to verify it fails**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai response_input_item_computer_call_output_serializes_with_safety_checks 2>&1 | tail -30`
Expected: FAIL to compile — `ResponseInputItem::ComputerCallOutput` / `ComputerCallOutputScreenshot` don't exist.

- [ ] **Step 3: Implement the types**

In `rs_openai/src/interfaces/responses.rs`, insert `ComputerCallOutputScreenshot` immediately before the `ResponseInputItem` enum (currently at line 23), i.e. right after `ResponseInputRole`'s closing brace (line 19) and before the `#[derive(...)]` line that starts `ResponseInputItem`:

```rust
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ComputerCallOutputScreenshot {
    #[builder(into)]
    #[serde(rename = "type")]
    pub kind: String,

    #[builder(into)]
    pub image_url: String,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}
```

Then modify the `ResponseInputItem` enum:

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
    ComputerCallOutput {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        call_id: String,
        output: ComputerCallOutputScreenshot,
        #[serde(skip_serializing_if = "Option::is_none")]
        acknowledged_safety_checks: Option<Vec<ComputerSafetyCheck>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<String>,
    },
}
```

Note: `ComputerCallOutputScreenshot` is defined before `ResponseInputItem` in the file, but `ComputerSafetyCheck` (used by `ComputerCallOutput`) is defined much later (Task 2's insertion point, before `ResponseOutputItem`). This is fine in Rust — item order within a module doesn't matter for type resolution — but if you place `ComputerCallOutputScreenshot` literally at the top of the file, do NOT duplicate `ComputerSafetyCheck`; it already exists from Task 2.

- [ ] **Step 4: Run test to verify it passes**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai response_input_item_computer_call_output_serializes_with_safety_checks 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 5: Write the mixed-array discrimination test**

`ResponseOutputItem` now has 9 variants. Add a new test (do not edit the existing 3-variant mcp discrimination test — leave it as a valid regression check) to `rs_openai/src/apis/responses.rs`, after `response_output_item_discriminates_mcp_variants_in_same_array`:

```rust
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
```

- [ ] **Step 6: Run test to verify it passes**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai response_output_item_discriminates_computer_call_in_same_array 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 7: Format, lint, full test run (final cross-check)**

Run:
```bash
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo build -p rs_openai
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai 2>&1 | tail -10
```
Expected: clippy clean, build clean, all tests pass (150 + 2 new = 152).

Also manually verify (read the file, don't just trust the count):
- `ResponseTool` has exactly 7 variants: `Function`, `FileSearch`, `WebSearch`, `ImageGeneration`, `Mcp`, `Computer`, `ComputerUsePreview`.
- `ResponseOutputItem` has exactly 9 variants: `Message`, `FunctionCall`, `FileSearchCall`, `WebSearchCall`, `ImageGenerationCall`, `McpListTools`, `McpCall`, `McpApprovalRequest`, `ComputerCall`.
- `ResponseInputItem` has exactly 4 variants: `Message`, `FunctionCallOutput`, `McpApprovalResponse`, `ComputerCallOutput`.
- `ComputerAction` has exactly 9 variants: `Click`, `DoubleClick`, `Drag`, `Keypress`, `Move`, `Screenshot`, `Scroll`, `Type`, `Wait`.
- `ComputerSafetyCheck.code` and `ResponseComputerCallItem.status` are both plain `String`, not enums (scope check).
- No streaming-related changes were made to `ResponseStreamEvent` (scope check).

- [ ] **Step 8: Commit**

```bash
git add rs_openai/src/interfaces/responses.rs rs_openai/src/apis/responses.rs
git commit -m "feat(responses): add computer_call_output input item and cross-array test"
```
