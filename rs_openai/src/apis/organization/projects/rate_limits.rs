//! A project's rate limits (`/v1/organization/projects/{project_id}/rate_limits`): read,
//! update. Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct OrganizationProjectRateLimits<'a> {
    openai: &'a OpenAI,
    project_id: String,
}

impl<'a> OrganizationProjectRateLimits<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String) -> Self {
        Self { openai, project_id }
    }
}
