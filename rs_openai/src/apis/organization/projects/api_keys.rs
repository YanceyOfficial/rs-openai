//! A project's API keys (`/v1/organization/projects/{project_id}/api_keys`): list, retrieve,
//! delete. Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct OrganizationProjectApiKeys<'a> {
    openai: &'a OpenAI,
    project_id: String,
}

impl<'a> OrganizationProjectApiKeys<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String) -> Self {
        Self { openai, project_id }
    }
}

#[allow(dead_code)]
pub struct OrganizationProjectApiKey<'a> {
    openai: &'a OpenAI,
    project_id: String,
    id: String,
}

impl<'a> OrganizationProjectApiKey<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String, id: String) -> Self {
        Self {
            openai,
            project_id,
            id,
        }
    }
}
