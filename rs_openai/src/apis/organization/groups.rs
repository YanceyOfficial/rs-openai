//! Organization groups (`/v1/organization/groups`) and their member users. Not yet
//! implemented — see docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct OrganizationGroups<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationGroups<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

pub struct OrganizationGroup<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> OrganizationGroup<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Scoped accessor for this group's member users. Not yet implemented.
    pub fn users(&self) -> OrganizationGroupUsers<'a> {
        OrganizationGroupUsers::new(self.openai, self.id.clone())
    }
}

#[allow(dead_code)]
pub struct OrganizationGroupUsers<'a> {
    openai: &'a OpenAI,
    group_id: String,
}

impl<'a> OrganizationGroupUsers<'a> {
    pub(crate) fn new(openai: &'a OpenAI, group_id: String) -> Self {
        Self { openai, group_id }
    }
}
