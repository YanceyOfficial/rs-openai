//! A project's groups (`/organization/projects/{project_id}/groups`) -- see
//! docs/superpowers/specs/2026-08-20-org-projects-design.md.

use crate::interfaces::chat::SortOrder;
use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct AddProjectGroupRequest {
    #[builder(into)]
    pub group_id: String,
    #[builder(into)]
    pub role: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ProjectGroup {
    pub created_at: i64,
    pub group_id: String,
    pub group_name: String,
    pub group_type: String,
    pub object: String,
    pub project_id: String,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListProjectGroupsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListProjectGroupsResponse {
    pub object: String,
    pub data: Vec<ProjectGroup>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct RemoveProjectGroupResponse {
    pub deleted: bool,
    pub object: String,
}
