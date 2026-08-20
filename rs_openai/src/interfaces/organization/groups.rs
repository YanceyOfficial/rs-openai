//! Organization groups (`/organization/groups`) and their member users -- see
//! docs/superpowers/specs/2026-08-20-org-users-invites-groups-design.md.

use crate::interfaces::chat::SortOrder;
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
