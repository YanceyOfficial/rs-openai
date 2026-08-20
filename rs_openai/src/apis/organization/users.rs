//! Organization users (`/organization/users`) and their assigned roles -- see
//! docs/superpowers/specs/2026-08-20-org-users-invites-groups-design.md.
//! Role assignment (`OrganizationUserRoles`) is not yet implemented -- deferred
//! to a future slice.

use crate::client::OpenAI;
use crate::interfaces::organization::users;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationUsers<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationUsers<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Lists organization users.
    pub async fn list(
        &self,
        req: &users::ListOrganizationUsersRequest,
    ) -> OpenAIResponse<users::ListOrganizationUsersResponse> {
        self.openai.get("/organization/users", req).await
    }
}

pub struct OrganizationUser<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> OrganizationUser<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Retrieves an organization user by ID.
    pub async fn retrieve(&self) -> OpenAIResponse<users::OrganizationUser> {
        self.openai
            .get(&format!("/organization/users/{}", self.id), &())
            .await
    }

    /// Updates an organization user's role or other mutable attributes.
    pub async fn update(
        &self,
        req: &users::UpdateOrganizationUserRequest,
    ) -> OpenAIResponse<users::OrganizationUser> {
        self.openai
            .post(&format!("/organization/users/{}", self.id), req)
            .await
    }

    /// Removes an organization user.
    pub async fn delete(&self) -> OpenAIResponse<users::DeleteOrganizationUserResponse> {
        self.openai
            .delete(&format!("/organization/users/{}", self.id), &())
            .await
    }

    /// Scoped accessor for this user's assigned roles. Not yet implemented.
    pub fn roles(&self) -> OrganizationUserRoles<'a> {
        OrganizationUserRoles::new(self.openai, self.id.clone())
    }
}

#[allow(dead_code)]
pub struct OrganizationUserRoles<'a> {
    openai: &'a OpenAI,
    user_id: String,
}

impl<'a> OrganizationUserRoles<'a> {
    pub(crate) fn new(openai: &'a OpenAI, user_id: String) -> Self {
        Self { openai, user_id }
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::users::{
        ListOrganizationUsersRequest, UpdateOrganizationUserRequest,
    };
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn retrieve_deserializes_nested_user_and_projects() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/users/user_abc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "user_abc",
                "added_at": 1700000000_i64,
                "object": "organization.user",
                "email": "dev@example.com",
                "name": "Dev User",
                "role": "owner",
                "is_service_account": false,
                "projects": {
                    "object": "list",
                    "data": [{"id": "proj_1", "name": "Default", "role": "owner"}]
                },
                "user": {
                    "id": "user_abc",
                    "object": "user",
                    "email": "dev@example.com",
                    "banned": false,
                    "enabled": true
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-admin-test").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .user("user_abc")
            .retrieve()
            .await
            .unwrap();

        assert_eq!(result.id, "user_abc");
        assert_eq!(result.role.as_deref(), Some("owner"));
        let projects = result.projects.unwrap();
        assert_eq!(projects.data.len(), 1);
        assert_eq!(projects.data[0].id.as_deref(), Some("proj_1"));
        let nested = result.user.unwrap();
        assert_eq!(nested.id, "user_abc");
        assert_eq!(nested.banned, Some(false));
    }

    #[tokio::test]
    async fn update_serializes_only_provided_fields() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/users/user_abc"))
            .and(body_json(serde_json::json!({"role": "reader"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "user_abc",
                "added_at": 1700000000_i64,
                "object": "organization.user",
                "role": "reader"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-admin-test").with_base_url(mock_server.uri());
        let req = UpdateOrganizationUserRequest::builder()
            .role("reader")
            .build();
        let result = client
            .organization()
            .user("user_abc")
            .update(&req)
            .await
            .unwrap();

        assert_eq!(result.role.as_deref(), Some("reader"));
    }

    #[tokio::test]
    async fn list_serializes_query_params_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/users"))
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

        let client = OpenAI::new("sk-admin-test").with_base_url(mock_server.uri());
        let req = ListOrganizationUsersRequest::builder().limit(10).build();
        let result = client.organization().users().list(&req).await.unwrap();

        assert!(!result.has_more);
        assert!(result.data.is_empty());
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/organization/users/user_abc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "user_abc",
                "deleted": true,
                "object": "organization.user.deleted"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-admin-test").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .user("user_abc")
            .delete()
            .await
            .unwrap();

        assert!(result.deleted);
    }
}
