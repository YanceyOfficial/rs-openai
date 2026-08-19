//! Organization audit logs -- see
//! docs/superpowers/specs/2026-08-19-org-admin-api-keys-audit-logs-design.md.

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ListAuditLogsRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AuditLogActorApiKeyServiceAccount {
    pub id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AuditLogActorUser {
    pub id: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AuditLogActorApiKey {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub service_account: Option<AuditLogActorApiKeyServiceAccount>,
    pub user: Option<AuditLogActorUser>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AuditLogActorSession {
    pub ip_address: Option<String>,
    pub user: Option<AuditLogActorUser>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AuditLogActor {
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub api_key: Option<AuditLogActorApiKey>,
    pub session: Option<AuditLogActorSession>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AuditLogProject {
    pub id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AuditLogEvent {
    pub id: String,
    pub effective_at: i64,
    #[serde(rename = "type")]
    pub kind: String,
    pub actor: Option<AuditLogActor>,
    pub project: Option<AuditLogProject>,
    #[serde(flatten)]
    pub detail: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListAuditLogsResponse {
    pub object: String,
    pub data: Vec<AuditLogEvent>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}
