//! Organization invites (`/organization/invites`) -- see
//! docs/superpowers/specs/2026-08-20-org-users-invites-groups-design.md.

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct InviteProject {
    #[builder(into)]
    pub id: String,
    #[builder(into)]
    pub role: String,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct CreateInviteRequest {
    #[builder(into)]
    pub email: String,

    #[builder(into)]
    pub role: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub projects: Option<Vec<InviteProject>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Invite {
    pub id: String,
    pub created_at: i64,
    pub email: String,
    pub object: String,
    pub projects: Vec<InviteProject>,
    pub role: String,
    pub status: String,
    pub accepted_at: Option<i64>,
    pub expires_at: Option<i64>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListInvitesRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListInvitesResponse {
    pub object: String,
    pub data: Vec<Invite>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteInviteResponse {
    pub id: String,
    pub deleted: bool,
    pub object: String,
}
