//! Sandboxed containers backing the Responses API's `code_interpreter` tool — see
//! docs/superpowers/specs/2026-08-14-containers-design.md.

pub mod files;

use crate::client::OpenAI;
use crate::interfaces::containers;
pub use crate::interfaces::containers::*;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct Containers<'a> {
    openai: &'a OpenAI,
}

impl<'a> Containers<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates a container.
    pub async fn create(
        &self,
        req: &containers::CreateContainerRequest,
    ) -> OpenAIResponse<containers::Container> {
        self.openai.post("/containers", req).await
    }

    /// Lists containers.
    pub async fn list(
        &self,
        req: &containers::ListContainersRequest,
    ) -> OpenAIResponse<containers::ListContainersResponse> {
        self.openai.get("/containers", req).await
    }
}

pub struct Container<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> Container<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Retrieves a container by ID.
    pub async fn retrieve(&self) -> OpenAIResponse<containers::Container> {
        self.openai
            .get(&format!("/containers/{}", self.id), &())
            .await
    }

    /// Deletes a container.
    pub async fn delete(&self) -> OpenAIResponse<containers::DeleteContainerResponse> {
        self.openai
            .delete(&format!("/containers/{}", self.id), &())
            .await
    }

    /// Scoped accessor for files inside this container.
    pub fn files(&self) -> files::ContainerFiles<'a> {
        files::ContainerFiles::new(self.openai, self.id.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::containers::{
        ContainerExpiresAfter, ContainerExpiresAfterAnchor, ContainerMemoryLimit,
        ContainerNetworkPolicy, ContainerSkill, CreateContainerRequest, InlineSkillSource,
        ListContainersRequest,
    };
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_full_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/containers"))
            .and(body_json(serde_json::json!({
                "name": "my-container",
                "expires_after": {"anchor": "last_active_at", "minutes": 30},
                "memory_limit": "4g",
                "network_policy": {
                    "type": "allowlist",
                    "allowed_domains": ["example.com"],
                    "domain_secrets": [
                        {"domain": "example.com", "name": "API_KEY", "value": "secret123"}
                    ]
                }
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cntr_1",
                "created_at": 1700000000_i64,
                "name": "my-container",
                "object": "container",
                "status": "running",
                "expires_after": {"anchor": "last_active_at", "minutes": 30},
                "last_active_at": 1700000000_i64,
                "memory_limit": "4g",
                "network_policy": {
                    "type": "allowlist",
                    "allowed_domains": ["example.com"],
                    "domain_secrets": null
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateContainerRequest::builder()
            .name("my-container")
            .expires_after(
                ContainerExpiresAfter::builder()
                    .anchor(ContainerExpiresAfterAnchor::LastActiveAt)
                    .minutes(30)
                    .build(),
            )
            .memory_limit(ContainerMemoryLimit::FourGb)
            .network_policy(ContainerNetworkPolicy::Allowlist {
                allowed_domains: vec!["example.com".to_string()],
                domain_secrets: Some(vec![
                    crate::interfaces::containers::ContainerDomainSecret::builder()
                        .domain("example.com")
                        .name("API_KEY")
                        .value("secret123")
                        .build(),
                ]),
            })
            .build();

        let res = client.containers().create(&req).await.unwrap();
        assert_eq!(res.id, "cntr_1");
        assert_eq!(res.status, "running");
    }

    #[tokio::test]
    async fn create_serializes_skill_reference() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/containers"))
            .and(body_json(serde_json::json!({
                "name": "my-container",
                "skills": [{"type": "skill_reference", "skill_id": "skill_1", "version": "2"}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cntr_4",
                "created_at": 1700000000_i64,
                "name": "my-container",
                "object": "container",
                "status": "running",
                "expires_after": null,
                "last_active_at": null,
                "memory_limit": null,
                "network_policy": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateContainerRequest::builder()
            .name("my-container")
            .skills(vec![ContainerSkill::SkillReference {
                skill_id: "skill_1".to_string(),
                version: Some("2".to_string()),
            }])
            .build();

        let res = client.containers().create(&req).await.unwrap();
        assert_eq!(res.id, "cntr_4");
    }

    #[tokio::test]
    async fn create_serializes_inline_skill() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/containers"))
            .and(body_json(serde_json::json!({
                "name": "my-container",
                "skills": [{
                    "type": "inline",
                    "description": "Formats spreadsheets",
                    "name": "spreadsheet-formatter",
                    "source": {"data": "base64stuff", "media_type": "application/zip"}
                }]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cntr_5",
                "created_at": 1700000000_i64,
                "name": "my-container",
                "object": "container",
                "status": "running",
                "expires_after": null,
                "last_active_at": null,
                "memory_limit": null,
                "network_policy": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateContainerRequest::builder()
            .name("my-container")
            .skills(vec![ContainerSkill::Inline {
                description: "Formats spreadsheets".to_string(),
                name: "spreadsheet-formatter".to_string(),
                source: InlineSkillSource::builder()
                    .data("base64stuff")
                    .media_type("application/zip")
                    .build(),
            }])
            .build();

        let res = client.containers().create(&req).await.unwrap();
        assert_eq!(res.id, "cntr_5");
    }

    #[tokio::test]
    async fn list_serializes_query_params_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/containers"))
            .and(query_param("limit", "10"))
            .and(query_param("name", "my-container"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{
                    "id": "cntr_2",
                    "created_at": 1700000000_i64,
                    "name": "my-container",
                    "object": "container",
                    "status": "running",
                    "expires_after": null,
                    "last_active_at": 1700000000_i64,
                    "memory_limit": "1g",
                    "network_policy": {"type": "disabled"}
                }],
                "first_id": "cntr_2",
                "last_id": "cntr_2",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ListContainersRequest::builder()
            .limit(10)
            .name("my-container")
            .build();

        let res = client.containers().list(&req).await.unwrap();
        assert_eq!(res.data.len(), 1);
        assert_eq!(res.data[0].id, "cntr_2");
        assert!(matches!(
            res.data[0].network_policy,
            Some(ContainerNetworkPolicy::Disabled)
        ));
    }

    #[tokio::test]
    async fn retrieve_deserializes_container() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/containers/cntr_3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cntr_3",
                "created_at": 1700000000_i64,
                "name": "my-container",
                "object": "container",
                "status": "running",
                "expires_after": null,
                "last_active_at": null,
                "memory_limit": null,
                "network_policy": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client.container("cntr_3").retrieve().await.unwrap();
        assert_eq!(res.id, "cntr_3");
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/containers/cntr_3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cntr_3",
                "object": "container.deleted",
                "deleted": true
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client.container("cntr_3").delete().await.unwrap();
        assert!(res.deleted);
    }
}
