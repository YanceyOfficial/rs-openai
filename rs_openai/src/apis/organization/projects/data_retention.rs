//! Project-scoped data retention controls
//! (`/organization/projects/{project_id}/data_retention`) -- see
//! docs/superpowers/specs/2026-08-25-org-billing-compliance-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::data_retention;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationProjectDataRetention<'a> {
    openai: &'a OpenAI,
    project_id: String,
}

impl<'a> OrganizationProjectDataRetention<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String) -> Self {
        Self { openai, project_id }
    }

    /// Retrieves this project's data retention controls.
    pub async fn retrieve(&self) -> OpenAIResponse<data_retention::DataRetention> {
        self.openai
            .get(
                &format!("/organization/projects/{}/data_retention", self.project_id),
                &(),
            )
            .await
    }

    /// Updates this project's data retention controls.
    pub async fn update(
        &self,
        req: &data_retention::UpdateDataRetentionRequest,
    ) -> OpenAIResponse<data_retention::DataRetention> {
        self.openai
            .post(
                &format!("/organization/projects/{}/data_retention", self.project_id),
                req,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::data_retention::{
        DataRetentionType, UpdateDataRetentionRequest,
    };
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn retrieve_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/projects/proj_1/data_retention"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "project.data_retention",
                "type": "organization_default"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .organization()
            .project("proj_1")
            .data_retention()
            .retrieve()
            .await
            .unwrap();
        assert_eq!(res.r#type.to_string(), "organization_default");
    }

    #[tokio::test]
    async fn update_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/projects/proj_1/data_retention"))
            .and(body_json(serde_json::json!({"retention_type": "none"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "project.data_retention",
                "type": "none"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = UpdateDataRetentionRequest::builder()
            .retention_type(DataRetentionType::None)
            .build();
        let res = client
            .organization()
            .project("proj_1")
            .data_retention()
            .update(&req)
            .await
            .unwrap();
        assert_eq!(res.r#type.to_string(), "none");
    }
}
