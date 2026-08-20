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
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::projects::users::{
        AddProjectUserRequest, ListProjectUsersRequest, UpdateProjectUserRequest,
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
}
