# Chat Tool-Calling — Design

Status: Approved
Date: 2026-08-04

## Context

Phase 2a (`docs/superpowers/specs/2026-07-31-phase2-restructuring-design.md`, `docs/superpowers/plans/2026-07-31-phase2a-restructuring-plan.md`) restructured `rs_openai`'s module layout to mirror OpenAI's current API hierarchy and scaffolded every category the crate was missing. It shipped 2026-08-04, merged to `master` at `34e9eca`. Phase 2a's own research (`docs/superpowers/research/2026-07-31-field-diff-chat-embeddings-moderations-models.md`) also catalogued field-level gaps in already-implemented modules, deliberately deferred as separate, later-scoped work — that document's single biggest finding: **Chat has zero tool/function-calling support**. `ChatCompletionMessage` has no `tool_calls`/`tool_call_id` field, and `Role` has no `Tool` variant, so the crate cannot represent a tool-calling turn in either direction.

This is the first slice of that deferred field-diff work (Phase 2b). The Chat module's field-diff research also found several other gaps (structured outputs/`response_format`, `reasoning_effort`, `service_tier`, `store`+retrieve/list/delete stored completions, `stream_options`, `seed`/`logprobs`) — the project owner explicitly scoped this slice to **tool-calling only**; those other gaps are separate, future slices.

## Goals

- Let callers build a `CreateChatRequest` that offers the model a set of callable tools (`tools`, `tool_choice`, `parallel_tool_calls`).
- Let callers construct and parse a full tool-calling turn: the assistant's response containing `tool_calls` (with `content` possibly `null`), and a caller-constructed follow-up message with `role: Tool` carrying the tool's result back via `tool_call_id`.
- Extend both the non-streaming (`create`) and streaming (`create_with_stream`) response paths — streaming gets raw per-chunk `tool_calls` deltas, with no accumulation/reconstruction logic built (that's the caller's responsibility, same as it is against OpenAI's own wire format).

## Non-goals (explicit, deferred to later slices)

- `response_format`/structured outputs, `reasoning_effort`, `service_tier`, `store` + the stored-completion retrieve/list/delete endpoints, `stream_options`, `seed`/`logprobs`, `modalities`/`audio`, `prediction`, request-level `metadata`.
- Multimodal message content (`content: String` → `String | Vec<ContentPart>` for image/audio/file input parts). Note this is a related but separate concern from making `content` *optional* (this slice's change) — optional-vs-required is about whether content can be absent; multimodal is about what shape content takes when present. This slice keeps `content` as `Option<String>`, not `Option<String | Vec<ContentPart>>`.
- `choices[].message.refusal`, `choices[].message.audio`, `choices[].logprobs`, `usage.completion_tokens_details`/`usage.prompt_tokens_details`, response-level `service_tier`.
- A `Developer` role variant (newer system-role naming for reasoning-model contexts) — orthogonal to tool-calling.
- Any accumulation/reconstruction of streamed partial tool-call arguments across chunks.

## Design

### `Role` — add `Tool`

```rust
#[derive(Debug, Serialize, Deserialize, Clone, Default, strum::Display)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    #[strum(serialize = "system")]
    System,
    #[default]
    #[strum(serialize = "user")]
    User,
    #[strum(serialize = "assistant")]
    Assistant,
    #[strum(serialize = "tool")]
    Tool,
}
```

### `ChatCompletionMessage` — `content` becomes optional, gains `tool_calls`/`tool_call_id`

```rust
#[derive(Builder, Debug, Clone, Deserialize, Serialize)]
pub struct ChatCompletionMessage {
    pub role: Role,

    /// The contents of the message. `None` (serializes as `null`) when an assistant message
    /// contains only tool calls — OpenAI's real responses do this, so this field must accept
    /// `null` on the way in, not just allow omitting it on the way out.
    #[builder(into)]
    pub content: Option<String>,

    #[builder(into)]
    pub name: Option<String>,

    /// Populated on assistant messages that call one or more tools instead of (or alongside)
    /// producing text.
    pub tool_calls: Option<Vec<ToolCall>>,

    /// Set on `role: Tool` messages to link this message's `content` (the tool's result) back
    /// to the `ToolCall.id` it answers.
    #[builder(into)]
    pub tool_call_id: Option<String>,
}
```

`content`, `tool_calls`, and `tool_call_id` deliberately do **not** get `#[serde(skip_serializing_if = "Option::is_none")]`, matching the existing `name` field's convention on this same struct: always send the key explicitly (as `null` when absent) rather than omitting it. This mirrors a lesson already learned once in this codebase (Phase 1: a test was made to pass by adding `skip_serializing_if` to `name`, which silently changed production wire behavior — reverted, the test was fixed instead). `#[builder(into)]` on an `Option<String>` field still accepts a plain string at call sites (`.content("hi")` keeps compiling unchanged) via `bon`'s existing `Into` conversion, so this is source-compatible for every current caller that sets `content`.

### New types: tool calls and tool definitions

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionCall {
    pub name: String,
    /// Raw JSON-encoded arguments, deliberately left unparsed — the caller decides how (and
    /// whether) to validate/deserialize this against their own tool's parameter schema.
    pub arguments: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String, // always "function" as of this writing; OpenAI has never shipped a second tool-call type
    pub function: FunctionCall,
}
```

```rust
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct FunctionDefinition {
    #[builder(into)]
    pub name: String,
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// A JSON Schema object describing the function's parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ChatCompletionTool {
    #[serde(rename = "type")]
    #[builder(default = "function".to_string())]
    pub kind: String,
    pub function: FunctionDefinition,
}
```

### `ToolChoice` — untagged enum, same pattern as `shared::types::Stop`

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoiceMode {
    None,
    Auto,
    Required,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolChoiceFunctionName {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ToolChoice {
    Mode(ToolChoiceMode),
    Specific {
        #[serde(rename = "type")]
        kind: String, // always "function"
        function: ToolChoiceFunctionName,
    },
}
```

### `CreateChatRequest` — three new optional fields

```rust
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ChatCompletionTool>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
```

These follow the existing convention for every other optional field already on `CreateChatRequest` (skip when absent) — the opposite convention from the message-level fields above, and that's intentional: request-level params are genuinely absent from the wire when unset; message-level fields are part of a fixed message shape OpenAI expects to see fully (if sparsely) populated.

### Streaming: `Delta` gains `tool_calls`

```rust
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Delta {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
}
```

Real streamed tool-call deltas are partial (each chunk carries an `index` and may carry only a fragment of `arguments`) — modeling that precisely would need an index-aware partial variant of `ToolCall`, not the full response-shaped one. For this slice, `Delta.tool_calls` reuses the same `ToolCall` type as the non-streaming path; fields that a given chunk doesn't carry simply deserialize as their default/empty value. No accumulation helper is built. If precise partial-delta modeling turns out to matter in practice, that's a follow-up refinement, not blocking this slice.

### No accessor/endpoint changes

`Chat::create` and `Chat::create_with_stream` are unchanged — this is a pure field-addition to existing request/response types they already send/receive.

## Testing

Extend `apis/chat.rs`'s existing `wiremock` test suite:
- A request-serialization test covering `tools`/`tool_choice`/`parallel_tool_calls` on the request, and a `role: Tool` message with `tool_call_id` in the `messages` array.
- A response-deserialization test where the assistant's message has `"content": null` and a populated `tool_calls` array — proving the `Option<String>` change actually fixes real-shape deserialization, not just compiles.
- A streaming test asserting `Delta.tool_calls` deserializes from an SSE chunk that carries a partial tool call.

The existing four tests should need no code changes (they never set `tool_calls`/`tool_choice`, and `content: Some("hi")` serializes identically to `content: "hi"` today).

## Versioning

Purely additive to `CreateChatRequest` (new optional fields, all correctly skip-serialized when absent) and to `ChatCompletionMessage`/`Delta` (new optional fields; `content`'s type change from `String` to `Option<String>` is source-compatible for every builder call site via `#[builder(into)]`, and wire-compatible for every existing serialized request). No compatibility break for existing callers. Ships as a pre-1.0 patch or minor bump (exact number decided at implementation time) — this is additive, not the "no compat constraint" breaking-change mode Phase 2a operated under.
