//! Organization users (`/organization/users`) and their assigned roles -- see
//! docs/superpowers/specs/2026-08-20-org-users-invites-groups-design.md and
//! docs/superpowers/specs/2026-08-20-org-roles-design.md.

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

    /// Scoped accessor for this user's assigned roles.
    pub fn roles(&self) -> OrganizationUserRoles<'a> {
        OrganizationUserRoles::new(self.openai, self.id.clone())
    }
}

pub struct OrganizationUserRoles<'a> {
    openai: &'a OpenAI,
    user_id: String,
}

impl<'a> OrganizationUserRoles<'a> {
    pub(crate) fn new(openai: &'a OpenAI, user_id: String) -> Self {
        Self { openai, user_id }
    }

    pub async fn create(
        &self,
        req: &users::AssignUserRoleRequest,
    ) -> OpenAIResponse<users::AssignUserRoleResponse> {
        self.openai
            .post(&format!("/organization/users/{}/roles", self.user_id), req)
            .await
    }

    pub async fn list(
        &self,
        req: &users::ListUserRolesRequest,
    ) -> OpenAIResponse<users::ListUserRolesResponse> {
        self.openai
            .get(&format!("/organization/users/{}/roles", self.user_id), req)
            .await
    }

    pub fn role(&self, id: impl Into<String>) -> OrganizationUserRole<'a> {
        OrganizationUserRole::new(self.openai, self.user_id.clone(), id.into())
    }
}

pub struct OrganizationUserRole<'a> {
    openai: &'a OpenAI,
    user_id: String,
    id: String,
}

impl<'a> OrganizationUserRole<'a> {
    pub(crate) fn new(openai: &'a OpenAI, user_id: String, id: String) -> Self {
        Self {
            openai,
            user_id,
            id,
        }
    }

    pub async fn retrieve(&self) -> OpenAIResponse<users::RoleAssignmentDetail> {
        self.openai
            .get(
                &format!("/organization/users/{}/roles/{}", self.user_id, self.id),
                &(),
            )
            .await
    }

    pub async fn delete(&self) -> OpenAIResponse<users::RemoveUserRoleResponse> {
        self.openai
            .delete(
                &format!("/organization/users/{}/roles/{}", self.user_id, self.id),
                &(),
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::users::{
        AssignUserRoleRequest, ListOrganizationUsersRequest, ListUserRolesRequest,
        UpdateOrganizationUserRequest,
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

    #[tokio::test]
    async fn user_roles_create_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/users/user_abc/roles"))
            .and(body_json(serde_json::json!({"role_id": "role_1"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "user.role",
                "role": {
                    "id": "role_1",
                    "name": "Custom Role",
                    "object": "role",
                    "permissions": ["api.model.request"],
                    "predefined_role": false,
                    "resource_type": "api.organization"
                },
                "user": {
                    "id": "user_abc",
                    "added_at": 1700000000_i64,
                    "object": "organization.user"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-admin-test").with_base_url(mock_server.uri());
        let req = AssignUserRoleRequest::builder().role_id("role_1").build();
        let result = client
            .organization()
            .user("user_abc")
            .roles()
            .create(&req)
            .await
            .unwrap();

        assert_eq!(result.role.id, "role_1");
        assert_eq!(result.user.id, "user_abc");
    }

    #[tokio::test]
    async fn user_roles_list_deserializes_role_assignment_detail() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/users/user_abc/roles"))
            .and(query_param("limit", "5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{
                    "id": "role_1",
                    "name": "Custom Role",
                    "permissions": ["api.model.request"],
                    "predefined_role": false,
                    "resource_type": "api.organization",
                    "assignment_sources": [{"principal_id": "group_1", "principal_type": "group"}]
                }],
                "first_id": "role_1",
                "last_id": "role_1",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-admin-test").with_base_url(mock_server.uri());
        let req = ListUserRolesRequest::builder().limit(5).build();
        let result = client
            .organization()
            .user("user_abc")
            .roles()
            .list(&req)
            .await
            .unwrap();

        assert_eq!(result.data.len(), 1);
        let sources = result.data[0].assignment_sources.as_ref().unwrap();
        assert_eq!(sources[0].principal_type, "group");
    }

    #[tokio::test]
    async fn user_role_retrieve_deserializes_role_assignment_detail() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/users/user_abc/roles/role_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "role_1",
                "name": "Custom Role",
                "permissions": ["api.model.request"],
                "predefined_role": false,
                "resource_type": "api.organization"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-admin-test").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .user("user_abc")
            .roles()
            .role("role_1")
            .retrieve()
            .await
            .unwrap();

        assert_eq!(result.name, "Custom Role");
    }

    #[tokio::test]
    async fn user_role_delete_deserializes_confirmation_without_id() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/organization/users/user_abc/roles/role_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "deleted": true,
                "object": "user.role.deleted"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-admin-test").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .user("user_abc")
            .roles()
            .role("role_1")
            .delete()
            .await
            .unwrap();

        assert!(result.deleted);
    }
}
