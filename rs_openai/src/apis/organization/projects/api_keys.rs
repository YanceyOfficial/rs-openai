//! A project's API keys (`/organization/projects/{project_id}/api_keys`): list, retrieve,
//! delete -- see docs/superpowers/specs/2026-08-20-org-projects-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::projects::api_keys;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationProjectApiKeys<'a> {
    openai: &'a OpenAI,
    project_id: String,
}

impl<'a> OrganizationProjectApiKeys<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String) -> Self {
        Self { openai, project_id }
    }

    pub async fn list(
        &self,
        req: &api_keys::ListProjectApiKeysRequest,
    ) -> OpenAIResponse<api_keys::ListProjectApiKeysResponse> {
        self.openai
            .get(
                &format!("/organization/projects/{}/api_keys", self.project_id),
                req,
            )
            .await
    }
}

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

    pub async fn retrieve(&self) -> OpenAIResponse<api_keys::ProjectApiKey> {
        self.openai
            .get(
                &format!(
                    "/organization/projects/{}/api_keys/{}",
                    self.project_id, self.id
                ),
                &(),
            )
            .await
    }

    pub async fn delete(&self) -> OpenAIResponse<api_keys::DeleteProjectApiKeyResponse> {
        self.openai
            .delete(
                &format!(
                    "/organization/projects/{}/api_keys/{}",
                    self.project_id, self.id
                ),
                &(),
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::projects::api_keys::ListProjectApiKeysRequest;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn list_deserializes_owner_with_nested_user() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/projects/proj_1/api_keys"))
            .and(query_param("limit", "10"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{
                    "id": "key_1",
                    "created_at": 1700000000_i64,
                    "name": "Dev Key",
                    "object": "organization.project.api_key",
                    "owner": {
                        "type": "user",
                        "user": {
                            "id": "user_1",
                            "created_at": 1700000000_i64,
                            "email": "dev@example.com",
                            "name": "Dev User",
                            "role": "owner"
                        }
                    },
                    "owner_project_access": "active",
                    "redacted_value": "sk-...abcd"
                }],
                "first_id": "key_1",
                "last_id": "key_1",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = ListProjectApiKeysRequest::builder().limit(10).build();
        let result = client
            .organization()
            .project("proj_1")
            .api_keys()
            .list(&req)
            .await
            .unwrap();

        assert_eq!(result.data.len(), 1);
        let owner = &result.data[0].owner;
        assert_eq!(owner.kind.as_deref(), Some("user"));
        assert_eq!(owner.user.as_ref().unwrap().email, "dev@example.com");
    }

    #[tokio::test]
    async fn retrieve_deserializes_service_account_owner() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/projects/proj_1/api_keys/key_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "key_1",
                "created_at": 1700000000_i64,
                "name": "Service Key",
                "object": "organization.project.api_key",
                "owner": {
                    "type": "service_account",
                    "service_account": {
                        "id": "sa_1",
                        "created_at": 1700000000_i64,
                        "name": "CI Bot",
                        "role": "member"
                    }
                },
                "owner_project_access": "active",
                "redacted_value": "sk-...wxyz"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .project("proj_1")
            .api_key("key_1")
            .retrieve()
            .await
            .unwrap();

        assert_eq!(result.owner.kind.as_deref(), Some("service_account"));
        assert_eq!(
            result.owner.service_account.as_ref().unwrap().name,
            "CI Bot"
        );
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/organization/projects/proj_1/api_keys/key_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "key_1",
                "deleted": true,
                "object": "organization.project.api_key.deleted"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .project("proj_1")
            .api_key("key_1")
            .delete()
            .await
            .unwrap();

        assert!(result.deleted);
    }
}
