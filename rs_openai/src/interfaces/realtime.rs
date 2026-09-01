//! Realtime API (Phase A: core text-only voice session) -- see
//! docs/superpowers/specs/2026-09-01-realtime-phase-a-design.md. A persistent, stateful,
//! full-duplex WebSocket connection, unlike every other module in this crate (all plain
//! HTTP request/response, even SSE streaming). Covers connection setup, session config,
//! conversation items, and response lifecycle for text-only sessions. Audio, function
//! calling, MCP tools, SIP calls, ephemeral tokens, transcription/translation session
//! types, and auto-reconnect are deliberately out of scope for this phase.

use bon::Builder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Builder, Debug, Clone, Default)]
pub struct RealtimeConnectRequest {
    pub model: Option<String>,
}

/// Deliberately trimmed from the real `RealtimeSessionCreateRequest`'s ~12 fields --
/// `audio`/`tools`/`tool_choice`/`tracing`/`reasoning`/`prompt`/`truncation`/`include`
/// deferred to their respective later phases.
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct RealtimeSessionConfig {
    #[builder(into)]
    pub r#type: String,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_modalities: Option<Vec<String>>,

    /// The real API allows an integer or the literal string `"inf"` -- modeled as raw JSON,
    /// an awkward small union not worth a dedicated type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<serde_json::Value>,
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

/// A single item within a Realtime conversation. A genuinely new pattern for this crate:
/// a **two-level tagged union**. The real API's system/user/assistant message variants all
/// share `type: "message"` and are only distinguished by `role` -- confirmed field-by-field
/// via source. Modeled as an outer `type`-tagged enum (one Phase-A variant, `Message`,
/// wrapping `ConversationMessageItem`) so serde checks `type` first, then delegates the same
/// JSON object to `ConversationMessageItem`'s own `role` tag. Deferred variants
/// (function_call, function_call_output, the 4 MCP item types) belong to Phase C.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConversationItem {
    Message(ConversationMessageItem),
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
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct RealtimeResponse {
    pub id: Option<String>,
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
/// `RealtimeClientEvent` union has 11 variants total; 5 (session.update's audio/tools
/// fields aside, `conversation.item.truncate`, `input_audio_buffer.*`,
/// `output_audio_buffer.clear`) are deferred to Phase B/C.
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type")]
pub enum RealtimeClientEvent {
    #[serde(rename = "session.update")]
    SessionUpdate {
        #[serde(skip_serializing_if = "Option::is_none")]
        event_id: Option<String>,
        session: RealtimeSessionConfig,
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
}

/// Every server-to-client event this phase supports. Confirmed via source: the real
/// `RealtimeServerEvent` union has 41 variants total; ~23 (all `input_audio_buffer.*`/
/// `output_audio_buffer.*` events, `conversation.item.truncated`, audio/audio_transcript
/// delta+done, function_call_arguments delta+done, all `mcp_list_tools.*`/
/// `response.mcp_call.*` events, DTMF, input-audio-transcription events) are deferred to
/// Phase B/C.
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(tag = "type")]
pub enum RealtimeServerEvent {
    #[serde(rename = "session.created")]
    SessionCreated {
        event_id: String,
        session: RealtimeSessionConfig,
    },
    #[serde(rename = "session.updated")]
    SessionUpdated {
        event_id: String,
        session: RealtimeSessionConfig,
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
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct RealtimeConversation {
    pub id: Option<String>,
    pub object: Option<String>,
}
