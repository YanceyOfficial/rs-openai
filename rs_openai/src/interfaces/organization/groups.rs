//! Organization groups (`/organization/groups`) and their member users -- see
//! docs/superpowers/specs/2026-08-20-org-users-invites-groups-design.md. Role
//! assignment types (Groups.Roles) are appended below -- see
//! docs/superpowers/specs/2026-08-20-org-roles-design.md.

use crate::interfaces::chat::SortOrder;
use crate::interfaces::organization::roles::Role;
use crate::interfaces::organization::users::RoleAssignmentDetail;
use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct CreateGroupRequest {
    #[builder(into)]
    pub name: String,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct UpdateGroupRequest {
    #[builder(into)]
    pub name: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Group {
    pub id: String,
    pub created_at: i64,
    pub group_type: String,
    pub is_scim_managed: bool,
    pub name: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UpdateGroupResponse {
    pub id: String,
    pub created_at: i64,
    pub is_scim_managed: bool,
    pub name: String,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListGroupsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListGroupsResponse {
    pub object: String,
    pub data: Vec<Group>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteGroupResponse {
    pub id: String,
    pub deleted: bool,
    pub object: String,
}

// Groups.Users sub-resource.

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct AddGroupUserRequest {
    #[builder(into)]
    pub user_id: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct GroupUser {
    pub id: String,
    pub email: Option<String>,
    pub name: String,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListGroupUsersRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListGroupUsersResponse {
    pub object: String,
    pub data: Vec<GroupUser>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct RemoveGroupUserResponse {
    pub deleted: bool,
    pub object: String,
}

// Groups.Roles sub-resource, colocated here (matching the established Groups.Users precedent).

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct AssignGroupRoleRequest {
    #[builder(into)]
    pub role_id: String,
}

/// Summary information about a group embedded in a role-assignment response --
/// deliberately NOT the same type as `Group` (confirmed distinct shape: no
/// `group_type` field, and `scim_managed` rather than `is_scim_managed`).
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct RoleAssignmentGroupSummary {
    pub id: String,
    pub created_at: i64,
    pub name: String,
    pub object: String,
    pub scim_managed: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AssignGroupRoleResponse {
    pub group: RoleAssignmentGroupSummary,
    pub object: String,
    pub role: Role,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListGroupRolesRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListGroupRolesResponse {
    pub object: String,
    pub data: Vec<RoleAssignmentDetail>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct RemoveGroupRoleResponse {
    pub deleted: bool,
    pub object: String,
}
