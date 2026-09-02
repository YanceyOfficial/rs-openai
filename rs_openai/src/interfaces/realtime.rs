//! Realtime API (Phase A: core text-only voice session; Phase B: audio input/output; Phase C:
//! function calling + MCP tools in-session; Phase D: SIP call management; Phase E: ephemeral
//! client secrets; Phase F: transcription session type; Phase G: auto-reconnect) -- see
//! docs/superpowers/specs/2026-09-01-realtime-phase-a-design.md,
//! docs/superpowers/specs/2026-09-02-realtime-phase-b-audio-design.md,
//! docs/superpowers/specs/2026-09-02-realtime-phase-c-tools-design.md,
//! docs/superpowers/specs/2026-09-02-realtime-phase-d-sip-calls-design.md,
//! docs/superpowers/specs/2026-09-02-realtime-phase-e-client-secrets-design.md,
//! docs/superpowers/specs/2026-09-02-realtime-phase-f-transcription-design.md, and
//! docs/superpowers/specs/2026-09-02-realtime-phase-g-auto-reconnect-design.md. The WebSocket
//! session (Phases A-C, F, G) is a persistent, stateful, full-duplex connection, unlike every
//! other module in this crate (all plain HTTP request/response, even SSE streaming) -- covers
//! connection setup, session config (realtime and transcription session types), conversation
//! items, response lifecycle, audio input/output (including VAD turn detection),
//! function-calling/MCP tool configuration + tool-call items/events, input-audio transcription
//! events, and opt-in transparent reconnection. Phase D's `calls` resource and Phase E's
//! `client_secrets` resource, by contrast, are plain REST (SIP call accept/hangup/refer/reject
//! plus WebRTC call bootstrapping; and minting `ek_...` tokens for browser/mobile/WebRTC
//! clients) and fit this crate's ordinary `interfaces`/`apis` + `wiremock` pattern directly.
//! `output_audio_buffer.*` events are WebRTC/SIP-only (confirmed via source) and are
//! permanently out of scope for the WebSocket session, not merely deferred.

use bon::Builder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Builder, Debug, Clone, Default)]
pub struct RealtimeConnectRequest {
    pub model: Option<String>,

    /// Opt-in automatic reconnection. When set, a connection that drops with a recoverable
    /// WebSocket close code is transparently re-established with exponential backoff, and the
    /// `recv()` loop continues -- the server's fresh `session.created` event is the caller's
    /// cue to re-apply any `session.update`. When `None` (the default), a dropped connection
    /// ends the stream.
    pub reconnect: Option<RealtimeReconnectConfig>,
}

/// Backoff policy for [`RealtimeConnectRequest::reconnect`]. Defaults mirror `openai-python`:
/// 5 attempts, `500ms` initial delay, `8000ms` cap. Actual delays are
/// `min(initial_delay_ms * 2^(attempt-1), max_delay_ms)` with a 0.75..1.0 jitter factor.
#[derive(Builder, Debug, Clone)]
pub struct RealtimeReconnectConfig {
    #[builder(default = 5)]
    pub max_retries: u32,

    #[builder(default = 500)]
    pub initial_delay_ms: u64,

    #[builder(default = 8000)]
    pub max_delay_ms: u64,
}

impl Default for RealtimeReconnectConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

/// A single audio format -- `type`-tagged 3-variant union. Explicit per-variant
/// `#[serde(rename)]` since wire tags (`audio/pcm`, etc.) contain `/` and aren't expressible
/// via `rename_all`.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum RealtimeAudioFormat {
    #[serde(rename = "audio/pcm")]
    AudioPcm {
        #[serde(skip_serializing_if = "Option::is_none")]
        rate: Option<u32>,
    },
    #[serde(rename = "audio/pcmu")]
    AudioPcmu,
    #[serde(rename = "audio/pcma")]
    AudioPcma,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct RealtimeNoiseReduction {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

/// Voice activity detection config -- `type`-tagged 2-variant union. Modeled fully (not
/// trimmed): VAD is the core mechanism that makes a voice conversation usable turn-by-turn;
/// without it the client must manually commit every input-audio-buffer chunk.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RealtimeTurnDetection {
    ServerVad {
        #[serde(skip_serializing_if = "Option::is_none")]
        create_response: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        idle_timeout_ms: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        interrupt_response: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        prefix_padding_ms: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        silence_duration_ms: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        threshold: Option<f32>,
    },
    SemanticVad {
        #[serde(skip_serializing_if = "Option::is_none")]
        create_response: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        eagerness: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        interrupt_response: Option<bool>,
    },
}

/// One shared type reused across every place the real API defines near-identical
/// output-audio shapes (session-level `audio.output`, per-response `audio.output`
/// override, and `RealtimeResponse.audio.output`) -- confirmed via source the only
/// difference is whether `speed` is present, harmless as an always-optional field left
/// unset where the real type doesn't have it. `voice` (`string | one-of-10-named-values |
/// {id}`) kept as raw JSON -- an awkward small union not worth a dedicated type, matching
/// `max_output_tokens`'s precedent.
#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct RealtimeAudioOutputConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<RealtimeAudioFormat>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<serde_json::Value>,
}

/// The `{ output: ... }` nesting the real API uses for `response.create`'s per-call `audio`
/// override and for `RealtimeResponse.audio` -- distinct from the session-level
/// `RealtimeAudioConfig`, which has both `input` and `output`.
#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct RealtimeAudioOutputConfigWrapper {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<RealtimeAudioOutputConfig>,
}

/// Config for input-audio transcription -- asynchronous speech-to-text run on the input
/// audio buffer (through the `/audio/transcriptions` model), separate from the Realtime
/// model's own native audio understanding. The request and response shapes are confirmed
/// field-for-field identical, so this one type serves both. `delay`
/// (`"minimal"|"low"|"medium"|"high"|"xhigh"`) and `model` (8 named values or a free
/// string) kept as `Option<String>`.
#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct RealtimeAudioTranscription {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay: Option<String>,

    /// Words or phrases to bias transcription toward. Supported by `gpt-transcribe` /
    /// `gpt-live-transcribe`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,

    /// The input-audio language, ISO-639-1 (e.g. `en`). Improves accuracy and latency.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    /// Candidate input-audio languages, ISO-639-1. Supported by `gpt-transcribe` /
    /// `gpt-live-transcribe`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub languages: Option<Vec<String>>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct RealtimeAudioConfigInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<RealtimeAudioFormat>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub noise_reduction: Option<RealtimeNoiseReduction>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcription: Option<RealtimeAudioTranscription>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_detection: Option<RealtimeTurnDetection>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct RealtimeAudioConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<RealtimeAudioConfigInput>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<RealtimeAudioOutputConfig>,
}

/// A function tool definition. A NEW dedicated type, NOT a reuse of
/// `crate::interfaces::responses::ResponseFunctionTool` -- confirmed via source these
/// genuinely differ: Responses' version requires `name`/`parameters` and has a `strict`
/// field Realtime's version lacks entirely; Realtime's version has all three fields
/// optional and no `strict`.
#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct RealtimeFunctionTool {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
}

/// A tool available to the model -- `type`-tagged 2-variant union. `Mcp` reuses
/// `crate::interfaces::responses::ResponseMcpTool` directly: confirmed field-by-field
/// identical to both the session-level and response-level MCP tool config the real API
/// defines (`server_label`, `allowed_callers`, `allowed_tools`, `authorization`,
/// `connector_id`, `defer_loading`, `headers`, `require_approval`, `server_description`,
/// `server_url`, `tunnel_id`), and already derives both `Serialize`/`Deserialize` with no
/// reachability gaps. Used for both `RealtimeSessionConfig.tools` and
/// `RealtimeResponseCreateParams.tools` -- confirmed via source both share this identical
/// two-variant shape.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RealtimeTool {
    Function(RealtimeFunctionTool),
    Mcp(Box<crate::interfaces::responses::ResponseMcpTool>),
}

/// How the model chooses tools -- an `untagged` 3-variant union. Unlike this crate's prior
/// untagged unions (safe because variants were JSON-*type*-disjoint, e.g. bare string vs.
/// object), `Mcp` and `Function` are BOTH objects with overlapping optional fields, and
/// serde's derived struct-variant deserialization does not reject unknown fields by
/// default. `Mcp` is declared BEFORE `Function` deliberately: `Mcp` requires `server_label`
/// (absent from any real `Function` payload), so it correctly fails-and-falls-through for
/// function payloads, while still matching every valid MCP payload including ones that set
/// the optional `name` -- if `Function` were tried first, an MCP choice with `name` set
/// would incorrectly deserialize as `Function`, silently dropping `server_label`. Covered by
/// a dedicated round-trip test.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum RealtimeToolChoice {
    Mode(crate::interfaces::responses::ResponseToolChoiceMode),
    Mcp {
        #[serde(rename = "type")]
        kind: String,
        server_label: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    Function {
        #[serde(rename = "type")]
        kind: String,
        name: String,
    },
}

/// Deliberately trimmed from the real `RealtimeSessionCreateRequest`'s ~12 fields --
/// `tracing`/`reasoning`/`prompt`/`truncation` deferred to a future phase.
///
/// Serves **both** session types. A realtime session is `r#type = "realtime"` with
/// `model`/`instructions`/`tools`/etc.; a transcription session (Phase F) is
/// `r#type = "transcription"` with just `audio.input.transcription` + `include` set and the
/// conversational fields left unset -- the real API models these as a union, but this crate
/// treats `RealtimeSessionConfig` as a bag of optional fields keyed by the `r#type` string,
/// matching how request-only and response-only fields are already conflated here.
///
/// Also used, unchanged, for the `session` field of `session.created` / `session.updated`
/// and of the client-secret response (`RealtimeClientSecret`), all of which are really the
/// richer `*SessionCreateResponse` on the wire -- `id` / `object` / `expires_at` below are
/// the response-only fields that let it round-trip losslessly.
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct RealtimeSessionConfig {
    #[builder(into)]
    pub r#type: String,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Response-only: the session id, looks like `sess_...`. Omitted when sent as a request.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Response-only: `"realtime.session"` for realtime sessions.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,

    /// Response-only: session expiration timestamp, in seconds since epoch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_modalities: Option<Vec<String>>,

    /// The real API allows an integer or the literal string `"inf"` -- modeled as raw JSON,
    /// an awkward small union not worth a dedicated type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<RealtimeAudioConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<RealtimeTool>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<RealtimeToolChoice>,

    /// Extra fields to include in server outputs. The only documented value is
    /// `"item.input_audio_transcription.logprobs"`. Valid on both session types.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<String>>,
}

/// A single content part of a conversation message. One flat, all-optional struct --
/// matches the real API's actual per-role `Content` shape exactly (confirmed NOT a tagged
/// enum in the wire format: each field is populated or omitted depending on `type`, e.g.
/// `text` for `input_text`/`output_text`, `audio`+`transcript` for `input_audio`, `image_url`
/// +`detail` for `input_image`). Also reused as-is for response content-part events' `part`
/// field (confirmed structurally compatible, a strict subset).
#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ConversationContentPart {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// A message item within a Realtime conversation, tagged by `role`. Nested inside
/// `ConversationItem::Message` -- see that type's doc comment for why this two-level
/// tagging is necessary.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum ConversationMessageItem {
    System {
        content: Vec<ConversationContentPart>,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        object: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<String>,
    },
    User {
        content: Vec<ConversationContentPart>,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        object: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<String>,
    },
    Assistant {
        content: Vec<ConversationContentPart>,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        object: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<String>,
    },
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct RealtimeConversationItemFunctionCall {
    #[builder(into)]
    pub arguments: String,

    #[builder(into)]
    pub name: String,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_id: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct RealtimeConversationItemFunctionCallOutput {
    #[builder(into)]
    pub call_id: String,

    #[builder(into)]
    pub output: String,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct RealtimeMcpApprovalResponse {
    #[builder(into)]
    pub id: String,

    #[builder(into)]
    pub approval_request_id: String,

    pub approve: bool,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// A tool available on an MCP server, as listed by `mcp_list_tools`. `input_schema`/
/// `annotations` kept as raw JSON -- arbitrary nested JSON Schema / free-form object, not
/// worth a dedicated type.
#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct RealtimeMcpServerTool {
    pub input_schema: serde_json::Value,

    #[builder(into)]
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<serde_json::Value>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct RealtimeMcpListTools {
    #[builder(into)]
    pub server_label: String,

    pub tools: Vec<RealtimeMcpServerTool>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

/// The error from an MCP tool call, if any -- `type`-tagged 3-variant union. Modeled fully
/// (not deferred to raw JSON) since it's small and the whole reason `error` exists on
/// `RealtimeMcpToolCall`.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RealtimeMcpToolCallError {
    ProtocolError { code: i64, message: String },
    ToolExecutionError { message: String },
    HttpError { code: i64, message: String },
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct RealtimeMcpToolCall {
    #[builder(into)]
    pub id: String,

    #[builder(into)]
    pub arguments: String,

    #[builder(into)]
    pub name: String,

    #[builder(into)]
    pub server_label: String,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_request_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RealtimeMcpToolCallError>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct RealtimeMcpApprovalRequest {
    #[builder(into)]
    pub id: String,

    #[builder(into)]
    pub arguments: String,

    #[builder(into)]
    pub name: String,

    #[builder(into)]
    pub server_label: String,
}

/// A single item within a Realtime conversation. A genuinely new pattern for this crate:
/// a **two-level tagged union**. The real API's system/user/assistant message variants all
/// share `type: "message"` and are only distinguished by `role` -- confirmed field-by-field
/// via source. Modeled as an outer `type`-tagged enum wrapping `ConversationMessageItem` so
/// serde checks `type` first, then delegates the same JSON object to
/// `ConversationMessageItem`'s own `role` tag. Every other variant is a flat, single-level
/// tag -- this is the complete, confirmed set of 7 top-level item types (`message` +
/// `function_call` + `function_call_output` + 4 MCP item types).
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConversationItem {
    Message(ConversationMessageItem),
    FunctionCall(RealtimeConversationItemFunctionCall),
    FunctionCallOutput(RealtimeConversationItemFunctionCallOutput),
    McpApprovalResponse(RealtimeMcpApprovalResponse),
    McpListTools(RealtimeMcpListTools),
    #[serde(rename = "mcp_call")]
    McpToolCall(RealtimeMcpToolCall),
    McpApprovalRequest(RealtimeMcpApprovalRequest),
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct RealtimeResponseCreateParams {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_modalities: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<RealtimeAudioOutputConfigWrapper>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<RealtimeTool>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<RealtimeToolChoice>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct RealtimeResponse {
    pub id: Option<String>,
    pub audio: Option<RealtimeAudioOutputConfigWrapper>,
    pub conversation_id: Option<String>,
    pub max_output_tokens: Option<serde_json::Value>,
    pub metadata: Option<HashMap<String, String>>,
    pub object: Option<String>,
    pub output: Option<Vec<ConversationItem>>,
    pub output_modalities: Option<Vec<String>>,
    pub status: Option<String>,
    /// Raw JSON -- deferred detailed modeling, matching this crate's established
    /// raw-JSON-escape-hatch convention for detail not core to Phase A.
    pub status_details: Option<serde_json::Value>,
    pub usage: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct RealtimeError {
    pub message: String,
    pub r#type: String,
    pub code: Option<String>,
    pub event_id: Option<String>,
    pub param: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct RealtimeRateLimit {
    pub limit: Option<i64>,
    pub name: Option<String>,
    pub remaining: Option<i64>,
    pub reset_seconds: Option<f64>,
}

/// Every client-to-server event this phase supports. Confirmed via source: the real
/// `RealtimeClientEvent` union has 11 variants total; `conversation.item.truncate`
/// (audio-interruption-specific) is deferred to a future phase, and `output_audio_buffer.
/// clear` is permanently out of scope (WebRTC/SIP only -- see this module's doc comment).
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type")]
pub enum RealtimeClientEvent {
    #[serde(rename = "session.update")]
    SessionUpdate {
        #[serde(skip_serializing_if = "Option::is_none")]
        event_id: Option<String>,
        /// Boxed: `RealtimeSessionConfig` is by far the largest event payload, and boxing it
        /// keeps `RealtimeClientEvent`'s variants size-balanced (same reason
        /// `RealtimeTool::Mcp` is boxed).
        session: Box<RealtimeSessionConfig>,
    },
    #[serde(rename = "conversation.item.create")]
    ConversationItemCreate {
        #[serde(skip_serializing_if = "Option::is_none")]
        event_id: Option<String>,
        item: ConversationItem,
        #[serde(skip_serializing_if = "Option::is_none")]
        previous_item_id: Option<String>,
    },
    #[serde(rename = "conversation.item.delete")]
    ConversationItemDelete {
        #[serde(skip_serializing_if = "Option::is_none")]
        event_id: Option<String>,
        item_id: String,
    },
    #[serde(rename = "conversation.item.retrieve")]
    ConversationItemRetrieve {
        #[serde(skip_serializing_if = "Option::is_none")]
        event_id: Option<String>,
        item_id: String,
    },
    #[serde(rename = "response.create")]
    ResponseCreate {
        #[serde(skip_serializing_if = "Option::is_none")]
        event_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        response: Option<RealtimeResponseCreateParams>,
    },
    #[serde(rename = "response.cancel")]
    ResponseCancel {
        #[serde(skip_serializing_if = "Option::is_none")]
        event_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        response_id: Option<String>,
    },
    /// `audio` is base64-encoded bytes in the format configured by
    /// `session.audio.input.format`.
    #[serde(rename = "input_audio_buffer.append")]
    InputAudioBufferAppend {
        #[serde(skip_serializing_if = "Option::is_none")]
        event_id: Option<String>,
        audio: String,
    },
    #[serde(rename = "input_audio_buffer.commit")]
    InputAudioBufferCommit {
        #[serde(skip_serializing_if = "Option::is_none")]
        event_id: Option<String>,
    },
    #[serde(rename = "input_audio_buffer.clear")]
    InputAudioBufferClear {
        #[serde(skip_serializing_if = "Option::is_none")]
        event_id: Option<String>,
    },
}

/// Every server-to-client event this phase supports. Confirmed via source: the real
/// `RealtimeServerEvent` union has 41 variants total. `output_audio_buffer.*` events are
/// permanently out of scope (WebRTC/SIP only). Still deferred to a future phase:
/// `conversation.item.truncated`, DTMF, input-audio-transcription events.
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(tag = "type")]
pub enum RealtimeServerEvent {
    #[serde(rename = "session.created")]
    SessionCreated {
        event_id: String,
        /// Boxed for the same size-balancing reason as `RealtimeClientEvent::SessionUpdate`.
        session: Box<RealtimeSessionConfig>,
    },
    #[serde(rename = "session.updated")]
    SessionUpdated {
        event_id: String,
        session: Box<RealtimeSessionConfig>,
    },
    #[serde(rename = "conversation.created")]
    ConversationCreated {
        event_id: String,
        conversation: RealtimeConversation,
    },
    #[serde(rename = "conversation.item.created")]
    ConversationItemCreated {
        event_id: String,
        item: ConversationItem,
        #[serde(skip_serializing_if = "Option::is_none")]
        previous_item_id: Option<String>,
    },
    #[serde(rename = "conversation.item.added")]
    ConversationItemAdded {
        event_id: String,
        item: ConversationItem,
        #[serde(skip_serializing_if = "Option::is_none")]
        previous_item_id: Option<String>,
    },
    #[serde(rename = "conversation.item.done")]
    ConversationItemDone {
        event_id: String,
        item: ConversationItem,
        #[serde(skip_serializing_if = "Option::is_none")]
        previous_item_id: Option<String>,
    },
    #[serde(rename = "conversation.item.deleted")]
    ConversationItemDeleted { event_id: String, item_id: String },
    #[serde(rename = "conversation.item.retrieved")]
    ConversationItemRetrieved {
        event_id: String,
        item: ConversationItem,
    },
    #[serde(rename = "response.created")]
    ResponseCreated {
        event_id: String,
        response: RealtimeResponse,
    },
    #[serde(rename = "response.done")]
    ResponseDone {
        event_id: String,
        response: RealtimeResponse,
    },
    #[serde(rename = "response.output_item.added")]
    ResponseOutputItemAdded {
        event_id: String,
        item: ConversationItem,
        output_index: u32,
        response_id: String,
    },
    #[serde(rename = "response.output_item.done")]
    ResponseOutputItemDone {
        event_id: String,
        item: ConversationItem,
        output_index: u32,
        response_id: String,
    },
    #[serde(rename = "response.content_part.added")]
    ResponseContentPartAdded {
        content_index: u32,
        event_id: String,
        item_id: String,
        output_index: u32,
        part: ConversationContentPart,
        response_id: String,
    },
    #[serde(rename = "response.content_part.done")]
    ResponseContentPartDone {
        content_index: u32,
        event_id: String,
        item_id: String,
        output_index: u32,
        part: ConversationContentPart,
        response_id: String,
    },
    #[serde(rename = "response.output_text.delta")]
    ResponseTextDelta {
        content_index: u32,
        delta: String,
        event_id: String,
        item_id: String,
        output_index: u32,
        response_id: String,
    },
    #[serde(rename = "response.output_text.done")]
    ResponseTextDone {
        content_index: u32,
        event_id: String,
        item_id: String,
        output_index: u32,
        response_id: String,
        text: String,
    },
    #[serde(rename = "error")]
    Error {
        event_id: String,
        error: RealtimeError,
    },
    #[serde(rename = "rate_limits.updated")]
    RateLimitsUpdated {
        event_id: String,
        rate_limits: Vec<RealtimeRateLimit>,
    },
    #[serde(rename = "input_audio_buffer.committed")]
    InputAudioBufferCommitted {
        event_id: String,
        item_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        previous_item_id: Option<String>,
    },
    #[serde(rename = "input_audio_buffer.cleared")]
    InputAudioBufferCleared { event_id: String },
    #[serde(rename = "input_audio_buffer.speech_started")]
    InputAudioBufferSpeechStarted {
        audio_start_ms: i64,
        event_id: String,
        item_id: String,
    },
    #[serde(rename = "input_audio_buffer.speech_stopped")]
    InputAudioBufferSpeechStopped {
        audio_end_ms: i64,
        event_id: String,
        item_id: String,
    },
    #[serde(rename = "input_audio_buffer.timeout_triggered")]
    InputAudioBufferTimeoutTriggered {
        audio_end_ms: i64,
        audio_start_ms: i64,
        event_id: String,
        item_id: String,
    },
    #[serde(rename = "response.output_audio.delta")]
    ResponseAudioDelta {
        content_index: u32,
        delta: String,
        event_id: String,
        item_id: String,
        output_index: u32,
        response_id: String,
    },
    #[serde(rename = "response.output_audio.done")]
    ResponseAudioDone {
        content_index: u32,
        event_id: String,
        item_id: String,
        output_index: u32,
        response_id: String,
    },
    #[serde(rename = "response.output_audio_transcript.delta")]
    ResponseAudioTranscriptDelta {
        content_index: u32,
        delta: String,
        event_id: String,
        item_id: String,
        output_index: u32,
        response_id: String,
    },
    #[serde(rename = "response.output_audio_transcript.done")]
    ResponseAudioTranscriptDone {
        content_index: u32,
        event_id: String,
        item_id: String,
        output_index: u32,
        response_id: String,
        transcript: String,
    },
    #[serde(rename = "response.function_call_arguments.delta")]
    ResponseFunctionCallArgumentsDelta {
        call_id: String,
        delta: String,
        event_id: String,
        item_id: String,
        output_index: u32,
        response_id: String,
    },
    #[serde(rename = "response.function_call_arguments.done")]
    ResponseFunctionCallArgumentsDone {
        arguments: String,
        call_id: String,
        event_id: String,
        item_id: String,
        name: String,
        output_index: u32,
        response_id: String,
    },
    #[serde(rename = "response.mcp_call_arguments.delta")]
    ResponseMcpCallArgumentsDelta {
        delta: String,
        event_id: String,
        item_id: String,
        output_index: u32,
        response_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        obfuscation: Option<String>,
    },
    #[serde(rename = "response.mcp_call_arguments.done")]
    ResponseMcpCallArgumentsDone {
        arguments: String,
        event_id: String,
        item_id: String,
        output_index: u32,
        response_id: String,
    },
    #[serde(rename = "response.mcp_call.in_progress")]
    ResponseMcpCallInProgress {
        event_id: String,
        item_id: String,
        output_index: u32,
    },
    #[serde(rename = "response.mcp_call.completed")]
    ResponseMcpCallCompleted {
        event_id: String,
        item_id: String,
        output_index: u32,
    },
    #[serde(rename = "response.mcp_call.failed")]
    ResponseMcpCallFailed {
        event_id: String,
        item_id: String,
        output_index: u32,
    },
    #[serde(rename = "mcp_list_tools.in_progress")]
    McpListToolsInProgress { event_id: String, item_id: String },
    #[serde(rename = "mcp_list_tools.completed")]
    McpListToolsCompleted { event_id: String, item_id: String },
    #[serde(rename = "mcp_list_tools.failed")]
    McpListToolsFailed { event_id: String, item_id: String },

    /// Input-audio transcription finished for a user-audio item. Runs asynchronously with
    /// response generation, so it may arrive before or after the response events.
    #[serde(rename = "conversation.item.input_audio_transcription.completed")]
    ConversationItemInputAudioTranscriptionCompleted {
        content_index: u32,
        event_id: String,
        item_id: String,
        transcript: String,
        /// Billed per the ASR model's pricing. A token-billed vs. duration-billed union --
        /// kept as raw JSON, matching `RealtimeResponse.usage`.
        usage: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        languages: Option<Vec<RealtimeTranscriptionLanguage>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        logprobs: Option<Vec<RealtimeLogProb>>,
    },
    #[serde(rename = "conversation.item.input_audio_transcription.delta")]
    ConversationItemInputAudioTranscriptionDelta {
        event_id: String,
        item_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        content_index: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        delta: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        logprobs: Option<Vec<RealtimeLogProb>>,
    },
    /// Transcription was configured but failed for a user message -- kept separate from
    /// `error` events so the client can tie it to the item.
    #[serde(rename = "conversation.item.input_audio_transcription.failed")]
    ConversationItemInputAudioTranscriptionFailed {
        content_index: u32,
        event_id: String,
        item_id: String,
        error: RealtimeTranscriptionError,
    },
    /// A transcription segment (with speaker label and timing) was identified for an item.
    #[serde(rename = "conversation.item.input_audio_transcription.segment")]
    ConversationItemInputAudioTranscriptionSegment {
        id: String,
        content_index: u32,
        end: f64,
        event_id: String,
        item_id: String,
        speaker: String,
        start: f64,
        text: String,
    },
}

/// A language detected in transcribed audio. Confirmed via source: an object with a single
/// `code` field, not a bare string (unlike `RealtimeAudioTranscription.languages`).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RealtimeTranscriptionLanguage {
    pub code: String,
}

/// A single log-probability entry. Enabled by setting the session's
/// `include: ["item.input_audio_transcription.logprobs"]`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RealtimeLogProb {
    pub token: String,
    pub bytes: Vec<u8>,
    pub logprob: f64,
}

/// Details of a failed input-audio transcription (all fields optional, per source).
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RealtimeTranscriptionError {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct RealtimeConversation {
    pub id: Option<String>,
    pub object: Option<String>,
}

/// Request body for `POST /realtime/calls` (WebRTC call bootstrapping -- the app relays an
/// SDP offer/answer exchange since browsers can't set an `Authorization` header directly
/// against OpenAI). NOT `Serialize`/`Deserialize` -- consumed directly by a dedicated
/// multipart-form builder, matching `RealtimeConnectRequest`'s precedent of a plain
/// `Builder`-only type for a non-JSON transport.
#[derive(Builder, Debug, Clone)]
pub struct CreateCallRequest {
    #[builder(into)]
    pub sdp: String,
    pub session: Option<RealtimeSessionConfig>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ReferCallRequest {
    #[builder(into)]
    pub target_uri: String,
}

/// Confirmed via source: defaults server-side to `603` (Decline) when `status_code` is
/// omitted.
#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct RejectCallRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RealtimeCallSipHeader {
    pub name: String,
    pub value: String,
}

/// The typed shape of `crate::interfaces::webhooks::WebhookEvent.data` when
/// `WebhookEvent.kind == "realtime.call.incoming"` -- the only way to learn a `call_id`
/// exists for an inbound SIP call, since `Calls` has no `list`/`retrieve`. `WebhookEvent`
/// itself needs no changes (already a fully generic envelope); deserialize its `data` field
/// into this type once `kind` is confirmed.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RealtimeCallIncomingEventData {
    pub call_id: String,
    pub sip_headers: Vec<RealtimeCallSipHeader>,
}

/// Configuration for a client secret's expiration -- the point after which the secret can
/// no longer create new sessions (a session already started may continue past it). Both
/// fields optional server-side.
#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ClientSecretExpiresAfter {
    /// Only `"created_at"` is currently supported; `seconds` is added to the secret's
    /// creation time to produce the expiration. `Option<String>` (not an enum) to stay
    /// forward-compatible.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<String>,

    /// Seconds from the anchor to expiration. Server default 600 (10 minutes); valid range
    /// 10..7200 (2 hours). Not range-checked client-side.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seconds: Option<i64>,
}

/// Request body for `POST /realtime/client_secrets`. Both params optional -- an empty
/// request serializes to `{}` and lets the server apply every default.
#[derive(Builder, Debug, Serialize, Clone, Default)]
pub struct CreateClientSecretRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_after: Option<ClientSecretExpiresAfter>,

    /// Session configuration applied to sessions started with the minted secret (the client
    /// connection can still override it). Phase E models the `realtime` session variant;
    /// Phase F adds the `transcription` variant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<RealtimeSessionConfig>,
}

/// Response from `POST /realtime/client_secrets` -- the minted ephemeral token plus the
/// effective session configuration.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RealtimeClientSecret {
    /// The generated client secret, a string that looks like `ek_...`. Pass it to a browser
    /// / mobile / WebRTC client in place of the main API key.
    pub value: String,

    /// Expiration timestamp for the client secret, in seconds since epoch.
    pub expires_at: i64,

    pub session: RealtimeSessionConfig,
}
