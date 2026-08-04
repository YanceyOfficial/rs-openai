# Images: `model` field + gpt-image capability set — Design

Status: Approved
Date: 2026-08-04

## Context

Phase 2a's field-diff research (`docs/superpowers/research/2026-07-31-field-diff-images-audio-files-fine-tuning.md`) flagged Images as having **no `model` field at all** — the single biggest capability gap after Chat's tool-calling (already shipped, `docs/superpowers/specs/2026-08-04-chat-tool-calling-design.md`). Without `model`, every `gpt-image-1`/`1.5`/`mini`-specific capability (quality tiers, background/transparency, streaming partial images, format control) is unreachable regardless of what else gets added, since there's no way to even select those models.

This is the second slice of Phase 2b (field-diff application to already-implemented modules), per the project owner's direction to resolve existing-module gaps before implementing Phase 2a's unimplemented scaffolds.

During design, live docs were fetched to verify and, in several cases, correct the earlier field-diff research:
- `CreateImageEditRequest`'s image parameter is `images` (plural) on the wire, sent as **repeated `image[]` multipart parts**, not a single `image` part — the earlier research didn't establish this.
- `response_format` **does not exist on the edit endpoint** in the current API at all (edit's parameter table has no such field) — edit is gpt-image-only now, which always returns base64.
- Variation's size options are confirmed as `1024x1024`/`1024x1536`/`1536x1024` only (no `256x256`/`512x512`) — resolves a `[verify]` flag from the original research.
- `ImageResponse.usage`'s exact shape is confirmed: `input_tokens`, `input_tokens_details: { image_tokens, text_tokens }`, `output_tokens`, `total_tokens`, `output_tokens_details: Option<{ image_tokens, text_tokens }>`.

## Goals

- Add `model` to `CreateImageRequest` and `CreateImageEditRequest`, unlocking every gpt-image-model-conditional field.
- Add the full set of gpt-image-model-only request fields OpenAI currently documents: `quality`, `background`, `output_format`, `output_compression`, `moderation`, plus `style` (generate-only, dall-e-3) and `input_fidelity` (edit-only).
- Expand `ImageSize` to the current full set of valid sizes across all three endpoints.
- Fix `ImageData`'s response shape: it's currently a plain (non-`untagged`) two-variant enum that only coincidentally parses today's single-field mock responses correctly (Rust's default externally-tagged enum representation for a newtype variant happens to look like `{"url": "..."}`) — it cannot represent a real `dall-e-3` response, which has `url` **and** `revised_prompt` together. This is an existing, real bug independent of the model-field gap.
- Add the new response-level fields OpenAI documents: `background`, `output_format`, `quality`, `size`, `usage` (token accounting, gpt-image only).
- Support multiple input images on edit (`images: Vec<File>` — gpt-image models accept up to 16), sent as repeated `image[]` multipart parts.

## Non-goals (explicit, deferred to later slices)

- Streaming partial-image generation (`stream`/`partial_images` request fields and their SSE event schema) — a distinct concern from the field/type work here, same reasoning as why Chat's streaming got its own dedicated task in the tool-calling slice. A future slice.
- `file_id`/URL references for edit's input images (an alternative to raw byte upload) — the edit `images` array supports these in the real API, but modeling the full union (raw upload | file_id | image_url) is meaningfully larger type design and a separate multipart-encoding path per variant. This slice ships `images: Vec<File>` (raw uploads only, pluralized from today's single-file pattern). A future slice can add reference support without breaking this one (e.g. an enum wrapping `File` alongside new reference variants).
- Any change to `CreateImageVariationRequest`'s fields — it's dall-e-2-only; none of the new gpt-image capability fields apply to it. It only gains the shared, expanded `ImageSize` enum (which is additive — no existing variant removed).

## Design

### Shared enums (new)

All new enums follow the crate's existing pattern (see `ImageSize`/`ResponseFormat` in `rs_openai/src/interfaces/images.rs`): `#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]`, manual per-variant `#[strum(serialize = "...")]` + `#[serde(rename = "...")]` where the wire value isn't already the natural lowercase of the variant name. `strum::Display` is required because `apis/images.rs`'s multipart form-building calls `.to_string()` on these fields (see Implementation section below) — this isn't optional boilerplate, it's how these fields will actually be sent.

```rust
#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
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

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum Background {
    #[strum(serialize = "transparent")]
    Transparent,
    #[strum(serialize = "opaque")]
    Opaque,
    #[strum(serialize = "auto")]
    Auto,
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    #[strum(serialize = "png")]
    Png,
    #[strum(serialize = "jpeg")]
    Jpeg,
    #[strum(serialize = "webp")]
    Webp,
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum Moderation {
    #[strum(serialize = "low")]
    Low,
    #[strum(serialize = "auto")]
    Auto,
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    #[strum(serialize = "vivid")]
    Vivid,
    #[strum(serialize = "natural")]
    Natural,
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum InputFidelity {
    #[strum(serialize = "high")]
    High,
    #[strum(serialize = "low")]
    Low,
}
```

### `ImageSize` — expand, don't replace

Add five new variants alongside the existing `S256x256`/`S512x512`/`S1024x1024`:

```rust
#[default]
#[strum(serialize = "auto")]
#[serde(rename = "auto")]
Auto,

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
```

Update the doc comment to state the full set and note per-endpoint validity: generate accepts all 8; edit accepts `auto`/`1024x1024`/`1536x1024`/`1024x1536` (4); variation accepts `1024x1024`/`1024x1536`/`1536x1024` (3, no `256x256`/`512x512`, no `auto`). The type itself doesn't enforce this subsetting — matching the crate's existing style of documenting validity in prose rather than modeling per-endpoint types, which would fork `ImageSize` into three types for no behavioral benefit.

Changing `#[default]` from `S1024x1024` to `Auto` is deliberate: `auto` is the real universal default across all three endpoints in the current API, and every constructor already treats `size` as `Option<ImageSize>` — this only affects what `ImageSize::default()` returns if called directly (not currently done anywhere in the crate), so it's a safe, more-correct default to adopt while already touching this type.

### `ImageData` — replace the broken enum with a struct

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

**This is a breaking change to the public API** (removes the `ImageData` enum, replaces with a struct — callers pattern-matching on `ImageData::Url(s)`/`ImageData::B64Json(s)` today must switch to field access `img.url`/`img.b64_json`). It's justified as a bug fix, not a feature: the current enum cannot deserialize a real `dall-e-3` response (`url` + `revised_prompt` together), which is the same class of "existing code doesn't correctly represent reality" fix already made twice in this codebase (Phase 2a's `CreateBatchRequest.meta`→`metadata` rename; Phase 1's `Images::create_edit` mask-duplication bug).

### `ImageResponse` — add response-level fields

```rust
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ImageResponse {
    pub created: i64,
    pub data: Vec<ImageData>,
    pub background: Option<Background>,
    pub output_format: Option<OutputFormat>,
    pub quality: Option<Quality>,
    pub size: Option<ImageSize>,
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

`data` stays `Vec<ImageData>` (not `Option`), unchanged from today — live docs describe it as an optional array at the schema level, but every real non-streaming response this crate produces (this slice excludes streaming, where partial results might plausibly omit `data`) includes it, and there's no confirmed case where a synchronous `create`/`create_edit`/`create_variations` call returns without `data`. Keeping it required avoids an unnecessary breaking change to every caller's `res.data` access; revisit if the streaming slice finds a real case where it's absent.

`ImageUsageTokenDetails` is shared between `input_tokens_details` (always present) and `output_tokens_details` (optional) — both have the identical `{ image_tokens, text_tokens }` shape per the live-verified docs.

### `CreateImageRequest` (generate) — new fields

```rust
#[builder(into)]
#[serde(skip_serializing_if = "Option::is_none")]
pub model: Option<String>, // e.g. "gpt-image-1.5", "gpt-image-1", "gpt-image-1-mini", "dall-e-2" (default), "dall-e-3"

#[serde(skip_serializing_if = "Option::is_none")]
pub quality: Option<Quality>,

#[serde(skip_serializing_if = "Option::is_none")]
pub background: Option<Background>, // gpt-image only

#[serde(skip_serializing_if = "Option::is_none")]
pub output_format: Option<OutputFormat>, // gpt-image only

#[serde(skip_serializing_if = "Option::is_none")]
pub output_compression: Option<u8>, // 0-100, jpeg/webp only, gpt-image only — [verify exact numeric type/range against live docs before implementing; source only confirmed "number"]

#[serde(skip_serializing_if = "Option::is_none")]
pub moderation: Option<Moderation>, // gpt-image only

#[serde(skip_serializing_if = "Option::is_none")]
pub style: Option<Style>, // dall-e-3 only
```

`model` is a plain `Option<String>` (matching `CreateChatRequest.model`'s convention), not a closed Rust enum — new model names (OpenAI ships new image models periodically) work without a crate update, same reasoning as every other `model` field in this crate. `response_format` is unchanged in shape, doc comment updated to note it only affects dall-e models (gpt-image always returns base64 and ignores it).

### `CreateImageEditRequest` (edit) — new fields, one rename, one removal

```rust
/// The image(s) to edit. Sent as repeated `image[]` multipart parts (up to 16 for gpt-image
/// models). Each entry must be a valid PNG, WebP, or JPG file, less than 25MB.
pub images: Vec<File>, // renamed from `image: File`

#[builder(into)]
pub prompt: String,

#[serde(skip_serializing_if = "Option::is_none")]
pub mask: Option<File>, // unchanged — still a single mask, sent as one "mask" part

#[builder(into)]
#[serde(skip_serializing_if = "Option::is_none")]
pub model: Option<String>, // "gpt-image-1.5", "gpt-image-1", "gpt-image-1-mini", "chatgpt-image-latest" — edit is gpt-image-only, no dall-e-2/3

#[serde(skip_serializing_if = "Option::is_none")]
pub quality: Option<Quality>,

#[serde(skip_serializing_if = "Option::is_none")]
pub size: Option<ImageSize>,

#[serde(skip_serializing_if = "Option::is_none")]
pub background: Option<Background>,

#[serde(skip_serializing_if = "Option::is_none")]
pub output_format: Option<OutputFormat>,

#[serde(skip_serializing_if = "Option::is_none")]
pub output_compression: Option<u8>,

#[serde(skip_serializing_if = "Option::is_none")]
pub moderation: Option<Moderation>,

#[serde(skip_serializing_if = "Option::is_none")]
pub input_fidelity: Option<InputFidelity>, // high/low

#[serde(skip_serializing_if = "Option::is_none")]
pub n: Option<u8>,

#[builder(into)]
#[serde(skip_serializing_if = "Option::is_none")]
pub user: Option<String>,
```

`response_format` is **removed entirely** — confirmed absent from the edit endpoint's current parameter list. `style` is **not added** — dall-e-3-only, and edit doesn't support dall-e-3 (confirmed: edit's `model` values are all gpt-image family). Both `image`→`images` and the `response_format` removal are breaking changes to the struct's shape, justified the same way as `ImageData`'s fix: this crate's current `CreateImageEditRequest` cannot represent a multi-image gpt-image edit request at all (single `File` field), and sends a field OpenAI's edit endpoint doesn't recognize.

### `CreateImageVariationRequest` (variation) — no field changes

Untouched except for sharing the expanded `ImageSize` enum (additive — variation only ever validly used the pre-existing three sizes, none of which were removed). No `model` field: variation is implicitly dall-e-2-only in the current API (no `model` parameter exists for this endpoint at all).

### Implementation: `apis/images.rs`

`create_edit`'s multipart form building changes from:

```rust
let file_part = reqwest::multipart::Part::stream(req.image.buffer.clone())...
let mut form = Form::new().part("image", file_part).text("prompt", ...);
```

to building one `"image[]"` part per entry in `req.images`:

```rust
let mut form = Form::new().text("prompt", req.prompt.to_string());
for image in &req.images {
    let file_part = reqwest::multipart::Part::stream(image.buffer.clone())
        .file_name(image.filename.clone())
        .mime_str("application/octet-stream")
        .expect("\"application/octet-stream\" is always a valid mime type");
    form = form.part("image[]", file_part);
}
```

Every new optional field (`model`, `quality`, `background`, `output_format`, `output_compression`, `moderation`, `input_fidelity`) follows the existing `if let Some(x) = req.x.clone() { form = form.text("x", x.to_string()); }` pattern already used for `n`/`size`/`response_format` in both `create_edit` and `create_variations`. `create` (generate, JSON body not multipart) needs no method-body changes — the new fields just serialize automatically as part of the existing `self.openai.post("/images/generations", req)` call, same as every other optional field already on `CreateImageRequest`.

## Testing

Extend `apis/images.rs`'s existing `wiremock` suite:
- A request-serialization test for `create` covering `model`/`quality`/`background`/`output_format`/`output_compression`/`moderation`/`style` together.
- A response-deserialization test proving the `ImageData` fix: a mock response with `url` **and** `revised_prompt` present together on the same item, asserting both fields are populated (this would have failed to deserialize under the old enum).
- A response-deserialization test for the new `ImageResponse` fields, including a populated `usage` object with both `input_tokens_details` and `output_tokens_details`.
- A multipart-body test for `create_edit` with two entries in `images`, asserting the raw multipart body contains two separate `image[]` parts with each image's own distinct bytes (extending the existing `ContainsInBody` matcher pattern already used for the mask-duplication regression test).

Existing tests (`create_serializes_1024_size_correctly`, `create_maps_api_error`, `create_edit_sends_masks_own_bytes_not_the_images`) need updates for the `images` rename and `ImageData` struct shape — these are direct, unavoidable consequences of this slice's own mandated changes, not scope creep (same category of edit made in the Chat tool-calling slice for its `content: Option<String>` change).

## Versioning

This slice contains real, justified breaking changes (`ImageData` enum → struct, `CreateImageEditRequest.image` → `images: Vec<File>`, `response_format` removed from edit) alongside purely additive fields. Not byte-compatible, not source-compatible for any caller touching the changed surfaces. Ships as a pre-1.0 minor bump (exact number decided at implementation time, following the same crate-versioning convention established in Phase 2a).
