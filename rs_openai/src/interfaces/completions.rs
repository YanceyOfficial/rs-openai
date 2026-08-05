#[allow(unused)]
use crate::apis;
use crate::shared::types::Stop;
use bon::Builder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Prompt {
    String(String),
    ArrayOfString(Vec<String>),
    ArrayOfTokens(Vec<u16>),
    ArrayOfTokenArrays(Vec<Vec<u16>>),
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateCompletionRequest {
    /// ID of the model to use.
    /// You can use the [List models](https://platform.openai.com/docs/api-reference/models/list) API to see all of your available models, or see our [Model overview](https://platform.openai.com/docs/models/overview) for descriptions of them.
    #[builder(into)]
    pub model: String,

    /// The prompt(s) to generate completions for, encoded as a string, array of strings, array of tokens, or array of token arrays.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<Prompt>, // default: <|endoftext|>

    /// The suffix that comes after a completion of inserted text.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>, // default: null

    /// The maximum number of [token](https://platform.openai.com/tokenizer) to generate in the completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u16>, // default: 16

    /// What sampling temperature to use, between 0 and 2.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>, // min: 0, max: 2, default: 1

    /// An alternative to sampling with temperature (nucleus sampling).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>, //  default: 1

    /// How many completions to generate for each prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u8>, // default: 1

    /// Whether to stream back partial progress. For streamed progress, use [`create_with_stream`](apis::completions::Completions::create_with_stream).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>, // default: false

    /// Include the log probabilities on the `logprobs` most likely tokens, as well the chosen tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<u8>, // default: null

    /// Echo back the prompt in addition to the completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub echo: Option<bool>, // default: false

    /// Up to 4 sequences where the API will stop generating further tokens.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Stop>, // default: null

    /// Number between -2.0 and 2.0. Positive values penalize new tokens based on whether they appear in the text so far.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>, // min: -2.0, max: 2.0, default: 0

    /// Number between -2.0 and 2.0. Positive values penalize new tokens based on their existing frequency in the text so far.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>, // min: -2.0, max: 2.0, default: 0

    /// Generates `best_of` completions server-side and returns the "best" one. Results cannot be streamed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_of: Option<f32>, // default: 1

    /// Modify the likelihood of specified tokens appearing in the completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<HashMap<String, serde_json::Value>>, // default: null

    /// A unique identifier representing your end-user, which can help OpenAI to monitor and detect abuse.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<crate::shared::types::StreamOptions>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u32,
    pub model: String,
    pub choices: Vec<CompletionChoice>,
    pub usage: Usage,
    pub system_fingerprint: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CompletionChoice {
    pub text: String,
    pub index: u32,
    pub logprobs: Option<u8>,
    pub finish_reason: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CompletionChoiceStream {
    pub text: String,
    pub index: usize,
    pub logprobs: Option<u8>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CompletionStreamResponse {
    pub id: String,
    pub object: String,
    pub created: u32,
    pub model: String,
    pub choices: Vec<CompletionChoiceStream>,
}
