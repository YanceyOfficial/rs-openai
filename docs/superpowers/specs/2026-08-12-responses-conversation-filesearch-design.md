# Responses API: `conversation` Param + `file_search` Tool — Design

Status: Approved
Date: 2026-08-12

## Context

The Responses core slice deferred both `conversation` (needed the not-yet-implemented Conversations resource) and every tool type beyond `function` (each needing its own field research, deferred deliberately to ship a scoped-down first slice). Both Conversations and Vector Stores have since shipped, unblocking `conversation` and the `file_search` tool type specifically (`file_search` depends on Vector Stores; the other 5 remaining tool types don't and stay deferred). Picked as the next Responses-focused slice per the dependency-ordered sequence, before moving to a new scaffold (ChatKit).

Live research this session (WebFetch against `developers.openai.com`) confirmed: `conversation` is `Option<string-or-{id: string}>` — an untagged union, structurally identical to `ResponseToolChoice`'s existing string-or-object pattern already in this file. The `file_search` tool's exact request shape (`type`, `vector_store_ids: Vec<String>`, `filters: ComparisonFilter | CompoundFilter | null`, `max_num_results`, `ranking_options: { hybrid_search: { embedding_weight, text_weight }, ranker: "auto" | "default-2024-11-15", score_threshold }`). The `FileSearchCall` output item's exact shape (`id`, `queries: Vec<String>`, `status: "in_progress" | "searching" | "completed" | 2 more (unconfirmed)`, `type`, `results: Option<Vec<{ attributes, file_id, filename, score, text }>>`).

## Goals

- `CreateResponseRequest.conversation: Option<ResponseConversationParam>` — untagged `Id(String) | Object { id: String }`, mirroring `ResponseToolChoice`'s existing pattern.
- `ResponseTool` gains a `FileSearch(ResponseFileSearchTool)` variant (the union is already designed to be extensible — this is exactly the non-breaking addition its doc comment anticipated).
- `ResponseFileSearchTool`: `vector_store_ids: Vec<String>` (required), `filters: Option<serde_json::Value>` (the real shape is a recursive `ComparisonFilter`/`CompoundFilter` grammar — kept as raw JSON, same deliberate deferral already established for Vector Stores' own `search.filters`), `max_num_results: Option<u32>`, `ranking_options: Option<FileSearchRankingOptions>`.
- `FileSearchRankingOptions`: `hybrid_search: Option<FileSearchRankingOptionsHybridSearch>` (`embedding_weight`/`text_weight`, both `Option<f32>`), `ranker: Option<String>` (kept as a plain string, not a closed enum — its documented values are versioned strings like `"default-2024-11-15"` that will keep changing over time, same reasoning already applied to Vector Stores search's `ranker`), `score_threshold: Option<f32>`.
- `ResponseOutputItem` gains a `FileSearchCall(ResponseFileSearchCallItem)` variant.
- `ResponseFileSearchCallItem`: `id: String`, `queries: Vec<String>`, `status: String` (kept as a plain string — only 3 of the real ~5 status values were confirmed this session, and guessing the remaining ones risks a closed enum that fails to deserialize a real response), `results: Option<Vec<FileSearchResult>>`.
- `FileSearchResult`: `attributes: Option<HashMap<String, serde_json::Value>>`, `file_id: Option<String>`, `filename: Option<String>`, `score: Option<f32>`, `text: Option<String>` — all fields genuinely optional per OpenAI's own documented shape (not just this crate's usual caution), since `results` is `null` by default and only populated when the caller opts in via `include`.

## Non-goals (explicit, deferred to later slices)

- **The other 5 remaining tool types** (`web_search`, `code_interpreter`, `computer_use`, `image_generation`, `mcp`) and their output items — each needs its own field research; `code_interpreter` also depends on the not-yet-implemented Containers resource.
- **A typed `filters` grammar** for `file_search`'s `filters` field — same deferral as Vector Stores' own `search.filters`, for the same reason (a non-trivial recursive comparison/compound-filter union).
- **`status`/`ranker` as closed enums** — kept as plain strings pending full confirmation of their value sets.
- **Streaming events for `file_search`** (`response.file_search_call.in_progress`/`.searching`/`.completed`) — the Responses streaming slice's `ResponseStreamEvent` union stays scoped to text + function-calling events; extending it to file_search is a separate future slice if needed.
- **The `include` request parameter** (needed to actually request `results` be populated) — not part of `CreateResponseRequest`'s current scope; `results` staying `Option` already handles both the default-null case and a hypothetical future where `include` gets added.

## Design

### File organization

All changes land in the existing `rs_openai/src/interfaces/responses.rs` — no new files. `ResponseTool` and `ResponseOutputItem` both already exist as extensible tagged unions from the core slice; this slice adds one variant to each, exactly as their own doc comments anticipated.

### `conversation` param

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ResponseConversationParam {
    Id(String),
    Object { id: String },
}
```

Added to `CreateResponseRequest`:

```rust
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation: Option<ResponseConversationParam>,
```

### `file_search` tool (request side)

```rust
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct FileSearchRankingOptionsHybridSearch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_weight: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_weight: Option<f32>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct FileSearchRankingOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hybrid_search: Option<FileSearchRankingOptionsHybridSearch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranker: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_threshold: Option<f32>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ResponseFileSearchTool {
    pub vector_store_ids: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_num_results: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranking_options: Option<FileSearchRankingOptions>,
}
```

`ResponseTool` gains one variant (existing enum, shown with context):

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseTool {
    Function(ResponseFunctionTool),
    FileSearch(ResponseFileSearchTool),
}
```

### `file_search_call` output item (response side)

```rust
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct FileSearchResult {
    pub attributes: Option<std::collections::HashMap<String, serde_json::Value>>,
    pub file_id: Option<String>,
    pub filename: Option<String>,
    pub score: Option<f32>,
    pub text: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseFileSearchCallItem {
    pub id: String,
    pub queries: Vec<String>,
    pub status: String,
    pub results: Option<Vec<FileSearchResult>>,
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
}
```

## Testing

`wiremock` tests covering: `create` serializing `conversation` as a bare string ID; `create` serializing `conversation` as an object (`{"id": "..."}`); `create` serializing a request with a `file_search` tool (including nested `ranking_options`) and deserializing a response whose `output` contains a `file_search_call` item with populated `results`; a deserialization test for a `file_search_call` item with `results: null` (the default, no-`include` case) proving the whole item still deserializes cleanly; a mixed-array test proving `ResponseOutputItem`'s tagged union discriminates `FileSearchCall` from the existing `Message`/`FunctionCall` variants within the same `output` array (the same lesson the core slice's own `Message`-vs-`FunctionCall` test already established, now extended to a third variant).

## Versioning

Purely additive at the crate level (`CreateResponseRequest` gains one new optional field; `ResponseTool`/`ResponseOutputItem` each gain one new non-breaking variant, exactly as their doc comments anticipated). Ships as a pre-1.0 minor bump alongside whatever else accumulates before the next release, per established practice.
