//! ChatKit (Beta) sessions and threads — see
//! docs/superpowers/specs/2026-08-14-chatkit-design.md. Thread Items types are
//! appended below — see
//! docs/superpowers/specs/2026-08-20-chatkit-thread-items-design.md.

use crate::interfaces::chat::SortOrder;
use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ChatKitWorkflowTracing {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ChatKitWorkflowParam {
    #[builder(into)]
    pub id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_variables: Option<std::collections::HashMap<String, serde_json::Value>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracing: Option<ChatKitWorkflowTracing>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ChatKitAutomaticThreadTitling {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ChatKitFileUploadConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_file_size: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_files: Option<u32>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ChatKitHistoryConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_threads: Option<u32>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ChatKitConfiguration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automatic_thread_titling: Option<ChatKitAutomaticThreadTitling>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_upload: Option<ChatKitFileUploadConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<ChatKitHistoryConfig>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ChatKitRateLimits {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_requests_per_1_minute: Option<u32>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct CreateChatKitSessionRequest {
    pub workflow: ChatKitWorkflowParam,

    #[builder(into)]
    pub user: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub chatkit_configuration: Option<ChatKitConfiguration>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_after: Option<crate::shared::types::ExpiresAfter>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limits: Option<ChatKitRateLimits>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ChatKitSession {
    pub id: String,
    pub object: String,
    pub client_secret: String,
    pub expires_at: i64,
    pub status: String,
    pub user: String,
    pub workflow: ChatKitWorkflowParam,
    pub chatkit_configuration: ChatKitConfiguration,
    pub rate_limits: ChatKitRateLimits,
    pub max_requests_per_1_minute: u32,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ListChatKitThreadsRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// Confirmed via source: a `type`-discriminated union, not a plain string. A real
/// `locked`/`closed` thread response could never deserialize against a bare `String`.
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ChatKitThreadStatus {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "locked")]
    Locked { reason: Option<String> },
    #[serde(rename = "closed")]
    Closed { reason: Option<String> },
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ChatKitThread {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub status: ChatKitThreadStatus,
    pub title: Option<String>,
    pub user: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListChatKitThreadsResponse {
    pub object: String,
    pub data: Vec<ChatKitThread>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteChatKitThreadResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}

// Thread Items sub-resource.

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ChatKitAttachment {
    pub id: String,
    pub mime_type: String,
    pub name: String,
    pub preview_url: Option<String>,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ChatKitUserMessageContent {
    #[serde(rename = "input_text")]
    InputText { text: String },
    #[serde(rename = "quoted_text")]
    QuotedText { text: String },
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ChatKitToolChoice {
    pub id: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ChatKitInferenceOptions {
    pub model: Option<String>,
    pub tool_choice: Option<ChatKitToolChoice>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ChatKitAnnotation {
    #[serde(rename = "file")]
    File { source: ChatKitAnnotationFileSource },
    #[serde(rename = "url")]
    Url { url: String },
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ChatKitAnnotationFileSource {
    pub filename: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ChatKitResponseOutputText {
    pub annotations: Vec<ChatKitAnnotation>,
    pub text: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ChatKitTaskGroupTask {
    pub heading: Option<String>,
    pub summary: Option<String>,
    pub task_type: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ChatKitThreadItem {
    #[serde(rename = "chatkit.user_message")]
    UserMessage {
        id: String,
        attachments: Vec<ChatKitAttachment>,
        content: Vec<ChatKitUserMessageContent>,
        created_at: i64,
        inference_options: Option<ChatKitInferenceOptions>,
        thread_id: String,
    },
    #[serde(rename = "chatkit.assistant_message")]
    AssistantMessage {
        id: String,
        content: Vec<ChatKitResponseOutputText>,
        created_at: i64,
        thread_id: String,
    },
    #[serde(rename = "chatkit.widget")]
    Widget {
        id: String,
        created_at: i64,
        thread_id: String,
        widget: String,
    },
    #[serde(rename = "chatkit.client_tool_call")]
    ClientToolCall {
        id: String,
        arguments: String,
        call_id: String,
        created_at: i64,
        name: String,
        output: Option<String>,
        status: String,
        thread_id: String,
    },
    #[serde(rename = "chatkit.task")]
    Task {
        id: String,
        created_at: i64,
        heading: Option<String>,
        summary: Option<String>,
        task_type: String,
        thread_id: String,
    },
    #[serde(rename = "chatkit.task_group")]
    TaskGroup {
        id: String,
        created_at: i64,
        tasks: Vec<ChatKitTaskGroupTask>,
        thread_id: String,
    },
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListChatKitThreadItemsRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListChatKitThreadItemsResponse {
    pub object: String,
    pub data: Vec<ChatKitThreadItem>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}
