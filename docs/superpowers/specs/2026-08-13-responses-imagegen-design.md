# Responses API: `image_generation` Tool — Design

Status: Approved
Date: 2026-08-13

## Context

The third of Responses' remaining tool types (after `file_search`, `web_search`), picked next since `computer_use`/`image_generation`/`mcp` are all now unblocked (`code_interpreter` still gates on the not-yet-implemented Containers scaffold and stays deferred). Fourth variant on `ResponseTool`, fifth on `ResponseOutputItem` — third exercise of the extensible-union design.

Live research this session (WebFetch against `developers.openai.com`) confirmed the `image_generation` tool's full request-side field list (`type`, `action`, `background`, `input_fidelity`, `model`, `moderation`, `output_compression`, `output_format`, `partial_images`, `quality`, `size`) and, via a live JSON example in OpenAI's image-generation guide, the `ImageGenerationCall` output item's core fields (`id`, `type`, `status`, `revised_prompt`, `result` — the last being base64-encoded image data, confirmed by the guide's own decode examples). The output item's complete field list could not be independently confirmed beyond this example (a dedicated schema fetch attempt returned no results) — `revised_prompt`/`result` are modeled as `Option` accordingly, matching this crate's established caution for fields whose presence-on-every-response isn't fully confirmed.

**Major reuse discovery**: this crate's existing `interfaces/images.rs` (the DALL-E-style Images API, unrelated to Responses until now) already defines `Quality`, `Background`, `OutputFormat`, `Moderation`, `InputFidelity`, and `ImageSize` — and every one of them matches `image_generation`'s corresponding tool field either exactly or as a strict superset (extra variants that are legitimate for the standalone Images API but simply never constructed via this tool, e.g. `Quality::Standard`/`Hd` are `dall-e-3`-only and not applicable to the Responses tool's `gpt-image-*` models). All six are reused directly rather than duplicated — the largest single instance of cross-module type reuse in this project so far, following the same "genuine shape/purpose identity" principle already established for Chat types in Responses.

## Goals

- `ResponseTool` gains an `ImageGeneration(ResponseImageGenerationTool)` variant (fourth variant).
- `ResponseImageGenerationTool`: `action: Option<ImageGenerationAction>` (new 3-value enum: `Generate`/`Edit`/`Auto` — no existing type to reuse, since `images.rs`'s standalone create/edit/variation endpoints don't have an analogous single "which mode" field), `background: Option<images::Background>`, `input_fidelity: Option<images::InputFidelity>`, `model: Option<String>` (plain string — matches this crate's universal convention for model-name fields, since the real field accepts both named presets and arbitrary future model strings), `moderation: Option<images::Moderation>`, `output_compression: Option<u32>`, `output_format: Option<images::OutputFormat>`, `partial_images: Option<u32>`, `quality: Option<images::Quality>`, `size: Option<images::ImageSize>`.
- `ResponseOutputItem` gains an `ImageGenerationCall(ResponseImageGenerationCallItem)` variant (fifth variant).
- `ResponseImageGenerationCallItem`: `id: String`, `status: String` (plain string, same established caution as `file_search`/`web_search`'s `status` fields — not confirmed as a closed set), `result: Option<String>` (base64 image data, likely absent while in-progress), `revised_prompt: Option<String>` (likely absent while in-progress).

## Non-goals (explicit, deferred to later slices)

- **The remaining 2 tool types** (`computer_use`, `mcp`) and their output items — each needs its own field research.
- **`code_interpreter`** — still gated on the not-yet-implemented Containers resource, not picked up by this slice or the next.
- **Partial-image streaming events** (`response.image_generation_call.partial_image`) — `ResponseStreamEvent` stays scoped to text + function-calling events, same deferral already established for `file_search`/`web_search`. The `partial_images` request field is still exposed (a caller can set it), but this crate has no streaming-side handling for the resulting partial-image events yet — consistent with how `stream` itself works today only for the already-scoped text/function-calling event set.
- **Echoing `background`/`output_format`/`quality`/`size` back on `ResponseImageGenerationCallItem`** — not confirmed present on the output item from the one example available this session; can be added later if confirmed.

## Design

### File organization

All changes land in the existing `rs_openai/src/interfaces/responses.rs` — no new files. Six enums are imported from the existing `rs_openai/src/interfaces/images.rs` via qualified paths, not duplicated.

### `image_generation` tool (request side)

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
```

`ResponseTool` gains one variant (existing enum, shown with context):

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseTool {
    Function(ResponseFunctionTool),
    FileSearch(ResponseFileSearchTool),
    WebSearch(ResponseWebSearchTool),
    ImageGeneration(ResponseImageGenerationTool),
}
```

### `image_generation_call` output item (response side)

```rust
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseImageGenerationCallItem {
    pub id: String,
    pub status: String,
    pub result: Option<String>,
    pub revised_prompt: Option<String>,
}
```

`ResponseOutputItem` gains one variant (existing enum, shown with context):

```rust
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

**Reuse note:** `images::Quality`'s `Standard`/`Hd` variants (`dall-e-3`-only) and `images::ImageSize`'s `S256x256`/`S512x512`/`S1792x1024`/`S1024x1792` variants (`dall-e-2`/`dall-e-3`-only) are not valid values for this tool's `gpt-image-*`-only context — the Rust type system doesn't prevent constructing them here, same as how `crate::interfaces::chat::ServiceTier` reuse in the Responses core slice doesn't restrict to only the values valid for Responses specifically. This is an accepted, precedented tradeoff: avoiding 6 duplicate enum definitions is worth a caller being able to construct a handful of values the real API would reject with a 400, exactly the same class of imprecision already accepted elsewhere in this project.

## Testing

`wiremock` tests covering: `create` serializing an `image_generation` tool reusing `images::Quality`/`images::Background`/`images::ImageSize` (proving the cross-module reuse actually serializes to the correct wire values, not just compiles), deserializing a response whose `output` contains a completed `image_generation_call` item with `result`/`revised_prompt` populated; a deserialization test for an in-progress `image_generation_call` with `result`/`revised_prompt` both `null` (proving the `Option` fields handle the documented in-progress case); a mixed-array test proving `ResponseOutputItem` discriminates `ImageGenerationCall` from the now-four other coexisting variants (`Message`/`FunctionCall`/`FileSearchCall`/`WebSearchCall`) within the same `output` array.

## Versioning

Purely additive at the crate level (`ResponseTool`/`ResponseOutputItem` each gain one new non-breaking variant; `images.rs`'s existing types are reused unmodified). Ships as a pre-1.0 minor bump alongside whatever else accumulates before the next release, per established practice.
