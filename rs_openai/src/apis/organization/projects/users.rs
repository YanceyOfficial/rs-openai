//! A project's users (`/organization/projects/{project_id}/users`): create, list,
//! retrieve, update, delete -- see docs/superpowers/specs/2026-08-20-org-projects-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::projects::users;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationProjectUsers<'a> {
    openai: &'a OpenAI,
    project_id: String,
}

impl<'a> OrganizationProjectUsers<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String) -> Self {
        Self { openai, project_id }
    }

    pub async fn create(
        &self,
        req: &users::AddProjectUserRequest,
    ) -> OpenAIResponse<users::ProjectUser> {
        self.openai
            .post(
                &format!("/organization/projects/{}/users", self.project_id),
                req,
            )
            .await
    }

    pub async fn list(
        &self,
        req: &users::ListProjectUsersRequest,
    ) -> OpenAIResponse<users::ListProjectUsersResponse> {
        self.openai
            .get(
                &format!("/organization/projects/{}/users", self.project_id),
                req,
            )
            .await
    }
}

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

    pub async fn retrieve(&self) -> OpenAIResponse<users::ProjectUser> {
        self.openai
            .get(
                &format!(
                    "/organization/projects/{}/users/{}",
                    self.project_id, self.id
                ),
                &(),
            )
            .await
    }

    pub async fn update(
        &self,
        req: &users::UpdateProjectUserRequest,
    ) -> OpenAIResponse<users::ProjectUser> {
        self.openai
            .post(
                &format!(
                    "/organization/projects/{}/users/{}",
                    self.project_id, self.id
                ),
                req,
            )
            .await
    }

    pub async fn delete(&self) -> OpenAIResponse<users::DeleteProjectUserResponse> {
        self.openai
            .delete(
                &format!(
                    "/organization/projects/{}/users/{}",
                    self.project_id, self.id
                ),
                &(),
            )
            .await
    }

    /// Scoped accessor for this user's assigned project roles.
    pub fn roles(&self) -> OrganizationProjectUserRoles<'a> {
        OrganizationProjectUserRoles::new(self.openai, self.project_id.clone(), self.id.clone())
    }
}

/// Note: this sub-resource's routes are confirmed to NOT carry the usual
/// `/organization` prefix every other Projects endpoint has -- verified directly
/// against the SDK source, not assumed.
pub struct OrganizationProjectUserRoles<'a> {
    openai: &'a OpenAI,
    project_id: String,
    user_id: String,
}

impl<'a> OrganizationProjectUserRoles<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String, user_id: String) -> Self {
        Self {
            openai,
            project_id,
            user_id,
        }
    }

    pub async fn create(
        &self,
        req: &users::AssignProjectUserRoleRequest,
    ) -> OpenAIResponse<users::AssignProjectUserRoleResponse> {
        self.openai
            .post(
                &format!("/projects/{}/users/{}/roles", self.project_id, self.user_id),
                req,
            )
            .await
    }

    pub async fn list(
        &self,
        req: &users::ListProjectUserRolesRequest,
    ) -> OpenAIResponse<users::ListProjectUserRolesResponse> {
        self.openai
            .get(
                &format!("/projects/{}/users/{}/roles", self.project_id, self.user_id),
                req,
            )
            .await
    }

    pub fn role(&self, id: impl Into<String>) -> OrganizationProjectUserRole<'a> {
        OrganizationProjectUserRole::new(
            self.openai,
            self.project_id.clone(),
            self.user_id.clone(),
            id.into(),
        )
    }
}

pub struct OrganizationProjectUserRole<'a> {
    openai: &'a OpenAI,
    project_id: String,
    user_id: String,
    id: String,
}

impl<'a> OrganizationProjectUserRole<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String, user_id: String, id: String) -> Self {
        Self {
            openai,
            project_id,
            user_id,
            id,
        }
    }

    pub async fn retrieve(
        &self,
    ) -> OpenAIResponse<crate::interfaces::organization::users::RoleAssignmentDetail> {
        self.openai
            .get(
                &format!(
                    "/projects/{}/users/{}/roles/{}",
                    self.project_id, self.user_id, self.id
                ),
                &(),
            )
            .await
    }

    pub async fn delete(&self) -> OpenAIResponse<users::RemoveProjectUserRoleResponse> {
        self.openai
            .delete(
                &format!(
                    "/projects/{}/users/{}/roles/{}",
                    self.project_id, self.user_id, self.id
                ),
                &(),
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::projects::users::{
        AddProjectUserRequest, AssignProjectUserRoleRequest, ListProjectUserRolesRequest,
        ListProjectUsersRequest, UpdateProjectUserRequest,
    };
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/projects/proj_1/users"))
            .and(body_json(serde_json::json!({
                "role": "member",
                "email": "dev@example.com"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "user_1",
                "added_at": 1700000000_i64,
                "object": "organization.project.user",
                "role": "member",
                "email": "dev@example.com",
                "name": "Dev User"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = AddProjectUserRequest::builder()
            .role("member")
            .email("dev@example.com")
            .build();
        let result = client
            .organization()
            .project("proj_1")
            .users()
            .create(&req)
            .await
            .unwrap();

        assert_eq!(result.role, "member");
        assert_eq!(result.name.as_deref(), Some("Dev User"));
    }

    #[tokio::test]
    async fn retrieve_deserializes_user() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/projects/proj_1/users/user_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "user_1",
                "added_at": 1700000000_i64,
                "object": "organization.project.user",
                "role": "owner",
                "email": "dev@example.com"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .project("proj_1")
            .user("user_1")
            .retrieve()
            .await
            .unwrap();

        assert_eq!(result.role, "owner");
    }

    #[tokio::test]
    async fn update_serializes_role_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/projects/proj_1/users/user_1"))
            .and(body_json(serde_json::json!({"role": "owner"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "user_1",
                "added_at": 1700000000_i64,
                "object": "organization.project.user",
                "role": "owner",
                "email": "dev@example.com"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UpdateProjectUserRequest::builder().role("owner").build();
        let result = client
            .organization()
            .project("proj_1")
            .user("user_1")
            .update(&req)
            .await
            .unwrap();

        assert_eq!(result.role, "owner");
    }

    #[tokio::test]
    async fn list_serializes_query_params_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/projects/proj_1/users"))
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
        let req = ListProjectUsersRequest::builder().limit(10).build();
        let result = client
            .organization()
            .project("proj_1")
            .users()
            .list(&req)
            .await
            .unwrap();

        assert!(result.data.is_empty());
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/organization/projects/proj_1/users/user_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "user_1",
                "deleted": true,
                "object": "organization.project.user.deleted"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .project("proj_1")
            .user("user_1")
            .delete()
            .await
            .unwrap();

        assert!(result.deleted);
    }

    #[tokio::test]
    async fn user_roles_create_uses_route_without_organization_prefix() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/projects/proj_1/users/user_1/roles"))
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
                    "id": "user_1",
                    "added_at": 1700000000_i64,
                    "object": "organization.user"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = AssignProjectUserRoleRequest::builder()
            .role_id("role_1")
            .build();
        let result = client
            .organization()
            .project("proj_1")
            .user("user_1")
            .roles()
            .create(&req)
            .await
            .unwrap();

        assert_eq!(result.role.id, "role_1");
        assert_eq!(result.user.id, "user_1");
    }

    #[tokio::test]
    async fn user_roles_list_uses_route_without_organization_prefix() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/projects/proj_1/users/user_1/roles"))
            .and(query_param("limit", "5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{
                    "id": "role_1",
                    "name": "Custom Role",
                    "permissions": ["api.model.request"],
                    "predefined_role": false,
                    "resource_type": "api.organization"
                }],
                "first_id": "role_1",
                "last_id": "role_1",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = ListProjectUserRolesRequest::builder().limit(5).build();
        let result = client
            .organization()
            .project("proj_1")
            .user("user_1")
            .roles()
            .list(&req)
            .await
            .unwrap();

        assert_eq!(result.data.len(), 1);
    }

    #[tokio::test]
    async fn user_role_retrieve_uses_route_without_organization_prefix() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/projects/proj_1/users/user_1/roles/role_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "role_1",
                "name": "Custom Role",
                "permissions": ["api.model.request"],
                "predefined_role": false,
                "resource_type": "api.organization"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .project("proj_1")
            .user("user_1")
            .roles()
            .role("role_1")
            .retrieve()
            .await
            .unwrap();

        assert_eq!(result.name, "Custom Role");
    }

    #[tokio::test]
    async fn user_role_delete_uses_route_without_organization_prefix_and_no_id() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/projects/proj_1/users/user_1/roles/role_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "deleted": true,
                "object": "user.role.deleted"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .project("proj_1")
            .user("user_1")
            .roles()
            .role("role_1")
            .delete()
            .await
            .unwrap();

        assert!(result.deleted);
    }
}
