//! Project-scoped hosted tool permissions
//! (`/organization/projects/{project_id}/hosted_tool_permissions`): retrieve, update -- see
//! docs/superpowers/specs/2026-08-25-org-billing-compliance-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::projects::hosted_tool_permissions;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationProjectHostedToolPermissions<'a> {
    openai: &'a OpenAI,
    project_id: String,
}

impl<'a> OrganizationProjectHostedToolPermissions<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String) -> Self {
        Self { openai, project_id }
    }

    /// Retrieves this project's hosted tool permissions.
    pub async fn retrieve(
        &self,
    ) -> OpenAIResponse<hosted_tool_permissions::ProjectHostedToolPermissions> {
        self.openai
            .get(
                &format!(
                    "/organization/projects/{}/hosted_tool_permissions",
                    self.project_id
                ),
                &(),
            )
            .await
    }

    /// Updates this project's hosted tool permissions (partial update).
    pub async fn update(
        &self,
        req: &hosted_tool_permissions::UpdateHostedToolPermissionsRequest,
    ) -> OpenAIResponse<hosted_tool_permissions::ProjectHostedToolPermissions> {
        self.openai
            .post(
                &format!(
                    "/organization/projects/{}/hosted_tool_permissions",
                    self.project_id
                ),
                req,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::projects::hosted_tool_permissions::{
        HostedToolPermissionState, UpdateHostedToolPermissionsRequest,
    };
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sample_response_json() -> serde_json::Value {
        serde_json::json!({
            "code_interpreter": {"enabled": true},
            "file_search": {"enabled": true},
            "image_generation": {"enabled": false},
            "mcp": {"enabled": true},
            "web_search": {"enabled": false}
        })
    }

    #[tokio::test]
    async fn retrieve_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path(
                "/organization/projects/proj_1/hosted_tool_permissions",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_response_json()))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .organization()
            .project("proj_1")
            .hosted_tool_permissions()
            .retrieve()
            .await
            .unwrap();
        assert!(res.code_interpreter.enabled);
        assert!(!res.web_search.enabled);
    }

    #[tokio::test]
    async fn update_serializes_only_provided_fields() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path(
                "/organization/projects/proj_1/hosted_tool_permissions",
            ))
            .and(body_json(
                serde_json::json!({"web_search": {"enabled": false}}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_response_json()))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = UpdateHostedToolPermissionsRequest::builder()
            .web_search(HostedToolPermissionState::builder().enabled(false).build())
            .build();
        let res = client
            .organization()
            .project("proj_1")
            .hosted_tool_permissions()
            .update(&req)
            .await
            .unwrap();
        assert!(!res.web_search.enabled);
    }
}
