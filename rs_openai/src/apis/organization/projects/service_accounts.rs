//! A project's service accounts
//! (`/v1/organization/projects/{project_id}/service_accounts`): create, list, retrieve,
//! delete. Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct OrganizationProjectServiceAccounts<'a> {
    openai: &'a OpenAI,
    project_id: String,
}

impl<'a> OrganizationProjectServiceAccounts<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String) -> Self {
        Self { openai, project_id }
    }
}

#[allow(dead_code)]
pub struct OrganizationProjectServiceAccount<'a> {
    openai: &'a OpenAI,
    project_id: String,
    id: String,
}

impl<'a> OrganizationProjectServiceAccount<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String, id: String) -> Self {
        Self {
            openai,
            project_id,
            id,
        }
    }
}
