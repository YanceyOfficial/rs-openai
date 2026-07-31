//! Organization invites (`/v1/organization/invites`). Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct OrganizationInvites<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationInvites<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

#[allow(dead_code)]
pub struct OrganizationInvite<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> OrganizationInvite<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }
}
