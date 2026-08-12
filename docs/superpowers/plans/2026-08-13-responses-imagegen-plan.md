# Responses API: `image_generation` Tool Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the `image_generation` tool type (request + `ImageGenerationCall` output item) to the Responses API — the fourth variant on `ResponseTool`/`ResponseOutputItem`, reusing six existing enums from `interfaces/images.rs` directly rather than duplicating them.

**Architecture:** Two feature tasks in the existing `rs_openai/src/interfaces/responses.rs` (no new files): Task 1 adds the request-side type and `ResponseTool::ImageGeneration`; Task 2 adds the response-side type and `ResponseOutputItem::ImageGenerationCall`. Task 3 is a lightweight final cross-check.

**Tech Stack:** Same as the rest of the crate (Rust edition 2024, `bon` builders, `serde`, `wiremock` tests) — no new dependencies.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-13-responses-imagegen-design.md`.
- **Deliberately out of scope this round — do not add:** the remaining 2 tool types (`computer_use`, `mcp`) and their output items, `code_interpreter` (still gated on the not-yet-implemented Containers resource), partial-image streaming events (`ResponseStreamEvent` stays scoped to text + function-calling events), echoing `background`/`output_format`/`quality`/`size` back on `ResponseImageGenerationCallItem` (not confirmed present on the real output item this session).
- **`ResponseImageGenerationTool` MUST reuse six existing enums from `crate::interfaces::images` via qualified paths — do NOT define duplicates:** `background: Option<crate::interfaces::images::Background>`, `input_fidelity: Option<crate::interfaces::images::InputFidelity>`, `moderation: Option<crate::interfaces::images::Moderation>`, `output_format: Option<crate::interfaces::images::OutputFormat>`, `quality: Option<crate::interfaces::images::Quality>`, `size: Option<crate::interfaces::images::ImageSize>`. Only `ImageGenerationAction` (a new 3-value enum: `Generate`/`Edit`/`Auto`) has no existing type to reuse.
- `ResponseTool::ImageGeneration(ResponseImageGenerationTool)` and `ResponseOutputItem::ImageGenerationCall(ResponseImageGenerationCallItem)` are ADDITIONS to already-existing extensible tagged unions (both already have three/four variants respectively) — do not redefine either enum from scratch, add one more variant to what's there.
- `ResponseImageGenerationCallItem.status` stays a plain `String`, NOT a closed enum — same caution already applied to `file_search`'s and `web_search`'s `status` fields. `result` and `revised_prompt` are both `Option<String>` — not confirmed present on every response (e.g. an in-progress call).
- `model` on `ResponseImageGenerationTool` stays a plain `Option<String>` — matches this crate's universal convention for model-name fields (they accept both named presets and arbitrary future model strings), same as `CreateResponseRequest.model`/`CreateChatRequest.model`.
- Every task's "done" check includes `cargo build --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace`, and `cargo fmt --all -- --check`.
- **This machine needs `NO_PROXY=127.0.0.1,localhost` prefixed on every `cargo test` invocation**, or every `wiremock`-based HTTP test fails with a misleading `JSONDeserialize(Error("EOF while parsing a value", ...))` (a local proxy-client environment quirk, not a code defect — pure `#[test]`s with no HTTP are unaffected). E.g. `NO_PROXY=127.0.0.1,localhost cargo test --workspace`.
- **Stage only the exact files you changed when committing — use explicit file paths in `git add`, never `git add -A` or `git add .`.** This repo has a known, harmless (and now gitignored) source of untracked binary files (`rs_openai/*.mp3`/`*.opus`, byproducts of an unrelated Audio module test); explicit paths are still the correct habit.
- Follow TDD: write the failing test first, confirm it fails for the expected reason, then implement.
- All changes land in the existing `rs_openai/src/interfaces/responses.rs` and `rs_openai/src/apis/responses.rs` (test module only) — no new files, no changes to `apis/mod.rs`, `interfaces/mod.rs`, `client.rs`, or `interfaces/images.rs`.

---

### Task 1: `image_generation` tool (request side)

**Files:**
- Modify: `rs_openai/src/interfaces/responses.rs`
- Modify: `rs_openai/src/apis/responses.rs` (test module only)

**Interfaces:**
- Consumes: `ResponseTool` (already exists); `crate::interfaces::images::{Background, InputFidelity, Moderation, OutputFormat, Quality, ImageSize}` (all already exist, from the pre-existing Images API module — this is the first slice to reuse them from Responses).
- Produces: `ImageGenerationAction`, `ResponseImageGenerationTool`, `ResponseTool::ImageGeneration` (new variant). Task 2 does not depend on this task's specifics.

- [ ] **Step 1: Write the failing tests**

In `rs_openai/src/apis/responses.rs`, extend the test module's imports. Find the block starting `use crate::interfaces::responses::{` and add `ImageGenerationAction`, `ResponseImageGenerationTool` to the list (keep it alphabetically sorted, matching the existing style) — replace:

```rust
    use crate::interfaces::responses::{
        CreateResponseRequest, FileSearchRankingOptions, ListInputItemsRequest,
        ResponseConversationParam, ResponseFileSearchTool, ResponseFunctionTool, ResponseInput,
        ResponseInputItem, ResponseInputRole, ResponseOutputItem, ResponseStreamEvent,
        ResponseTool, ResponseToolChoice, ResponseToolChoiceFunctionName, ResponseToolChoiceMode,
        ResponseWebSearchTool, WebSearchAction, WebSearchContextSize, WebSearchUserLocation,
    };
```

with:

```rust
    use crate::interfaces::images::{Background, ImageSize, Quality};
    use crate::interfaces::responses::{
        CreateResponseRequest, FileSearchRankingOptions, ImageGenerationAction,
        ListInputItemsRequest, ResponseConversationParam, ResponseFileSearchTool,
        ResponseFunctionTool, ResponseImageGenerationTool, ResponseInput, ResponseInputItem,
        ResponseInputRole, ResponseOutputItem, ResponseStreamEvent, ResponseTool,
        ResponseToolChoice, ResponseToolChoiceFunctionName, ResponseToolChoiceMode,
        ResponseWebSearchTool, WebSearchAction, WebSearchContextSize, WebSearchUserLocation,
    };
```

(If your local copy of this import block has diverged slightly in wording, that's fine — the key change is adding `use crate::interfaces::images::{Background, ImageSize, Quality};` as a new import line, plus adding `ImageGenerationAction`/`ResponseImageGenerationTool` to the existing `responses::` import list.)

Add this test to the end of the `mod tests` block:

```rust

    #[tokio::test]
    async fn create_serializes_image_generation_tool_reusing_images_module_enums() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/responses"))
            .and(body_json(serde_json::json!({
                "model": "gpt-5-mini",
                "input": "Draw a gray tabby cat hugging an otter",
                "tools": [{
                    "type": "image_generation",
                    "action": "generate",
                    "background": "transparent",
                    "quality": "high",
                    "size": "1024x1024"
                }]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "resp_5",
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
        let tool = ResponseTool::ImageGeneration(
            ResponseImageGenerationTool::builder()
                .action(ImageGenerationAction::Generate)
                .background(Background::Transparent)
                .quality(Quality::High)
                .size(ImageSize::S1024x1024)
                .build(),
        );
        let req = CreateResponseRequest::builder()
            .model("gpt-5-mini")
            .input(ResponseInput::Text(
                "Draw a gray tabby cat hugging an otter".to_string(),
            ))
            .tools(vec![tool])
            .build();

        let res = client.responses().create(&req).await.unwrap();
        assert_eq!(res.id, "resp_5");
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::responses`
Expected: FAIL with compile errors — `ImageGenerationAction`, `ResponseImageGenerationTool`, and `ResponseTool::ImageGeneration` don't exist yet.

- [ ] **Step 3: Implement**

In `rs_openai/src/interfaces/responses.rs`, add the request-side types right before `ResponseTool`'s definition. Replace:

```rust
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

with:

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ImageGenerationAction {
    Generate,
    Edit,
    Auto,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ResponseImageGenerationTool {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<ImageGenerationAction>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<crate::interfaces::images::Background>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_fidelity: Option<crate::interfaces::images::InputFidelity>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub moderation: Option<crate::interfaces::images::Moderation>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_compression: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<crate::interfaces::images::OutputFormat>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial_images: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<crate::interfaces::images::Quality>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<crate::interfaces::images::ImageSize>,
}

/// A tool the model may call. Extensible union — this slice implements `Function`,
/// `FileSearch`, `WebSearch`, and `ImageGeneration`; later slices add `CodeInterpreter`/
/// `ComputerUse`/`Mcp` as new non-breaking variants.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseTool {
    Function(ResponseFunctionTool),
    FileSearch(ResponseFileSearchTool),
    WebSearch(ResponseWebSearchTool),
    ImageGeneration(ResponseImageGenerationTool),
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
feat: add the image_generation tool request type to Responses

ResponseTool::ImageGeneration is the fourth variant on the already-
extensible union. Reuses Background/InputFidelity/Moderation/
OutputFormat/Quality/ImageSize directly from interfaces/images.rs
(the pre-existing DALL-E-style Images API) rather than duplicating
them -- the largest cross-module type reuse in this project so far.
See docs/superpowers/specs/2026-08-13-responses-imagegen-design.md.
EOF
)"
```

---

### Task 2: `image_generation_call` output item (response side)

**Files:**
- Modify: `rs_openai/src/interfaces/responses.rs`
- Modify: `rs_openai/src/apis/responses.rs` (test module only)

**Interfaces:**
- Consumes: `ResponseOutputItem` (already exists).
- Produces: `ResponseImageGenerationCallItem`, `ResponseOutputItem::ImageGenerationCall` (new variant). Nothing later in this plan depends on this task's specifics — Task 3 is a read-only cross-check.

- [ ] **Step 1: Write the failing tests**

In `rs_openai/src/apis/responses.rs`, extend the test module's imports. Add `ResponseImageGenerationCallItem` is not needed directly (tests will match on `ResponseOutputItem::ImageGenerationCall(item)` and access `item.id` etc. without importing the item type by name) — no import changes needed beyond what Task 1 already added, since `ResponseOutputItem` is already imported.

Add these tests to the end of the `mod tests` block:

```rust

    #[tokio::test]
    async fn create_deserializes_completed_image_generation_call() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/responses"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "resp_6",
                "object": "response",
                "created_at": 1700000000_i64,
                "status": "completed",
                "error": null,
                "incomplete_details": null,
                "model": "gpt-5-mini",
                "output": [{
                    "type": "image_generation_call",
                    "id": "ig_1",
                    "status": "completed",
                    "revised_prompt": "A gray tabby cat hugging an otter, warm heartwarming style.",
                    "result": "base64imagedatahere"
                }],
                "usage": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateResponseRequest::builder()
            .model("gpt-5-mini")
            .input(ResponseInput::Text("Draw a cat".to_string()))
            .build();

        let res = client.responses().create(&req).await.unwrap();
        match &res.output[0] {
            ResponseOutputItem::ImageGenerationCall(call) => {
                assert_eq!(call.id, "ig_1");
                assert_eq!(call.status, "completed");
                assert_eq!(call.result.as_deref(), Some("base64imagedatahere"));
                assert_eq!(
                    call.revised_prompt.as_deref(),
                    Some("A gray tabby cat hugging an otter, warm heartwarming style.")
                );
            }
            other => panic!("expected ImageGenerationCall, got {other:?}"),
        }
    }

    #[test]
    fn response_image_generation_call_deserializes_with_null_result_and_prompt() {
        let value = serde_json::json!({
            "type": "image_generation_call",
            "id": "ig_2",
            "status": "in_progress",
            "result": null,
            "revised_prompt": null
        });

        let item: ResponseOutputItem = serde_json::from_value(value).unwrap();
        match item {
            ResponseOutputItem::ImageGenerationCall(call) => {
                assert_eq!(call.status, "in_progress");
                assert!(call.result.is_none());
                assert!(call.revised_prompt.is_none());
            }
            other => panic!("expected ImageGenerationCall, got {other:?}"),
        }
    }

    #[test]
    fn response_output_item_discriminates_all_five_variants_in_same_array() {
        let value = serde_json::json!([
            {
                "type": "message",
                "id": "msg_1",
                "role": "assistant",
                "status": "completed",
                "content": [{"type": "output_text", "text": "Here you go.", "annotations": []}]
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
            },
            {
                "type": "image_generation_call",
                "id": "ig_3",
                "status": "completed",
                "result": "abc",
                "revised_prompt": null
            }
        ]);

        let items: Vec<ResponseOutputItem> = serde_json::from_value(value).unwrap();
        assert_eq!(items.len(), 5);
        assert!(matches!(items[0], ResponseOutputItem::Message(_)));
        assert!(matches!(items[1], ResponseOutputItem::FunctionCall(_)));
        assert!(matches!(items[2], ResponseOutputItem::FileSearchCall(_)));
        assert!(matches!(items[3], ResponseOutputItem::WebSearchCall(_)));
        assert!(matches!(items[4], ResponseOutputItem::ImageGenerationCall(_)));
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace --lib apis::responses`
Expected: FAIL with compile errors — `ResponseOutputItem::ImageGenerationCall` doesn't exist yet.

- [ ] **Step 3: Implement**

In `rs_openai/src/interfaces/responses.rs`, add the response-side type right before `ResponseOutputItem`'s definition. Replace:

```rust
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

with:

```rust
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseImageGenerationCallItem {
    pub id: String,
    pub status: String,
    pub result: Option<String>,
    pub revised_prompt: Option<String>,
}

/// One item in a response's `output` array. Extensible union — this slice implements
/// `Message`/`FunctionCall`/`FileSearchCall`/`WebSearchCall`/`ImageGenerationCall`; later
/// slices add `Reasoning`/`ComputerCall`/`CodeInterpreterCall`/`McpCall`/etc. as new
/// non-breaking variants.
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseOutputItem {
    Message(ResponseOutputMessage),
    FunctionCall(ResponseFunctionCallItem),
    FileSearchCall(ResponseFileSearchCallItem),
    WebSearchCall(ResponseWebSearchCallItem),
    ImageGenerationCall(ResponseImageGenerationCallItem),
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

Stage only the two files you changed by exact path:

```bash
git add rs_openai/src/interfaces/responses.rs rs_openai/src/apis/responses.rs
git commit -m "$(cat <<'EOF'
feat: add the image_generation_call output item to Responses

ResponseOutputItem::ImageGenerationCall is the fifth variant on the
already-extensible union. result/revised_prompt stay Option<String>
-- not confirmed present on an in-progress call, matching this
crate's established caution for partially-confirmed response shapes.
See docs/superpowers/specs/2026-08-13-responses-imagegen-design.md.
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

Run: `grep -n "pub struct\|pub enum" rs_openai/src/interfaces/responses.rs | grep -i "imagegen"`
Expected: exactly 3 new items — `ImageGenerationAction`, `ResponseImageGenerationTool`, `ResponseImageGenerationCallItem`.

Run: `grep -n "ImageGeneration(ResponseImageGenerationTool)\|ImageGenerationCall(ResponseImageGenerationCallItem)" rs_openai/src/interfaces/responses.rs`
Expected: exactly 2 matches — the two new enum variant declarations.

Run: `grep -n "crate::interfaces::images::" rs_openai/src/interfaces/responses.rs`
Expected: exactly 6 matches — `Background`, `InputFidelity`, `Moderation`, `OutputFormat`, `Quality`, `ImageSize`, each referenced exactly once as a field type (confirming reuse, not duplication).

Run: `grep -n "pub enum Background\|pub enum InputFidelity\|pub enum Moderation\|pub enum OutputFormat\|pub enum Quality\|pub enum ImageSize" rs_openai/src/interfaces/responses.rs`
Expected: no output — confirms none of the six reused enums were accidentally redefined in `responses.rs` instead of imported from `images.rs`.

Run: `grep -rn "ComputerUse\|Mcp\b\|CodeInterpreter\|partial_image" rs_openai/src/interfaces/responses.rs`
Expected: no output (or only doc-comment mentions explaining deferral — confirm any hits are doc-comment-only, not actual fields/variants).

- [ ] **Step 2: Run the full verification suite on the assembled branch**

Run: `cargo fmt --all -- --check`
Expected: clean.

Run: `cargo build --workspace --all-targets`
Expected: clean build.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0.

Run: `NO_PROXY=127.0.0.1,localhost cargo test --workspace`
Expected: PASS — full suite green, with the responses module contributing 4 new tests total (1 from Task 1, 3 from Task 2) beyond the pre-existing baseline.

- [ ] **Step 3: Done**

If Steps 1-2 all passed clean, this task made no changes — no commit needed. If any check found a real gap, stop and report exactly what it found rather than silently patching it — the fix belongs in the task that owns the affected file, following that task's own conventions.

---
