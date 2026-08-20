//! Organization users (`/organization/users`) -- see
//! docs/superpowers/specs/2026-08-20-org-users-invites-groups-design.md.

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct OrganizationUserNested {
    pub id: String,
    pub object: String,
    pub banned: Option<bool>,
    pub banned_at: Option<i64>,
    pub email: Option<String>,
    pub enabled: Option<bool>,
    pub name: Option<String>,
    pub picture: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct OrganizationUserProjectsData {
    pub id: Option<String>,
    pub name: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct OrganizationUserProjects {
    pub object: String,
    pub data: Vec<OrganizationUserProjectsData>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct OrganizationUser {
    pub id: String,
    pub added_at: i64,
    pub object: String,
    pub api_key_last_used_at: Option<i64>,
    pub created: Option<i64>,
    pub developer_persona: Option<String>,
    pub email: Option<String>,
    pub is_default: Option<bool>,
    pub is_scale_tier_authorized_purchaser: Option<bool>,
    pub is_scim_managed: Option<bool>,
    pub is_service_account: Option<bool>,
    pub name: Option<String>,
    pub projects: Option<OrganizationUserProjects>,
    pub role: Option<String>,
    pub technical_level: Option<String>,
    pub user: Option<OrganizationUserNested>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct UpdateOrganizationUserRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub developer_persona: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub role: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub role_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub technical_level: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListOrganizationUsersRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListOrganizationUsersResponse {
    pub object: String,
    pub data: Vec<OrganizationUser>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteOrganizationUserResponse {
    pub id: String,
    pub deleted: bool,
    pub object: String,
}
