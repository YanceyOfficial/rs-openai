//! Organization users (`/v1/organization/users`) and their assigned roles. Not yet
//! implemented — see docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct OrganizationUsers<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationUsers<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

pub struct OrganizationUser<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> OrganizationUser<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Scoped accessor for this user's assigned roles. Not yet implemented.
    pub fn roles(&self) -> OrganizationUserRoles<'a> {
        OrganizationUserRoles::new(self.openai, self.id.clone())
    }
}

#[allow(dead_code)]
pub struct OrganizationUserRoles<'a> {
    openai: &'a OpenAI,
    user_id: String,
}

impl<'a> OrganizationUserRoles<'a> {
    pub(crate) fn new(openai: &'a OpenAI, user_id: String) -> Self {
        Self { openai, user_id }
    }
}
