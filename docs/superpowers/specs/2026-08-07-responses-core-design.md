# Responses API: Core (Non-Streaming, Text + Function-Calling) — Design

Status: Approved
Date: 2026-08-07

## Context

Phase 2a scaffolded 9 categories the crate had zero coverage of; Phase 2b has since closed out every gap in already-implemented modules (three prior slices: Chat tool-calling, Images, and a consolidated existing-modules pass). This is the first slice implementing one of Phase 2a's scaffolds — per explicit project-owner choice, **Responses**, OpenAI's now-recommended primary API (superseding Chat Completions, and eventually Assistants).

Responses' full surface is large: 7 tool types (`function`, `web_search`, `file_search`, `code_interpreter`, `computer_use`, `image_generation`, `mcp`), 10+ output item types, and a large SSE streaming event taxonomy. Attempting all of it in one slice would be several times the size of any prior slice in this project. Per explicit project-owner scoping direction, **this slice covers only**: the five core CRUD/lifecycle operations, plain-text input/output, and function-calling (the same capability level Chat already has) — non-streaming only.

Live documentation fetches during design were only partially successful — OpenAI's `resources/responses/methods/create.md` page is large enough that the fetch tool's summarization repeatedly truncated mid-list. What was confirmed live: the `cancel` operation exists and its purpose ("Terminates a background model response"); `input` accepts a string, an array of `EasyInputMessage`, or `ResponseOutputMessage`; `background`/`context_management`/`conversation`/`include` exist as request fields (all deferred, see Non-goals); the `output` array's `Message`/`FunctionCall`/`Reasoning`/`WebSearchCall`/`FileSearchCall`/`ComputerCall` item shapes (confirmed field lists for `Message` and `FunctionCall`, the two in scope this slice); tool type discriminator strings (`"function"`, `"web_search"`/`"web_search_2025_08_26"`, `"file_search"`, `"code_interpreter"`). Fields not independently confirmed this session (`model`, `instructions`, `tool_choice`, `temperature`, `top_p`, `max_output_tokens`, `previous_response_id`, `store`, `metadata`, `parallel_tool_calls`, `text`, `truncation`, `safety_identifier`, `prompt_cache_key`, `service_tier`, and the `usage` object's exact shape) rely on stable, well-established knowledge of this API rather than a live fetch this session — flagged here for the implementer's awareness, not blocking, since these are long-stable, extensively-documented parameters.

## Goals

- Five operations: `create`, `retrieve`, `delete`, `cancel`, and `list_input_items` (a single-operation "sub-resource" exposed as a direct method on the ID-scoped accessor, not a forced nested struct — mirrors the "flat unless there's real multi-operation depth" convention already established in Phase 2a).
- `CreateResponseRequest` supporting: a plain-text `input`, OR a small array of input items covering the two shapes this slice needs (`Message` for conversational turns, `FunctionCallOutput` for returning a tool's result — Responses' `input` model is a flat list of turn items, not an ever-growing message array like Chat's); `instructions`; `previous_response_id` (Responses' mechanism for continuing a prior turn without resending full history); `store`; `metadata`; `temperature`/`top_p`/`max_output_tokens`; `truncation`; `parallel_tool_calls`; `tool_choice`; `tools` (function-type only, modeled as an extensible tagged union so later slices can add more variants non-breakingly); `text` (output-format config, reusing Chat's `ChatResponseFormat`/`JsonSchemaFormat` — identical shape and purpose, no reason to duplicate); `safety_identifier`/`prompt_cache_key`; `service_tier` (reusing Chat's `ServiceTier`).
- `Response` (the returned object): full status/error/incomplete-detail lifecycle fields, `output: Vec<ResponseOutputItem>` scoped to `Message` and `FunctionCall` variants (extensible tagged union, same reasoning as `tools` above), `usage`.
- Reuse existing Chat types where the shape is genuinely identical (`ChatResponseFormat`, `JsonSchemaFormat`, `ServiceTier`) rather than redefining Responses-specific duplicates.

## Non-goals (explicit, deferred to later slices)

- **Streaming** (`stream` field, and the SSE event taxonomy). Unlike Chat/Completions (which already have a working `create_with_stream` to redirect a rejected `stream: true` request to), Responses has no streaming method at all yet — exposing a `stream` field that can never usefully be set to `true` this round would be confusing partial surface. `stream` is left off `CreateResponseRequest` entirely; it and `create_with_stream` return together in a dedicated future slice.
- **The other six tool types** (`web_search`, `file_search`, `code_interpreter`, `computer_use`, `image_generation`, `mcp`) and their corresponding output item types (`ReasoningItem`, `WebSearchCall`, `FileSearchCall`, `ComputerCall`, `CodeInterpreterCall`, `ImageGenerationCall`, `McpCall`/`McpListTools`/`McpApprovalRequest`). `file_search` specifically depends on Vector Stores, not yet implemented. These arrive incrementally as new variants on the already-extensible `ResponseTool`/`ResponseOutputItem` unions — additive, not a rework of this slice's design.
- **Multimodal input** (image/file/audio content parts on input messages) — `Message.content` stays a plain `String` this round, matching the same multimodal deferral already established for Chat.
- **`conversation`** parameter — depends on the not-yet-implemented Conversations resource.
- **`background`**, **`context_management`**, **`include`**, **`reasoning`** (reasoning-model chain-of-thought config/output), **`modalities`** — all confirmed-to-exist-but-newer/niche fields, deferred as a deliberate scope cut rather than a gap that was missed.
- Response content annotations (file citations, URL citations, etc.) — `ResponseOutputText.annotations` stays a generic `Vec<serde_json::Value>` this round rather than a fully-typed annotation union.

## Design

### File organization

New file `rs_openai/src/interfaces/responses.rs` (doesn't exist yet — Phase 2a's scaffold only created `apis/responses.rs`, per its own stated convention of scaffolding without interface types until real implementation). Add `pub mod responses;` to `interfaces/mod.rs`. `apis/responses.rs`'s existing `Responses<'a>`/`Response<'a>` scaffold structs get real methods added — no restructuring, no `#[allow(dead_code)]` removal needed beyond what naturally happens once fields are read.

### Core types

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ResponseInput {
    Text(String),
    Items(Vec<ResponseInputItem>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseInputItem {
    Message { role: ResponseInputRole, content: String },
    FunctionCallOutput { call_id: String, output: String },
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum ResponseInputRole {
    User,
    Assistant,
    System,
    Developer,
}
```

(`ResponseInputItem::Message` has no explicit `"type": "message"` wire discriminator issue to worry about beyond the standard internally-tagged shape — confirmed pattern already used successfully for `Grader`/`ChatResponseFormat` in this crate.)

```rust
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ResponseFunctionTool {
    #[builder(into)]
    pub name: String,
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub parameters: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// Extensible union — this slice implements only `Function`; later slices add
/// `WebSearch`/`FileSearch`/`CodeInterpreter`/`ComputerUse`/`ImageGeneration`/`Mcp` as new
/// non-breaking variants.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseTool {
    Function(ResponseFunctionTool),
}
```

`ResponseToolChoice` mirrors Chat's `ToolChoice` exactly (mode string or forced-specific-function object) — same untagged-enum pattern, distinct type since the two APIs' shapes, while similar, are documented separately and shouldn't be silently coupled:

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ResponseToolChoiceMode {
    None,
    Auto,
    Required,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResponseToolChoiceFunctionName {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ResponseToolChoice {
    Mode(ResponseToolChoiceMode),
    Specific {
        #[serde(rename = "type")]
        kind: String,
        function: ResponseToolChoiceFunctionName,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum TruncationStrategy {
    Auto,
    Disabled,
}
```

### `CreateResponseRequest`

```rust
#[derive(Builder, Debug, Serialize, Clone)]
pub struct CreateResponseRequest {
    #[builder(into)]
    pub model: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<ResponseInput>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub store: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncation: Option<TruncationStrategy>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ResponseToolChoice>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ResponseTool>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<ResponseTextConfig>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_identifier: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<crate::interfaces::chat::ServiceTier>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ResponseTextConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<crate::interfaces::chat::ChatResponseFormat>,
}
```

`input` is `Option` (not required) since a follow-up turn can rely solely on `previous_response_id` plus a single new `FunctionCallOutput` item, or even omit input entirely in some flows — matching how OpenAI documents it as optional.

### `Response` (the returned object) and output items

```rust
#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    Queued,
    InProgress,
    Completed,
    Incomplete,
    Failed,
    Cancelled,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseIncompleteDetails {
    pub reason: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseOutputContent {
    OutputText {
        text: String,
        annotations: Vec<serde_json::Value>,
    },
    Refusal {
        refusal: String,
    },
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseOutputMessage {
    pub id: String,
    pub role: String,
    pub content: Vec<ResponseOutputContent>,
    pub status: ResponseStatus,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseFunctionCallItem {
    pub id: String,
    pub call_id: String,
    pub name: String,
    pub arguments: String,
    pub status: ResponseStatus,
}

/// Extensible union — this slice implements only `Message`/`FunctionCall`; later slices add
/// `Reasoning`/`WebSearchCall`/`FileSearchCall`/`ComputerCall`/`CodeInterpreterCall`/
/// `ImageGenerationCall`/`McpCall`/etc. as new non-breaking variants.
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseOutputItem {
    Message(ResponseOutputMessage),
    FunctionCall(ResponseFunctionCallItem),
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseUsageTokenDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u32>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseUsage {
    pub input_tokens: u32,
    pub input_tokens_details: Option<ResponseUsageTokenDetails>,
    pub output_tokens: u32,
    pub output_tokens_details: Option<ResponseUsageTokenDetails>,
    pub total_tokens: u32,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Response {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub status: ResponseStatus,
    pub error: Option<ResponseError>,
    pub incomplete_details: Option<ResponseIncompleteDetails>,
    pub model: String,
    pub output: Vec<ResponseOutputItem>,
    pub usage: Option<ResponseUsage>,
    pub previous_response_id: Option<String>,
    pub instructions: Option<String>,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}
```

`ResponseUsageTokenDetails`' sub-fields are `Option<u32>`, not bare `u32` — directly applying the lesson from the existing-modules-field-fixes slice's `ChatUsage` finding, where required sub-fields on an uncertain-presence nested object risked failing deserialization of the entire response. `Response` intentionally does NOT echo back every request-side config field (`temperature`/`top_p`/`tools`/etc.) — OpenAI's real response does echo most of them, but since this crate has no code path that needs to read them back off a `Response` this round (no test or documented use case), adding a dozen more `Option` echo fields with zero behavior depending on them would be scope padding; add them when an actual need appears (revisit if it turns out to be surprising in practice).

### Accessor wiring (`apis/responses.rs`)

```rust
impl<'a> Responses<'a> {
    pub async fn create(&self, req: &CreateResponseRequest) -> OpenAIResponse<Response> {
        self.openai.post("/responses", req).await
    }
}

impl<'a> Response<'a> {
    pub async fn retrieve(&self) -> OpenAIResponse<responses::Response> {
        self.openai.get(&format!("/responses/{}", self.id), &()).await
    }

    pub async fn delete(&self) -> OpenAIResponse<DeleteResponseResponse> {
        self.openai.delete(&format!("/responses/{}", self.id), &()).await
    }

    pub async fn cancel(&self) -> OpenAIResponse<responses::Response> {
        self.openai.post(&format!("/responses/{}/cancel", self.id), &()).await
    }

    pub async fn list_input_items(&self, req: &ListInputItemsRequest) -> OpenAIResponse<ListInputItemsResponse> {
        self.openai.get(&format!("/responses/{}/input_items", self.id), req).await
    }
}
```

**Naming note:** every other module in this crate suffixes its response-body type with `Response` (`BatchResponse`, `ThreadResponse`, `FileResponse`, ...), keeping the bare resource name free for the accessor struct. That convention can't apply cleanly here, since the resource itself is literally named "Response" — `ResponseResponse` would be absurd. This design keeps the interface type named bare `Response` (matching OpenAI's own object name, same as e.g. `Checkpoint`/`Segment`/`Word` elsewhere in this crate when there's no naming conflict to avoid), which does collide with `apis::responses::Response<'a>` (the singular ID-scoped accessor struct, already fixed by Phase 2a's scaffold). Since `apis/responses.rs` re-exports `interfaces::responses::*`, both names land in the same module scope — resolved with an ordinary qualified path (`responses::Response`) in `apis/responses.rs`'s own method signatures wherever the body type is meant, exactly as `apis/fine_tuning/checkpoints/mod.rs` already does for its own `checkpoints::FineTuningCheckpoint` vs. bare-imported names. No new pattern, just a locally qualified reference.

```rust
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteResponseResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}

#[derive(Builder, Debug, Serialize, Clone)]
pub struct ListInputItemsRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<crate::interfaces::chat::SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListInputItemsResponse {
    pub object: String,
    pub data: Vec<ResponseInputItem>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}
```

(Reuses Chat's `SortOrder` for `list_input_items`' `order` param — same shape, same reasoning as reusing `ChatResponseFormat`/`ServiceTier`.)

## Testing

`wiremock` tests covering: `create` serializing a text-only input, `create` serializing a message-array input with a tool defined and `tool_choice` set, a response-deserialization test proving `ResponseOutputItem`'s tagged union correctly discriminates a `Message` item (with `OutputText` content) from a `FunctionCall` item in the same `output` array — the direct Responses-API analog of the Chat tool-calling slice's lesson that an assistant turn can be all-tool-calls with no accompanying text, `retrieve`/`delete`/`cancel` each mapping API errors correctly, and `list_input_items` deserializing a `FunctionCallOutput`-shaped input item (the harder of the two `ResponseInputItem` variants to get right, since it never appears as message-shaped).

## Versioning

Purely additive at the crate level (a previously-scaffolded, zero-method category now has real methods) — no existing type or method changes elsewhere in the crate. Ships as a pre-1.0 minor bump alongside whatever else accumulates before the next release, per this project's established practice of deferring version bumps to release time rather than per-slice.
