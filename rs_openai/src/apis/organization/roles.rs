//! Organization roles (`/organization/roles`): create, retrieve, update, list,
//! delete -- see docs/superpowers/specs/2026-08-20-org-roles-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::roles;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationRoles<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationRoles<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    pub async fn create(&self, req: &roles::CreateRoleRequest) -> OpenAIResponse<roles::Role> {
        self.openai.post("/organization/roles", req).await
    }

    pub async fn list(
        &self,
        req: &roles::ListRolesRequest,
    ) -> OpenAIResponse<roles::ListRolesResponse> {
        self.openai.get("/organization/roles", req).await
    }
}

pub struct OrganizationRole<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> OrganizationRole<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    pub async fn retrieve(&self) -> OpenAIResponse<roles::Role> {
        self.openai
            .get(&format!("/organization/roles/{}", self.id), &())
            .await
    }

    pub async fn update(&self, req: &roles::UpdateRoleRequest) -> OpenAIResponse<roles::Role> {
        self.openai
            .post(&format!("/organization/roles/{}", self.id), req)
            .await
    }

    pub async fn delete(&self) -> OpenAIResponse<roles::DeleteRoleResponse> {
        self.openai
            .delete(&format!("/organization/roles/{}", self.id), &())
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::chat::SortOrder;
    use crate::interfaces::organization::roles::{
        CreateRoleRequest, ListRolesRequest, UpdateRoleRequest,
    };
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/roles"))
            .and(body_json(serde_json::json!({
                "permissions": ["api.model.request"],
                "role_name": "Custom Role"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "role_1",
                "name": "Custom Role",
                "object": "role",
                "permissions": ["api.model.request"],
                "predefined_role": false,
                "resource_type": "api.organization"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = CreateRoleRequest::builder()
            .permissions(vec!["api.model.request".to_string()])
            .role_name("Custom Role")
            .build();
        let result = client.organization().roles().create(&req).await.unwrap();

        assert_eq!(result.id, "role_1");
        assert!(!result.predefined_role);
    }

    #[tokio::test]
    async fn retrieve_deserializes_role() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/roles/role_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "role_1",
                "name": "Custom Role",
                "object": "role",
                "permissions": ["api.model.request"],
                "predefined_role": false,
                "resource_type": "api.organization",
                "description": "A custom role"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .role("role_1")
            .retrieve()
            .await
            .unwrap();

        assert_eq!(result.description.as_deref(), Some("A custom role"));
    }

    #[tokio::test]
    async fn update_serializes_only_provided_fields() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/roles/role_1"))
            .and(body_json(serde_json::json!({"role_name": "Renamed Role"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "role_1",
                "name": "Renamed Role",
                "object": "role",
                "permissions": ["api.model.request"],
                "predefined_role": false,
                "resource_type": "api.organization"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UpdateRoleRequest::builder()
            .role_name("Renamed Role")
            .build();
        let result = client
            .organization()
            .role("role_1")
            .update(&req)
            .await
            .unwrap();

        assert_eq!(result.name, "Renamed Role");
    }

    #[tokio::test]
    async fn list_serializes_order_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/roles"))
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
        let req = ListRolesRequest::builder().order(SortOrder::Desc).build();
        let result = client.organization().roles().list(&req).await.unwrap();

        assert!(result.data.is_empty());
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/organization/roles/role_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "role_1",
                "deleted": true,
                "object": "role.deleted"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client.organization().role("role_1").delete().await.unwrap();

        assert!(result.deleted);
    }
}
