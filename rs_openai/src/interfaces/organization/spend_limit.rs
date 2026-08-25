//! Organization/project hard spend limit (`/organization/spend_limit`,
//! `/organization/projects/{id}/spend_limit`) -- see
//! docs/superpowers/specs/2026-08-25-org-billing-compliance-design.md. A singleton resource
//! (no ID), shared by both scopes (object field differs only by wire string).

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct SpendLimitEnforcement {
    pub status: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct SpendLimit {
    pub currency: String,
    pub enforcement: SpendLimitEnforcement,
    pub interval: String,
    pub object: String,
    pub threshold_amount: i64,
}

#[derive(Builder, Debug, Serialize, Clone)]
pub struct UpdateSpendLimitRequest {
    #[builder(into)]
    pub currency: String,

    #[builder(into)]
    pub interval: String,

    pub threshold_amount: i64,
}

/// Confirmed via source: NO `id` field, unlike `SpendAlertDeleted` -- a genuine per-resource
/// asymmetry, not an oversight.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct SpendLimitDeleted {
    pub deleted: bool,
    pub object: String,
}
