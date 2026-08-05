//! Create large batches of API requests for asynchronous processing.
//! The Batch API returns completions within 24 hours for a 50% discount. Related guide: [Batch](https://platform.openai.com/docs/guides/batch)

use crate::client::OpenAI;
use crate::interfaces::batch;
pub use crate::interfaces::batch::*;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct Batch<'a> {
    openai: &'a OpenAI,
}

impl<'a> Batch<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates and executes a batch from an uploaded file of requests
    pub async fn create(
        &self,
        req: &batch::CreateBatchRequest,
    ) -> OpenAIResponse<batch::BatchResponse> {
        self.openai.post("/batches", req).await
    }

    /// Retrieve a batch
    pub async fn retrieve(&self, batch_id: String) -> OpenAIResponse<batch::BatchResponse> {
        self.openai.get(&format!("/batches/{batch_id}"), &()).await
    }

    /// Cancels an in-progress batch. The batch will be in status `cancelling` for up to 10 minutes, before changing to `cancelled`,
    /// where it will have partial results (if any) available in the output file.
    pub async fn cancel(&self, batch_id: String) -> OpenAIResponse<batch::BatchResponse> {
        self.openai
            .post(&format!("/batches/{batch_id}/cancel"), &())
            .await
    }

    /// A list of paginated [Batch](https://platform.openai.com/docs/api-reference/batch/object) objects.
    pub async fn list(
        &self,
        req: &batch::ListBatchRequest,
    ) -> OpenAIResponse<batch::BatchResponse> {
        self.openai.get("/batches", req).await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::batch::{BatchEndpoint, CreateBatchRequest};
    use crate::shared::response_wrapper::OpenAIError;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn batch_response_json(id: &str, status: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id, "object": "batch", "endpoint": "/v1/chat/completions", "errors": null,
            "input_file_id": "file-abc", "completion_window": "24h", "status": status,
            "output_file_id": null, "error_file_id": null, "created_at": 1700000000_i64,
            "in_progress_at": null, "expires_at": null, "finalizing_at": null,
            "completed_at": null, "failed_at": null, "expired_at": null,
            "cancelling_at": null, "cancelled_at": null,
            "request_counts": {"total": 0, "completed": 0, "failed": 0},
            "metadata": {}
        })
    }

    #[tokio::test]
    async fn create_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/batches"))
            .and(body_json(serde_json::json!({
                "input_file_id": "file-abc",
                "endpoint": "/v1/chat/completions",
                "completion_window": "24h"
            })))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(batch_response_json("batch_123", "validating")),
            )
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateBatchRequest::builder()
            .input_file_id("file-abc")
            .endpoint(BatchEndpoint::ChatCompletions)
            .completion_window("24h")
            .build();

        let res = client.batch().create(&req).await.unwrap();
        assert_eq!(res.id, "batch_123");
    }

    #[tokio::test]
    async fn create_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/batches"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "message": "input_file_id is required",
                    "type": "invalid_request_error",
                    "param": "input_file_id",
                    "code": null
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateBatchRequest::builder()
            .input_file_id("")
            .endpoint(BatchEndpoint::ChatCompletions)
            .completion_window("24h")
            .build();

        let err = client.batch().create(&req).await.unwrap_err();
        match err {
            OpenAIError::ApiError(e) => {
                assert_eq!(e.error.param, Some(serde_json::json!("input_file_id")));
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn create_serializes_videos_endpoint_and_output_expires_after() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/batches"))
            .and(body_json(serde_json::json!({
                "input_file_id": "file-vid",
                "endpoint": "/v1/videos",
                "completion_window": "24h",
                "output_expires_after": {"anchor": "created_at", "seconds": 7200}
            })))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(batch_response_json("batch_456", "validating")),
            )
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateBatchRequest::builder()
            .input_file_id("file-vid")
            .endpoint(BatchEndpoint::Videos)
            .completion_window("24h")
            .output_expires_after(
                crate::shared::types::ExpiresAfter::builder()
                    .anchor(crate::shared::types::ExpiresAfterAnchor::CreatedAt)
                    .seconds(7200)
                    .build(),
            )
            .build();

        let res = client.batch().create(&req).await.unwrap();
        assert_eq!(res.id, "batch_456");
    }
}
