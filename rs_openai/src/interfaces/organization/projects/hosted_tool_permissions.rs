//! Project-scoped hosted tool permissions
//! (`/organization/projects/{project_id}/hosted_tool_permissions`) -- project only, no org
//! equivalent -- see docs/superpowers/specs/2026-08-25-org-billing-compliance-design.md.

use bon::Builder;
use serde::{Deserialize, Serialize};

/// Confirmed identical shape reused for all 5 tool permissions, in both the response and
/// the update-params.
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct HostedToolPermissionState {
    pub enabled: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ProjectHostedToolPermissions {
    pub code_interpreter: HostedToolPermissionState,
    pub file_search: HostedToolPermissionState,
    pub image_generation: HostedToolPermissionState,
    pub mcp: HostedToolPermissionState,
    pub web_search: HostedToolPermissionState,
}

/// All 5 fields are genuinely optional (partial update) -- confirmed via source
/// (`Optional[X] | Omit = omit`), unlike Vector Store Files' explicit-null case.
#[derive(Builder, Debug, Serialize, Clone, Default)]
pub struct UpdateHostedToolPermissionsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_interpreter: Option<HostedToolPermissionState>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_search: Option<HostedToolPermissionState>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_generation: Option<HostedToolPermissionState>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp: Option<HostedToolPermissionState>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search: Option<HostedToolPermissionState>,
}
