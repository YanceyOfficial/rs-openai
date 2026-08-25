//! Project-scoped spend alerts (`/organization/projects/{project_id}/spend_alerts`): create,
//! retrieve, update, list, delete -- see
//! docs/superpowers/specs/2026-08-25-org-billing-compliance-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::spend_alerts;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationProjectSpendAlerts<'a> {
    openai: &'a OpenAI,
    project_id: String,
}

impl<'a> OrganizationProjectSpendAlerts<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String) -> Self {
        Self { openai, project_id }
    }

    /// Creates a project spend alert.
    pub async fn create(
        &self,
        req: &spend_alerts::SpendAlertRequest,
    ) -> OpenAIResponse<spend_alerts::SpendAlert> {
        self.openai
            .post(
                &format!("/organization/projects/{}/spend_alerts", self.project_id),
                req,
            )
            .await
    }

    /// Lists project spend alerts.
    pub async fn list(
        &self,
        req: &spend_alerts::ListSpendAlertsRequest,
    ) -> OpenAIResponse<spend_alerts::ListSpendAlertsResponse> {
        self.openai
            .get(
                &format!("/organization/projects/{}/spend_alerts", self.project_id),
                req,
            )
            .await
    }
}

pub struct OrganizationProjectSpendAlert<'a> {
    openai: &'a OpenAI,
    project_id: String,
    id: String,
}

impl<'a> OrganizationProjectSpendAlert<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String, id: String) -> Self {
        Self {
            openai,
            project_id,
            id,
        }
    }

    /// Retrieves a project spend alert.
    pub async fn retrieve(&self) -> OpenAIResponse<spend_alerts::SpendAlert> {
        self.openai
            .get(
                &format!(
                    "/organization/projects/{}/spend_alerts/{}",
                    self.project_id, self.id
                ),
                &(),
            )
            .await
    }

    /// Updates a project spend alert (full replace).
    pub async fn update(
        &self,
        req: &spend_alerts::SpendAlertRequest,
    ) -> OpenAIResponse<spend_alerts::SpendAlert> {
        self.openai
            .post(
                &format!(
                    "/organization/projects/{}/spend_alerts/{}",
                    self.project_id, self.id
                ),
                req,
            )
            .await
    }

    /// Deletes a project spend alert.
    pub async fn delete(&self) -> OpenAIResponse<spend_alerts::SpendAlertDeleted> {
        self.openai
            .delete(
                &format!(
                    "/organization/projects/{}/spend_alerts/{}",
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
    use crate::interfaces::organization::spend_alerts::{
        SpendAlertNotificationChannel, SpendAlertRequest,
    };
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sample_request() -> SpendAlertRequest {
        SpendAlertRequest::builder()
            .currency("USD")
            .interval("month")
            .notification_channel(
                SpendAlertNotificationChannel::builder()
                    .recipients(vec!["ops@example.com".to_string()])
                    .r#type("email".to_string())
                    .build(),
            )
            .threshold_amount(50000)
            .build()
    }

    #[tokio::test]
    async fn create_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/projects/proj_1/spend_alerts"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "alert_1",
                "currency": "USD",
                "interval": "month",
                "notification_channel": {"recipients": ["ops@example.com"], "type": "email"},
                "object": "project.spend_alert",
                "threshold_amount": 50000
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .organization()
            .project("proj_1")
            .spend_alerts()
            .create(&sample_request())
            .await
            .unwrap();
        assert_eq!(res.object, "project.spend_alert");
    }

    #[tokio::test]
    async fn retrieve_and_update_and_delete_use_correct_routes() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/projects/proj_1/spend_alerts/alert_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "alert_1",
                "currency": "USD",
                "interval": "month",
                "notification_channel": {"recipients": ["ops@example.com"], "type": "email"},
                "object": "project.spend_alert",
                "threshold_amount": 50000
            })))
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path("/organization/projects/proj_1/spend_alerts/alert_1"))
            .and(body_json(serde_json::json!({
                "currency": "USD",
                "interval": "month",
                "notification_channel": {"recipients": ["ops@example.com"], "type": "email"},
                "threshold_amount": 50000
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "alert_1",
                "currency": "USD",
                "interval": "month",
                "notification_channel": {"recipients": ["ops@example.com"], "type": "email"},
                "object": "project.spend_alert",
                "threshold_amount": 50000
            })))
            .mount(&mock_server)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/organization/projects/proj_1/spend_alerts/alert_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "alert_1",
                "deleted": true,
                "object": "project.spend_alert.deleted"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let alert = client
            .organization()
            .project("proj_1")
            .spend_alert("alert_1");

        assert_eq!(alert.retrieve().await.unwrap().id, "alert_1");
        assert_eq!(alert.update(&sample_request()).await.unwrap().id, "alert_1");
        assert!(alert.delete().await.unwrap().deleted);
    }
}
