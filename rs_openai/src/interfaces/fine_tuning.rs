use crate::shared::response_wrapper::OpenAIError;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "CreateFineTuningRequestBuilder")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct CreateFineTuningRequest {
    /// The name of the model to fine-tune. You can select one of the [supported models](https://platform.openai.com/docs/guides/fine-tuning/which-models-can-be-fine-tuned).
    pub model: String,

    /// The ID of an uploaded file that contains training data.
    ///
    /// See [upload file](https://platform.openai.com/docs/api-reference/files/create) for how to upload a file.
    ///
    /// Your dataset must be formatted as a JSONL file. Additionally, you must upload your file with the purpose `fine-tune`.
    ///
    /// The contents of the file should differ depending on if the model uses the [chat](https://platform.openai.com/docs/api-reference/fine-tuning/chat-input) or [completions](https://platform.openai.com/docs/api-reference/fine-tuning/completions-input) format.
    ///
    /// See the [fine-tuning guide](https://platform.openai.com/docs/guides/fine-tuning) for more details.
    pub training_file: String,

    /// The hyperparameters used for the fine-tuning job.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hyperparameters: Option<Hyperparameters>,

    /// A string of up to 40 characters that will be added to your fine-tuned model name.
    ///
    /// For example, a `suffix` of "custom-model-name" would produce a model name like `ada:ft-your-org:custom-model-name-2022-02-15-04-21-04`.
    #[serde(skip_serializing_if = "Option::is_none")]
    suffix: Option<String>, // Defaults to null

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

    /// A list of integrations to enable for your fine-tuning job.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integrations: Option<Vec<Integration>>,

    /// The seed controls the reproducibility of the job.
    /// Passing in the same seed and job parameters should produce the same results, but may differ in rare cases.
    /// If a seed is not specified, one will be generated for you.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<f32>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Hyperparameters {
    /// Number of examples in each batch.
    /// A larger batch size means that model parameters are updated less frequently, but with lower variance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<AutoOrInteger>, // Defaults to auto

    /// Scaling factor for the learning rate. A smaller learning rate may be useful to avoid overfitting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub learning_rate_multiplier: Option<AutoOrInteger>, // Defaults to auto

    /// The number of epochs to train the model for. An epoch refers to one full cycle through the training dataset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_epochs: Option<AutoOrInteger>, // Defaults to auto
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
pub enum AutoOrInteger {
    Auto(String),
    Integer(f32),
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Integration {
    /// The type of integration to enable. Currently, only "wandb" (Weights and Biases) is supported.
    pub r#type: String,

    /// The settings for your integration with Weights and Biases.
    /// This payload specifies the project that metrics will be sent to.
    /// Optionally, you can set an explicit display name for your run, add tags to your run, and set a default entity (team, username, etc) to be associated with your run.
    pub wandb: Wandb,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Wandb {
    /// The name of the project that the new run will be created under.
    pub project: String,

    /// A display name to set for the run. If not set, we will use the Job ID as the name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// The entity to use for the run.
    /// This allows you to set the team or username of the WandB user that you would like associated with the run.
    /// If not set, the default entity for the registered WandB API key is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<String>,

    /// A list of tags to be attached to the newly created run.
    /// These tags are passed through directly to WandB. Some default tags are generated by OpenAI: "openai/finetune", "openai/{base-model}", "openai/{ftjob-abcdef}".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

/// The fine_tuning.job object represents a fine-tuning job that has been created through the API.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct FineTuningResponse {
    /// The object type, which is always "fine_tuning.job".
    pub object: String,
    /// The object identifier, which can be referenced in the API endpoints.
    pub id: String,
    /// The base model that is being fine-tuned.
    pub model: String,
    /// The Unix timestamp (in seconds) for when the fine-tuning job was created.
    pub created_at: u64,
    /// The Unix timestamp (in seconds) for when the fine-tuning job was finished. The value will be null if the fine-tuning job is still running.
    pub finished_at: Option<u64>,
    /// The name of the fine-tuned model that is being created. The value will be null if the fine-tuning job is still running.
    pub fine_tuned_model: Option<String>,
    /// The organization that owns the fine-tuning job.
    pub organization_id: String,
    /// The compiled results file ID(s) for the fine-tuning job. You can retrieve the results with the [Files API](https://platform.openai.com/docs/api-reference/files/retrieve-contents).
    pub result_files: Vec<String>,
    /// The current status of the fine-tuning job, which can be either `validating_files`, `queued`, `running`, `succeeded`, `failed`, or `cancelled`.
    pub status: String,
    /// The file ID used for validation. You can retrieve the validation results with the [Files API](https://platform.openai.com/docs/api-reference/files/retrieve-contents).
    pub validation_file: Option<String>,
    /// The file ID used for training. You can retrieve the training data with the [Files API](https://platform.openai.com/docs/api-reference/files/retrieve-contents).
    pub training_file: String,
    /// The total number of billable tokens processed by this fine-tuning job. The value will be null if the fine-tuning job is still running.
    pub trained_tokens: Option<i32>,
    /// The hyperparameters used for the fine-tuning job. See the [fine-tuning guide](https://platform.openai.com/docs/guides/fine-tuning) for more details.
    pub hyperparameters: Hyperparameters,
    /// A list of integrations to enable for this fine-tuning job.
    pub integrations: Option<Vec<Integration>>,
    /// The seed used for the fine-tuning job.
    pub seed: i32,
    /// The Unix timestamp (in seconds) for when the fine-tuning job is estimated to finish. The value will be null if the fine-tuning job is not running.
    pub estimated_finish: Option<i32>,
}

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "ListFineTuningRequestBuilder")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct ListFineTuningRequest {
    /// Identifier for the last job from the previous pagination request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    /// Number of fine-tuning jobs to retrieve.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<f32>, // Defaults to 20
}

/// Fine-tuning job event object
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct FineTuningEventResponse {
    pub object: String,
    pub data: Vec<FineTuningEvent>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct FineTuningEvent {
    object: String,
    id: String,
    created_at: u64,
    level: String,
    message: String,
    r#type: String,
    data: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Metrics {
    step: f64,
    train_loss: f64,
    train_mean_token_accuracy: f64,
    valid_loss: f64,
    valid_mean_token_accuracy: f64,
    full_valid_loss: f64,
    full_valid_mean_token_accuracy: f64,
}

/// The `fine_tuning.job.checkpoint` object represents a model checkpoint for a fine-tuning job that is ready to use.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Checkpoint {
    /// The object type, which is always "fine_tuning.job.checkpoint".
    object: String,
    /// The checkpoint identifier, which can be referenced in the API endpoints.
    id: String,
    /// The Unix timestamp (in seconds) for when the checkpoint was created.
    created_at: u64,
    /// The name of the fine-tuned checkpoint model that is created.
    fine_tuned_model_checkpoint: String,
    /// Metrics at the step number during the fine-tuning job.
    metrics: Metrics,
    /// The name of the fine-tuning job that this checkpoint was created from.
    fine_tuning_job_id: String,
    /// The step number that the checkpoint was created at.
    step_number: u32,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct FineTuningCheckpointResponse {
    object: String,
    data: Vec<Checkpoint>,
    first_id: String,
    last_id: String,
    has_more: bool,
}
