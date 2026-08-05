# Images: `model` Field + gpt-image Capability Set Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the `model` field and the full gpt-image-model-conditional capability set to `rs_openai`'s Images module (generate + edit), fix a real pre-existing bug in `ImageData`'s response shape, and add the new response-level fields OpenAI currently documents — all live-verified against current OpenAI docs during design.

**Architecture:** Three tasks, ordered so each builds on a stable foundation: (1) shared enums + the `ImageData`/`ImageResponse` shape fixes that both `create` and `create_edit` responses depend on, (2) `CreateImageRequest` (generate) new fields, (3) `CreateImageEditRequest` new fields plus its two breaking changes (`image`→`images` rename, `response_format` removed) and the corresponding multipart rewrite in `apis/images.rs`. `CreateImageVariationRequest` needs no field changes — it only benefits from Task 1's `ImageSize` expansion.

**Tech Stack:** Same as the rest of the crate (Rust edition 2024, `bon` builders, `serde`, `strum::Display` for multipart form values, `wiremock` tests) — no new dependencies.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-04-images-model-field-design.md`. Background research: `docs/superpowers/research/2026-07-31-field-diff-images-audio-files-fine-tuning.md` (note: several `[verify]` items in that doc were resolved during design against live docs — trust the spec's verified values over the original research doc where they differ).
- **This slice contains real, intentional breaking changes** — unlike the Chat tool-calling slice, this is not purely additive. Do not treat these as mistakes to avoid: `ImageData` changes from an enum to a struct; `CreateImageEditRequest.image: File` becomes `images: Vec<File>`; `CreateImageEditRequest.response_format` is removed entirely (confirmed absent from the real edit endpoint). Each is justified in the spec as fixing existing-broken-or-dead code, not as a compatibility-optional feature choice.
- Do not add streaming (`stream`/`partial_images`) or `file_id`/URL reference support for edit's input images — both explicitly out of scope for this slice per the spec's Non-goals, deferred to future slices.
- `CreateImageVariationRequest` gets no field changes in this plan — do not add `model` or any gpt-image field to it; it's dall-e-2-only in the current API.
- Every task's "done" check includes `cargo build --workspace --all-targets`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace`, and `cargo fmt --all -- --check`.
- Follow TDD: write the failing test first, confirm it fails for the expected reason (compile error, since tests reference types/fields that don't exist yet), then implement.
- New enum derive lists are exact and deliberate, not to be "made consistent" across enums: `Quality`, `Background`, `OutputFormat` need `Deserialize` and `PartialEq` because they appear in `ImageResponse` (response-side); `Moderation`, `Style`, `InputFidelity` are request-only, matching the existing `ResponseFormat` type's minimal `Serialize`-only pattern — do not add `Deserialize`/`PartialEq` to these three.

---

### Task 1: Shared enums, `ImageSize` expansion, `ImageData` shape fix, `ImageResponse` new fields

**Files:**
- Modify: `rs_openai/src/interfaces/images.rs`
- Modify: `rs_openai/src/apis/images.rs` (tests only)

**Interfaces:**
- Consumes: nothing from earlier tasks (first task).
- Produces: `Quality`, `Background`, `OutputFormat`, `Moderation`, `Style`, `InputFidelity` enums; `ImageSize` gains `Auto`/`S1536x1024`/`S1024x1536`/`S1792x1024`/`S1024x1792` variants plus `Deserialize`+`PartialEq` derives; `ImageData { url: Option<String>, b64_json: Option<String>, revised_prompt: Option<String> }` (struct, replacing the old enum); `ImageResponse` gains `background`/`output_format`/`quality`/`size`/`usage` fields; `ImageUsage`/`ImageUsageTokenDetails` structs. Tasks 2 and 3 both consume `Quality`/`Background`/`OutputFormat`/`Moderation`/`InputFidelity`/`ImageSize` on their respective request types — use these exact names.

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)] mod tests` block in `rs_openai/src/apis/images.rs`. First, extend the existing imports line — replace:

```rust
    use crate::interfaces::images::{
        CreateImageEditRequest, CreateImageRequest, ImageSize, ResponseFormat,
    };
```

with:

```rust
    use crate::interfaces::images::{
        Background, CreateImageEditRequest, CreateImageRequest, ImageSize, OutputFormat,
        Quality, ResponseFormat,
    };
```

Then add these two tests (place them after the existing `create_maps_api_error` test, before `create_edit_sends_masks_own_bytes_not_the_images`):

```rust
    /// Regression test for a real bug: `ImageData` used to be a two-variant enum
    /// (`Url(String) | B64Json(String)`) that only coincidentally deserialized today's
    /// single-field mock responses (Rust's default externally-tagged newtype-variant JSON
    /// happens to look like `{"url": "..."}"`) — it could never represent a real dall-e-3
    /// response, which has `url` AND `revised_prompt` together on the same item.
    #[tokio::test]
    async fn create_deserializes_dalle3_response_with_url_and_revised_prompt() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/images/generations"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "created": 1700000000_i64,
                "data": [{
                    "url": "https://example.com/image.png",
                    "revised_prompt": "A vivid red bicycle leaning against a brick wall"
                }]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateImageRequest::builder()
            .prompt("a red bicycle")
            .build();

        let res = client.images().create(&req).await.unwrap();
        assert_eq!(
            res.data[0].url.as_deref(),
            Some("https://example.com/image.png")
        );
        assert_eq!(
            res.data[0].revised_prompt.as_deref(),
            Some("A vivid red bicycle leaning against a brick wall")
        );
        assert_eq!(res.data[0].b64_json, None);
    }

    #[tokio::test]
    async fn create_deserializes_response_level_fields_including_usage() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/images/generations"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "created": 1700000000_i64,
                "background": "transparent",
                "output_format": "png",
                "quality": "high",
                "size": "1024x1024",
                "usage": {
                    "input_tokens": 50,
                    "input_tokens_details": {"image_tokens": 30, "text_tokens": 20},
                    "output_tokens": 1200,
                    "total_tokens": 1250,
                    "output_tokens_details": {"image_tokens": 1200, "text_tokens": 0}
                },
                "data": [{"b64_json": "aGVsbG8="}]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateImageRequest::builder()
            .prompt("a transparent PNG icon")
            .model("gpt-image-1")
            .build();

        let res = client.images().create(&req).await.unwrap();
        assert_eq!(res.background, Some(Background::Transparent));
        assert_eq!(res.output_format, Some(OutputFormat::Png));
        assert_eq!(res.quality, Some(Quality::High));
        assert_eq!(res.size, Some(ImageSize::S1024x1024));

        let usage = res.usage.as_ref().expect("expected usage to be present");
        assert_eq!(usage.input_tokens, 50);
        assert_eq!(usage.input_tokens_details.image_tokens, 30);
        assert_eq!(usage.input_tokens_details.text_tokens, 20);
        assert_eq!(usage.output_tokens, 1200);
        assert_eq!(usage.total_tokens, 1250);
        let output_details = usage
            .output_tokens_details
            .as_ref()
            .expect("expected output_tokens_details to be present");
        assert_eq!(output_details.image_tokens, 1200);
        assert_eq!(output_details.text_tokens, 0);
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --workspace --lib apis::images`
Expected: FAIL with compile errors — `Background`, `OutputFormat`, `Quality` types don't exist yet, `ImageData.url`/`.revised_prompt`/`.b64_json` field access doesn't compile against the old enum, `CreateImageRequest.model(...)` builder method doesn't exist, `ImageResponse.background`/`.output_format`/`.quality`/`.size`/`.usage` fields don't exist, `ImageSize` has no `PartialEq` for the `assert_eq!` comparisons.

- [ ] **Step 3: Implement — new shared enums**

In `rs_openai/src/interfaces/images.rs`, after the existing `ResponseFormat` enum definition (before `ImageSize`), add:

```rust
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum Quality {
    #[strum(serialize = "standard")]
    Standard, // dall-e-3
    #[strum(serialize = "hd")]
    Hd, // dall-e-3
    #[strum(serialize = "low")]
    Low, // gpt-image
    #[strum(serialize = "medium")]
    Medium, // gpt-image
    #[strum(serialize = "high")]
    High, // gpt-image
    #[strum(serialize = "auto")]
    Auto, // gpt-image
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum Background {
    #[strum(serialize = "transparent")]
    Transparent,
    #[strum(serialize = "opaque")]
    Opaque,
    #[strum(serialize = "auto")]
    Auto,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    #[strum(serialize = "png")]
    Png,
    #[strum(serialize = "jpeg")]
    Jpeg,
    #[strum(serialize = "webp")]
    Webp,
}

/// Content-moderation strictness for gpt-image models. Request-only — OpenAI does not echo
/// this back on the response, unlike `Quality`/`Background`/`OutputFormat`.
#[derive(Debug, Serialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum Moderation {
    #[strum(serialize = "low")]
    Low,
    #[strum(serialize = "auto")]
    Auto,
}

/// `dall-e-3`-only: nudges the model toward a more natural, less hyper-real/dramatic look.
/// Request-only.
#[derive(Debug, Serialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    #[strum(serialize = "vivid")]
    Vivid,
    #[strum(serialize = "natural")]
    Natural,
}

/// Edit-only, gpt-image models: how closely the edited output should match the input
/// image's fine details. Request-only.
#[derive(Debug, Serialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum InputFidelity {
    #[strum(serialize = "high")]
    High,
    #[strum(serialize = "low")]
    Low,
}
```

- [ ] **Step 4: Implement — expand `ImageSize`**

Replace:

```rust
#[derive(Default, Debug, Serialize, Clone, strum::Display)]
pub enum ImageSize {
    #[strum(serialize = "256x256")]
    #[serde(rename = "256x256")]
    S256x256,

    #[strum(serialize = "512x512")]
    #[serde(rename = "512x512")]
    S512x512,

    #[default]
    #[strum(serialize = "1024x1024")]
    #[serde(rename = "1024x1024")]
    S1024x1024,
}
```

with:

```rust
/// The size of the generated/edited image(s). Valid values differ per endpoint:
/// - `create` (generate): all 8 values below.
/// - `create_edit`: `Auto`, `S1024x1024`, `S1536x1024`, `S1024x1536` only.
/// - `create_variations`: `S1024x1024`, `S1536x1024`, `S1024x1536` only (no `256x256`/
///   `512x512`/`Auto` — dall-e-2 only).
#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq, strum::Display)]
pub enum ImageSize {
    #[strum(serialize = "256x256")]
    #[serde(rename = "256x256")]
    S256x256,

    #[strum(serialize = "512x512")]
    #[serde(rename = "512x512")]
    S512x512,

    #[strum(serialize = "1024x1024")]
    #[serde(rename = "1024x1024")]
    S1024x1024,

    #[strum(serialize = "1536x1024")]
    #[serde(rename = "1536x1024")]
    S1536x1024,

    #[strum(serialize = "1024x1536")]
    #[serde(rename = "1024x1536")]
    S1024x1536,

    #[strum(serialize = "1792x1024")]
    #[serde(rename = "1792x1024")]
    S1792x1024,

    #[strum(serialize = "1024x1792")]
    #[serde(rename = "1024x1792")]
    S1024x1792,

    #[default]
    #[strum(serialize = "auto")]
    #[serde(rename = "auto")]
    Auto,
}
```

- [ ] **Step 5: Implement — replace `ImageData` with a struct**

Replace:

```rust
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageData {
    Url(String),

    #[serde(rename = "b64_json")]
    B64Json(String),
}
```

with:

```rust
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ImageData {
    pub url: Option<String>,
    pub b64_json: Option<String>,
    /// Only present for `dall-e-3` generations — the prompt actually used, which may differ
    /// from the requested prompt due to automatic revision.
    pub revised_prompt: Option<String>,
}
```

- [ ] **Step 6: Implement — extend `ImageResponse`, add `ImageUsage`/`ImageUsageTokenDetails`**

Replace:

```rust
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ImageResponse {
    pub created: i64,
    pub data: Vec<ImageData>,
}
```

with:

```rust
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ImageResponse {
    pub created: i64,
    pub data: Vec<ImageData>,
    pub background: Option<Background>,
    pub output_format: Option<OutputFormat>,
    pub quality: Option<Quality>,
    pub size: Option<ImageSize>,
    /// Token accounting for gpt-image-model generations/edits. `None` for dall-e models.
    pub usage: Option<ImageUsage>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ImageUsage {
    pub input_tokens: u32,
    pub input_tokens_details: ImageUsageTokenDetails,
    pub output_tokens: u32,
    pub total_tokens: u32,
    pub output_tokens_details: Option<ImageUsageTokenDetails>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ImageUsageTokenDetails {
    pub image_tokens: u32,
    pub text_tokens: u32,
}
```

- [ ] **Step 7: Add a `model` field to `CreateImageRequest` (needed by Step 1's second test)**

This anticipates Task 2 slightly — Step 1's second test calls `.model("gpt-image-1")`, so `CreateImageRequest` needs a minimal `model` field now for this task's own tests to compile. Task 2 will not need to re-add this field; it will only add the remaining generate-specific fields (`quality`, `background`, `output_format`, `output_compression`, `moderation`, `style`).

In `rs_openai/src/interfaces/images.rs`, in `CreateImageRequest`, immediately before the closing `}` (after the existing `user` field), add:

```rust

    /// ID of the model to use (e.g. `"gpt-image-1.5"`, `"gpt-image-1"`, `"gpt-image-1-mini"`,
    /// `"dall-e-2"` (default), `"dall-e-3"`).
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
```

- [ ] **Step 8: Run the tests to verify they pass**

Run: `cargo test --workspace --lib apis::images`
Expected: PASS — all images tests green, including the two new ones.

Run: `cargo test --workspace`
Expected: PASS — full suite green (same count as before this task plus 2 new tests).

- [ ] **Step 9: Verify**

Run: `cargo build --workspace --all-targets`
Expected: clean build.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0.

Run: `cargo fmt --all -- --check`
Expected: clean (run `cargo fmt --all` first if needed).

- [ ] **Step 10: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat!: fix ImageData response shape, add ImageResponse fields, expand ImageSize

ImageData was a two-variant enum that only coincidentally deserialized
today's single-field mock responses correctly — it could never
represent a real dall-e-3 response, which has url and revised_prompt
together. Replaced with a struct. ImageResponse gains
background/output_format/quality/size/usage (token accounting,
gpt-image only). ImageSize gains auto/1536x1024/1024x1536/1792x1024/
1024x1792. Adds the new Quality/Background/OutputFormat/Moderation/
Style/InputFidelity enums used by CreateImageRequest/
CreateImageEditRequest in the next two tasks, plus a minimal `model`
field on CreateImageRequest.

Live-verified against current OpenAI docs during design — see
docs/superpowers/specs/2026-08-04-images-model-field-design.md.
EOF
)"
```

---

### Task 2: `CreateImageRequest` (generate) — remaining new fields

**Files:**
- Modify: `rs_openai/src/interfaces/images.rs`
- Modify: `rs_openai/src/apis/images.rs` (tests only)

**Interfaces:**
- Consumes: `Quality`, `Background`, `OutputFormat`, `Moderation`, `Style` from Task 1.
- Produces: `CreateImageRequest` gains `quality`, `background`, `output_format`, `output_compression`, `moderation`, `style`. Task 3 does not depend on anything from this task — `CreateImageEditRequest` is a separate struct.

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` block in `rs_openai/src/apis/images.rs`. Extend the imports line from Task 1 — replace:

```rust
    use crate::interfaces::images::{
        Background, CreateImageEditRequest, CreateImageRequest, ImageSize, OutputFormat,
        Quality, ResponseFormat,
    };
```

with:

```rust
    use crate::interfaces::images::{
        Background, CreateImageEditRequest, CreateImageRequest, ImageSize, Moderation,
        OutputFormat, Quality, ResponseFormat, Style,
    };
```

Add this test (place it after `create_deserializes_response_level_fields_including_usage`, before `create_edit_sends_masks_own_bytes_not_the_images`):

```rust
    #[tokio::test]
    async fn create_serializes_gpt_image_capability_fields() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/images/generations"))
            .and(body_json(serde_json::json!({
                "prompt": "a vivid sunset over mountains",
                "model": "gpt-image-1",
                "quality": "high",
                "background": "opaque",
                "output_format": "png",
                "output_compression": 80,
                "moderation": "low",
                "style": "vivid"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "created": 1700000000_i64,
                "data": [{"b64_json": "aGVsbG8="}]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateImageRequest::builder()
            .prompt("a vivid sunset over mountains")
            .model("gpt-image-1")
            .quality(Quality::High)
            .background(Background::Opaque)
            .output_format(OutputFormat::Png)
            .output_compression(80)
            .moderation(Moderation::Low)
            .style(Style::Vivid)
            .build();

        let res = client.images().create(&req).await.unwrap();
        assert_eq!(res.data.len(), 1);
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --workspace --lib apis::images::tests::create_serializes_gpt_image_capability_fields`
Expected: FAIL with a compile error — `.quality(...)`, `.background(...)`, `.output_format(...)`, `.output_compression(...)`, `.moderation(...)`, `.style(...)` builder methods don't exist on `CreateImageRequest` yet.

- [ ] **Step 3: Implement**

In `rs_openai/src/interfaces/images.rs`, in `CreateImageRequest`, immediately after the `model` field added in Task 1 (still before the closing `}`), add:

```rust

    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<Quality>,

    /// gpt-image models only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<Background>,

    /// gpt-image models only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<OutputFormat>,

    /// Compression level 0-100 (percentage). `output_format` must be `jpeg` or `webp`.
    /// gpt-image models only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_compression: Option<u8>,

    /// gpt-image models only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moderation: Option<Moderation>,

    /// `dall-e-3` only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
```

Also update `response_format`'s doc comment — replace:

```rust
    /// The format in which the generated images are returned. Must be one of `url` or `b64_json`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>, // default: "url"
```

with:

```rust
    /// The format in which the generated images are returned. Must be one of `url` or `b64_json`.
    /// Only meaningful for `dall-e-2`/`dall-e-3` — gpt-image models always return base64 and
    /// ignore this field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>, // default: "url"
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --workspace --lib apis::images`
Expected: PASS — all images tests green, including the new one.

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
feat: add gpt-image capability fields to CreateImageRequest

Adds quality/background/output_format/output_compression/moderation
(gpt-image-model-only) and style (dall-e-3-only) to the generate
endpoint's request. Documents response_format as dall-e-only (gpt-image
always returns base64 and ignores it). Purely additive on top of
Task 1's foundation — no shape changes to existing fields.
EOF
)"
```

---

### Task 3: `CreateImageEditRequest` — `images` rename, new fields, `response_format` removal, multipart rewrite

**Files:**
- Modify: `rs_openai/src/interfaces/images.rs`
- Modify: `rs_openai/src/apis/images.rs` (production code and tests)

**Interfaces:**
- Consumes: `Quality`, `Background`, `OutputFormat`, `Moderation`, `InputFidelity`, `ImageSize` from Task 1.
- Produces: `CreateImageEditRequest { images: Vec<File>, prompt: String, mask: Option<File>, model: Option<String>, quality: Option<Quality>, size: Option<ImageSize>, background: Option<Background>, output_format: Option<OutputFormat>, output_compression: Option<u8>, moderation: Option<Moderation>, input_fidelity: Option<InputFidelity>, n: Option<u8>, user: Option<String> }` — no `response_format` field. Nothing later depends on this — it's the last task in this plan.

This task changes an existing test's body (`create_edit_sends_masks_own_bytes_not_the_images`) because `images` replaces `image` — this is a direct, unavoidable consequence of this task's own mandated rename, not scope creep (same category of edit already made in the Chat tool-calling slice for its `content: Option<String>` change).

- [ ] **Step 1: Write the failing tests**

First, update the existing `create_edit_sends_masks_own_bytes_not_the_images` test's request construction. In `rs_openai/src/apis/images.rs`, replace:

```rust
        let req = CreateImageEditRequest::builder()
            .image(File {
                buffer: image_bytes.to_vec(),
                filename: "image.png".into(),
            })
            .mask(File {
                buffer: mask_bytes.to_vec(),
                filename: "mask.png".into(),
            })
            .prompt("add a hat")
            .build();
```

with:

```rust
        let req = CreateImageEditRequest::builder()
            .images(vec![File {
                buffer: image_bytes.to_vec(),
                filename: "image.png".into(),
            }])
            .mask(File {
                buffer: mask_bytes.to_vec(),
                filename: "mask.png".into(),
            })
            .prompt("add a hat")
            .build();
```

Then add a new multipart-shape test and a small counting matcher, right after `create_edit_sends_masks_own_bytes_not_the_images` (the last test in the file). First add the counting matcher next to the existing `ContainsInBody` struct definition — replace:

```rust
    struct ContainsInBody(&'static [u8]);

    impl wiremock::Match for ContainsInBody {
        fn matches(&self, request: &Request) -> bool {
            request
                .body
                .windows(self.0.len())
                .any(|window| window == self.0)
        }
    }
```

with:

```rust
    struct ContainsInBody(&'static [u8]);

    impl wiremock::Match for ContainsInBody {
        fn matches(&self, request: &Request) -> bool {
            request
                .body
                .windows(self.0.len())
                .any(|window| window == self.0)
        }
    }

    /// Counts non-overlapping occurrences of `needle` in the raw request body and matches if
    /// the count is at least `min_count`. Used to prove multiple `image[]` multipart parts
    /// were actually sent, not just that some bytes appear once.
    struct ContainsAtLeastNInBody(&'static [u8], usize);

    impl wiremock::Match for ContainsAtLeastNInBody {
        fn matches(&self, request: &Request) -> bool {
            let mut count = 0;
            let mut start = 0;
            while let Some(pos) = request.body[start..]
                .windows(self.0.len())
                .position(|window| window == self.0)
            {
                count += 1;
                start += pos + self.0.len();
            }
            count >= self.1
        }
    }
```

Now add the new test at the end of the `mod tests` block:

```rust

    /// Proves multiple images are sent as repeated `image[]` multipart parts (the real
    /// OpenAI edit endpoint's wire field name — confirmed via live docs, see
    /// docs/superpowers/specs/2026-08-04-images-model-field-design.md), each with its own
    /// distinct bytes, not collapsed into a single part or dropped.
    #[tokio::test]
    async fn create_edit_sends_multiple_images_as_repeated_image_bracket_parts() {
        let mock_server = MockServer::start().await;

        let first_image_bytes: &[u8] = &[10, 20, 30];
        let second_image_bytes: &[u8] = &[40, 50, 60];

        Mock::given(method("POST"))
            .and(path("/images/edits"))
            .and(ContainsInBody(first_image_bytes))
            .and(ContainsInBody(second_image_bytes))
            .and(ContainsAtLeastNInBody(br#"name="image[]""#, 2))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "created": 1700000000_i64,
                "data": [{"b64_json": "aGVsbG8="}]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateImageEditRequest::builder()
            .images(vec![
                File {
                    buffer: first_image_bytes.to_vec(),
                    filename: "first.png".into(),
                },
                File {
                    buffer: second_image_bytes.to_vec(),
                    filename: "second.png".into(),
                },
            ])
            .prompt("combine these into one scene")
            .model("gpt-image-1")
            .quality(Quality::High)
            .input_fidelity(InputFidelity::High)
            .build();

        let res = client.images().create_edit(&req).await.unwrap();
        assert_eq!(res.data.len(), 1);
    }
```

Extend the imports line one more time — replace:

```rust
    use crate::interfaces::images::{
        Background, CreateImageEditRequest, CreateImageRequest, ImageSize, Moderation,
        OutputFormat, Quality, ResponseFormat, Style,
    };
```

with:

```rust
    use crate::interfaces::images::{
        Background, CreateImageEditRequest, CreateImageRequest, ImageSize, InputFidelity,
        Moderation, OutputFormat, Quality, ResponseFormat, Style,
    };
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --workspace --lib apis::images`
Expected: FAIL with compile errors — `.images(...)` builder method doesn't exist yet (`CreateImageEditRequest` still has `.image(...)`), `.input_fidelity(...)` doesn't exist, `InputFidelity` import doesn't resolve.

- [ ] **Step 3: Implement — `CreateImageEditRequest`**

Replace the entire `CreateImageEditRequest` struct:

```rust
#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateImageEditRequest {
    /// The image to edit. Must be a valid PNG file, less than 4MB, and square.
    /// If mask is not provided, image must have transparency, which will be used as the mask.
    pub image: File,

    /// A text description of the desired image(s). The maximum length is 1000 characters.
    #[builder(into)]
    pub prompt: String,

    /// An additional image whose fully transparent areas indicate where `image` should be edited.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask: Option<File>,

    /// The number of images to generate. Must be between 1 and 10.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u8>, // default: 1, min: 1, max: 10

    /// The size of the generated images. Must be one of `256x256`, `512x512`, or `1024x1024`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<ImageSize>, // default: "1024x1024"

    /// The format in which the generated images are returned. Must be one of `url` or `b64_json`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>, // default: "url"

    /// A unique identifier representing your end-user.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}
```

with:

```rust
#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateImageEditRequest {
    /// The image(s) to edit. Sent as repeated `image[]` multipart parts (up to 16 for
    /// gpt-image models). Each entry must be a valid PNG, WebP, or JPG file, less than 25MB.
    /// If no `mask` is provided, an image with transparency will have that transparency used
    /// as the mask.
    pub images: Vec<File>,

    /// A text description of the desired edit. The maximum length is 1000 characters.
    #[builder(into)]
    pub prompt: String,

    /// An additional image whose fully transparent areas indicate where the input image(s)
    /// should be edited. Sent as a single `mask` part.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask: Option<File>,

    /// ID of the model to use. Edit is gpt-image-only — no `dall-e-2`/`dall-e-3` support
    /// (e.g. `"gpt-image-1.5"`, `"gpt-image-1"`, `"gpt-image-1-mini"`, `"chatgpt-image-latest"`).
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<Quality>,

    /// Only `Auto`, `S1024x1024`, `S1536x1024`, `S1024x1536` are valid for edit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<ImageSize>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<Background>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<OutputFormat>,

    /// Compression level 0-100 (percentage). `output_format` must be `jpeg` or `webp`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_compression: Option<u8>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub moderation: Option<Moderation>,

    /// How closely the edited output should match the input image's fine details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_fidelity: Option<InputFidelity>,

    /// The number of images to generate. Must be between 1 and 10.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u8>,

    /// A unique identifier representing your end-user.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}
```

Note `response_format` is gone entirely — confirmed absent from the real edit endpoint's parameter list (edit is gpt-image-only, which always returns base64).

- [ ] **Step 4: Implement — rewrite `create_edit`'s multipart form building in `apis/images.rs`**

Replace the entire `create_edit` method body:

```rust
    /// Creates an edited or extended image given an original image and a prompt.
    pub async fn create_edit(
        &self,
        req: &images::CreateImageEditRequest,
    ) -> OpenAIResponse<images::ImageResponse> {
        let file_part = reqwest::multipart::Part::stream(req.image.buffer.clone())
            .file_name(req.image.filename.clone())
            .mime_str("application/octet-stream")
            .expect("\"application/octet-stream\" is always a valid mime type");

        let mut form = Form::new()
            .part("image", file_part)
            .text("prompt", req.prompt.to_string());

        if let Some(mask) = req.mask.clone() {
            let file_part = reqwest::multipart::Part::stream(mask.buffer.clone())
                .file_name(mask.filename)
                .mime_str("application/octet-stream")
                .expect("\"application/octet-stream\" is always a valid mime type");

            form = form.part("mask", file_part);
        }

        if let Some(n) = req.n {
            form = form.text("n", n.to_string());
        }

        if let Some(size) = req.size.clone() {
            form = form.text("size", size.to_string());
        }

        if let Some(response_format) = req.response_format.clone() {
            form = form.text("response_format", response_format.to_string());
        }

        if let Some(user) = req.user.clone() {
            form = form.text("user", user);
        }

        self.openai.post_form("/images/edits", form).await
    }
```

with:

```rust
    /// Creates an edited or extended image given one or more original images and a prompt.
    pub async fn create_edit(
        &self,
        req: &images::CreateImageEditRequest,
    ) -> OpenAIResponse<images::ImageResponse> {
        let mut form = Form::new().text("prompt", req.prompt.to_string());

        for image in &req.images {
            let file_part = reqwest::multipart::Part::stream(image.buffer.clone())
                .file_name(image.filename.clone())
                .mime_str("application/octet-stream")
                .expect("\"application/octet-stream\" is always a valid mime type");

            form = form.part("image[]", file_part);
        }

        if let Some(mask) = req.mask.clone() {
            let file_part = reqwest::multipart::Part::stream(mask.buffer.clone())
                .file_name(mask.filename)
                .mime_str("application/octet-stream")
                .expect("\"application/octet-stream\" is always a valid mime type");

            form = form.part("mask", file_part);
        }

        if let Some(model) = req.model.clone() {
            form = form.text("model", model);
        }

        if let Some(quality) = req.quality.clone() {
            form = form.text("quality", quality.to_string());
        }

        if let Some(size) = req.size.clone() {
            form = form.text("size", size.to_string());
        }

        if let Some(background) = req.background.clone() {
            form = form.text("background", background.to_string());
        }

        if let Some(output_format) = req.output_format.clone() {
            form = form.text("output_format", output_format.to_string());
        }

        if let Some(output_compression) = req.output_compression {
            form = form.text("output_compression", output_compression.to_string());
        }

        if let Some(moderation) = req.moderation.clone() {
            form = form.text("moderation", moderation.to_string());
        }

        if let Some(input_fidelity) = req.input_fidelity.clone() {
            form = form.text("input_fidelity", input_fidelity.to_string());
        }

        if let Some(n) = req.n {
            form = form.text("n", n.to_string());
        }

        if let Some(user) = req.user.clone() {
            form = form.text("user", user);
        }

        self.openai.post_form("/images/edits", form).await
    }
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --workspace --lib apis::images`
Expected: PASS — all images tests green, including the updated and new ones.

Run: `cargo test --workspace`
Expected: PASS — full suite green.

- [ ] **Step 6: Verify**

Run: `cargo build --workspace --all-targets`
Expected: clean build.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0.

Run: `cargo fmt --all -- --check`
Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
feat!: pluralize CreateImageEditRequest.image, add gpt-image fields, drop response_format

image: File becomes images: Vec<File>, sent as repeated image[]
multipart parts (the real wire field name, confirmed via live docs —
gpt-image models accept up to 16 input images). Adds
model/quality/background/output_format/output_compression/moderation/
input_fidelity. Removes response_format entirely — confirmed absent
from the real edit endpoint (edit is gpt-image-only, always returns
base64).

Both changes are breaking: the crate could not represent a
multi-image gpt-image edit before this, and was sending a field
OpenAI's edit endpoint doesn't recognize. See
docs/superpowers/specs/2026-08-04-images-model-field-design.md.
EOF
)"
```

---
