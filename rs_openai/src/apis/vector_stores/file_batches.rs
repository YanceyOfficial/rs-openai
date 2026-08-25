//! Batch file-attach operations for a vector store
//! (`/v1/vector_stores/{vector_store_id}/file_batches`) -- see
//! docs/superpowers/specs/2026-08-20-vector-stores-extensions-design.md.

use crate::client::OpenAI;
use crate::interfaces::vector_stores;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct VectorStoreFileBatches<'a> {
    openai: &'a OpenAI,
    vector_store_id: String,
}

impl<'a> VectorStoreFileBatches<'a> {
    pub(crate) fn new(openai: &'a OpenAI, vector_store_id: String) -> Self {
        Self {
            openai,
            vector_store_id,
        }
    }

    /// Creates a file batch, attaching multiple files to a vector store in one operation.
    pub async fn create(
        &self,
        req: &vector_stores::CreateVectorStoreFileBatchRequest,
    ) -> OpenAIResponse<vector_stores::VectorStoreFileBatch> {
        self.openai
            .vector_stores_post(
                &format!("/vector_stores/{}/file_batches", self.vector_store_id),
                req,
            )
            .await
    }

    /// Scoped accessor for a single file batch, mirroring the crate's plural/singular
    /// convention one level deeper (`vector_store.file_batches()` -> `.file_batch(id)`).
    pub fn file_batch(&self, batch_id: impl Into<String>) -> VectorStoreFileBatch<'a> {
        VectorStoreFileBatch::new(self.openai, self.vector_store_id.clone(), batch_id.into())
    }
}

pub struct VectorStoreFileBatch<'a> {
    openai: &'a OpenAI,
    vector_store_id: String,
    batch_id: String,
}

impl<'a> VectorStoreFileBatch<'a> {
    pub(crate) fn new(openai: &'a OpenAI, vector_store_id: String, batch_id: String) -> Self {
        Self {
            openai,
            vector_store_id,
            batch_id,
        }
    }

    /// Retrieves a file batch by ID.
    pub async fn retrieve(&self) -> OpenAIResponse<vector_stores::VectorStoreFileBatch> {
        self.openai
            .vector_stores_get(
                &format!(
                    "/vector_stores/{}/file_batches/{}",
                    self.vector_store_id, self.batch_id
                ),
                &(),
            )
            .await
    }

    /// Cancels a file batch that is in progress.
    pub async fn cancel(&self) -> OpenAIResponse<vector_stores::VectorStoreFileBatch> {
        self.openai
            .vector_stores_post(
                &format!(
                    "/vector_stores/{}/file_batches/{}/cancel",
                    self.vector_store_id, self.batch_id
                ),
                &(),
            )
            .await
    }

    /// Lists the files in a file batch.
    pub async fn list_files(
        &self,
        req: &vector_stores::ListVectorStoreFileBatchFilesRequest,
    ) -> OpenAIResponse<vector_stores::ListVectorStoreFilesResponse> {
        self.openai
            .vector_stores_get(
                &format!(
                    "/vector_stores/{}/file_batches/{}/files",
                    self.vector_store_id, self.batch_id
                ),
                req,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::vector_stores::{
        CreateVectorStoreFileBatchRequest, ListVectorStoreFileBatchFilesRequest,
    };
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/vector_stores/vs_456/file_batches"))
            .and(header("OpenAI-Beta", "assistants=v2"))
            .and(body_json(serde_json::json!({
                "file_ids": ["file-abc123", "file-def456"]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "vsfb_1",
                "object": "vector_store.file_batch",
                "created_at": 1700000000_i64,
                "vector_store_id": "vs_456",
                "status": "in_progress",
                "file_counts": {
                    "cancelled": 0, "completed": 0, "failed": 0, "in_progress": 2, "total": 2
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateVectorStoreFileBatchRequest::builder()
            .file_ids(vec!["file-abc123".to_string(), "file-def456".to_string()])
            .build();

        let res = client
            .vector_store("vs_456")
            .file_batches()
            .create(&req)
            .await
            .unwrap();
        assert_eq!(res.id, "vsfb_1");
        assert_eq!(res.file_counts.total, 2);
    }

    #[tokio::test]
    async fn retrieve_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/vector_stores/vs_456/file_batches/vsfb_1"))
            .and(header("OpenAI-Beta", "assistants=v2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "vsfb_1",
                "object": "vector_store.file_batch",
                "created_at": 1700000000_i64,
                "vector_store_id": "vs_456",
                "status": "completed",
                "file_counts": {
                    "cancelled": 0, "completed": 2, "failed": 0, "in_progress": 0, "total": 2
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .vector_store("vs_456")
            .file_batches()
            .file_batch("vsfb_1")
            .retrieve()
            .await
            .unwrap();
        assert_eq!(res.status.to_string(), "completed");
    }

    #[tokio::test]
    async fn cancel_posts_to_cancel_route() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/vector_stores/vs_456/file_batches/vsfb_1/cancel"))
            .and(header("OpenAI-Beta", "assistants=v2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "vsfb_1",
                "object": "vector_store.file_batch",
                "created_at": 1700000000_i64,
                "vector_store_id": "vs_456",
                "status": "cancelled",
                "file_counts": {
                    "cancelled": 2, "completed": 0, "failed": 0, "in_progress": 0, "total": 2
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .vector_store("vs_456")
            .file_batches()
            .file_batch("vsfb_1")
            .cancel()
            .await
            .unwrap();
        assert_eq!(res.status.to_string(), "cancelled");
    }

    #[tokio::test]
    async fn list_files_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/vector_stores/vs_456/file_batches/vsfb_1/files"))
            .and(header("OpenAI-Beta", "assistants=v2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{
                    "id": "vsf_1",
                    "object": "vector_store.file",
                    "created_at": 1700000000_i64,
                    "usage_bytes": 512,
                    "vector_store_id": "vs_456",
                    "status": "completed",
                    "last_error": null,
                    "chunking_strategy": {"type": "auto"},
                    "attributes": null
                }],
                "first_id": "vsf_1",
                "last_id": "vsf_1",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ListVectorStoreFileBatchFilesRequest::builder().build();
        let res = client
            .vector_store("vs_456")
            .file_batches()
            .file_batch("vsfb_1")
            .list_files(&req)
            .await
            .unwrap();
        assert_eq!(res.data.len(), 1);
    }
}
