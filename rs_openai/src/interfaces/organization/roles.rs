//! Organization roles (`/organization/roles`) -- see
//! docs/superpowers/specs/2026-08-20-org-roles-design.md.

use crate::interfaces::chat::SortOrder;
use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct CreateRoleRequest {
    pub permissions: Vec<String>,

    #[builder(into)]
    pub role_name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub object: String,
    pub permissions: Vec<String>,
    pub predefined_role: bool,
    pub resource_type: String,
    pub description: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct UpdateRoleRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub role_name: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListRolesRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListRolesResponse {
    pub object: String,
    pub data: Vec<Role>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteRoleResponse {
    pub id: String,
    pub deleted: bool,
    pub object: String,
}
