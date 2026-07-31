//! A project's groups (`/v1/organization/projects/{project_id}/groups`): list, delete. Not
//! yet implemented — see docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct OrganizationProjectGroups<'a> {
    openai: &'a OpenAI,
    project_id: String,
}

impl<'a> OrganizationProjectGroups<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String) -> Self {
        Self { openai, project_id }
    }
}
