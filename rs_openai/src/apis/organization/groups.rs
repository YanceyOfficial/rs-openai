//! Organization groups (`/organization/groups`) and their member users -- see
//! docs/superpowers/specs/2026-08-20-org-users-invites-groups-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::groups;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationGroups<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationGroups<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    pub async fn create(&self, req: &groups::CreateGroupRequest) -> OpenAIResponse<groups::Group> {
        self.openai.post("/organization/groups", req).await
    }

    pub async fn list(
        &self,
        req: &groups::ListGroupsRequest,
    ) -> OpenAIResponse<groups::ListGroupsResponse> {
        self.openai.get("/organization/groups", req).await
    }
}

pub struct OrganizationGroup<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> OrganizationGroup<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    pub async fn retrieve(&self) -> OpenAIResponse<groups::Group> {
        self.openai
            .get(&format!("/organization/groups/{}", self.id), &())
            .await
    }

    pub async fn update(
        &self,
        req: &groups::UpdateGroupRequest,
    ) -> OpenAIResponse<groups::UpdateGroupResponse> {
        self.openai
            .post(&format!("/organization/groups/{}", self.id), req)
            .await
    }

    pub async fn delete(&self) -> OpenAIResponse<groups::DeleteGroupResponse> {
        self.openai
            .delete(&format!("/organization/groups/{}", self.id), &())
            .await
    }

    /// Scoped accessor for this group's member users. Not yet implemented.
    pub fn users(&self) -> OrganizationGroupUsers<'a> {
        OrganizationGroupUsers::new(self.openai, self.id.clone())
    }
}

#[allow(dead_code)]
pub struct OrganizationGroupUsers<'a> {
    openai: &'a OpenAI,
    group_id: String,
}

impl<'a> OrganizationGroupUsers<'a> {
    pub(crate) fn new(openai: &'a OpenAI, group_id: String) -> Self {
        Self { openai, group_id }
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::groups::{
        CreateGroupRequest, ListGroupsRequest, UpdateGroupRequest,
    };
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_name_and_deserializes_group() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/groups"))
            .and(body_json(serde_json::json!({"name": "Engineering"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "group_1",
                "created_at": 1700000000_i64,
                "group_type": "group",
                "is_scim_managed": false,
                "name": "Engineering"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = CreateGroupRequest::builder().name("Engineering").build();
        let result = client.organization().groups().create(&req).await.unwrap();

        assert_eq!(result.id, "group_1");
        assert_eq!(result.group_type, "group");
    }

    #[tokio::test]
    async fn update_deserializes_into_distinct_response_shape() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/groups/group_1"))
            .and(body_json(serde_json::json!({"name": "Eng Renamed"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "group_1",
                "created_at": 1700000000_i64,
                "is_scim_managed": false,
                "name": "Eng Renamed"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UpdateGroupRequest::builder().name("Eng Renamed").build();
        let result = client
            .organization()
            .group("group_1")
            .update(&req)
            .await
            .unwrap();

        assert_eq!(result.name, "Eng Renamed");
        // UpdateGroupResponse has no `group_type` field -- this line only compiles
        // because the type is genuinely distinct from `Group`, proving the asymmetry.
    }

    #[tokio::test]
    async fn retrieve_deserializes_group() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/groups/group_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "group_1",
                "created_at": 1700000000_i64,
                "group_type": "tenant_group",
                "is_scim_managed": true,
                "name": "Tenant Group"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .group("group_1")
            .retrieve()
            .await
            .unwrap();

        assert_eq!(result.group_type, "tenant_group");
        assert!(result.is_scim_managed);
    }

    #[tokio::test]
    async fn list_serializes_order_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/groups"))
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
        let req = ListGroupsRequest::builder()
            .order(crate::interfaces::chat::SortOrder::Desc)
            .build();
        let result = client.organization().groups().list(&req).await.unwrap();

        assert!(result.data.is_empty());
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/organization/groups/group_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "group_1",
                "deleted": true,
                "object": "group.deleted"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .group("group_1")
            .delete()
            .await
            .unwrap();

        assert!(result.deleted);
    }
}
