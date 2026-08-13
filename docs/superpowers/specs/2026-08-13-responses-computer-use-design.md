# Responses API: `computer_use` Tool — Design

Status: Approved
Date: 2026-08-13

## Context

The fifth and final Responses tool type researched in this project (after `file_search`, `web_search`, `image_generation`, `mcp`) — sixth and seventh variants on `ResponseTool`, ninth on `ResponseOutputItem`, fourth on `ResponseInputItem`. `computer_use` lets the model operate a virtual computer (click, type, scroll, screenshot) via a batch of low-level UI actions.

Live research this session (WebFetch against `developers.openai.com`, cross-checked across the tool guide, a dedicated migration section, and — most authoritatively — the actual API reference schema page) surfaced a real, non-obvious finding: OpenAI's guide prose describes the older `computer_use_preview` tool as **deprecated** in favor of a newer, minimal `computer` (GA) tool, but the **API reference schema still defines both as separate, coexisting tool types** — `Computer` (`type: "computer"`, no configurable fields) and `ComputerUsePreview` (`type: "computer_use_preview"`, requires `display_width`/`display_height`/`environment`). Both remain valid to send. Per `AskUserQuestion` ("Full: both tool variants... (Recommended)"), this slice models both.

The schema also confirmed `computer_call`'s output shape carries **both** an optional singular `action` field and an optional plural `actions` array as separate properties — the GA `computer` tool populates `actions`, the older `computer_use_preview` tool populates `action`; a caller must check whichever is present. Also confirmed: `pending_safety_checks`/`acknowledged_safety_checks` (shape `{id, code, message}`) are real, current schema fields — a guide-page fetch that turned up no mention of them was simply incomplete documentation, not evidence of removal; the reference schema is the authoritative source and was trusted over the prose guide where they conflicted.

This is the largest, most complex tool type modeled in this project so far: 2 tool config variants, a 9-action tagged union, a 5-value button enum, and safety-check plumbing on both the output item and its answering input item.

## Goals

- `ResponseTool` gains a `Computer` unit variant (serializes to bare `{"type": "computer"}`, no fields — matches the GA tool's empty config) and a `ComputerUsePreview(ResponseComputerUsePreviewTool)` variant (sixth and seventh variants on the enum).
- `ResponseComputerUsePreviewTool`: `display_width: u32`, `display_height: u32`, `environment: ComputerEnvironment` — all three required and confirmed non-optional in the schema.
- `ComputerEnvironment` (5-value enum, `strum::Display` + matching serde rename): `Windows`, `Mac`, `Linux`, `Ubuntu`, `Browser`.
- `ComputerActionButton` (5-value enum, `strum::Display` + matching serde rename): `Left`, `Right`, `Wheel`, `Back`, `Forward` — the confirmed complete set (wider than the 3-value set an earlier, less-authoritative fetch suggested).
- `ComputerAction` — a 9-variant extensible-in-spirit (but schema-closed, all 9 confirmed) tagged union covering every action the model can emit: `Click { x, y, button: Option<ComputerActionButton>, keys: Option<Vec<String>> }`, `DoubleClick { x, y, keys: Option<Vec<String>> }`, `Drag { path: Vec<(i64, i64)>, keys: Option<Vec<String>> }`, `Keypress { keys: Vec<String> }` (required — this is the actual key combo being pressed, distinct semantically from the optional `keys` modifier field on the pointer actions), `Move { x, y, keys: Option<Vec<String>> }`, `Screenshot {}`, `Scroll { x, y, scroll_x, scroll_y, keys: Option<Vec<String>> }`, `Type { text: String }`, `Wait {}`. `x`/`y`/`scroll_x`/`scroll_y` are `i64` (not constrained to unsigned — multi-monitor setups can have negative coordinates, and nothing in the schema confirms a non-negative constraint).
- `ComputerSafetyCheck`: `id: String`, `code: String` (kept a plain string, not a closed enum — the three known check categories were only described in prose ("malicious instruction detection" etc.), never with their actual wire-value strings, so this follows the established under-typing pattern for unconfirmed enums), `message: String`.
- `ResponseOutputItem` gains a `ComputerCall(ResponseComputerCallItem)` variant (ninth variant). `ResponseComputerCallItem`: `id: String`, `call_id: String`, `status: String` (kept a plain string for uniformity with every other `*_call` output item in this file — `file_search_call`/`web_search_call`/`image_generation_call` all keep `status` as `String` despite varying confirmation levels; introducing the one confirmed closed enum here would create an inconsistency the rest of the file doesn't have), `action: Option<ComputerAction>`, `actions: Option<Vec<ComputerAction>>`, `pending_safety_checks: Option<Vec<ComputerSafetyCheck>>`.
- `ResponseInputItem` gains a `ComputerCallOutput` variant (fourth variant on that union): `id: Option<String>`, `call_id: String`, `output: ComputerCallOutputScreenshot`, `acknowledged_safety_checks: Option<Vec<ComputerSafetyCheck>>`, `status: Option<String>`. This lets a caller answer a `computer_call` by executing its action(s) locally, taking a screenshot, and sending it back in a follow-up request (or a `Conversation`'s `items`, since that already reuses `ResponseInputItem` directly).
- `ComputerCallOutputScreenshot`: `kind: String` (`#[serde(rename = "type")]`, caller sets `"computer_screenshot"` explicitly — matches the established `kind`-field-for-wire-`"type"`-collision convention already used by `WebSearchUserLocation`/`WebSearchActionSource`/`ResponseToolChoice::Specific`), `image_url: String` (a `data:image/png;base64,...` URI), `detail: Option<String>`.

## Non-goals (explicit, deferred to later slices)

- **`code_interpreter`** — still gated on the not-yet-implemented Containers resource. With `computer_use` shipped, `code_interpreter` is now the ONLY Responses tool type left unimplemented.
- **Computer-use-specific streaming events** — `ResponseStreamEvent` stays scoped to text + function-calling events, the same deferral already established for every other tool type.
- **`ComputerSafetyCheck.code` as a closed enum** — stays a plain string; the known category names were never confirmed with exact wire values this session.
- **`ResponseComputerCallItem.status` as a closed enum** — stays a plain string despite the schema confirming a 3-value set (`in_progress`/`completed`/`incomplete`), for uniformity with sibling `*_call` items in this file.

## Design

### File organization

All changes land in the existing `rs_openai/src/interfaces/responses.rs` — no new files.

### `computer`/`computer_use_preview` tools (request side)

```rust
#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ComputerEnvironment {
    Windows,
    Mac,
    Linux,
    Ubuntu,
    Browser,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ResponseComputerUsePreviewTool {
    pub display_width: u32,
    pub display_height: u32,
    pub environment: ComputerEnvironment,
}
```

`ResponseTool` gains two variants (existing enum, shown with context):

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
}
```

`Computer` is a unit variant — under `#[serde(tag = "type", rename_all = "snake_case")]`, this serializes to exactly `{"type": "computer"}` with no other keys, matching the GA tool's empty config.

### `computer_call` output item (response side)

```rust
#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ComputerActionButton {
    Left,
    Right,
    Wheel,
    Back,
    Forward,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ComputerAction {
    Click {
        x: i64,
        y: i64,
        #[serde(skip_serializing_if = "Option::is_none")]
        button: Option<ComputerActionButton>,
        #[serde(skip_serializing_if = "Option::is_none")]
        keys: Option<Vec<String>>,
    },
    DoubleClick {
        x: i64,
        y: i64,
        #[serde(skip_serializing_if = "Option::is_none")]
        keys: Option<Vec<String>>,
    },
    Drag {
        path: Vec<(i64, i64)>,
        #[serde(skip_serializing_if = "Option::is_none")]
        keys: Option<Vec<String>>,
    },
    Keypress {
        keys: Vec<String>,
    },
    Move {
        x: i64,
        y: i64,
        #[serde(skip_serializing_if = "Option::is_none")]
        keys: Option<Vec<String>>,
    },
    Screenshot {},
    Scroll {
        x: i64,
        y: i64,
        scroll_x: i64,
        scroll_y: i64,
        #[serde(skip_serializing_if = "Option::is_none")]
        keys: Option<Vec<String>>,
    },
    Type {
        text: String,
    },
    Wait {},
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ComputerSafetyCheck {
    #[builder(into)]
    pub id: String,
    #[builder(into)]
    pub code: String,
    #[builder(into)]
    pub message: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseComputerCallItem {
    pub id: String,
    pub call_id: String,
    pub status: String,
    pub action: Option<ComputerAction>,
    pub actions: Option<Vec<ComputerAction>>,
    pub pending_safety_checks: Option<Vec<ComputerSafetyCheck>>,
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
}
```

### `computer_call_output` (input side)

```rust
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ComputerCallOutputScreenshot {
    #[builder(into)]
    #[serde(rename = "type")]
    pub kind: String,

    #[builder(into)]
    pub image_url: String,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}
```

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
    ComputerCallOutput {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        call_id: String,
        output: ComputerCallOutputScreenshot,
        #[serde(skip_serializing_if = "Option::is_none")]
        acknowledged_safety_checks: Option<Vec<ComputerSafetyCheck>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<String>,
    },
}
```

## Testing

`wiremock` tests covering: `create` serializing a `computer` (GA) tool as bare `{"type": "computer"}`; `create` serializing a `computer_use_preview` tool with `display_width`/`display_height`/`environment`; deserializing a `computer_call` output item populated via the GA shape (`actions` array, `action: null`) with a `screenshot` and a `click` action in the same array (proving the 9-variant union discriminates correctly within a real array, and that `button`/`keys` omission works); a deserialization test for a `computer_call` with `pending_safety_checks` populated (proving the safety-check struct deserializes); a serialization test proving `ResponseInputItem::ComputerCallOutput` (with `acknowledged_safety_checks` populated) round-trips to the exact wire shape including the nested `ComputerCallOutputScreenshot`'s renamed `type` field; a mixed-array test proving `ResponseOutputItem` discriminates `ComputerCall` from the other eight coexisting variants.

## Versioning

Purely additive at the crate level (`ResponseTool`/`ResponseOutputItem`/`ResponseInputItem` each gain new non-breaking variants). Ships as a pre-1.0 minor bump alongside whatever else accumulates before the next release, per established practice.
