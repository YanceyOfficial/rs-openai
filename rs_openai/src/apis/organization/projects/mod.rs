//! A project-scoped view of organization resources: API keys, groups, rate limits, service
//! accounts, and users, all scoped to one project. Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.
//!
//! Note: the structure research also found a parallel, slightly differently-scoped
//! "Projects" family (Groups Roles, Roles, Users Roles) in OpenAI's reference index — likely
//! a project-scoped-API-key view of the same underlying resources. Not scaffolded here;
//! resolve the ambiguity against live docs before implementing.

pub mod api_keys;
pub mod groups;
pub mod rate_limits;
pub mod service_accounts;
pub mod users;

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct OrganizationProjects<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationProjects<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

pub struct OrganizationProject<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> OrganizationProject<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Scoped accessor for this project's API keys. Not yet implemented.
    pub fn api_keys(&self) -> api_keys::OrganizationProjectApiKeys<'a> {
        api_keys::OrganizationProjectApiKeys::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for a specific project API key. Not yet implemented.
    pub fn api_key(&self, id: impl Into<String>) -> api_keys::OrganizationProjectApiKey<'a> {
        api_keys::OrganizationProjectApiKey::new(self.openai, self.id.clone(), id.into())
    }

    /// Scoped accessor for this project's groups. Not yet implemented.
    pub fn groups(&self) -> groups::OrganizationProjectGroups<'a> {
        groups::OrganizationProjectGroups::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for this project's rate limits. Not yet implemented.
    pub fn rate_limits(&self) -> rate_limits::OrganizationProjectRateLimits<'a> {
        rate_limits::OrganizationProjectRateLimits::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for this project's service accounts. Not yet implemented.
    pub fn service_accounts(&self) -> service_accounts::OrganizationProjectServiceAccounts<'a> {
        service_accounts::OrganizationProjectServiceAccounts::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for a specific project service account. Not yet implemented.
    pub fn service_account(
        &self,
        id: impl Into<String>,
    ) -> service_accounts::OrganizationProjectServiceAccount<'a> {
        service_accounts::OrganizationProjectServiceAccount::new(
            self.openai,
            self.id.clone(),
            id.into(),
        )
    }

    /// Scoped accessor for this project's users. Not yet implemented.
    pub fn users(&self) -> users::OrganizationProjectUsers<'a> {
        users::OrganizationProjectUsers::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for a specific project user. Not yet implemented.
    pub fn user(&self, id: impl Into<String>) -> users::OrganizationProjectUser<'a> {
        users::OrganizationProjectUser::new(self.openai, self.id.clone(), id.into())
    }
}
