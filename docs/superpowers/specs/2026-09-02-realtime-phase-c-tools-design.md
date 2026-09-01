# Realtime API — Phase C: Function Calling + MCP Tools In-Session — Design

Status: Approved
Date: 2026-09-02

## Context

Third Realtime slice, following the same condensed spec-then-direct-implementation rhythm Phase B used — Phase A already settled every architectural decision (connection, `RealtimeConnection`, the two-level `ConversationItem` tagging, the trim-aggressively-and-defer-richness discipline). This slice applies that pattern to the tool-calling surface: session/response-level tool configuration (`tools`, `tool_choice`), the conversation items tool calls produce, and the streaming events that report tool-call progress.

Confirmed via `gh api` against `openai-python`'s `types/realtime/` (all relevant files fetched and read directly, not guessed): no new client events are needed. Function calling and MCP both work entirely through the three client events Phase A already implemented (`session.update`, `conversation.item.create`, `response.create`) — this slice is purely additive to existing event/type shapes.

**A significant reuse opportunity, confirmed field-by-field**: Realtime's MCP tool config (`realtime_tools_config_union.py`'s `Mcp` class, used for the session's `tools` array) and its response-level override (`realtime_response_create_mcp_tool.py`'s `RealtimeResponseCreateMcpTool`) are BOTH confirmed structurally identical to this crate's existing `crate::interfaces::responses::ResponseMcpTool` (`server_label`, `type: "mcp"`, `allowed_callers`, `allowed_tools`, `authorization`, `connector_id`, `defer_loading`, `headers`, `require_approval`, `server_description`, `server_url`, `tunnel_id` — every field matches). `ResponseMcpTool` and everything it depends on (`McpConnectorId`, `McpRequireApproval`, `McpAllowedTools`, `McpToolFilter`, `McpAllowedCaller`, `McpApprovalMode`, `McpApprovalFilter`) already derive both `Serialize` and `Deserialize`, so reusing it directly here has no derive-reachability gap (unlike the imagegen slice's `Moderation`/`InputFidelity` case, which needed a `Deserialize` derive added). This is reused as-is, not duplicated.

**A real, unrelated pre-existing bug was found and fixed while researching this slice** (see the standalone commit before this branch): `ResponseToolChoice::Specific` in `interfaces/responses.rs` serialized a forced-function tool choice as `{"type":"function","function":{"name":...}}`, copied from Chat Completions' nested shape. Confirmed via source (`ToolChoiceFunctionParam`) that Responses' actual shape is flat: `{"type":"function","name":...}`. Fixed with a regression test, landed directly on `master` ahead of this branch since it's unrelated to Realtime.

## Goals

### New `ConversationItem` variants (6, growing the union from 1 to 7)

All confirmed via `conversation_item.py`'s full union list. Each is a flat, single-level `type`-tagged variant (unlike `Message`, none of these need a second-level tag):

- `FunctionCall(RealtimeConversationItemFunctionCall)` -- tag `function_call`. Fields: `arguments: String`, `name: String` (both required), `id`/`call_id`/`object`/`status: Option<String>`.
- `FunctionCallOutput(RealtimeConversationItemFunctionCallOutput)` -- tag `function_call_output`. Fields: `call_id: String`, `output: String` (both required), `id`/`object`/`status: Option<String>`.
- `McpApprovalResponse(RealtimeMcpApprovalResponse)` -- tag `mcp_approval_response`. Fields: `id: String`, `approval_request_id: String`, `approve: bool` (all required), `reason: Option<String>`.
- `McpListTools(RealtimeMcpListTools)` -- tag `mcp_list_tools`. Fields: `server_label: String`, `tools: Vec<RealtimeMcpServerTool>` (required), `id: Option<String>`. `RealtimeMcpServerTool { input_schema: serde_json::Value, name: String, annotations: Option<serde_json::Value>, description: Option<String> }` -- `input_schema`/`annotations` kept as raw JSON (arbitrary nested JSON Schema / free-form object, not worth a dedicated type, matching this crate's established raw-JSON-escape-hatch convention).
- `McpToolCall(RealtimeMcpToolCall)` -- tag `mcp_call`. Fields: `id: String`, `arguments: String`, `name: String`, `server_label: String` (all required), `approval_request_id: Option<String>`, `output: Option<String>`, `error: Option<RealtimeMcpToolCallError>`. `RealtimeMcpToolCallError` is a `type`-tagged 3-variant union (`ProtocolError { code: i64, message: String }` / `ToolExecutionError { message: String }` / `HttpError { code: i64, message: String }`, tags `protocol_error`/`tool_execution_error`/`http_error`), modeled fully since it's small and the whole reason `error` exists.
- `McpApprovalRequest(RealtimeMcpApprovalRequest)` -- tag `mcp_approval_request`. Fields: `id: String`, `arguments: String`, `name: String`, `server_label: String` (all required, no optional fields at all -- confirmed via source).

### Tool configuration types (`interfaces/realtime.rs`)

- `RealtimeFunctionTool { description: Option<String>, name: Option<String>, parameters: Option<serde_json::Value> }` -- a NEW dedicated type, not a reuse of `crate::interfaces::responses::ResponseFunctionTool`. Confirmed via source these genuinely differ: Responses' version requires `name`/`parameters` and has a `strict` field Realtime's version lacks entirely; Realtime's version has all three fields optional (`total=False` TypedDict) and no `strict`. A "looks reusable but isn't" case, same judgment call already made for `VectorStoreExpiresAfter`/`ContainerExpiresAfter`.
- `RealtimeTool` -- `type`-tagged 2-variant union: `Function(RealtimeFunctionTool)` / `Mcp(crate::interfaces::responses::ResponseMcpTool)` (the confirmed direct reuse from Context, above). Used for both `RealtimeSessionConfig.tools` and `RealtimeResponseCreateParams.tools` -- confirmed via source (`realtime_response_create_params.py`'s `Tool: TypeAlias = Union[RealtimeFunctionTool, RealtimeResponseCreateMcpTool]`) that both places share the identical two-variant shape.
- `RealtimeToolChoice` -- an `untagged` 3-variant union: `Mode(crate::interfaces::responses::ResponseToolChoiceMode)` (reused directly -- confirmed identical 3-value `"none"|"auto"|"required"` set) / `Mcp { #[serde(rename="type")] kind: String, server_label: String, name: Option<String> }` / `Function { #[serde(rename="type")] kind: String, name: String }`.

  **A genuine untagged-ordering risk, flagged and resolved explicitly**: unlike this crate's prior untagged unions (which stayed safe because their variants were JSON-*type*-disjoint, e.g. bare string vs. object), `Mcp` and `Function` are BOTH objects with overlapping optional fields, and serde's derived struct-variant deserialization does not reject unknown fields by default. A `{"type":"mcp","server_label":"s","name":"n"}` payload would incorrectly deserialize as `Function` (which only requires `type`+`name`, silently ignoring the extra `server_label`) if `Function` were tried first. **Fixed by declaring `Mcp` before `Function`** in the enum: `Mcp` requires `server_label` (absent from any real `Function` payload), so it correctly fails-and-falls-through for function payloads, while still matching every valid MCP payload including ones that happen to set the optional `name`. A dedicated round-trip test covers exactly this case (an MCP choice with `name` set, proving it does NOT get misparsed as `Function`).
- `RealtimeSessionConfig` gains `tools: Option<Vec<RealtimeTool>>` and `tool_choice: Option<RealtimeToolChoice>`.
- `RealtimeResponseCreateParams` gains the same two fields, same types.
- `RealtimeResponse` needs NO changes -- confirmed via source (`realtime_response.py`) it does not echo `tools`/`tool_choice` at all.

### New server events (10)

All confirmed via `realtime_server_event.py`'s full union list -- these are the only tool-related entries not yet modeled:

- `ResponseFunctionCallArgumentsDelta { call_id, delta, event_id, item_id, output_index: u32, response_id }` -- tag `response.function_call_arguments.delta`.
- `ResponseFunctionCallArgumentsDone { arguments, call_id, event_id, item_id, name, output_index: u32, response_id }` -- tag `response.function_call_arguments.done`.
- `ResponseMcpCallArgumentsDelta { delta, event_id, item_id, output_index: u32, response_id, obfuscation: Option<String> }` -- tag `response.mcp_call_arguments.delta`.
- `ResponseMcpCallArgumentsDone { arguments, event_id, item_id, output_index: u32, response_id }` -- tag `response.mcp_call_arguments.done`.
- `ResponseMcpCallInProgress { event_id, item_id, output_index: u32 }` -- tag `response.mcp_call.in_progress`.
- `ResponseMcpCallCompleted { event_id, item_id, output_index: u32 }` -- tag `response.mcp_call.completed`.
- `ResponseMcpCallFailed { event_id, item_id, output_index: u32 }` -- tag `response.mcp_call.failed`.
- `McpListToolsInProgress { event_id, item_id }` -- tag `mcp_list_tools.in_progress`.
- `McpListToolsCompleted { event_id, item_id }` -- tag `mcp_list_tools.completed`.
- `McpListToolsFailed { event_id, item_id }` -- tag `mcp_list_tools.failed`.

### No client event changes

Confirmed via `realtime_client_event.py`'s full union list: it is unchanged from Phase A/B's 11 entries (9 of which this crate implements; `conversation.item.truncate` and `output_audio_buffer.clear` remain out of scope per their existing reasons). Function calling and MCP both flow entirely through `session.update` (config), `conversation.item.create` (client sends `function_call_output`/`mcp_approval_response` items; server sends back `function_call`/`mcp_list_tools`/`mcp_call`/`mcp_approval_request` items via the existing item-lifecycle server events), and `response.create` (per-call tool overrides).

## Non-goals

- `defer_loading`-driven tool search / lazy MCP tool discovery beyond what `ResponseMcpTool.defer_loading` already exposes as a plain bool -- no dedicated modeling, matches this crate's existing scope for that field in Responses.
- `conversation.item.truncate`/`.truncated`, DTMF events, input-audio-transcription events, `conversation.item.added`/`.done`'s cross-cutting alternates to `.created` -- all unrelated to tool calling, already deferred from earlier phases or out of this slice's scope.
- Client-side tool EXECUTION -- this crate is a thin binding layer; it models the wire types needed to configure tools and read/write tool-call items and events, not a tool-invocation runtime.

## Testing

Same layered discipline as Phases A/B: plain `serde_json` round-trip tests for every new item type, tool config type, and server event -- no live socket needed for shape correctness. One dedicated test for the flagged `RealtimeToolChoice` ordering risk (an MCP choice with `name` set, asserting it deserializes to `Mcp` not `Function`). One test confirming `ConversationItem`'s new variants round-trip through the same tag-dispatch mechanism the two-level `Message` variant already proved works in Phase A (a single-level tag is strictly simpler, but still worth a smoke test given the union just grew 6x). No new live-WebSocket test needed -- Phase A/B's existing `send`/`recv` tests already prove the transport layer handles arbitrary `RealtimeClientEvent`/`RealtimeServerEvent` payloads; this slice adds no new transport behavior, only new payload shapes.

## Versioning

Purely additive, except the untagged `RealtimeToolChoice` variant-ordering fix is a new type with no prior existence, so no backward-compat concern.
