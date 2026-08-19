//! Organization audit logs (`/organization/audit_logs`, list-only) -- see
//! docs/superpowers/specs/2026-08-19-org-admin-api-keys-audit-logs-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::audit_logs;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationAuditLogs<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationAuditLogs<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Lists organization audit log events.
    pub async fn list(
        &self,
        req: &audit_logs::ListAuditLogsRequest,
    ) -> OpenAIResponse<audit_logs::ListAuditLogsResponse> {
        self.openai.get("/organization/audit_logs", req).await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::audit_logs::ListAuditLogsRequest;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn list_deserializes_event_with_flattened_api_key_created_detail() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/audit_logs"))
            .and(query_param("limit", "5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{
                    "id": "audit_log_1",
                    "effective_at": 1700000000_i64,
                    "type": "api_key.created",
                    "actor": {
                        "type": "session",
                        "api_key": null,
                        "session": {
                            "ip_address": "203.0.113.1",
                            "user": {"id": "user_1", "email": "admin@example.com"}
                        }
                    },
                    "project": {"id": "proj_1", "name": "Default Project"},
                    "api_key.created": {
                        "id": "key_1",
                        "data": {"scopes": ["api.model.request"]}
                    }
                }],
                "first_id": "audit_log_1",
                "last_id": "audit_log_1",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-admin-test").with_base_url(mock_server.uri());
        let req = ListAuditLogsRequest::builder().limit(5).build();
        let res = client.organization().audit_logs().list(&req).await.unwrap();

        assert_eq!(res.data.len(), 1);
        let event = &res.data[0];
        assert_eq!(event.id, "audit_log_1");
        assert_eq!(event.kind, "api_key.created");
        assert_eq!(
            event
                .actor
                .as_ref()
                .unwrap()
                .session
                .as_ref()
                .unwrap()
                .ip_address
                .as_deref(),
            Some("203.0.113.1")
        );
        assert_eq!(
            event.project.as_ref().unwrap().id.as_deref(),
            Some("proj_1")
        );

        let detail = event.detail.get("api_key.created").expect("detail present");
        assert_eq!(detail["id"], "key_1");
        assert_eq!(detail["data"]["scopes"][0], "api.model.request");
    }

    #[tokio::test]
    async fn list_deserializes_event_with_different_flattened_detail_key() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/audit_logs"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{
                    "id": "audit_log_2",
                    "effective_at": 1700000100_i64,
                    "type": "invite.sent",
                    "actor": null,
                    "project": null,
                    "invite.sent": {
                        "id": "invite_1",
                        "data": {"email": "newmember@example.com", "role": "member"}
                    }
                }],
                "first_id": "audit_log_2",
                "last_id": "audit_log_2",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-admin-test").with_base_url(mock_server.uri());
        let req = ListAuditLogsRequest::builder().build();
        let res = client.organization().audit_logs().list(&req).await.unwrap();

        let event = &res.data[0];
        assert_eq!(event.kind, "invite.sent");
        assert!(event.actor.is_none());
        let detail = event.detail.get("invite.sent").expect("detail present");
        assert_eq!(detail["data"]["email"], "newmember@example.com");
        // Confirm the OTHER event's detail key is genuinely absent from this one, not
        // just unchecked -- proves the flatten map only contains what was actually sent.
        assert!(!event.detail.contains_key("api_key.created"));
    }
}
