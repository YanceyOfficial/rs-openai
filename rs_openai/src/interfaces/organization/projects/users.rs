//! A project's users (`/organization/projects/{project_id}/users`) -- see
//! docs/superpowers/specs/2026-08-20-org-projects-design.md.

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
