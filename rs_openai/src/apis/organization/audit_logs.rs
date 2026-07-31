//! Organization audit logs (`/v1/organization/audit_logs`, list-only). Not yet implemented —
//! see docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct OrganizationAuditLogs<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationAuditLogs<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}
