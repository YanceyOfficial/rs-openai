//! Organization admin API keys (`/organization/admin_api_keys`) -- see
//! docs/superpowers/specs/2026-08-19-org-admin-api-keys-audit-logs-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::admin_api_keys;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationAdminApiKeys<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationAdminApiKeys<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates an organization admin API key.
    pub async fn create(
        &self,
        req: &admin_api_keys::CreateAdminApiKeyRequest,
    ) -> OpenAIResponse<admin_api_keys::AdminApiKey> {
        self.openai.post("/organization/admin_api_keys", req).await
    }

    /// Lists organization admin API keys.
    pub async fn list(
        &self,
        req: &admin_api_keys::ListAdminApiKeysRequest,
    ) -> OpenAIResponse<admin_api_keys::ListAdminApiKeysResponse> {
        self.openai.get("/organization/admin_api_keys", req).await
    }
}

pub struct OrganizationAdminApiKey<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> OrganizationAdminApiKey<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Retrieves an organization admin API key by ID.
    pub async fn retrieve(&self) -> OpenAIResponse<admin_api_keys::AdminApiKey> {
        self.openai
            .get(&format!("/organization/admin_api_keys/{}", self.id), &())
            .await
    }

    /// Deletes an organization admin API key.
    pub async fn delete(&self) -> OpenAIResponse<admin_api_keys::DeleteAdminApiKeyResponse> {
        self.openai
            .delete(&format!("/organization/admin_api_keys/{}", self.id), &())
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::admin_api_keys::{
        CreateAdminApiKeyRequest, ListAdminApiKeysRequest,
    };
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_full_request_and_deserializes_response_with_value() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/admin_api_keys"))
            .and(body_json(serde_json::json!({
                "name": "CI deploy key",
                "expires_in_seconds": 3600
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "key_1",
                "created_at": 1711471533_i64,
                "expires_at": 1711475133_i64,
                "object": "organization.admin_api_key",
                "owner": {
                    "id": "user_1",
                    "created_at": 1700000000_i64,
                    "name": "Jane Admin",
                    "object": "organization.user",
                    "role": "owner",
                    "type": "user"
                },
                "redacted_value": "sk-admin...abcd",
                "last_used_at": null,
                "name": "CI deploy key",
                "value": "sk-admin-1234abcd"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-admin-test").with_base_url(mock_server.uri());
        let req = CreateAdminApiKeyRequest::builder()
            .name("CI deploy key")
            .expires_in_seconds(3600)
            .build();

        let res = client
            .organization()
            .admin_api_keys()
            .create(&req)
            .await
            .unwrap();
        assert_eq!(res.id, "key_1");
        assert_eq!(res.value.as_deref(), Some("sk-admin-1234abcd"));
        assert_eq!(res.owner.id.as_deref(), Some("user_1"));
    }

    #[tokio::test]
    async fn list_serializes_query_params_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/admin_api_keys"))
            .and(query_param("limit", "10"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{
                    "id": "key_2",
                    "created_at": 1711471533_i64,
                    "expires_at": null,
                    "object": "organization.admin_api_key",
                    "owner": {
                        "id": "user_2",
                        "created_at": 1700000000_i64,
                        "name": "Service Bot",
                        "object": "organization.user",
                        "role": "owner",
                        "type": "user"
                    },
                    "redacted_value": "sk-admin...wxyz",
                    "last_used_at": 1711471600_i64,
                    "name": "Bot key",
                    "value": null
                }],
                "first_id": "key_2",
                "last_id": "key_2",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-admin-test").with_base_url(mock_server.uri());
        let req = ListAdminApiKeysRequest::builder().limit(10).build();
        let res = client
            .organization()
            .admin_api_keys()
            .list(&req)
            .await
            .unwrap();
        assert_eq!(res.data.len(), 1);
        assert!(res.data[0].value.is_none());
    }

    #[tokio::test]
    async fn retrieve_deserializes_key_with_value_absent() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/admin_api_keys/key_3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "key_3",
                "created_at": 1711471533_i64,
                "expires_at": null,
                "object": "organization.admin_api_key",
                "owner": {
                    "id": "user_3",
                    "created_at": 1700000000_i64,
                    "name": "Jane Admin",
                    "object": "organization.user",
                    "role": "owner",
                    "type": "user"
                },
                "redacted_value": "sk-admin...efgh",
                "last_used_at": null,
                "name": "Retrieved key",
                "value": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-admin-test").with_base_url(mock_server.uri());
        let res = client
            .organization()
            .admin_api_key("key_3")
            .retrieve()
            .await
            .unwrap();
        assert_eq!(res.id, "key_3");
        assert!(res.value.is_none());
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/organization/admin_api_keys/key_3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "key_3",
                "deleted": true,
                "object": "organization.admin_api_key.deleted"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-admin-test").with_base_url(mock_server.uri());
        let res = client
            .organization()
            .admin_api_key("key_3")
            .delete()
            .await
            .unwrap();
        assert!(res.deleted);
    }
}
