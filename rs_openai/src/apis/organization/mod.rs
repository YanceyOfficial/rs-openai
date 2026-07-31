//! Organization-level administration: audit logs, admin API keys, usage, groups, invites,
//! projects, roles, and users. A distinct "admin tier", separate from the "call the model"
//! surface the rest of this crate exposes. Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

pub mod admin_api_keys;
pub mod audit_logs;
pub mod groups;
pub mod invites;
pub mod projects;
pub mod roles;
pub mod usage;
pub mod users;

use crate::client::OpenAI;

pub struct Organization<'a> {
    openai: &'a OpenAI,
}

impl<'a> Organization<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Scoped accessor for organization audit logs. Not yet implemented.
    pub fn audit_logs(&self) -> audit_logs::OrganizationAuditLogs<'a> {
        audit_logs::OrganizationAuditLogs::new(self.openai)
    }

    /// Scoped accessor for organization admin API keys. Not yet implemented.
    pub fn admin_api_keys(&self) -> admin_api_keys::OrganizationAdminApiKeys<'a> {
        admin_api_keys::OrganizationAdminApiKeys::new(self.openai)
    }

    /// Scoped accessor for a specific admin API key. Not yet implemented.
    pub fn admin_api_key(
        &self,
        id: impl Into<String>,
    ) -> admin_api_keys::OrganizationAdminApiKey<'a> {
        admin_api_keys::OrganizationAdminApiKey::new(self.openai, id.into())
    }

    /// Scoped accessor for organization usage reporting. Not yet implemented.
    pub fn usage(&self) -> usage::OrganizationUsage<'a> {
        usage::OrganizationUsage::new(self.openai)
    }

    /// Scoped accessor for organization groups. Not yet implemented.
    pub fn groups(&self) -> groups::OrganizationGroups<'a> {
        groups::OrganizationGroups::new(self.openai)
    }

    /// Scoped accessor for a specific organization group. Not yet implemented.
    pub fn group(&self, id: impl Into<String>) -> groups::OrganizationGroup<'a> {
        groups::OrganizationGroup::new(self.openai, id.into())
    }

    /// Scoped accessor for organization invites. Not yet implemented.
    pub fn invites(&self) -> invites::OrganizationInvites<'a> {
        invites::OrganizationInvites::new(self.openai)
    }

    /// Scoped accessor for a specific organization invite. Not yet implemented.
    pub fn invite(&self, id: impl Into<String>) -> invites::OrganizationInvite<'a> {
        invites::OrganizationInvite::new(self.openai, id.into())
    }

    /// Scoped accessor for organization projects. Not yet implemented.
    pub fn projects(&self) -> projects::OrganizationProjects<'a> {
        projects::OrganizationProjects::new(self.openai)
    }

    /// Scoped accessor for a specific organization project. Not yet implemented.
    pub fn project(&self, id: impl Into<String>) -> projects::OrganizationProject<'a> {
        projects::OrganizationProject::new(self.openai, id.into())
    }

    /// Scoped accessor for organization roles. Not yet implemented.
    pub fn roles(&self) -> roles::OrganizationRoles<'a> {
        roles::OrganizationRoles::new(self.openai)
    }

    /// Scoped accessor for a specific organization role. Not yet implemented.
    pub fn role(&self, id: impl Into<String>) -> roles::OrganizationRole<'a> {
        roles::OrganizationRole::new(self.openai, id.into())
    }

    /// Scoped accessor for organization users. Not yet implemented.
    pub fn users(&self) -> users::OrganizationUsers<'a> {
        users::OrganizationUsers::new(self.openai)
    }

    /// Scoped accessor for a specific organization user. Not yet implemented.
    pub fn user(&self, id: impl Into<String>) -> users::OrganizationUser<'a> {
        users::OrganizationUser::new(self.openai, id.into())
    }
}
