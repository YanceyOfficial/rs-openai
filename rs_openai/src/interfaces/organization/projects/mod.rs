//! Organization projects (`/organization/projects`) -- see
//! docs/superpowers/specs/2026-08-20-org-projects-design.md.

pub mod api_keys;
pub mod groups;
pub mod hosted_tool_permissions;
pub mod model_permissions;
pub mod rate_limits;
pub mod service_accounts;
pub mod users;

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct CreateProjectRequest {
    #[builder(into)]
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub external_key_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub geography: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct UpdateProjectRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub external_key_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub geography: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Project {
    pub id: String,
    pub created_at: i64,
    pub object: String,
    pub archived_at: Option<i64>,
    pub external_key_id: Option<String>,
    pub name: Option<String>,
    pub status: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListProjectsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_archived: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListProjectsResponse {
    pub object: String,
    pub data: Vec<Project>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}
