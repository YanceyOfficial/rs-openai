# Responses API: `code_interpreter` Tool — Design

Status: Approved
Date: 2026-08-14

## Context

The sixth and final Responses tool type — eighth variant on `ResponseTool`, tenth on `ResponseOutputItem`. `code_interpreter` was gated on the Containers resource, which shipped this session (`618acc5`... actually `6196057`); this slice unblocks it, meaning every Responses tool type OpenAI documents is now implementable by this crate.

Live research this session (WebFetch, 4 sources tried) confirmed the tool config's `container` field is a union: either a plain string (an existing container ID, referencing the `Container` resource this crate just implemented) or an object `{type: "auto", memory_limit, file_ids}` requesting an auto-provisioned container. `memory_limit`'s value set (`1g`/`4g`/`16g`/`64g`) is identical to Containers' own `ContainerMemoryLimit` — a genuine, confirmed reuse case, reused directly rather than duplicated (the same "auto" object is, in effect, a lightweight inline container-creation request).

**A genuine, unresolved field-naming ambiguity** for the `code_interpreter_call` output item's execution-results field: sources disagreed on whether it's named `outputs` or `results`, and none gave a confirmed shape for its entries. A further clue narrows this to a low-priority gap rather than a blocking one: one source noted this field is only populated "when you specify `code_interpreter_call.outputs` in the `include` parameter" — and this crate has already explicitly deferred the `include` param entirely (a standing, previously-recorded gap). Since the field would rarely if ever be populated without `include` support anyway, and its very name isn't confirmed, this slice defers it entirely rather than guessing at a field name that could be silently wrong on the wire.

## Goals

- `ResponseTool` gains a `CodeInterpreter(ResponseCodeInterpreterTool)` variant (eighth variant).
- `ResponseCodeInterpreterContainer` (untagged 2-variant union, same pattern as `ResponseConversationParam`): `Id(String)` (an existing container ID) or `Auto(ResponseCodeInterpreterAutoContainer)`.
- `ResponseCodeInterpreterAutoContainer`: `kind: String` (`#[serde(rename = "type")]`, caller sets `"auto"` explicitly — the established `kind`-field-for-wire-`"type"`-collision convention), `memory_limit: Option<crate::interfaces::containers::ContainerMemoryLimit>` (reused directly — identical confirmed value set to Containers' own type), `file_ids: Option<Vec<String>>`.
- `ResponseCodeInterpreterTool`: `container: ResponseCodeInterpreterContainer` (required).
- `ResponseOutputItem` gains a `CodeInterpreterCall(ResponseCodeInterpreterCallItem)` variant (tenth variant).
- `ResponseCodeInterpreterCallItem`: `id: String`, `code: String` (required — analogous to `ResponseFunctionCallItem.arguments`, present as soon as the call object exists, unlike execution results which arrive later), `container_id: String`, `status: String` (plain string, matching this crate's established status-field uniformity convention).

## Non-goals (explicit, deferred to later slices)

- **The execution-results field** (`outputs` or `results` — name unconfirmed) — not modeled at all this slice, pending confirmation of its actual field name and shape, and likely gated on the `include` param anyway (already a standing deferred gap for this crate).
- **The `include` param generally** — still not modeled, as previously recorded.
- **`code_interpreter`-specific streaming events** — `ResponseStreamEvent` stays scoped to text + function-calling events, the same deferral already established for every other tool type.

## Design

### File organization

All changes land in the existing `rs_openai/src/interfaces/responses.rs` — no new files.

```rust
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ResponseCodeInterpreterAutoContainer {
    #[builder(into)]
    #[serde(rename = "type")]
    pub kind: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_limit: Option<crate::interfaces::containers::ContainerMemoryLimit>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_ids: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ResponseCodeInterpreterContainer {
    Id(String),
    Auto(ResponseCodeInterpreterAutoContainer),
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ResponseCodeInterpreterTool {
    pub container: ResponseCodeInterpreterContainer,
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
    Mcp(ResponseMcpTool),
    Computer,
    ComputerUsePreview(ResponseComputerUsePreviewTool),
    CodeInterpreter(ResponseCodeInterpreterTool),
}
```

```rust
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseCodeInterpreterCallItem {
    pub id: String,
    pub code: String,
    pub container_id: String,
    pub status: String,
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
    McpListTools(ResponseMcpListToolsItem),
    McpCall(ResponseMcpCallItem),
    McpApprovalRequest(ResponseMcpApprovalRequestItem),
    ComputerCall(ResponseComputerCallItem),
    CodeInterpreterCall(ResponseCodeInterpreterCallItem),
}
```

With this variant, `ResponseTool` reaches its full, confirmed set (`Function`/`FileSearch`/`WebSearch`/`ImageGeneration`/`Mcp`/`Computer`/`ComputerUsePreview`/`CodeInterpreter` — 8 variants) — every Responses tool type OpenAI documents is now implemented.

## Testing

`wiremock` tests covering: `create` serializing a `code_interpreter` tool with `container` as a plain string ID; a second test with `container` as the `Auto` object form (with `memory_limit` reusing `containers::ContainerMemoryLimit` and `file_ids`), proving the untagged union picks the right shape for each; deserializing a response whose `output` contains a `code_interpreter_call` item; a mixed-array test proving `ResponseOutputItem` discriminates `CodeInterpreterCall` from the other nine coexisting variants.

## Versioning

Purely additive at the crate level. Ships as a pre-1.0 minor bump alongside whatever else accumulates before the next release, per established practice.
