//! A project's service accounts
//! (`/organization/projects/{project_id}/service_accounts`): create, list, retrieve,
//! update, delete. Includes the nested ServiceAccounts.ApiKeys sub-resource (create
//! only) -- see docs/superpowers/specs/2026-08-20-org-projects-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::projects::service_accounts;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationProjectServiceAccounts<'a> {
    openai: &'a OpenAI,
    project_id: String,
}

impl<'a> OrganizationProjectServiceAccounts<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String) -> Self {
        Self { openai, project_id }
    }

    pub async fn create(
        &self,
        req: &service_accounts::CreateProjectServiceAccountRequest,
    ) -> OpenAIResponse<service_accounts::CreateProjectServiceAccountResponse> {
        self.openai
            .post(
                &format!(
                    "/organization/projects/{}/service_accounts",
                    self.project_id
                ),
                req,
            )
            .await
    }

    pub async fn list(
        &self,
        req: &service_accounts::ListProjectServiceAccountsRequest,
    ) -> OpenAIResponse<service_accounts::ListProjectServiceAccountsResponse> {
        self.openai
            .get(
                &format!(
                    "/organization/projects/{}/service_accounts",
                    self.project_id
                ),
                req,
            )
            .await
    }
}

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

    pub async fn retrieve(&self) -> OpenAIResponse<service_accounts::ProjectServiceAccount> {
        self.openai
            .get(
                &format!(
                    "/organization/projects/{}/service_accounts/{}",
                    self.project_id, self.id
                ),
                &(),
            )
            .await
    }

    pub async fn update(
        &self,
        req: &service_accounts::UpdateProjectServiceAccountRequest,
    ) -> OpenAIResponse<service_accounts::ProjectServiceAccount> {
        self.openai
            .post(
                &format!(
                    "/organization/projects/{}/service_accounts/{}",
                    self.project_id, self.id
                ),
                req,
            )
            .await
    }

    pub async fn delete(
        &self,
    ) -> OpenAIResponse<service_accounts::DeleteProjectServiceAccountResponse> {
        self.openai
            .delete(
                &format!(
                    "/organization/projects/{}/service_accounts/{}",
                    self.project_id, self.id
                ),
                &(),
            )
            .await
    }

    pub fn api_keys(&self) -> OrganizationProjectServiceAccountApiKeys<'a> {
        OrganizationProjectServiceAccountApiKeys::new(
            self.openai,
            self.project_id.clone(),
            self.id.clone(),
        )
    }
}

pub struct OrganizationProjectServiceAccountApiKeys<'a> {
    openai: &'a OpenAI,
    project_id: String,
    service_account_id: String,
}

impl<'a> OrganizationProjectServiceAccountApiKeys<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String, service_account_id: String) -> Self {
        Self {
            openai,
            project_id,
            service_account_id,
        }
    }

    pub async fn create(
        &self,
        req: &service_accounts::CreateProjectServiceAccountApiKeyRequest,
    ) -> OpenAIResponse<service_accounts::CreateProjectServiceAccountApiKeyResponse> {
        self.openai
            .post(
                &format!(
                    "/organization/projects/{}/service_accounts/{}/api_keys",
                    self.project_id, self.service_account_id
                ),
                req,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::projects::service_accounts::{
        CreateProjectServiceAccountApiKeyRequest, CreateProjectServiceAccountRequest,
        ListProjectServiceAccountsRequest, UpdateProjectServiceAccountRequest,
    };
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_deserializes_distinct_response_shape_with_api_key() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/projects/proj_1/service_accounts"))
            .and(body_json(serde_json::json!({"name": "CI Bot"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "sa_1",
                "created_at": 1700000000_i64,
                "name": "CI Bot",
                "object": "organization.project.service_account",
                "role": "member",
                "api_key": {
                    "id": "key_1",
                    "created_at": 1700000000_i64,
                    "name": "CI Bot API Key",
                    "object": "organization.project.service_account.api_key",
                    "value": "sk-live-abc123"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = CreateProjectServiceAccountRequest::builder()
            .name("CI Bot")
            .build();
        let result = client
            .organization()
            .project("proj_1")
            .service_accounts()
            .create(&req)
            .await
            .unwrap();

        assert_eq!(result.role, "member");
        let api_key = result.api_key.unwrap();
        assert_eq!(api_key.value, "sk-live-abc123");
        // CreateProjectServiceAccountResponse has an `api_key` field that
        // ProjectServiceAccount (retrieve/update/list) does not -- this test
        // compiling and passing is itself the proof the two types stayed separate.
    }

    #[tokio::test]
    async fn retrieve_deserializes_base_shape_without_api_key_field() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/projects/proj_1/service_accounts/sa_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "sa_1",
                "created_at": 1700000000_i64,
                "name": "CI Bot",
                "object": "organization.project.service_account",
                "role": "owner"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .project("proj_1")
            .service_account("sa_1")
            .retrieve()
            .await
            .unwrap();

        assert_eq!(result.role, "owner");
    }

    #[tokio::test]
    async fn update_serializes_only_provided_fields() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/projects/proj_1/service_accounts/sa_1"))
            .and(body_json(serde_json::json!({"role": "owner"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "sa_1",
                "created_at": 1700000000_i64,
                "name": "CI Bot",
                "object": "organization.project.service_account",
                "role": "owner"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UpdateProjectServiceAccountRequest::builder()
            .role("owner")
            .build();
        let result = client
            .organization()
            .project("proj_1")
            .service_account("sa_1")
            .update(&req)
            .await
            .unwrap();

        assert_eq!(result.role, "owner");
    }

    #[tokio::test]
    async fn list_serializes_query_params_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/projects/proj_1/service_accounts"))
            .and(query_param("limit", "10"))
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
        let req = ListProjectServiceAccountsRequest::builder()
            .limit(10)
            .build();
        let result = client
            .organization()
            .project("proj_1")
            .service_accounts()
            .list(&req)
            .await
            .unwrap();

        assert!(result.data.is_empty());
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/organization/projects/proj_1/service_accounts/sa_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "sa_1",
                "deleted": true,
                "object": "organization.project.service_account.deleted"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .project("proj_1")
            .service_account("sa_1")
            .delete()
            .await
            .unwrap();

        assert!(result.deleted);
    }

    #[tokio::test]
    async fn api_keys_create_serializes_scopes_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path(
                "/organization/projects/proj_1/service_accounts/sa_1/api_keys",
            ))
            .and(body_json(serde_json::json!({
                "name": "CI Key",
                "scopes": ["api.model.request"]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "key_1",
                "created_at": 1700000000_i64,
                "name": "CI Key",
                "object": "organization.project.service_account.api_key",
                "value": "sk-live-xyz789"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = CreateProjectServiceAccountApiKeyRequest::builder()
            .name("CI Key")
            .scopes(vec!["api.model.request".to_string()])
            .build();
        let result = client
            .organization()
            .project("proj_1")
            .service_account("sa_1")
            .api_keys()
            .create(&req)
            .await
            .unwrap();

        assert_eq!(result.value, "sk-live-xyz789");
    }
}
