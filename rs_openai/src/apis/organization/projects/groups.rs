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
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::chat::SortOrder;
    use crate::interfaces::organization::projects::groups::{
        AddProjectGroupRequest, ListProjectGroupsRequest,
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
}
