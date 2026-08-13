//! ChatKit (Beta) sessions and threads — see
//! docs/superpowers/specs/2026-08-14-chatkit-design.md.

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
