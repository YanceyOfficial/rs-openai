//! Manage fine-tuning jobs to tailor a model to your specific training data.
//!
//! Related guide: [Fine-tune models](https://platform.openai.com/docs/guides/fine-tuning)

use crate::shared::response_wrapper::OpenAIError;
use crate::{OpenAI, OpenAIResponse};
use derive_builder::Builder;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "CreateFineTuneRequestBuilder")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct CreateFineTuneRequest {
    /// The ID of an uploaded file that contains training data.
    ///
    /// See [upload file](https://platform.openai.com/docs/api-reference/files/upload) for how to upload a file.
    ///
    ///
    /// Your dataset must be formatted as a JSONL file, where each training example is a JSON object with the keys "prompt" and "completion".
    /// Additionally, you must upload your file with the purpose `fine-tune`.
    ///
    ///
    /// See the [fine-tuning guide](https://platform.openai.com/docs/guides/fine-tuning/creating-training-data) for more details.
    pub training_file: String,

    /// The ID of an uploaded file that contains validation data.
    ///
    /// If you provide this file, the data is used to generate validation metrics periodically during fine-tuning.
    /// These metrics can be viewed in the [fine-tuning results file](https://platform.openai.com/docs/guides/fine-tuning/analyzing-your-fine-tuned-model).
    /// Your train and validation data should be mutually exclusive.
    ///
    /// Your dataset must be formatted as a JSONL file, where each validation example is a JSON object with the keys "prompt" and "completion".
    /// Additionally, you must upload your file with the purpose `fine-tune`.
    ///
    /// See the [fine-tuning guide](https://platform.openai.com/docs/guides/fine-tuning/creating-training-data) for more details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_file: Option<String>,

    /// The name of the base model to fine-tune.
    /// You can select one of "ada", "babbage", "curie", "davinci", or a fine-tuned model created after 2022-04-21.
    /// To learn more about these models, see the [Models](https://platform.openai.com/docs/models) documentation.
    pub model: Option<String>,

    /// The number of epochs to train the model for.
    /// An epoch refers to one full cycle through the training dataset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_epochs: Option<u32>,

    /// The batch size to use for training.
    /// The batch size is the number of training examples used to train a single forward and backward pass.
    ///
    /// By default, the batch size will be dynamically configured to be ~0.2% of the number of examples in the training set, capped at 256.
    /// In general, we've found that larger batch sizes tend to work better for larger datasets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<u32>,

    /// The learning rate multiplier to use for training.
    /// The fine-tuning learning rate is the original learning rate used for pretraining multiplied by this value.
    ///
    /// By default, the learning rate multiplier is 0.05, 0.1, or 0.2 depending on final `batch_size` (larger learning rates tend to perform better with larger batch sizes).
    /// We recommend experimenting with values in the range 0.02 to 0.2 to see what produces the best results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub learning_rate_multiplier: Option<f32>,

    /// The weight to use for loss on the prompt tokens.
    /// This controls how much the model tries to learn to generate the prompt (as compared to the completion which always has a weight of 1.0), and can add a stabilizing effect to training when completions are short.
    ///
    /// If prompts are extremely long (relative to completions), it may make sense to reduce this weight so as to avoid over-prioritizing learning the prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_loss_weight: Option<f32>,

    /// If set, we calculate classification-specific metrics such as accuracy and F-1 score using the validation set at the end of every epoch.
    /// These metrics can be viewed in the [results file](https://platform.openai.com/docs/guides/fine-tuning/analyzing-your-fine-tuned-model).
    ///
    /// In order to compute classification metrics, you must provide a `validation_file`.
    /// Additionally, you must specify `classification_n_classes` for multiclass classification or `classification_positive_class` for binary classification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compute_classification_metrics: Option<bool>,

    /// The number of classes in a classification task.
    ///
    /// This parameter is required for multiclass classification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classification_n_classes: Option<u32>,

    /// The positive class in binary classification.
    ///
    /// This parameter is needed to generate precision, recall, and F1 metrics when doing binary classification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classification_positive_class: Option<String>,

    /// If provided, we calculate F-beta scores at the specified beta values. The F-beta score is a generalization of F-1 score. This is only used for binary classification.
    ///
    /// With a beta of 1 (i.e. the F-1 score), precision and recall are given the same weight. A larger beta score puts more weight on recall and less on precision. A smaller beta score puts more weight on precision and less on recall.
    #[serde(skip_serializing_if = "Option::is_none")]
    classification_betas: Option<Vec<f32>>,

    /// A string of up to 40 characters that will be added to your fine-tuned model name.
    ///
    /// For example, a `suffix` of "custom-model-name" would produce a model name like `ada:ft-your-org:custom-model-name-2022-02-15-04-21-04`.
    #[serde(skip_serializing_if = "Option::is_none")]
    suffix: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FineTuneResponse {
    pub id: String,
    pub object: String,
    pub model: String,
    pub created_at: u32,
    pub events: Option<Vec<FineTuneEvent>>,
    pub fine_tuned_model: Option<String>,
    pub hyperparams: HyperParams,
    pub organization_id: String,
    pub result_files: Vec<TrainingFile>,
    pub status: String,
    pub validation_files: Vec<TrainingFile>,
    pub training_files: Vec<TrainingFile>,
    pub updated_at: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FineTuneEvent {
    pub object: String,
    pub created_at: u32,
    pub level: String,
    pub message: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HyperParams {
    pub batch_size: u32,
    pub learning_rate_multiplier: f32,
    pub n_epochs: u32,
    pub prompt_loss_weight: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TrainingFile {
    pub id: String,
    pub object: String,
    pub bytes: u32,
    pub created_at: u32,
    pub filename: String,
    pub purpose: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FineTuneListResponse {
    pub object: String,
    pub data: Vec<FineTuneResponse>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EventListResponse {
    pub object: String,
    pub data: Vec<FineTuneEvent>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DeleteFileResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}

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
    pub async fn create(&self, req: &CreateFineTuneRequest) -> OpenAIResponse<FineTuneResponse> {
        self.openai.post("/fine-tunes", req).await
    }

    /// Gets info about the fine-tune job.
    ///
    /// # Path parameters
    ///
    /// - `fine_tune_id` - The ID of the fine-tune job
    ///
    /// [Learn more about Fine-tuning](https://platform.openai.com/docs/guides/fine-tuning)
    pub async fn retrieve(&self, fine_tune_id: &str) -> OpenAIResponse<FineTuneResponse> {
        self.openai
            .get(&format!("/fine-tunes/{fine_tune_id}"), &())
            .await
    }

    /// Immediately cancel a fine-tune job.
    ///
    /// # Path parameters
    ///
    /// - `fine_tune_id` - The ID of the fine-tune job to cancel
    pub async fn cancel(&self, fine_tune_id: &str) -> OpenAIResponse<FineTuneResponse> {
        self.openai
            .post(&format!("/fine-tunes/{fine_tune_id}/cancel"), &())
            .await
    }

    /// List your organization's fine-tuning jobs
    pub async fn list(&self) -> OpenAIResponse<FineTuneListResponse> {
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
    pub async fn retrieve_content(&self, fine_tune_id: &str) -> OpenAIResponse<EventListResponse> {
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
    ) -> Result<Pin<Box<dyn Stream<Item = OpenAIResponse<EventListResponse>> + Send>>,OpenAIError> {
        Ok(self.openai
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
    pub async fn delete_model(&self, model: &str) -> OpenAIResponse<DeleteFileResponse> {
        self.openai.delete(&format!("/models/{model}"), &()).await
    }
}
