//! A project's users (`/organization/projects/{project_id}/users`) -- see
//! docs/superpowers/specs/2026-08-20-org-projects-design.md. Role assignment types
//! (Users.Roles) are appended below -- see
//! docs/superpowers/specs/2026-08-20-org-project-roles-design.md.

use crate::interfaces::chat::SortOrder;
use crate::interfaces::organization::roles::Role;
use crate::interfaces::organization::users::{OrganizationUser, RoleAssignmentDetail};
use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct AddProjectUserRequest {
    #[builder(into)]
    pub role: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub email: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub user_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ProjectUser {
    pub id: String,
    pub added_at: i64,
    pub object: String,
    pub role: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct UpdateProjectUserRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub role: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListProjectUsersRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListProjectUsersResponse {
    pub object: String,
    pub data: Vec<ProjectUser>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteProjectUserResponse {
    pub id: String,
    pub deleted: bool,
    pub object: String,
}

// Users.Roles sub-resource, colocated here (matching established precedent).
// NOTE: these two endpoints' routes are confirmed to NOT carry the usual
// `/organization` prefix -- see docs/superpowers/specs/2026-08-20-org-project-roles-design.md.

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct AssignProjectUserRoleRequest {
    #[builder(into)]
    pub role_id: String,
}

/// Confirmed via source: embeds the ORG-level `OrganizationUser`, not a
/// project-scoped user type, despite this being a project-scoped endpoint.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AssignProjectUserRoleResponse {
    pub object: String,
    pub role: Role,
    pub user: OrganizationUser,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListProjectUserRolesRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListProjectUserRolesResponse {
    pub object: String,
    pub data: Vec<RoleAssignmentDetail>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct RemoveProjectUserRoleResponse {
    pub deleted: bool,
    pub object: String,
}
