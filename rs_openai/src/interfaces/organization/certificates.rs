//! Organization/project certificates (`/organization/certificates`,
//! `/organization/projects/{id}/certificates`) -- see
//! docs/superpowers/specs/2026-08-25-org-billing-compliance-design.md. Certificates are
//! uploaded once at the org level; a project can only list/activate/deactivate them, not
//! create/retrieve/update/delete its own -- so this one type set is shared by both scopes.

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Serialize, Default)]
pub struct CertificateDetails {
    pub content: Option<String>,
    pub expires_at: Option<i64>,
    pub valid_at: Option<i64>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Certificate {
    pub id: String,
    pub certificate_details: CertificateDetails,
    pub created_at: i64,
    pub name: Option<String>,
    pub object: String,
    pub active: Option<bool>,
}

#[derive(Builder, Debug, Serialize, Clone)]
pub struct CreateCertificateRequest {
    #[builder(into)]
    pub certificate: String,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Builder, Debug, Serialize, Clone, Default)]
pub struct UpdateCertificateRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// `include` is a `Vec<String>` query param -- serialized via `to_query_pairs()`, matching
/// the established workaround for this crate's confirmed inability to serialize a bare
/// `Vec<String>` struct field through `serde_urlencoded`.
#[derive(Builder, Debug, Clone, Default)]
pub struct RetrieveCertificateRequest {
    pub include: Option<Vec<String>>,
}

impl RetrieveCertificateRequest {
    pub(crate) fn to_query_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        if let Some(values) = &self.include {
            for v in values {
                pairs.push(("include".to_string(), v.clone()));
            }
        }
        pairs
    }
}

#[derive(Builder, Debug, Serialize, Clone, Default)]
pub struct ListCertificatesRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<crate::interfaces::chat::SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListCertificatesResponse {
    pub object: String,
    pub data: Vec<Certificate>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteCertificateResponse {
    pub id: String,
    pub object: String,
}

#[derive(Builder, Debug, Serialize, Clone)]
pub struct ActivateCertificatesRequest {
    pub certificate_ids: Vec<String>,
}

#[derive(Builder, Debug, Serialize, Clone)]
pub struct DeactivateCertificatesRequest {
    pub certificate_ids: Vec<String>,
}

/// Response shape for `activate`/`deactivate` -- confirmed via source (`SyncPage`) to be a
/// flat, non-cursor page: just `{data, object}`, no `first_id`/`last_id`/`has_more`.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CertificatesPage {
    pub object: String,
    pub data: Vec<Certificate>,
}
