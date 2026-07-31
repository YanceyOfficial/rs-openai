//! Organization usage reporting (`/v1/organization/usage`). Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct OrganizationUsage<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationUsage<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}
