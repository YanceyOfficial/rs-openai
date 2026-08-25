//! Project-scoped hard spend limit (`/organization/projects/{project_id}/spend_limit`):
//! retrieve, update, delete -- see
//! docs/superpowers/specs/2026-08-25-org-billing-compliance-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::spend_limit;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationProjectSpendLimit<'a> {
    openai: &'a OpenAI,
    project_id: String,
}

impl<'a> OrganizationProjectSpendLimit<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String) -> Self {
        Self { openai, project_id }
    }

    /// Gets this project's hard spend limit.
    pub async fn retrieve(&self) -> OpenAIResponse<spend_limit::SpendLimit> {
        self.openai
            .get(
                &format!("/organization/projects/{}/spend_limit", self.project_id),
                &(),
            )
            .await
    }

    /// Creates or replaces this project's hard spend limit.
    pub async fn update(
        &self,
        req: &spend_limit::UpdateSpendLimitRequest,
    ) -> OpenAIResponse<spend_limit::SpendLimit> {
        self.openai
            .post(
                &format!("/organization/projects/{}/spend_limit", self.project_id),
                req,
            )
            .await
    }

    /// Deletes this project's hard spend limit.
    pub async fn delete(&self) -> OpenAIResponse<spend_limit::SpendLimitDeleted> {
        self.openai
            .delete(
                &format!("/organization/projects/{}/spend_limit", self.project_id),
                &(),
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::spend_limit::UpdateSpendLimitRequest;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn retrieve_update_delete_use_correct_routes() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/projects/proj_1/spend_limit"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "currency": "USD",
                "enforcement": {"status": "enforcing"},
                "interval": "month",
                "object": "project.spend_limit",
                "threshold_amount": 500000
            })))
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path("/organization/projects/proj_1/spend_limit"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "currency": "USD",
                "enforcement": {"status": "enforcing"},
                "interval": "month",
                "object": "project.spend_limit",
                "threshold_amount": 750000
            })))
            .mount(&mock_server)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/organization/projects/proj_1/spend_limit"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "deleted": true,
                "object": "project.spend_limit.deleted"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let spend_limit = client.organization().project("proj_1").spend_limit();

        assert_eq!(
            spend_limit.retrieve().await.unwrap().threshold_amount,
            500000
        );
        let req = UpdateSpendLimitRequest::builder()
            .currency("USD")
            .interval("month")
            .threshold_amount(750000)
            .build();
        assert_eq!(
            spend_limit.update(&req).await.unwrap().threshold_amount,
            750000
        );
        assert!(spend_limit.delete().await.unwrap().deleted);
    }
}
