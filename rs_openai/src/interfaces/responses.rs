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

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ImageDetail {
    Low,
    High,
    Auto,
    Original,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ResponseInputText {
    #[builder(into)]
    pub text: String,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ResponseInputImage {
    pub detail: ImageDetail,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ResponseInputFile {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_data: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_id: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_url: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
}

/// A single content part of a multimodal message. Text/image/file only this round -- audio
/// input (`ResponseInputAudioParam` in the source SDK) is deliberately deferred, matching
/// this crate's standing pattern of scoping large unions down to the well-documented common
/// cases.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseInputContent {
    InputText(ResponseInputText),
    InputImage(ResponseInputImage),
    InputFile(ResponseInputFile),
}

/// A message's `content` is either a plain string (the common case) or a list of content
/// parts for multimodal (text/image/file) input -- confirmed via source
/// (`EasyInputMessageParam.content: Union[str, ResponseInputMessageContentListParam]`).
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ResponseInputMessageContent {
    Text(String),
    Items(Vec<ResponseInputContent>),
}

impl From<String> for ResponseInputMessageContent {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<&str> for ResponseInputMessageContent {
    fn from(text: &str) -> Self {
        Self::Text(text.to_string())
    }
}

impl From<Vec<ResponseInputContent>> for ResponseInputMessageContent {
    fn from(items: Vec<ResponseInputContent>) -> Self {
        Self::Items(items)
    }
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
        content: ResponseInputMessageContent,
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

/// Confirmed via source (`total=False` TypedDict): BOTH fields are optional. Reused for
/// `require_approval`'s `always`/`never` filters AND `allowed_tools`' filter form -- both
/// confirmed identical shape. Renamed from `McpToolNameFilter` (which was missing
/// `read_only` and wrongly required `tool_names`) -- a deliberate breaking change.
#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct McpToolFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_names: Option<Vec<String>>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct McpApprovalFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub never: Option<McpToolFilter>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub always: Option<McpToolFilter>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum McpRequireApproval {
    Mode(McpApprovalMode),
    Filter(McpApprovalFilter),
}

/// `allowed_tools` is either a plain list of tool names, or a filter object -- confirmed via
/// source (`McpAllowedTools: TypeAlias = Union[SequenceNotStr[str], McpAllowedToolsMcpToolFilter]`).
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum McpAllowedTools {
    Names(Vec<String>),
    Filter(McpToolFilter),
}

impl From<Vec<String>> for McpAllowedTools {
    fn from(names: Vec<String>) -> Self {
        Self::Names(names)
    }
}

/// Identifier for a service connector (e.g. Dropbox, Gmail) as an alternative to a raw
/// `server_url`. Each variant is spelled out explicitly since the wire values don't follow
/// a uniform snake_case-from-PascalCase conversion (e.g. `connector_googlecalendar`, not
/// `connector_google_calendar`).
#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
pub enum McpConnectorId {
    #[serde(rename = "connector_dropbox")]
    #[strum(serialize = "connector_dropbox")]
    Dropbox,
    #[serde(rename = "connector_gmail")]
    #[strum(serialize = "connector_gmail")]
    Gmail,
    #[serde(rename = "connector_googlecalendar")]
    #[strum(serialize = "connector_googlecalendar")]
    GoogleCalendar,
    #[serde(rename = "connector_googledrive")]
    #[strum(serialize = "connector_googledrive")]
    GoogleDrive,
    #[serde(rename = "connector_microsoftteams")]
    #[strum(serialize = "connector_microsoftteams")]
    MicrosoftTeams,
    #[serde(rename = "connector_outlookcalendar")]
    #[strum(serialize = "connector_outlookcalendar")]
    OutlookCalendar,
    #[serde(rename = "connector_outlookemail")]
    #[strum(serialize = "connector_outlookemail")]
    OutlookEmail,
    #[serde(rename = "connector_sharepoint")]
    #[strum(serialize = "connector_sharepoint")]
    SharePoint,
}

/// The tool invocation context(s) allowed to call this MCP tool.
#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum McpAllowedCaller {
    Direct,
    Programmatic,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ResponseMcpTool {
    #[builder(into)]
    pub server_label: String,

    /// One of `server_url`, `connector_id`, or `tunnel_id` must be provided -- confirmed
    /// via source; not enforced client-side, matching this crate's convention of not
    /// re-implementing server-side "one of" validation. Was previously a required `String`;
    /// changed to `Option<String>` to allow the connector/tunnel alternatives -- a
    /// deliberate breaking change, though `#[builder(into)]` keeps existing
    /// `.server_url("...")` call sites source-compatible.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub connector_id: Option<McpConnectorId>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tunnel_id: Option<String>,

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

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<McpAllowedTools>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_callers: Option<Vec<McpAllowedCaller>>,

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

/// A tool the model may call. This is the complete, confirmed set of Responses tool types --
/// `Function`, `FileSearch`, `WebSearch`, `ImageGeneration`, `Mcp`, `Computer`,
/// `ComputerUsePreview`, and `CodeInterpreter`.
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

/// Additional output data a caller can request beyond the default response shape.
/// `strum`/`serde` renames use dotted values (not valid Rust snake_case), so each variant
/// is spelled out explicitly rather than via a blanket `rename_all`.
#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
pub enum ResponseIncludable {
    #[serde(rename = "file_search_call.results")]
    #[strum(serialize = "file_search_call.results")]
    FileSearchCallResults,
    #[serde(rename = "web_search_call.results")]
    #[strum(serialize = "web_search_call.results")]
    WebSearchCallResults,
    #[serde(rename = "web_search_call.action.sources")]
    #[strum(serialize = "web_search_call.action.sources")]
    WebSearchCallActionSources,
    #[serde(rename = "message.input_image.image_url")]
    #[strum(serialize = "message.input_image.image_url")]
    MessageInputImageImageUrl,
    #[serde(rename = "computer_call_output.output.image_url")]
    #[strum(serialize = "computer_call_output.output.image_url")]
    ComputerCallOutputOutputImageUrl,
    #[serde(rename = "code_interpreter_call.outputs")]
    #[strum(serialize = "code_interpreter_call.outputs")]
    CodeInterpreterCallOutputs,
    #[serde(rename = "reasoning.encrypted_content")]
    #[strum(serialize = "reasoning.encrypted_content")]
    ReasoningEncryptedContent,
    #[serde(rename = "message.output_text.logprobs")]
    #[strum(serialize = "message.output_text.logprobs")]
    MessageOutputTextLogprobs,
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

    /// Additional output data to include (e.g. file/web search results, image URLs,
    /// reasoning content). A JSON body field, unlike `retrieve`/`list_input_items`'s
    /// same-named query param -- no `Vec<String>`-in-query workaround needed here.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<ResponseIncludable>>,
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

/// Extensible tagged union, reused both for `ResponseOutputMessage.content` and for the
/// `content_part.added`/`.done` stream events' `part` field -- both confirmed to share the
/// same `type`-tagged shape (`output_text`/`refusal`/`reasoning_text`), matching this
/// project's established extensible-tagged-union design (`ResponseTool`/`ResponseOutputItem`).
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
    ReasoningText {
        text: String,
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

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseCodeInterpreterCallItem {
    pub id: String,
    pub code: String,
    pub container_id: String,
    pub status: String,
}

/// One item in a response's `output` array. This is the complete, confirmed set of output
/// item types for the tools this crate implements -- `Message`, `FunctionCall`,
/// `FileSearchCall`, `WebSearchCall`, `ImageGenerationCall`, `McpListTools`, `McpCall`,
/// `McpApprovalRequest`, `ComputerCall`, and `CodeInterpreterCall`.
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

/// Retrieves a response. `include_obfuscation`/`starting_after` (streaming-resume params,
/// confirmed via source) are deliberately out of scope -- this crate's `retrieve()` always
/// does a plain non-streaming fetch, with no support for resuming an in-flight stream.
#[derive(Builder, Debug, Clone, Default)]
pub struct RetrieveResponseRequest {
    pub include: Option<Vec<ResponseIncludable>>,
}

impl RetrieveResponseRequest {
    pub(crate) fn to_query_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        if let Some(values) = &self.include {
            for v in values {
                pairs.push(("include".to_string(), v.to_string()));
            }
        }
        pairs
    }
}

/// `include` (a `Vec<ResponseIncludable>` query param) needs `to_query_pairs()` rather than
/// the generic `get()` path, matching the established `Vec<String>`-in-query workaround.
#[derive(Builder, Debug, Clone, Default)]
pub struct ListInputItemsRequest {
    #[builder(into)]
    pub after: Option<String>,

    pub include: Option<Vec<ResponseIncludable>>,

    pub limit: Option<u32>,

    pub order: Option<crate::interfaces::chat::SortOrder>,
}

impl ListInputItemsRequest {
    pub(crate) fn to_query_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        if let Some(v) = &self.after {
            pairs.push(("after".to_string(), v.clone()));
        }
        if let Some(values) = &self.include {
            for v in values {
                pairs.push(("include".to_string(), v.to_string()));
            }
        }
        if let Some(v) = self.limit {
            pairs.push(("limit".to_string(), v.to_string()));
        }
        if let Some(v) = &self.order {
            pairs.push(("order".to_string(), v.to_string()));
        }
        pairs
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListInputItemsResponse {
    pub object: String,
    pub data: Vec<ResponseInputItem>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseLogprobTopLogprob {
    pub token: Option<String>,
    pub logprob: Option<f64>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ResponseLogprob {
    pub token: String,
    pub logprob: f64,
    pub top_logprobs: Option<Vec<ResponseLogprobTopLogprob>>,
}

/// A reasoning summary part, carried by `ReasoningSummaryPartAdded`/`Done` stream events.
/// An extensible tagged union (matching this crate's established pattern) with one
/// confirmed variant.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ReasoningSummaryPart {
    SummaryText { text: String },
}

/// A single Server-Sent Event from a streamed `Responses::create_with_stream` call. Covers
/// lifecycle + text + function-calling events, the full streaming lifecycle for every tool
/// type this crate implements (file_search, web_search, image_generation, mcp,
/// code_interpreter), content_part, refusal, reasoning (text + summary), and audio events --
/// see docs/superpowers/specs/2026-08-11-responses-streaming-design.md and the later specs
/// under docs/superpowers/specs/ prefixed `responses-streaming-`. The full union is 60
/// confirmed variants; the remaining ~6 (shell and custom-tool events) are deliberately
/// deferred -- this crate doesn't implement programmatic shell/custom tools, so those events
/// would be permanently unreachable. `computer_use` has no per-item streaming events of its
/// own (confirmed absent from the union).
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
        /// Confirmed via source to always be present (an empty array when logprobs weren't
        /// requested via `include: ["message.output_text.logprobs"]`) -- `#[serde(default)]`
        /// keeps this crate's own pre-existing test fixtures (written before this field was
        /// modeled) deserializing unchanged.
        #[serde(default)]
        logprobs: Vec<ResponseLogprob>,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.output_text.done")]
    OutputTextDone {
        item_id: String,
        output_index: u32,
        content_index: u32,
        text: String,
        #[serde(default)]
        logprobs: Vec<ResponseLogprob>,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    /// Emitted when a new content part is added.
    #[serde(rename = "response.content_part.added")]
    ContentPartAdded {
        content_index: u32,
        item_id: String,
        output_index: u32,
        part: ResponseOutputContent,
        sequence_number: u64,
    },
    /// Emitted when a content part is done.
    #[serde(rename = "response.content_part.done")]
    ContentPartDone {
        content_index: u32,
        item_id: String,
        output_index: u32,
        part: ResponseOutputContent,
        sequence_number: u64,
    },
    /// Emitted for a partial refusal text.
    #[serde(rename = "response.refusal.delta")]
    RefusalDelta {
        content_index: u32,
        delta: String,
        item_id: String,
        output_index: u32,
        sequence_number: u64,
    },
    /// Emitted when refusal text is finalized.
    #[serde(rename = "response.refusal.done")]
    RefusalDone {
        content_index: u32,
        item_id: String,
        output_index: u32,
        refusal: String,
        sequence_number: u64,
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
    /// Emitted when a file search call is initiated. `sequence_number` is required here
    /// (confirmed via source), unlike the lifecycle events above where it's optional.
    #[serde(rename = "response.file_search_call.in_progress")]
    FileSearchCallInProgress {
        item_id: String,
        output_index: u32,
        sequence_number: u64,
    },
    /// Emitted while a file search call is actively searching.
    #[serde(rename = "response.file_search_call.searching")]
    FileSearchCallSearching {
        item_id: String,
        output_index: u32,
        sequence_number: u64,
    },
    /// Emitted when a file search call has completed (results found).
    #[serde(rename = "response.file_search_call.completed")]
    FileSearchCallCompleted {
        item_id: String,
        output_index: u32,
        sequence_number: u64,
    },
    /// Emitted when a web search call is initiated.
    #[serde(rename = "response.web_search_call.in_progress")]
    WebSearchCallInProgress {
        item_id: String,
        output_index: u32,
        sequence_number: u64,
    },
    /// Emitted when a web search call is executing.
    #[serde(rename = "response.web_search_call.searching")]
    WebSearchCallSearching {
        item_id: String,
        output_index: u32,
        sequence_number: u64,
    },
    /// Emitted when a web search call is completed.
    #[serde(rename = "response.web_search_call.completed")]
    WebSearchCallCompleted {
        item_id: String,
        output_index: u32,
        sequence_number: u64,
    },
    /// Emitted when an image generation tool call is in progress.
    #[serde(rename = "response.image_generation_call.in_progress")]
    ImageGenerationCallInProgress {
        item_id: String,
        output_index: u32,
        sequence_number: u64,
    },
    /// Emitted while an image generation tool call is actively generating (intermediate).
    #[serde(rename = "response.image_generation_call.generating")]
    ImageGenerationCallGenerating {
        item_id: String,
        output_index: u32,
        sequence_number: u64,
    },
    /// Emitted when a partial image is available during image generation streaming.
    #[serde(rename = "response.image_generation_call.partial_image")]
    ImageGenerationCallPartialImage {
        item_id: String,
        output_index: u32,
        partial_image_b64: String,
        partial_image_index: u32,
        sequence_number: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        background: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        output_format: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        quality: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        size: Option<String>,
    },
    /// Emitted when an image generation tool call has completed (final image available).
    #[serde(rename = "response.image_generation_call.completed")]
    ImageGenerationCallCompleted {
        item_id: String,
        output_index: u32,
        sequence_number: u64,
    },
    /// Emitted when an MCP tool call is in progress.
    #[serde(rename = "response.mcp_call.in_progress")]
    McpCallInProgress {
        item_id: String,
        output_index: u32,
        sequence_number: u64,
    },
    /// Emitted for a partial update to an MCP tool call's arguments.
    #[serde(rename = "response.mcp_call_arguments.delta")]
    McpCallArgumentsDelta {
        item_id: String,
        output_index: u32,
        delta: String,
        sequence_number: u64,
    },
    /// Emitted when an MCP tool call's arguments are finalized.
    #[serde(rename = "response.mcp_call_arguments.done")]
    McpCallArgumentsDone {
        item_id: String,
        output_index: u32,
        arguments: String,
        sequence_number: u64,
    },
    /// Emitted when an MCP tool call has completed successfully.
    #[serde(rename = "response.mcp_call.completed")]
    McpCallCompleted {
        item_id: String,
        output_index: u32,
        sequence_number: u64,
    },
    /// Emitted when an MCP tool call has failed.
    #[serde(rename = "response.mcp_call.failed")]
    McpCallFailed {
        item_id: String,
        output_index: u32,
        sequence_number: u64,
    },
    /// Emitted while retrieving the list of available MCP tools.
    #[serde(rename = "response.mcp_list_tools.in_progress")]
    McpListToolsInProgress {
        item_id: String,
        output_index: u32,
        sequence_number: u64,
    },
    /// Emitted when the list of available MCP tools has been successfully retrieved.
    #[serde(rename = "response.mcp_list_tools.completed")]
    McpListToolsCompleted {
        item_id: String,
        output_index: u32,
        sequence_number: u64,
    },
    /// Emitted when listing available MCP tools has failed.
    #[serde(rename = "response.mcp_list_tools.failed")]
    McpListToolsFailed {
        item_id: String,
        output_index: u32,
        sequence_number: u64,
    },
    /// Emitted when a code interpreter call is in progress.
    #[serde(rename = "response.code_interpreter_call.in_progress")]
    CodeInterpreterCallInProgress {
        item_id: String,
        output_index: u32,
        sequence_number: u64,
    },
    /// Emitted while the code interpreter is actively interpreting the code snippet.
    #[serde(rename = "response.code_interpreter_call.interpreting")]
    CodeInterpreterCallInterpreting {
        item_id: String,
        output_index: u32,
        sequence_number: u64,
    },
    /// Emitted for a partial code snippet streamed by the code interpreter.
    #[serde(rename = "response.code_interpreter_call_code.delta")]
    CodeInterpreterCallCodeDelta {
        item_id: String,
        output_index: u32,
        delta: String,
        sequence_number: u64,
    },
    /// Emitted when the code interpreter's code snippet is finalized.
    #[serde(rename = "response.code_interpreter_call_code.done")]
    CodeInterpreterCallCodeDone {
        item_id: String,
        output_index: u32,
        code: String,
        sequence_number: u64,
    },
    /// Emitted when a code interpreter call has completed.
    #[serde(rename = "response.code_interpreter_call.completed")]
    CodeInterpreterCallCompleted {
        item_id: String,
        output_index: u32,
        sequence_number: u64,
    },
    /// Emitted when a delta is added to a reasoning text.
    #[serde(rename = "response.reasoning_text.delta")]
    ReasoningTextDelta {
        content_index: u32,
        delta: String,
        item_id: String,
        output_index: u32,
        sequence_number: u64,
    },
    /// Emitted when a reasoning text is completed.
    #[serde(rename = "response.reasoning_text.done")]
    ReasoningTextDone {
        content_index: u32,
        item_id: String,
        output_index: u32,
        sequence_number: u64,
        text: String,
    },
    /// Emitted when a new reasoning summary part is added.
    #[serde(rename = "response.reasoning_summary_part.added")]
    ReasoningSummaryPartAdded {
        item_id: String,
        output_index: u32,
        part: ReasoningSummaryPart,
        sequence_number: u64,
        summary_index: u32,
    },
    /// Emitted when a reasoning summary part is completed. `status` is `Some("incomplete")`
    /// when generation was interrupted, omitted otherwise.
    #[serde(rename = "response.reasoning_summary_part.done")]
    ReasoningSummaryPartDone {
        item_id: String,
        output_index: u32,
        part: ReasoningSummaryPart,
        sequence_number: u64,
        summary_index: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<String>,
    },
    /// Emitted when a delta is added to a reasoning summary text.
    #[serde(rename = "response.reasoning_summary_text.delta")]
    ReasoningSummaryTextDelta {
        delta: String,
        item_id: String,
        output_index: u32,
        sequence_number: u64,
        summary_index: u32,
    },
    /// Emitted when a reasoning summary text is completed.
    #[serde(rename = "response.reasoning_summary_text.done")]
    ReasoningSummaryTextDone {
        item_id: String,
        output_index: u32,
        sequence_number: u64,
        summary_index: u32,
        text: String,
    },
    /// Emitted when there is a partial audio response. `delta` is base64-encoded audio bytes.
    #[serde(rename = "response.audio.delta")]
    AudioDelta { delta: String, sequence_number: u64 },
    /// Emitted when the audio response is complete.
    #[serde(rename = "response.audio.done")]
    AudioDone { sequence_number: u64 },
    /// Emitted when there is a partial transcript of audio.
    #[serde(rename = "response.audio.transcript.delta")]
    AudioTranscriptDelta { delta: String, sequence_number: u64 },
    /// Emitted when the full audio transcript is completed.
    #[serde(rename = "response.audio.transcript.done")]
    AudioTranscriptDone { sequence_number: u64 },
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
