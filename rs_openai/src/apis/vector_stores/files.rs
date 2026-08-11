//! A vector store's attached files (`/v1/vector_stores/{vector_store_id}/files`). Core
//! lifecycle only this round: attach/list/retrieve/delete -- see
//! docs/superpowers/specs/2026-08-12-vector-stores-design.md.

use crate::client::OpenAI;
use crate::interfaces::vector_stores;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct VectorStoreFiles<'a> {
    openai: &'a OpenAI,
    vector_store_id: String,
}

impl<'a> VectorStoreFiles<'a> {
    pub(crate) fn new(openai: &'a OpenAI, vector_store_id: String) -> Self {
        Self {
            openai,
            vector_store_id,
        }
    }

    /// Attaches a file to a vector store.
    pub async fn create(
        &self,
        req: &vector_stores::CreateVectorStoreFileRequest,
    ) -> OpenAIResponse<vector_stores::VectorStoreFile> {
        self.openai
            .post(
                &format!("/vector_stores/{}/files", self.vector_store_id),
                req,
            )
            .await
    }

    /// Lists a vector store's attached files.
    pub async fn list(
        &self,
        req: &vector_stores::ListVectorStoreFilesRequest,
    ) -> OpenAIResponse<vector_stores::ListVectorStoreFilesResponse> {
        self.openai
            .get(
                &format!("/vector_stores/{}/files", self.vector_store_id),
                req,
            )
            .await
    }

    /// Scoped accessor for a single attached file, mirroring the crate's plural/singular
    /// convention one level deeper (`client.responses()` -> `client.response(id)`, here
    /// `vector_store.files()` -> `.file(id)`).
    pub fn file(&self, file_id: impl Into<String>) -> VectorStoreFile<'a> {
        VectorStoreFile::new(self.openai, self.vector_store_id.clone(), file_id.into())
    }
}

pub struct VectorStoreFile<'a> {
    openai: &'a OpenAI,
    vector_store_id: String,
    file_id: String,
}

impl<'a> VectorStoreFile<'a> {
    pub(crate) fn new(openai: &'a OpenAI, vector_store_id: String, file_id: String) -> Self {
        Self {
            openai,
            vector_store_id,
            file_id,
        }
    }

    /// Retrieves a vector store file by ID.
    pub async fn retrieve(&self) -> OpenAIResponse<vector_stores::VectorStoreFile> {
        self.openai
            .get(
                &format!(
                    "/vector_stores/{}/files/{}",
                    self.vector_store_id, self.file_id
                ),
                &(),
            )
            .await
    }

    /// Deletes a vector store file (detaches it -- does not delete the underlying File object).
    pub async fn delete(&self) -> OpenAIResponse<vector_stores::DeleteVectorStoreFileResponse> {
        self.openai
            .delete(
                &format!(
                    "/vector_stores/{}/files/{}",
                    self.vector_store_id, self.file_id
                ),
                &(),
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::chat::SortOrder;
    use crate::interfaces::vector_stores::{
        CreateVectorStoreFileRequest, ListVectorStoreFilesRequest,
    };
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/vector_stores/vs_456/files"))
            .and(body_json(serde_json::json!({"file_id": "file-abc123"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "vsf_1",
                "object": "vector_store.file",
                "created_at": 1700000000_i64,
                "usage_bytes": 512,
                "vector_store_id": "vs_456",
                "status": "in_progress",
                "last_error": null,
                "chunking_strategy": {
                    "type": "static",
                    "static": {"max_chunk_size_tokens": 800, "chunk_overlap_tokens": 400}
                },
                "attributes": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateVectorStoreFileRequest::builder()
            .file_id("file-abc123")
            .build();

        let res = client
            .vector_store("vs_456")
            .files()
            .create(&req)
            .await
            .unwrap();
        assert_eq!(res.id, "vsf_1");
        assert_eq!(res.vector_store_id, "vs_456");
    }

    #[tokio::test]
    async fn list_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/vector_stores/vs_456/files"))
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
        let req = ListVectorStoreFilesRequest::builder()
            .limit(10_u32)
            .order(SortOrder::Desc)
            .build();
        let res = client
            .vector_store("vs_456")
            .files()
            .list(&req)
            .await
            .unwrap();
        assert_eq!(res.data.len(), 1);
    }

    #[tokio::test]
    async fn retrieve_deserializes_response_with_failed_status_and_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/vector_stores/vs_456/files/vsf_2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "vsf_2",
                "object": "vector_store.file",
                "created_at": 1700000000_i64,
                "usage_bytes": 0,
                "vector_store_id": "vs_456",
                "status": "failed",
                "last_error": {"code": "server_error", "message": "internal error"},
                "chunking_strategy": {"type": "other"},
                "attributes": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .vector_store("vs_456")
            .files()
            .file("vsf_2")
            .retrieve()
            .await
            .unwrap();

        assert_eq!(res.status.to_string(), "failed");
        assert_eq!(
            res.last_error.as_ref().map(|e| e.message.as_str()),
            Some("internal error")
        );
    }

    #[tokio::test]
    async fn delete_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/vector_stores/vs_456/files/does-not-exist"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": {
                    "message": "No file found",
                    "type": "invalid_request_error",
                    "param": null,
                    "code": "not_found"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let err = client
            .vector_store("vs_456")
            .files()
            .file("does-not-exist")
            .delete()
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            crate::shared::response_wrapper::OpenAIError::ApiError(_)
        ));
    }
}
