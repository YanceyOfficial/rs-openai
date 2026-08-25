//! Organization spend alerts (`/organization/spend_alerts`): create, retrieve, update, list,
//! delete -- see docs/superpowers/specs/2026-08-25-org-billing-compliance-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::spend_alerts;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationSpendAlerts<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationSpendAlerts<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates an organization spend alert.
    pub async fn create(
        &self,
        req: &spend_alerts::SpendAlertRequest,
    ) -> OpenAIResponse<spend_alerts::SpendAlert> {
        self.openai.post("/organization/spend_alerts", req).await
    }

    /// Lists organization spend alerts.
    pub async fn list(
        &self,
        req: &spend_alerts::ListSpendAlertsRequest,
    ) -> OpenAIResponse<spend_alerts::ListSpendAlertsResponse> {
        self.openai.get("/organization/spend_alerts", req).await
    }
}

pub struct OrganizationSpendAlert<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> OrganizationSpendAlert<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Retrieves an organization spend alert.
    pub async fn retrieve(&self) -> OpenAIResponse<spend_alerts::SpendAlert> {
        self.openai
            .get(&format!("/organization/spend_alerts/{}", self.id), &())
            .await
    }

    /// Updates an organization spend alert (full replace).
    pub async fn update(
        &self,
        req: &spend_alerts::SpendAlertRequest,
    ) -> OpenAIResponse<spend_alerts::SpendAlert> {
        self.openai
            .post(&format!("/organization/spend_alerts/{}", self.id), req)
            .await
    }

    /// Deletes an organization spend alert.
    pub async fn delete(&self) -> OpenAIResponse<spend_alerts::SpendAlertDeleted> {
        self.openai
            .delete(&format!("/organization/spend_alerts/{}", self.id), &())
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::spend_alerts::{
        ListSpendAlertsRequest, SpendAlertNotificationChannel, SpendAlertRequest,
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
            .and(path("/organization/spend_alerts"))
            .and(body_json(serde_json::json!({
                "currency": "USD",
                "interval": "month",
                "notification_channel": {
                    "recipients": ["ops@example.com"],
                    "type": "email"
                },
                "threshold_amount": 50000
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "alert_1",
                "currency": "USD",
                "interval": "month",
                "notification_channel": {"recipients": ["ops@example.com"], "type": "email"},
                "object": "organization.spend_alert",
                "threshold_amount": 50000
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .organization()
            .spend_alerts()
            .create(&sample_request())
            .await
            .unwrap();
        assert_eq!(res.id, "alert_1");
        assert_eq!(res.threshold_amount, 50000);
    }

    #[tokio::test]
    async fn list_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/spend_alerts"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [],
                "first_id": null,
                "last_id": null,
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ListSpendAlertsRequest::builder().limit(10_u32).build();
        let res = client
            .organization()
            .spend_alerts()
            .list(&req)
            .await
            .unwrap();
        assert!(res.data.is_empty());
    }

    #[tokio::test]
    async fn retrieve_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/spend_alerts/alert_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "alert_1",
                "currency": "USD",
                "interval": "month",
                "notification_channel": {"recipients": ["ops@example.com"], "type": "email"},
                "object": "organization.spend_alert",
                "threshold_amount": 50000
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .organization()
            .spend_alert("alert_1")
            .retrieve()
            .await
            .unwrap();
        assert_eq!(res.id, "alert_1");
    }

    #[tokio::test]
    async fn update_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/spend_alerts/alert_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "alert_1",
                "currency": "USD",
                "interval": "month",
                "notification_channel": {"recipients": ["ops@example.com"], "type": "email"},
                "object": "organization.spend_alert",
                "threshold_amount": 90000
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let mut req = sample_request();
        req.threshold_amount = 90000;
        let res = client
            .organization()
            .spend_alert("alert_1")
            .update(&req)
            .await
            .unwrap();
        assert_eq!(res.threshold_amount, 90000);
    }

    #[tokio::test]
    async fn delete_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/organization/spend_alerts/does-not-exist"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": {
                    "message": "No spend alert found",
                    "type": "invalid_request_error",
                    "param": null,
                    "code": "not_found"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let err = client
            .organization()
            .spend_alert("does-not-exist")
            .delete()
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            crate::shared::response_wrapper::OpenAIError::ApiError(_)
        ));
    }
}
