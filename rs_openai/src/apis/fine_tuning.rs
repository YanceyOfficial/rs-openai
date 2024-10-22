//! Manage fine-tuning jobs to tailor a model to your specific training data. Related guide: [Fine-tune models](https://platform.openai.com/docs/guides/fine-tuning)

use crate::client::OpenAI;
use crate::interfaces::fine_tuning;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct FineTuning<'a> {
    openai: &'a OpenAI,
}

impl<'a> FineTuning<'a> {
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
        req: &fine_tuning::CreateFineTuningRequest,
    ) -> OpenAIResponse<fine_tuning::FineTuningResponse> {
        self.openai.post("/fine-tuning/jobs", req).await
    }

    /// List your organization's fine-tuning jobs
    pub async fn list(
        &self,
        req: &fine_tuning::ListFineTuningRequest,
    ) -> OpenAIResponse<fine_tuning::FineTuningEventResponse> {
        self.openai.get("/fine-tuning/jobs", req).await
    }

    /// Get status updates for a fine-tuning job.
    pub async fn list_events(
        &self,
        fine_tuning_job_id: &str, // The ID of the fine-tuning job to get events for.
        req: &fine_tuning::ListFineTuningRequest,
    ) -> OpenAIResponse<fine_tuning::FineTuningEventResponse> {
        self.openai
            .get(
                &format!("/fine-tuning/jobs/{fine_tuning_job_id}/events"),
                req,
            )
            .await
    }

    /// Get status updates for a fine-tuning job.
    pub async fn list_checkpoints(
        &self,
        fine_tuning_job_id: &str, // The ID of the fine-tuning job to get checkpoints for.
        req: &fine_tuning::ListFineTuningRequest,
    ) -> OpenAIResponse<fine_tuning::FineTuningCheckpointResponse> {
        self.openai
            .get(
                &format!("/fine-tuning/jobs/{fine_tuning_job_id}/checkpoints"),
                req,
            )
            .await
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
        fine_tuning_job_id: &str,
    ) -> OpenAIResponse<fine_tuning::FineTuningResponse> {
        self.openai
            .get(&format!("/fine-tuning/jobs/{fine_tuning_job_id}"), &())
            .await
    }

    /// Immediately cancel a fine-tune job.
    ///
    /// # Path parameters
    ///
    /// - `fine_tune_job_id` - The ID of the fine-tune job to cancel
    pub async fn cancel(
        &self,
        fine_tune_job_id: &str,
    ) -> OpenAIResponse<fine_tuning::FineTuningResponse> {
        self.openai
            .post(&format!("/fine-tuning/jobs/{fine_tune_job_id}/cancel"), &())
            .await
    }
}
