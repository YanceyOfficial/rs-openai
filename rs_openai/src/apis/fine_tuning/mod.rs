//! Manage fine-tuning jobs to tailor a model to your specific training data. Related guide: [Fine-tune models](https://platform.openai.com/docs/guides/fine-tuning)

pub mod checkpoints;

use crate::client::OpenAI;
use crate::interfaces::fine_tuning;
pub use crate::interfaces::fine_tuning::*;
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

    /// Scoped accessor for a specific fine-tuning checkpoint's operations (e.g. sharing
    /// permissions). Not yet implemented — see
    /// docs/superpowers/research/2026-07-31-field-diff-images-audio-files-fine-tuning.md.
    pub fn checkpoint(&self, id: impl Into<String>) -> checkpoints::FineTuningCheckpoint<'a> {
        checkpoints::FineTuningCheckpoint::new(self.openai, id.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::fine_tuning::CreateFineTuningRequest;
    use crate::shared::response_wrapper::OpenAIError;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/fine-tuning/jobs"))
            .and(body_json(serde_json::json!({
                "model": "davinci",
                "training_file": "file-abc"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "fine_tuning.job",
                "id": "ftjob-123",
                "model": "davinci",
                "created_at": 1700000000_u64,
                "finished_at": null,
                "fine_tuned_model": null,
                "organization_id": "org-123",
                "result_files": [],
                "status": "queued",
                "validation_file": null,
                "training_file": "file-abc",
                "trained_tokens": null,
                "hyperparameters": {"batch_size": "auto", "learning_rate_multiplier": "auto", "n_epochs": "auto"},
                "integrations": null,
                "seed": 42,
                "estimated_finish": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateFineTuningRequest::builder()
            .model("davinci")
            .training_file("file-abc")
            .build();

        let res = client.fine_tuning().create(&req).await.unwrap();
        assert_eq!(res.id, "ftjob-123");
    }

    #[tokio::test]
    async fn create_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/fine-tuning/jobs"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "message": "training_file is required",
                    "type": "invalid_request_error",
                    "param": "training_file",
                    "code": null
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateFineTuningRequest::builder()
            .model("davinci")
            .training_file("")
            .build();

        let err = client.fine_tuning().create(&req).await.unwrap_err();
        match err {
            OpenAIError::ApiError(e) => {
                assert_eq!(e.error.param, Some(serde_json::json!("training_file")));
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }
}
