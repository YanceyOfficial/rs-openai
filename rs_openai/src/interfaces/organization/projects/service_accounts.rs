//! A project's service accounts
//! (`/organization/projects/{project_id}/service_accounts`) -- see
//! docs/superpowers/specs/2026-08-20-org-projects-design.md.

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct CreateProjectServiceAccountRequest {
    #[builder(into)]
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_service_account_only: Option<bool>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ProjectServiceAccount {
    pub id: String,
    pub created_at: i64,
    pub name: String,
    pub object: String,
    pub role: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CreateProjectServiceAccountApiKey {
    pub id: String,
    pub created_at: i64,
    pub name: String,
    pub object: String,
    pub value: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CreateProjectServiceAccountResponse {
    pub id: String,
    pub created_at: i64,
    pub name: String,
    pub object: String,
    pub role: String,
    pub api_key: Option<CreateProjectServiceAccountApiKey>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct UpdateProjectServiceAccountRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub role: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListProjectServiceAccountsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListProjectServiceAccountsResponse {
    pub object: String,
    pub data: Vec<ProjectServiceAccount>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteProjectServiceAccountResponse {
    pub id: String,
    pub deleted: bool,
    pub object: String,
}

// ServiceAccounts.ApiKeys sub-resource, colocated here (see File organization in the spec).

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct CreateProjectServiceAccountApiKeyRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CreateProjectServiceAccountApiKeyResponse {
    pub id: String,
    pub created_at: i64,
    pub name: String,
    pub object: String,
    pub value: String,
}
