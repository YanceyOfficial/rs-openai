//! Organization/project data retention controls (`/organization/data_retention`,
//! `/organization/projects/{id}/data_retention`) -- see
//! docs/superpowers/specs/2026-08-25-org-billing-compliance-design.md. A singleton resource
//! (no ID), shared by both scopes -- project's valid `type` set is confirmed a strict
//! superset of org's (adds `organization_default`/`none`), modeled as one enum per this
//! crate's "don't re-implement server-side validation" convention.

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum DataRetentionType {
    ZeroDataRetention,
    ModifiedAbuseMonitoring,
    EnhancedZeroDataRetention,
    EnhancedModifiedAbuseMonitoring,
    OrganizationDefault,
    None,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DataRetention {
    pub object: String,
    pub r#type: DataRetentionType,
}

#[derive(Builder, Debug, Serialize, Clone)]
pub struct UpdateDataRetentionRequest {
    pub retention_type: DataRetentionType,
}
