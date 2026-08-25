//! Organization/project spend alerts (`/organization/spend_alerts`,
//! `/organization/projects/{id}/spend_alerts`) -- see
//! docs/superpowers/specs/2026-08-25-org-billing-compliance-design.md. Shared type set --
//! `create` and `update` are confirmed identical, full-replace shapes at both scopes.

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct SpendAlertNotificationChannel {
    pub recipients: Vec<String>,

    pub r#type: String,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_prefix: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct SpendAlert {
    pub id: String,
    pub currency: String,
    pub interval: String,
    pub notification_channel: SpendAlertNotificationChannel,
    pub object: String,
    pub threshold_amount: i64,
}

#[derive(Builder, Debug, Serialize, Clone)]
pub struct SpendAlertRequest {
    #[builder(into)]
    pub currency: String,

    #[builder(into)]
    pub interval: String,

    pub notification_channel: SpendAlertNotificationChannel,

    pub threshold_amount: i64,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct SpendAlertDeleted {
    pub id: String,
    pub deleted: bool,
    pub object: String,
}

#[derive(Builder, Debug, Serialize, Clone, Default)]
pub struct ListSpendAlertsRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<crate::interfaces::chat::SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListSpendAlertsResponse {
    pub object: String,
    pub data: Vec<SpendAlert>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}
