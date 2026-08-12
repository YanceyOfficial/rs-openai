# Responses API: `web_search` Tool — Design

Status: Approved
Date: 2026-08-12

## Context

The second of Responses' 6 remaining tool types (after `file_search`, shipped in the previous slice), picked next per the dependency-ordered sequence since `web_search` has no additional resource dependency (unlike `code_interpreter`, which needs the not-yet-implemented Containers scaffold). This is also the first slice to exercise adding a THIRD variant to `ResponseTool`/`ResponseOutputItem` — confirming the extensible-union design keeps scaling cleanly.

Live research this session (WebFetch against `developers.openai.com`) confirmed: the `web_search` tool's exact request shape (`type: "web_search"` — a versioned `"web_search_2025_08_26"` alternate discriminator also exists but is not modeled this round, see Non-goals; `filters: Option<{ allowed_domains: Vec<String> }>`; `search_context_size: Option<"low"|"medium"|"high">`; `user_location: Option<{ type: "approximate", city, country, region, timezone }>`, all sub-fields except `type` optional, confirmed via a live JSON example). The `WebSearchCall` output item's exact shape (`id`, `action` — a 3-variant tagged union confirmed via reference docs and a live JSON example: `Search { type: "search", query, queries, sources }`, `OpenPage { type: "open_page", url }`, `FindInPage { type: "find_in_page", pattern, url }`; `status: "in_progress"|"searching"|"completed"|"failed"`; `type: "web_search_call"`).

Per explicit project-owner direction (`AskUserQuestion`), this slice fully types the `action` union rather than deferring it to raw JSON (unlike `file_search`'s `filters`, which is a genuinely complex recursive grammar) — `action`'s three variants are simple enough to model properly, and it's the central, not-optional payload of a web search call.

## Goals

- `ResponseTool` gains a `WebSearch(ResponseWebSearchTool)` variant (third variant on the extensible union, after `Function`/`FileSearch`).
- `ResponseWebSearchTool`: `filters: Option<WebSearchFilters>` (`allowed_domains: Option<Vec<String>>`), `search_context_size: Option<WebSearchContextSize>` (closed 3-value enum — confirmed closed and stable, unlike `file_search`'s `ranker`), `user_location: Option<WebSearchUserLocation>`.
- `WebSearchUserLocation`: `kind: String` (wire key `type`, matching the `ResponseToolChoice::Specific`-established precedent of naming a literal-discriminator field `kind` since `type` collides with Rust's keyword), `city`/`country`/`region`/`timezone` all `Option<String>`.
- `ResponseOutputItem` gains a `WebSearchCall(ResponseWebSearchCallItem)` variant (third variant).
- `ResponseWebSearchCallItem`: `id: String`, `action: WebSearchAction`, `status: String` (kept as a plain string — same caution already applied to `file_search`'s `status`, since only 4 of an unconfirmed-total value set were seen and this crate prefers not guessing at closed enums from partial confirmation).
- `WebSearchAction`: a 3-variant tagged union — `Search { query: Option<String>, queries: Option<Vec<String>>, sources: Option<Vec<WebSearchActionSource>> }`, `OpenPage { url: Option<String> }`, `FindInPage { pattern: String, url: String }`.
- `WebSearchActionSource`: `kind: String` (wire key `type`, always `"url"` per docs — same `kind` precedent as `WebSearchUserLocation`), `url: String`.

## Non-goals (explicit, deferred to later slices)

- **The versioned `"web_search_2025_08_26"` tool-type discriminator** — the tool tag stays the standard `"web_search"` string (the enum variant name `WebSearch` converts automatically via the existing `rename_all = "snake_case"` container attribute); the dated-snapshot variant is not modeled.
- **The remaining 4 tool types** (`code_interpreter`, `computer_use`, `image_generation`, `mcp`) and their output items — each needs its own field research; `code_interpreter` also depends on the not-yet-implemented Containers resource.
- **Streaming events for `web_search`** (`response.web_search_call.in_progress`/`.searching`/`.completed`) — `ResponseStreamEvent` stays scoped to text + function-calling events, same deferral already established for `file_search`.
- **`return_token_budget`** — a newer, narrower-use parameter (GPT-5+ reasoning models specifically) not confirmed in enough field-shape detail this session.

## Design

### File organization

All changes land in the existing `rs_openai/src/interfaces/responses.rs` — no new files. `ResponseTool` and `ResponseOutputItem` both already have two variants each (`Function`/`FileSearch` and `Message`/`FunctionCall`/`FileSearchCall` respectively); this slice adds a third to each.

### `web_search` tool (request side)

```rust
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct WebSearchFilters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_domains: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum WebSearchContextSize {
    Low,
    Medium,
    High,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct WebSearchUserLocation {
    #[builder(into)]
    #[serde(rename = "type")]
    pub kind: String,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ResponseWebSearchTool {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<WebSearchFilters>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_context_size: Option<WebSearchContextSize>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_location: Option<WebSearchUserLocation>,
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
}
```

### `web_search_call` output item (response side)

```rust
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct WebSearchActionSource {
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WebSearchAction {
    Search {
        #[serde(skip_serializing_if = "Option::is_none")]
        query: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        queries: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        sources: Option<Vec<WebSearchActionSource>>,
    },
    OpenPage {
        #[serde(skip_serializing_if = "Option::is_none")]
        url: Option<String>,
    },
    FindInPage {
        pattern: String,
        url: String,
    },
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseWebSearchCallItem {
    pub id: String,
    pub action: WebSearchAction,
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
}
```

## Testing

`wiremock` tests covering: `create` serializing a `web_search` tool with `user_location` and `search_context_size` set, deserializing a response whose `output` contains a `web_search_call` item with a `Search`-variant `action`; a deserialization test for `WebSearchAction::OpenPage` (proving the tagged union discriminates a second action variant correctly); a deserialization test for `WebSearchAction::FindInPage` (the variant with required, not optional, fields — proving the "some fields are required within a variant" case works, distinct from `Search`'s all-optional fields); a mixed-array test proving `ResponseOutputItem` discriminates `WebSearchCall` from the now-three other coexisting variants (`Message`/`FunctionCall`/`FileSearchCall`) within the same `output` array.

## Versioning

Purely additive at the crate level (`ResponseTool`/`ResponseOutputItem` each gain one new non-breaking variant, exactly as their doc comments anticipate). Ships as a pre-1.0 minor bump alongside whatever else accumulates before the next release, per established practice.
