# Responses API: `mcp` Tool — Design

Status: Approved
Date: 2026-08-13

## Context

Fourth of Responses' remaining tool types (after `file_search`, `web_search`, `image_generation`). Fifth variant on `ResponseTool`, sixth on `ResponseOutputItem` — fourth exercise of the extensible-union design. `mcp` lets the model call tools exposed by a remote MCP (Model Context Protocol) server directly from within a Responses call.

Live research this session (WebFetch against `developers.openai.com`, cross-checked across two pages) confirmed the tool config's field list and the output item shapes. The tool config supports three mutually-exclusive ways to reach an MCP server: `server_url` (a plain remote URL), `connector_id` (OpenAI-hosted connectors like `connector_dropbox`/`connector_gmail`), and `tunnel_id` (Secure MCP Tunnels). Scoped down (per `AskUserQuestion`, "Core: server_url only (Recommended)") to `server_url`-based remote servers only — `connector_id`/`tunnel_id` are separate, less-documented connection mechanisms deferred to a future slice. `allowed_callers` (filters whether a tool is callable by the model directly vs. only programmatically) is also deferred for the same reason.

`require_approval` is modeled fully (not deferred to raw JSON) since — per the project's established "central, non-optional payload gets full typing" principle (already applied to `web_search`'s `action`) — it is the tool's core safety-relevant field, not incidental metadata: it's either a plain mode string (`"never"`/`"always"`) or a per-tool-name filter object (`{"never": {"tool_names": [...]}, "always": {"tool_names": [...]}}`), confirmed by both research sources.

`allowed_tools` also has a confirmed alternate object-filter shape (`McpToolFilter`) beyond a plain string array; that alternate form is deferred (kept as `Option<Vec<String>>` only) — same class of deferral as `file_search`'s `filters` staying raw JSON, since the plain-array form covers the common case and the object form's exact shape wasn't independently confirmed with a live example.

The output item examples (from OpenAI's MCP guide) show `mcp_list_tools`, `mcp_call`, and `mcp_approval_request` **without a `status` field** — a confirmed, deliberate contrast with `file_search_call`/`web_search_call`/`image_generation_call`, which do have one. `mcp_approval_response` is an input-only item (the caller's answer to an approval request), so it's added to the existing `ResponseInputItem` union, not `ResponseOutputItem`.

## Goals

- `ResponseTool` gains an `Mcp(ResponseMcpTool)` variant (fifth variant).
- `ResponseMcpTool`: `server_label: String` (required — always present, the tool's own identifying label), `server_url: String` (required in this scoped slice — the only supported connection mechanism), `server_description: Option<String>`, `headers: Option<HashMap<String, String>>`, `authorization: Option<String>` (OAuth access token), `require_approval: Option<McpRequireApproval>`, `allowed_tools: Option<Vec<String>>`, `defer_loading: Option<bool>`.
- `McpRequireApproval` (untagged 2-variant union): `Mode(McpApprovalMode)` (`"never"`/`"always"`) or `Filter(McpApprovalFilter)` (`{ never: Option<McpToolNameFilter>, always: Option<McpToolNameFilter> }`, each side `{ tool_names: Vec<String> }`).
- `ResponseOutputItem` gains an `McpListTools(ResponseMcpListToolsItem)`, `McpCall(ResponseMcpCallItem)`, and `McpApprovalRequest(ResponseMcpApprovalRequestItem)` variant (three new variants — sixth, seventh, eighth on the enum).
- `ResponseMcpListToolsItem`: `id: String`, `server_label: String`, `tools: Vec<McpListedTool>` where `McpListedTool` is `{ name: String, description: Option<String>, input_schema: serde_json::Value, annotations: Option<serde_json::Value> }` (both `input_schema`/`annotations` stay raw JSON — genuinely variable per-server JSON-Schema/metadata shapes, same deferral class as `file_search`'s `filters`).
- `ResponseMcpCallItem`: `id: String`, `server_label: String`, `name: String`, `arguments: String` (raw JSON-encoded string, not parsed — matches how `ResponseFunctionCallItem.arguments` already works), `output: Option<String>`, `error: Option<String>`, `approval_request_id: Option<String>`.
- `ResponseMcpApprovalRequestItem`: `id: String`, `server_label: String`, `name: String`, `arguments: String`.
- `ResponseInputItem` gains an `McpApprovalResponse { approval_request_id: String, approve: bool }` variant (fourth variant on that union — first time it's grown since Responses core).

## Non-goals (explicit, deferred to later slices)

- **`connector_id`/`tunnel_id`** on the tool config — alternate connection mechanisms to `server_url`, need their own research/confirmation.
- **`allowed_callers`** — filters direct-vs-programmatic callability, not confirmed with a live example this session.
- **`allowed_tools`'s object-filter (`McpToolFilter`) form** — stays a plain string array only.
- **The last remaining tool type, `computer_use`** — its own slice, larger action union.
- **`code_interpreter`** — still gated on the not-yet-implemented Containers resource.
- **MCP-specific streaming events** (`response.mcp_call.*`, `response.mcp_list_tools.*`) — `ResponseStreamEvent` stays scoped to text + function-calling events, same deferral already established for every other tool type's own streaming events.

## Design

### File organization

All changes land in the existing `rs_openai/src/interfaces/responses.rs` — no new files. Uses `std::collections::HashMap`, already imported elsewhere in the crate via full path (matching `vector_stores.rs`'s convention of `std::collections::HashMap<...>` inline rather than a top-level `use`).

### `mcp` tool (request side)

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum McpApprovalMode {
    Never,
    Always,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct McpToolNameFilter {
    pub tool_names: Vec<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct McpApprovalFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub never: Option<McpToolNameFilter>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub always: Option<McpToolNameFilter>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum McpRequireApproval {
    Mode(McpApprovalMode),
    Filter(McpApprovalFilter),
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ResponseMcpTool {
    #[builder(into)]
    pub server_label: String,

    #[builder(into)]
    pub server_url: String,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<std::collections::HashMap<String, String>>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_approval: Option<McpRequireApproval>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub defer_loading: Option<bool>,
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
}
```

### `mcp_list_tools` / `mcp_call` / `mcp_approval_request` output items (response side)

```rust
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct McpListedTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
    pub annotations: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseMcpListToolsItem {
    pub id: String,
    pub server_label: String,
    pub tools: Vec<McpListedTool>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseMcpCallItem {
    pub id: String,
    pub server_label: String,
    pub name: String,
    pub arguments: String,
    pub output: Option<String>,
    pub error: Option<String>,
    pub approval_request_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseMcpApprovalRequestItem {
    pub id: String,
    pub server_label: String,
    pub name: String,
    pub arguments: String,
}
```

`ResponseOutputItem` gains three variants (existing enum, shown with context):

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
}
```

### `mcp_approval_response` (input side)

`ResponseInputItem` gains one variant (existing enum, shown with context):

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
}
```

This lets a caller answer an `mcp_approval_request` by pushing an `McpApprovalResponse` item into a follow-up `CreateResponseRequest.input` (or a `Conversation`'s `items`, since `CreateConversationRequest.items` already reuses `ResponseInputItem` directly).

## Testing

`wiremock` tests covering: `create` serializing an `mcp` tool with `server_url`+`headers`+`require_approval` set to the plain-mode form, and a second test with `require_approval` set to the object-filter form (proving the untagged union picks the right shape for each); deserializing a response whose `output` contains a completed `mcp_call` item with `output` populated and `error: null`; a deserialization test for `mcp_list_tools` with a 2-tool array (proving nested `McpListedTool` deserializes, with `input_schema` as an arbitrary JSON object); a deserialization test for `mcp_approval_request`; a serialization test proving `McpApprovalResponse` sent as part of `input` round-trips to `{"type": "mcp_approval_response", "approval_request_id": ..., "approve": true}`; a mixed-array test proving `ResponseOutputItem` discriminates all three new variants from the now-five other coexisting variants within the same `output` array.

## Versioning

Purely additive at the crate level (`ResponseTool`/`ResponseOutputItem`/`ResponseInputItem` each gain new non-breaking variants). Ships as a pre-1.0 minor bump alongside whatever else accumulates before the next release, per established practice.
