//! A project-scoped view of organization resources: API keys, groups, rate limits, service
//! accounts, users, certificates, data retention, spend alerts, spend limit, hosted tool
//! permissions, and model permissions, all scoped to one project -- see
//! docs/superpowers/specs/2026-08-20-org-projects-design.md. Project-scoped role
//! assignment (Groups.Roles/Users.Roles, both confirmed to route WITHOUT the usual
//! `/organization` prefix) is implemented too -- see
//! docs/superpowers/specs/2026-08-20-org-project-roles-design.md. Certificates/DataRetention/
//! SpendAlerts/SpendLimit/HostedToolPermissions/ModelPermissions (unlike Groups.Roles/
//! Users.Roles) DO carry the usual `/organization` prefix -- see
//! docs/superpowers/specs/2026-08-25-org-billing-compliance-design.md.

pub mod api_keys;
pub mod certificates;
pub mod data_retention;
pub mod groups;
pub mod hosted_tool_permissions;
pub mod model_permissions;
pub mod rate_limits;
pub mod service_accounts;
pub mod spend_alerts;
pub mod spend_limit;
pub mod users;

use crate::client::OpenAI;
use crate::interfaces::organization::projects;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationProjects<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationProjects<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    pub async fn create(
        &self,
        req: &projects::CreateProjectRequest,
    ) -> OpenAIResponse<projects::Project> {
        self.openai.post("/organization/projects", req).await
    }

    pub async fn list(
        &self,
        req: &projects::ListProjectsRequest,
    ) -> OpenAIResponse<projects::ListProjectsResponse> {
        self.openai.get("/organization/projects", req).await
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

    pub async fn retrieve(&self) -> OpenAIResponse<projects::Project> {
        self.openai
            .get(&format!("/organization/projects/{}", self.id), &())
            .await
    }

    pub async fn update(
        &self,
        req: &projects::UpdateProjectRequest,
    ) -> OpenAIResponse<projects::Project> {
        self.openai
            .post(&format!("/organization/projects/{}", self.id), req)
            .await
    }

    pub async fn archive(&self) -> OpenAIResponse<projects::Project> {
        self.openai
            .post(&format!("/organization/projects/{}/archive", self.id), &())
            .await
    }

    /// Scoped accessor for this project's API keys.
    pub fn api_keys(&self) -> api_keys::OrganizationProjectApiKeys<'a> {
        api_keys::OrganizationProjectApiKeys::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for a specific project API key.
    pub fn api_key(&self, id: impl Into<String>) -> api_keys::OrganizationProjectApiKey<'a> {
        api_keys::OrganizationProjectApiKey::new(self.openai, self.id.clone(), id.into())
    }

    /// Scoped accessor for this project's groups.
    pub fn groups(&self) -> groups::OrganizationProjectGroups<'a> {
        groups::OrganizationProjectGroups::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for this project's rate limits.
    pub fn rate_limits(&self) -> rate_limits::OrganizationProjectRateLimits<'a> {
        rate_limits::OrganizationProjectRateLimits::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for this project's service accounts.
    pub fn service_accounts(&self) -> service_accounts::OrganizationProjectServiceAccounts<'a> {
        service_accounts::OrganizationProjectServiceAccounts::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for a specific project service account.
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

    /// Scoped accessor for this project's users.
    pub fn users(&self) -> users::OrganizationProjectUsers<'a> {
        users::OrganizationProjectUsers::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for a specific project user.
    pub fn user(&self, id: impl Into<String>) -> users::OrganizationProjectUser<'a> {
        users::OrganizationProjectUser::new(self.openai, self.id.clone(), id.into())
    }

    /// Scoped accessor for this project's certificates.
    pub fn certificates(&self) -> certificates::OrganizationProjectCertificates<'a> {
        certificates::OrganizationProjectCertificates::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for this project's data retention controls.
    pub fn data_retention(&self) -> data_retention::OrganizationProjectDataRetention<'a> {
        data_retention::OrganizationProjectDataRetention::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for this project's spend alerts.
    pub fn spend_alerts(&self) -> spend_alerts::OrganizationProjectSpendAlerts<'a> {
        spend_alerts::OrganizationProjectSpendAlerts::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for a specific project spend alert.
    pub fn spend_alert(
        &self,
        id: impl Into<String>,
    ) -> spend_alerts::OrganizationProjectSpendAlert<'a> {
        spend_alerts::OrganizationProjectSpendAlert::new(self.openai, self.id.clone(), id.into())
    }

    /// Scoped accessor for this project's hard spend limit.
    pub fn spend_limit(&self) -> spend_limit::OrganizationProjectSpendLimit<'a> {
        spend_limit::OrganizationProjectSpendLimit::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for this project's hosted tool permissions.
    pub fn hosted_tool_permissions(
        &self,
    ) -> hosted_tool_permissions::OrganizationProjectHostedToolPermissions<'a> {
        hosted_tool_permissions::OrganizationProjectHostedToolPermissions::new(
            self.openai,
            self.id.clone(),
        )
    }

    /// Scoped accessor for this project's model permissions.
    pub fn model_permissions(&self) -> model_permissions::OrganizationProjectModelPermissions<'a> {
        model_permissions::OrganizationProjectModelPermissions::new(self.openai, self.id.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::projects::{
        CreateProjectRequest, ListProjectsRequest, UpdateProjectRequest,
    };
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_full_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/projects"))
            .and(body_json(serde_json::json!({
                "name": "New Project",
                "external_key_id": "ext_1",
                "geography": "us"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "proj_1",
                "created_at": 1700000000_i64,
                "object": "organization.project",
                "name": "New Project",
                "status": "active"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = CreateProjectRequest::builder()
            .name("New Project")
            .external_key_id("ext_1")
            .geography("us")
            .build();
        let result = client.organization().projects().create(&req).await.unwrap();

        assert_eq!(result.id, "proj_1");
        assert_eq!(result.status.as_deref(), Some("active"));
    }

    #[tokio::test]
    async fn create_with_only_required_fields_omits_optional_ones() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/projects"))
            .and(body_json(serde_json::json!({"name": "Minimal Project"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "proj_2",
                "created_at": 1700000000_i64,
                "object": "organization.project",
                "name": "Minimal Project",
                "status": "active"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = CreateProjectRequest::builder()
            .name("Minimal Project")
            .build();
        let result = client.organization().projects().create(&req).await.unwrap();

        assert_eq!(result.id, "proj_2");
    }

    #[tokio::test]
    async fn retrieve_deserializes_project() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/projects/proj_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "proj_1",
                "created_at": 1700000000_i64,
                "object": "organization.project",
                "name": "New Project",
                "status": "active"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .project("proj_1")
            .retrieve()
            .await
            .unwrap();

        assert_eq!(result.name.as_deref(), Some("New Project"));
    }

    #[tokio::test]
    async fn update_serializes_only_provided_fields() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/projects/proj_1"))
            .and(body_json(serde_json::json!({"name": "Renamed Project"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "proj_1",
                "created_at": 1700000000_i64,
                "object": "organization.project",
                "name": "Renamed Project",
                "status": "active"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UpdateProjectRequest::builder()
            .name("Renamed Project")
            .build();
        let result = client
            .organization()
            .project("proj_1")
            .update(&req)
            .await
            .unwrap();

        assert_eq!(result.name.as_deref(), Some("Renamed Project"));
    }

    #[tokio::test]
    async fn list_serializes_query_params_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/projects"))
            .and(query_param("limit", "10"))
            .and(query_param("include_archived", "true"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [],
                "first_id": null,
                "last_id": null,
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = ListProjectsRequest::builder()
            .limit(10)
            .include_archived(true)
            .build();
        let result = client.organization().projects().list(&req).await.unwrap();

        assert!(result.data.is_empty());
    }

    #[tokio::test]
    async fn archive_deserializes_archived_project() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/projects/proj_1/archive"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "proj_1",
                "created_at": 1700000000_i64,
                "object": "organization.project",
                "name": "New Project",
                "status": "archived",
                "archived_at": 1700000200_i64
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .project("proj_1")
            .archive()
            .await
            .unwrap();

        assert_eq!(result.status.as_deref(), Some("archived"));
        assert_eq!(result.archived_at, Some(1700000200));
    }
}
