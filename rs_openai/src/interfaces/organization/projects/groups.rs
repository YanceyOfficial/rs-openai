//! A project's groups (`/organization/projects/{project_id}/groups`) -- see
//! docs/superpowers/specs/2026-08-20-org-projects-design.md. Role assignment types
//! (Groups.Roles) are appended below -- see
//! docs/superpowers/specs/2026-08-20-org-project-roles-design.md.

use crate::interfaces::chat::SortOrder;
use crate::interfaces::organization::groups::RoleAssignmentGroupSummary;
use crate::interfaces::organization::roles::Role;
use crate::interfaces::organization::users::RoleAssignmentDetail;
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

// Groups.Roles sub-resource, colocated here (matching established precedent).
// NOTE: these two endpoints' routes are confirmed to NOT carry the usual
// `/organization` prefix -- see docs/superpowers/specs/2026-08-20-org-project-roles-design.md.

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct AssignProjectGroupRoleRequest {
    #[builder(into)]
    pub role_id: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AssignProjectGroupRoleResponse {
    pub group: RoleAssignmentGroupSummary,
    pub object: String,
    pub role: Role,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListProjectGroupRolesRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListProjectGroupRolesResponse {
    pub object: String,
    pub data: Vec<RoleAssignmentDetail>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct RemoveProjectGroupRoleResponse {
    pub deleted: bool,
    pub object: String,
}
