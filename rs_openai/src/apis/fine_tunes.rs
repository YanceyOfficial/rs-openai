//! Manage fine-tuning jobs to tailor a model to your specific training data.
//!
//! Related guide: [Fine-tune models](https://platform.openai.com/docs/guides/fine-tuning)

use crate::client::OpenAI;
use crate::interfaces::fine_tunes;
use crate::shared::response_wrapper::{OpenAIError, OpenAIResponse};
use futures::Stream;
use std::pin::Pin;

pub struct FineTunes<'a> {
    openai: &'a OpenAI,
}

impl<'a> FineTunes<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates a job that fine-tunes a specified model from a given dataset.
    ///
    /// OpenAIResponse includes details of the enqueued job including job status and the name of the fine-tuned models once complete.
    ///
    /// [Learn more about Fine-tuning](https://platform.openai.com/docs/guides/fine-tuning)
    pub async fn create(
        &self,
        req: &fine_tunes::CreateFineTuneRequest,
    ) -> OpenAIResponse<fine_tunes::FineTuneResponse> {
        self.openai.post("/fine-tunes", req).await
    }

    /// Gets info about the fine-tune job.
    ///
    /// # Path parameters
    ///
    /// - `fine_tune_id` - The ID of the fine-tune job
    ///
    /// [Learn more about Fine-tuning](https://platform.openai.com/docs/guides/fine-tuning)
    pub async fn retrieve(
        &self,
        fine_tune_id: &str,
    ) -> OpenAIResponse<fine_tunes::FineTuneResponse> {
        self.openai
            .get(&format!("/fine-tunes/{fine_tune_id}"), &())
            .await
    }

    /// Immediately cancel a fine-tune job.
    ///
    /// # Path parameters
    ///
    /// - `fine_tune_id` - The ID of the fine-tune job to cancel
    pub async fn cancel(&self, fine_tune_id: &str) -> OpenAIResponse<fine_tunes::FineTuneResponse> {
        self.openai
            .post(&format!("/fine-tunes/{fine_tune_id}/cancel"), &())
            .await
    }

    /// List your organization's fine-tuning jobs
    pub async fn list(&self) -> OpenAIResponse<fine_tunes::FineTuneListResponse> {
        self.openai.get("/fine-tunes", &()).await
    }

    /// Get fine-grained status updates for a fine-tune job.
    ///
    /// Only events generated so far will be returned.
    ///
    /// # Path parameters
    ///
    /// - `fine_tune_id` - The ID of the fine-tune job to get events for.
    ///
    /// TODO: Since free accounts cannot read fine-tune event content, I have to verify this api until purchase a Plus.
    pub async fn retrieve_content(
        &self,
        fine_tune_id: &str,
    ) -> OpenAIResponse<fine_tunes::EventListResponse> {
        self.openai
            .get(&format!("/fine-tunes/{fine_tune_id}/events"), &())
            .await
    }

    /// Get fine-grained status updates for a fine-tune job by stream.
    ///
    /// Events will be sent as data-only [server-sent events](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events#Event_stream_format) as they become available.
    /// The stream will terminate with a `data: [DONE]` message when the job is finished (succeeded, cancelled, or failed).
    ///
    /// # Path parameters
    ///
    /// - `fine_tune_id` - The ID of the fine-tune job to get events for.
    ///
    /// TODO: Since free accounts cannot read fine-tune event content, I have to verify this api until purchase a Plus.
    pub async fn retrieve_content_stream(
        &self,
        fine_tune_id: &str,
    ) -> Result<
        Pin<Box<dyn Stream<Item = OpenAIResponse<fine_tunes::EventListResponse>> + Send>>,
        OpenAIError,
    > {
        Ok(self
            .openai
            .get_stream(
                &format!("/fine-tunes/{fine_tune_id}/events"),
                &("stream", true),
            )
            .await)
    }

    /// Delete a fine-tuned model. You must have the Owner role in your organization.
    ///
    /// # Path parameters
    ///
    /// - `model` - The model to delete
    pub async fn delete_model(
        &self,
        model: &str,
    ) -> OpenAIResponse<fine_tunes::DeleteFileResponse> {
        self.openai.delete(&format!("/models/{model}"), &()).await
    }
}
