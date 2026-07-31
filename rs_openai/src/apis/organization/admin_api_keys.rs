//! Organization admin API keys (`/v1/organization/admin_api_keys`). Not yet implemented —
//! see docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct OrganizationAdminApiKeys<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationAdminApiKeys<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

#[allow(dead_code)]
pub struct OrganizationAdminApiKey<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> OrganizationAdminApiKey<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }
}
