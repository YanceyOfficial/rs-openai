//! A project's users (`/v1/organization/projects/{project_id}/users`): create, list,
//! retrieve, update, delete. Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct OrganizationProjectUsers<'a> {
    openai: &'a OpenAI,
    project_id: String,
}

impl<'a> OrganizationProjectUsers<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String) -> Self {
        Self { openai, project_id }
    }
}

#[allow(dead_code)]
pub struct OrganizationProjectUser<'a> {
    openai: &'a OpenAI,
    project_id: String,
    id: String,
}

impl<'a> OrganizationProjectUser<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String, id: String) -> Self {
        Self {
            openai,
            project_id,
            id,
        }
    }
}
