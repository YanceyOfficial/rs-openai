//! Organization admin API keys -- see
//! docs/superpowers/specs/2026-08-19-org-admin-api-keys-audit-logs-design.md.

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct CreateAdminApiKeyRequest {
    #[builder(into)]
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in_seconds: Option<i64>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AdminApiKeyOwner {
    pub id: Option<String>,
    pub created_at: Option<i64>,
    pub name: Option<String>,
    pub object: Option<String>,
    pub role: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AdminApiKey {
    pub id: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub object: String,
    pub owner: AdminApiKeyOwner,
    pub redacted_value: String,
    pub last_used_at: Option<i64>,
    pub name: Option<String>,
    pub value: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ListAdminApiKeysRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<crate::interfaces::chat::SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListAdminApiKeysResponse {
    pub object: String,
    pub data: Vec<AdminApiKey>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteAdminApiKeyResponse {
    pub id: String,
    pub deleted: bool,
    pub object: String,
}
