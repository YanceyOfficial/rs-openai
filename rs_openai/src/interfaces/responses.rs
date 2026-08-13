use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ResponseInput {
    Text(String),
    Items(Vec<ResponseInputItem>),
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ResponseInputRole {
    User,
    Assistant,
    System,
    Developer,
}

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

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ResponseFunctionTool {
    #[builder(into)]
    pub name: String,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// A JSON Schema object describing the function's parameters.
    pub parameters: serde_json::Value,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

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

    #[builder(into)]
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

/// Config for the older `computer_use_preview` tool. OpenAI's guides steer new integrations
/// toward the minimal GA `ResponseTool::Computer` instead, but the API still accepts this
/// tool type, and its `computer_call` output populates a single `action` field rather than
/// the GA tool's `actions` array — see `ResponseComputerCallItem`.
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ResponseComputerUsePreviewTool {
    pub display_width: u32,
    pub display_height: u32,
    pub environment: ComputerEnvironment,
}

/// A tool the model may call. Extensible union — this slice implements `Function`,
/// `FileSearch`, `WebSearch`, `ImageGeneration`, `Mcp`, `Computer`, and `ComputerUsePreview`;
/// only `CodeInterpreter` remains as a future non-breaking variant (gated on the Containers
/// resource).
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

/// The plain string modes `tool_choice` can take.
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

/// Controls which (if any) tool is called by the model. Either a plain mode string
/// (`"none"`/`"auto"`/`"required"`) or an object forcing one specific function. Mirrors
/// `crate::interfaces::chat::ToolChoice`'s shape exactly, but kept as a distinct type since
/// the two APIs document this parameter separately.
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
#[serde(untagged)]
pub enum ResponseConversationParam {
    Id(String),
    Object { id: String },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum TruncationStrategy {
    Auto,
    Disabled,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ResponseTextConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<crate::interfaces::chat::ChatResponseFormat>,
}

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

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation: Option<ResponseConversationParam>,
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
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

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseImageGenerationCallItem {
    pub id: String,
    pub status: String,
    pub result: Option<String>,
    pub revised_prompt: Option<String>,
}

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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, strum::Display)]
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

/// One item in a response's `output` array. Extensible union — this slice implements
/// `Message`/`FunctionCall`/`FileSearchCall`/`WebSearchCall`/`ImageGenerationCall`/`McpListTools`/`McpCall`/`McpApprovalRequest`/`ComputerCall`;
/// only `Reasoning`/`CodeInterpreterCall`/etc. remain as future non-breaking variants.
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

/// A single Server-Sent Event from a streamed `Responses::create_with_stream` call. Covers
/// only the events relevant to this crate's current (text + function-calling only) type
/// surface — see docs/superpowers/specs/2026-08-11-responses-streaming-design.md for the
/// full list of deferred event families (content_part, refusal, other tool types, reasoning).
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ResponseStreamEvent {
    #[serde(rename = "response.created")]
    Created {
        response: Response,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.in_progress")]
    InProgress {
        response: Response,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.completed")]
    Completed {
        response: Response,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.failed")]
    Failed {
        response: Response,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.incomplete")]
    Incomplete {
        response: Response,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.output_item.added")]
    OutputItemAdded {
        output_index: u32,
        item: ResponseOutputItem,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.output_item.done")]
    OutputItemDone {
        output_index: u32,
        item: ResponseOutputItem,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.output_text.delta")]
    OutputTextDelta {
        item_id: String,
        output_index: u32,
        content_index: u32,
        delta: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.output_text.done")]
    OutputTextDone {
        item_id: String,
        output_index: u32,
        content_index: u32,
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.function_call_arguments.delta")]
    FunctionCallArgumentsDelta {
        item_id: String,
        output_index: u32,
        delta: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.function_call_arguments.done")]
    FunctionCallArgumentsDone {
        item_id: String,
        output_index: u32,
        arguments: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "error")]
    Error {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        param: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
}
