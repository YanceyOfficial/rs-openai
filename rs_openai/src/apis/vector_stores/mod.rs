//! Semantic search over uploaded files, used by the Responses API's `file_search` tool. Core
//! surface only this round: CRUD + search, plus a basic file-attach lifecycle -- see
//! docs/superpowers/specs/2026-08-12-vector-stores-design.md.

pub mod file_batches;
pub mod files;

use crate::client::OpenAI;
use crate::interfaces::vector_stores;
pub use crate::interfaces::vector_stores::*;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct VectorStores<'a> {
    openai: &'a OpenAI,
}

impl<'a> VectorStores<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates a vector store.
    pub async fn create(
        &self,
        req: &vector_stores::CreateVectorStoreRequest,
    ) -> OpenAIResponse<vector_stores::VectorStore> {
        self.openai.post("/vector_stores", req).await
    }

    /// Lists vector stores.
    pub async fn list(
        &self,
        req: &vector_stores::ListVectorStoresRequest,
    ) -> OpenAIResponse<vector_stores::ListVectorStoresResponse> {
        self.openai.get("/vector_stores", req).await
    }
}

pub struct VectorStore<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> VectorStore<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Retrieves a vector store by ID.
    pub async fn retrieve(&self) -> OpenAIResponse<vector_stores::VectorStore> {
        self.openai
            .get(&format!("/vector_stores/{}", self.id), &())
            .await
    }

    /// Updates a vector store's name, expiration policy, or metadata.
    pub async fn update(
        &self,
        req: &vector_stores::UpdateVectorStoreRequest,
    ) -> OpenAIResponse<vector_stores::VectorStore> {
        self.openai
            .post(&format!("/vector_stores/{}", self.id), req)
            .await
    }

    /// Deletes a vector store.
    pub async fn delete(&self) -> OpenAIResponse<vector_stores::DeleteVectorStoreResponse> {
        self.openai
            .delete(&format!("/vector_stores/{}", self.id), &())
            .await
    }

    /// Scoped accessor for files attached to this vector store.
    pub fn files(&self) -> files::VectorStoreFiles<'a> {
        files::VectorStoreFiles::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for batch file-attach operations on this vector store. Not yet
    /// implemented.
    pub fn file_batches(&self) -> file_batches::VectorStoreFileBatches<'a> {
        file_batches::VectorStoreFileBatches::new(self.openai, self.id.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::chat::SortOrder;
    use crate::interfaces::vector_stores::{
        ChunkingStrategy, CreateVectorStoreRequest, ListVectorStoresRequest,
        StaticChunkingStrategy, UpdateVectorStoreRequest,
    };
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn chunking_strategy_auto_serializes_with_type_tag_only() {
        let strategy = ChunkingStrategy::Auto;
        let json = serde_json::to_value(&strategy).unwrap();
        assert_eq!(json, serde_json::json!({"type": "auto"}));
    }

    #[test]
    fn chunking_strategy_static_serializes_with_nested_static_key() {
        let strategy = ChunkingStrategy::Static {
            config: StaticChunkingStrategy::builder()
                .max_chunk_size_tokens(400_u32)
                .chunk_overlap_tokens(100_u32)
                .build(),
        };
        let json = serde_json::to_value(&strategy).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "static",
                "static": {"max_chunk_size_tokens": 400, "chunk_overlap_tokens": 100}
            })
        );
    }

    #[test]
    fn chunking_strategy_other_deserializes_with_no_fields() {
        let value = serde_json::json!({"type": "other"});
        let strategy: ChunkingStrategy = serde_json::from_value(value).unwrap();
        assert!(matches!(strategy, ChunkingStrategy::Other));
    }

    #[tokio::test]
    async fn create_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/vector_stores"))
            .and(body_json(serde_json::json!({
                "name": "Support docs",
                "file_ids": ["file-abc123"]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "vs_123",
                "object": "vector_store",
                "name": "Support docs",
                "description": null,
                "status": "in_progress",
                "created_at": 1700000000_i64,
                "last_active_at": null,
                "expires_at": null,
                "expires_after": null,
                "usage_bytes": 0,
                "file_counts": {
                    "total": 1, "completed": 0, "in_progress": 1, "failed": 0, "cancelled": 0
                },
                "metadata": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateVectorStoreRequest::builder()
            .name("Support docs")
            .file_ids(vec!["file-abc123".to_string()])
            .build();

        let res = client.vector_stores().create(&req).await.unwrap();
        assert_eq!(res.id, "vs_123");
        assert_eq!(res.file_counts.in_progress, 1);
    }

    #[tokio::test]
    async fn create_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/vector_stores"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "message": "name must be a string",
                    "type": "invalid_request_error",
                    "param": "name",
                    "code": null
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateVectorStoreRequest::builder().build();

        let err = client.vector_stores().create(&req).await.unwrap_err();
        match err {
            crate::shared::response_wrapper::OpenAIError::ApiError(e) => {
                assert_eq!(e.error.param, Some(serde_json::json!("name")));
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn list_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/vector_stores"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{
                    "id": "vs_123",
                    "object": "vector_store",
                    "name": "Support docs",
                    "description": null,
                    "status": "completed",
                    "created_at": 1700000000_i64,
                    "last_active_at": 1700000100_i64,
                    "expires_at": null,
                    "expires_after": null,
                    "usage_bytes": 1024,
                    "file_counts": {
                        "total": 1, "completed": 1, "in_progress": 0, "failed": 0, "cancelled": 0
                    },
                    "metadata": null
                }],
                "first_id": "vs_123",
                "last_id": "vs_123",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ListVectorStoresRequest::builder()
            .limit(10_u32)
            .order(SortOrder::Desc)
            .build();
        let res = client.vector_stores().list(&req).await.unwrap();

        assert_eq!(res.data.len(), 1);
        assert_eq!(res.data[0].usage_bytes, 1024);
        assert!(!res.has_more);
    }

    #[tokio::test]
    async fn retrieve_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/vector_stores/vs_456"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "vs_456",
                "object": "vector_store",
                "name": null,
                "description": null,
                "status": "completed",
                "created_at": 1700000000_i64,
                "last_active_at": null,
                "expires_at": null,
                "expires_after": null,
                "usage_bytes": 0,
                "file_counts": {
                    "total": 0, "completed": 0, "in_progress": 0, "failed": 0, "cancelled": 0
                },
                "metadata": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client.vector_store("vs_456").retrieve().await.unwrap();
        assert_eq!(res.id, "vs_456");
    }

    #[tokio::test]
    async fn update_serializes_only_updatable_fields() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/vector_stores/vs_456"))
            .and(body_json(serde_json::json!({"name": "Renamed"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "vs_456",
                "object": "vector_store",
                "name": "Renamed",
                "description": null,
                "status": "completed",
                "created_at": 1700000000_i64,
                "last_active_at": null,
                "expires_at": null,
                "expires_after": null,
                "usage_bytes": 0,
                "file_counts": {
                    "total": 0, "completed": 0, "in_progress": 0, "failed": 0, "cancelled": 0
                },
                "metadata": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = UpdateVectorStoreRequest::builder().name("Renamed").build();
        let res = client.vector_store("vs_456").update(&req).await.unwrap();
        assert_eq!(res.name.as_deref(), Some("Renamed"));
    }

    #[tokio::test]
    async fn delete_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/vector_stores/does-not-exist"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": {
                    "message": "No vector store found",
                    "type": "invalid_request_error",
                    "param": null,
                    "code": "not_found"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let err = client
            .vector_store("does-not-exist")
            .delete()
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            crate::shared::response_wrapper::OpenAIError::ApiError(_)
        ));
    }
}
