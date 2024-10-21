//! Create large batches of API requests for asynchronous processing.
//! The Batch API returns completions within 24 hours for a 50% discount. Related guide: [Batch](https://platform.openai.com/docs/guides/batch)

use crate::client::OpenAI;
use crate::interfaces::batch;
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
