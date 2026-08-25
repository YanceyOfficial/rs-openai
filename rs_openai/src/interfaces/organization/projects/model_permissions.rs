//! Project-scoped model permissions (`/organization/projects/{project_id}/model_permissions`)
//! -- an allowlist or denylist of model IDs, project only, no org equivalent -- see
//! docs/superpowers/specs/2026-08-25-org-billing-compliance-design.md.

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ModelPermissionsMode {
    AllowList,
    DenyList,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ProjectModelPermissions {
    pub mode: ModelPermissionsMode,
    pub model_ids: Vec<String>,
    pub object: String,
}

#[derive(Builder, Debug, Serialize, Clone)]
pub struct UpdateModelPermissionsRequest {
    pub mode: ModelPermissionsMode,
    pub model_ids: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ModelPermissionsDeleted {
    pub deleted: bool,
    pub object: String,
}
