//! A project's groups (`/organization/projects/{project_id}/groups`): create, list,
//! retrieve, delete -- see docs/superpowers/specs/2026-08-20-org-projects-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::projects::groups;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationProjectGroups<'a> {
    openai: &'a OpenAI,
    project_id: String,
}

impl<'a> OrganizationProjectGroups<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String) -> Self {
        Self { openai, project_id }
    }

    pub async fn create(
        &self,
        req: &groups::AddProjectGroupRequest,
    ) -> OpenAIResponse<groups::ProjectGroup> {
        self.openai
            .post(
                &format!("/organization/projects/{}/groups", self.project_id),
                req,
            )
            .await
    }

    pub async fn list(
        &self,
        req: &groups::ListProjectGroupsRequest,
    ) -> OpenAIResponse<groups::ListProjectGroupsResponse> {
        self.openai
            .get(
                &format!("/organization/projects/{}/groups", self.project_id),
                req,
            )
            .await
    }

    pub fn group(&self, id: impl Into<String>) -> OrganizationProjectGroup<'a> {
        OrganizationProjectGroup::new(self.openai, self.project_id.clone(), id.into())
    }
}

pub struct OrganizationProjectGroup<'a> {
    openai: &'a OpenAI,
    project_id: String,
    id: String,
}

impl<'a> OrganizationProjectGroup<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String, id: String) -> Self {
        Self {
            openai,
            project_id,
            id,
        }
    }

    pub async fn retrieve(&self) -> OpenAIResponse<groups::ProjectGroup> {
        self.openai
            .get(
                &format!(
                    "/organization/projects/{}/groups/{}",
                    self.project_id, self.id
                ),
                &(),
            )
            .await
    }

    pub async fn delete(&self) -> OpenAIResponse<groups::RemoveProjectGroupResponse> {
        self.openai
            .delete(
                &format!(
                    "/organization/projects/{}/groups/{}",
                    self.project_id, self.id
                ),
                &(),
            )
            .await
    }

    /// Scoped accessor for this group's assigned project roles.
    pub fn roles(&self) -> OrganizationProjectGroupRoles<'a> {
        OrganizationProjectGroupRoles::new(self.openai, self.project_id.clone(), self.id.clone())
    }
}

/// Note: this sub-resource's routes are confirmed to NOT carry the usual
/// `/organization` prefix every other Projects endpoint has -- verified directly
/// against the SDK source, not assumed.
pub struct OrganizationProjectGroupRoles<'a> {
    openai: &'a OpenAI,
    project_id: String,
    group_id: String,
}

impl<'a> OrganizationProjectGroupRoles<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String, group_id: String) -> Self {
        Self {
            openai,
            project_id,
            group_id,
        }
    }

    pub async fn create(
        &self,
        req: &groups::AssignProjectGroupRoleRequest,
    ) -> OpenAIResponse<groups::AssignProjectGroupRoleResponse> {
        self.openai
            .post(
                &format!(
                    "/projects/{}/groups/{}/roles",
                    self.project_id, self.group_id
                ),
                req,
            )
            .await
    }

    pub async fn list(
        &self,
        req: &groups::ListProjectGroupRolesRequest,
    ) -> OpenAIResponse<groups::ListProjectGroupRolesResponse> {
        self.openai
            .get(
                &format!(
                    "/projects/{}/groups/{}/roles",
                    self.project_id, self.group_id
                ),
                req,
            )
            .await
    }

    pub fn role(&self, id: impl Into<String>) -> OrganizationProjectGroupRole<'a> {
        OrganizationProjectGroupRole::new(
            self.openai,
            self.project_id.clone(),
            self.group_id.clone(),
            id.into(),
        )
    }
}

pub struct OrganizationProjectGroupRole<'a> {
    openai: &'a OpenAI,
    project_id: String,
    group_id: String,
    id: String,
}

impl<'a> OrganizationProjectGroupRole<'a> {
    pub(crate) fn new(
        openai: &'a OpenAI,
        project_id: String,
        group_id: String,
        id: String,
    ) -> Self {
        Self {
            openai,
            project_id,
            group_id,
            id,
        }
    }

    pub async fn retrieve(
        &self,
    ) -> OpenAIResponse<crate::interfaces::organization::users::RoleAssignmentDetail> {
        self.openai
            .get(
                &format!(
                    "/projects/{}/groups/{}/roles/{}",
                    self.project_id, self.group_id, self.id
                ),
                &(),
            )
            .await
    }

    pub async fn delete(&self) -> OpenAIResponse<groups::RemoveProjectGroupRoleResponse> {
        self.openai
            .delete(
                &format!(
                    "/projects/{}/groups/{}/roles/{}",
                    self.project_id, self.group_id, self.id
                ),
                &(),
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::chat::SortOrder;
    use crate::interfaces::organization::projects::groups::{
        AddProjectGroupRequest, AssignProjectGroupRoleRequest, ListProjectGroupRolesRequest,
        ListProjectGroupsRequest,
    };
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/projects/proj_1/groups"))
            .and(body_json(
                serde_json::json!({"group_id": "group_1", "role": "member"}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "created_at": 1700000000_i64,
                "group_id": "group_1",
                "group_name": "Engineering",
                "group_type": "group",
                "object": "project.group",
                "project_id": "proj_1"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = AddProjectGroupRequest::builder()
            .group_id("group_1")
            .role("member")
            .build();
        let result = client
            .organization()
            .project("proj_1")
            .groups()
            .create(&req)
            .await
            .unwrap();

        assert_eq!(result.group_name, "Engineering");
    }

    #[tokio::test]
    async fn retrieve_deserializes_group() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/projects/proj_1/groups/group_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "created_at": 1700000000_i64,
                "group_id": "group_1",
                "group_name": "Engineering",
                "group_type": "group",
                "object": "project.group",
                "project_id": "proj_1"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .project("proj_1")
            .groups()
            .group("group_1")
            .retrieve()
            .await
            .unwrap();

        assert_eq!(result.group_type, "group");
    }

    #[tokio::test]
    async fn list_serializes_order_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/projects/proj_1/groups"))
            .and(query_param("order", "desc"))
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
        let req = ListProjectGroupsRequest::builder()
            .order(SortOrder::Desc)
            .build();
        let result = client
            .organization()
            .project("proj_1")
            .groups()
            .list(&req)
            .await
            .unwrap();

        assert!(result.data.is_empty());
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation_without_id() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/organization/projects/proj_1/groups/group_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "deleted": true,
                "object": "project.group.deleted"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .project("proj_1")
            .groups()
            .group("group_1")
            .delete()
            .await
            .unwrap();

        assert!(result.deleted);
        // RemoveProjectGroupResponse has no `id` field -- this test compiling and
        // passing is itself the proof that the asymmetry is modeled correctly.
    }

    #[tokio::test]
    async fn group_roles_create_uses_route_without_organization_prefix() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/projects/proj_1/groups/group_1/roles"))
            .and(body_json(serde_json::json!({"role_id": "role_1"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "group": {
                    "id": "group_1",
                    "created_at": 1700000000_i64,
                    "name": "Engineering",
                    "object": "group",
                    "scim_managed": false
                },
                "object": "group.role",
                "role": {
                    "id": "role_1",
                    "name": "Custom Role",
                    "object": "role",
                    "permissions": ["api.model.request"],
                    "predefined_role": false,
                    "resource_type": "api.organization"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = AssignProjectGroupRoleRequest::builder()
            .role_id("role_1")
            .build();
        let result = client
            .organization()
            .project("proj_1")
            .groups()
            .group("group_1")
            .roles()
            .create(&req)
            .await
            .unwrap();

        assert_eq!(result.group.id, "group_1");
        assert_eq!(result.role.id, "role_1");
    }

    #[tokio::test]
    async fn group_roles_list_uses_route_without_organization_prefix() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/projects/proj_1/groups/group_1/roles"))
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
        let req = ListProjectGroupRolesRequest::builder().limit(5).build();
        let result = client
            .organization()
            .project("proj_1")
            .groups()
            .group("group_1")
            .roles()
            .list(&req)
            .await
            .unwrap();

        assert_eq!(result.data.len(), 1);
    }

    #[tokio::test]
    async fn group_role_retrieve_uses_route_without_organization_prefix() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/projects/proj_1/groups/group_1/roles/role_1"))
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
            .groups()
            .group("group_1")
            .roles()
            .role("role_1")
            .retrieve()
            .await
            .unwrap();

        assert_eq!(result.name, "Custom Role");
    }

    #[tokio::test]
    async fn group_role_delete_uses_route_without_organization_prefix_and_no_id() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/projects/proj_1/groups/group_1/roles/role_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "deleted": true,
                "object": "group.role.deleted"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .project("proj_1")
            .groups()
            .group("group_1")
            .roles()
            .role("role_1")
            .delete()
            .await
            .unwrap();

        assert!(result.deleted);
    }
}
