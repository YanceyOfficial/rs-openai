//! A project's API keys (`/organization/projects/{project_id}/api_keys`) -- see
//! docs/superpowers/specs/2026-08-20-org-projects-design.md.

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ProjectApiKeyOwnerServiceAccount {
    pub id: String,
    pub created_at: i64,
    pub name: String,
    pub role: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ProjectApiKeyOwnerUser {
    pub id: String,
    pub created_at: i64,
    pub email: String,
    pub name: String,
    pub role: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ProjectApiKeyOwner {
    pub service_account: Option<ProjectApiKeyOwnerServiceAccount>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub user: Option<ProjectApiKeyOwnerUser>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ProjectApiKey {
    pub id: String,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
    pub name: String,
    pub object: String,
    pub owner: ProjectApiKeyOwner,
    pub owner_project_access: String,
    pub redacted_value: String,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListProjectApiKeysRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub owner_project_access: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListProjectApiKeysResponse {
    pub object: String,
    pub data: Vec<ProjectApiKey>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteProjectApiKeyResponse {
    pub id: String,
    pub deleted: bool,
    pub object: String,
}
