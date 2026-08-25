//! Organization data retention controls (`/organization/data_retention`) -- see
//! docs/superpowers/specs/2026-08-25-org-billing-compliance-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::data_retention;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationDataRetention<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationDataRetention<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Retrieves organization data retention controls.
    pub async fn retrieve(&self) -> OpenAIResponse<data_retention::DataRetention> {
        self.openai.get("/organization/data_retention", &()).await
    }

    /// Updates organization data retention controls.
    pub async fn update(
        &self,
        req: &data_retention::UpdateDataRetentionRequest,
    ) -> OpenAIResponse<data_retention::DataRetention> {
        self.openai.post("/organization/data_retention", req).await
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
            .and(path("/organization/data_retention"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "organization.data_retention",
                "type": "zero_data_retention"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .organization()
            .data_retention()
            .retrieve()
            .await
            .unwrap();
        assert_eq!(res.r#type.to_string(), "zero_data_retention");
    }

    #[tokio::test]
    async fn update_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/data_retention"))
            .and(body_json(
                serde_json::json!({"retention_type": "enhanced_zero_data_retention"}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "organization.data_retention",
                "type": "enhanced_zero_data_retention"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = UpdateDataRetentionRequest::builder()
            .retention_type(DataRetentionType::EnhancedZeroDataRetention)
            .build();
        let res = client
            .organization()
            .data_retention()
            .update(&req)
            .await
            .unwrap();
        assert_eq!(res.r#type.to_string(), "enhanced_zero_data_retention");
    }
}
