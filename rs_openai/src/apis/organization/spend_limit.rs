//! Organization hard spend limit (`/organization/spend_limit`): retrieve, update, delete --
//! see docs/superpowers/specs/2026-08-25-org-billing-compliance-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::spend_limit;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationSpendLimit<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationSpendLimit<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Gets the organization's hard spend limit.
    pub async fn retrieve(&self) -> OpenAIResponse<spend_limit::SpendLimit> {
        self.openai.get("/organization/spend_limit", &()).await
    }

    /// Creates or replaces the organization's hard spend limit.
    pub async fn update(
        &self,
        req: &spend_limit::UpdateSpendLimitRequest,
    ) -> OpenAIResponse<spend_limit::SpendLimit> {
        self.openai.post("/organization/spend_limit", req).await
    }

    /// Deletes the organization's hard spend limit.
    pub async fn delete(&self) -> OpenAIResponse<spend_limit::SpendLimitDeleted> {
        self.openai.delete("/organization/spend_limit", &()).await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::spend_limit::UpdateSpendLimitRequest;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn retrieve_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/spend_limit"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "currency": "USD",
                "enforcement": {"status": "enforcing"},
                "interval": "month",
                "object": "organization.spend_limit",
                "threshold_amount": 1000000
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .organization()
            .spend_limit()
            .retrieve()
            .await
            .unwrap();
        assert_eq!(res.enforcement.status, "enforcing");
    }

    #[tokio::test]
    async fn update_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/spend_limit"))
            .and(body_json(serde_json::json!({
                "currency": "USD",
                "interval": "month",
                "threshold_amount": 2000000
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "currency": "USD",
                "enforcement": {"status": "inactive"},
                "interval": "month",
                "object": "organization.spend_limit",
                "threshold_amount": 2000000
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = UpdateSpendLimitRequest::builder()
            .currency("USD")
            .interval("month")
            .threshold_amount(2000000)
            .build();
        let res = client
            .organization()
            .spend_limit()
            .update(&req)
            .await
            .unwrap();
        assert_eq!(res.threshold_amount, 2000000);
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/organization/spend_limit"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "deleted": true,
                "object": "organization.spend_limit.deleted"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client.organization().spend_limit().delete().await.unwrap();
        assert!(res.deleted);
    }
}
