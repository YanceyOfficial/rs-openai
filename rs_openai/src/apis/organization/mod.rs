//! Organization-level administration: audit logs, admin API keys, usage, groups, invites,
//! projects, roles, users, certificates, data retention, spend alerts, spend limit, hosted
//! tool permissions, and model permissions. A distinct "admin tier", separate from the "call
//! the model" surface the rest of this crate exposes. Every sub-resource is now implemented,
//! at both org and project scope where applicable -- see
//! docs/superpowers/specs/2026-08-19-org-admin-api-keys-audit-logs-design.md,
//! docs/superpowers/specs/2026-08-20-org-users-invites-groups-design.md,
//! docs/superpowers/specs/2026-08-20-org-usage-design.md,
//! docs/superpowers/specs/2026-08-20-org-projects-design.md,
//! docs/superpowers/specs/2026-08-20-org-roles-design.md,
//! docs/superpowers/specs/2026-08-20-org-project-roles-design.md, and
//! docs/superpowers/specs/2026-08-25-org-billing-compliance-design.md.

pub mod admin_api_keys;
pub mod audit_logs;
pub mod certificates;
pub mod data_retention;
pub mod groups;
pub mod invites;
pub mod projects;
pub mod roles;
pub mod spend_alerts;
pub mod spend_limit;
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

    /// Scoped accessor for organization audit logs.
    pub fn audit_logs(&self) -> audit_logs::OrganizationAuditLogs<'a> {
        audit_logs::OrganizationAuditLogs::new(self.openai)
    }

    /// Scoped accessor for organization admin API keys.
    pub fn admin_api_keys(&self) -> admin_api_keys::OrganizationAdminApiKeys<'a> {
        admin_api_keys::OrganizationAdminApiKeys::new(self.openai)
    }

    /// Scoped accessor for a specific admin API key.
    pub fn admin_api_key(
        &self,
        id: impl Into<String>,
    ) -> admin_api_keys::OrganizationAdminApiKey<'a> {
        admin_api_keys::OrganizationAdminApiKey::new(self.openai, id.into())
    }

    /// Scoped accessor for organization usage reporting.
    pub fn usage(&self) -> usage::OrganizationUsage<'a> {
        usage::OrganizationUsage::new(self.openai)
    }

    /// Scoped accessor for organization groups.
    pub fn groups(&self) -> groups::OrganizationGroups<'a> {
        groups::OrganizationGroups::new(self.openai)
    }

    /// Scoped accessor for a specific organization group.
    pub fn group(&self, id: impl Into<String>) -> groups::OrganizationGroup<'a> {
        groups::OrganizationGroup::new(self.openai, id.into())
    }

    /// Scoped accessor for organization invites.
    pub fn invites(&self) -> invites::OrganizationInvites<'a> {
        invites::OrganizationInvites::new(self.openai)
    }

    /// Scoped accessor for a specific organization invite.
    pub fn invite(&self, id: impl Into<String>) -> invites::OrganizationInvite<'a> {
        invites::OrganizationInvite::new(self.openai, id.into())
    }

    /// Scoped accessor for organization projects.
    pub fn projects(&self) -> projects::OrganizationProjects<'a> {
        projects::OrganizationProjects::new(self.openai)
    }

    /// Scoped accessor for a specific organization project.
    pub fn project(&self, id: impl Into<String>) -> projects::OrganizationProject<'a> {
        projects::OrganizationProject::new(self.openai, id.into())
    }

    /// Scoped accessor for organization roles.
    pub fn roles(&self) -> roles::OrganizationRoles<'a> {
        roles::OrganizationRoles::new(self.openai)
    }

    /// Scoped accessor for a specific organization role.
    pub fn role(&self, id: impl Into<String>) -> roles::OrganizationRole<'a> {
        roles::OrganizationRole::new(self.openai, id.into())
    }

    /// Scoped accessor for organization users.
    pub fn users(&self) -> users::OrganizationUsers<'a> {
        users::OrganizationUsers::new(self.openai)
    }

    /// Scoped accessor for a specific organization user.
    pub fn user(&self, id: impl Into<String>) -> users::OrganizationUser<'a> {
        users::OrganizationUser::new(self.openai, id.into())
    }

    /// Scoped accessor for organization certificates.
    pub fn certificates(&self) -> certificates::OrganizationCertificates<'a> {
        certificates::OrganizationCertificates::new(self.openai)
    }

    /// Scoped accessor for a specific organization certificate.
    pub fn certificate(&self, id: impl Into<String>) -> certificates::OrganizationCertificate<'a> {
        certificates::OrganizationCertificate::new(self.openai, id.into())
    }

    /// Scoped accessor for organization data retention controls.
    pub fn data_retention(&self) -> data_retention::OrganizationDataRetention<'a> {
        data_retention::OrganizationDataRetention::new(self.openai)
    }

    /// Scoped accessor for organization spend alerts.
    pub fn spend_alerts(&self) -> spend_alerts::OrganizationSpendAlerts<'a> {
        spend_alerts::OrganizationSpendAlerts::new(self.openai)
    }

    /// Scoped accessor for a specific organization spend alert.
    pub fn spend_alert(&self, id: impl Into<String>) -> spend_alerts::OrganizationSpendAlert<'a> {
        spend_alerts::OrganizationSpendAlert::new(self.openai, id.into())
    }

    /// Scoped accessor for the organization's hard spend limit.
    pub fn spend_limit(&self) -> spend_limit::OrganizationSpendLimit<'a> {
        spend_limit::OrganizationSpendLimit::new(self.openai)
    }
}
