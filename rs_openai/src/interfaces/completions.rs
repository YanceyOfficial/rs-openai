#[allow(unused)]
use crate::apis;
use crate::shared::response_wrapper::OpenAIError;
use crate::shared::types::Stop;
use derive_builder::Builder;
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

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "CreateCompletionRequestBuilder")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct CreateCompletionRequest {
    /// ID of the model to use.
    /// You can use the [List models](https://platform.openai.com/docs/api-reference/models/list) API to see all of your available models, or see our [Model overview](https://platform.openai.com/docs/models/overview) for descriptions of them.
    pub model: String,

    /// The prompt(s) to generate completions for, encoded as a string, array of strings, array of tokens, or array of token arrays.
    ///
    /// Note that <|endoftext|> is the document separator that the model sees during training, so if a prompt is not specified the model will generate as if from the beginning of a new document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<Prompt>, // default: <|endoftext|>

    /// The suffix that comes after a completion of inserted text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>, // default: null

    /// The maximum number of [token](https://platform.openai.com/tokenizer) to generate in the completion
    ///
    /// The token count of your prompt plus `max_tokens` cannot exceed the model's context length.
    /// Most models have a context length of 2048 tokens (except for the newest models, which support 4096).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u16>, // default: 16

    /// What sampling temperature to use, between 0 and 2.
    /// Higher values like 0.8 will make the output more random, while lower values like 0.2 will make it more focused and deterministic.
    ///
    /// We generally recommend altering this or `top_p` but not both.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>, // min: 0, max: 2, default: 1

    /// An alternative to sampling with temperature, called nucleus sampling, where the model considers the results of the tokens with top_p probability mass.
    /// So 0.1 means only the tokens comprising the top 10% probability mass are considered.
    ///
    /// We generally recommend altering this or `temperature` but not both.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>, //  default: 1

    /// How many completions to generate for each prompt
    ///
    /// **Note**: Because this parameter generates many completions, it can quickly consume your token quota.
    /// Use carefully and ensure that you have reasonable settings for `max_tokens` and `stop`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u8>, // default: 1

    /// Whether to stream back partial progress.
    /// If set, tokens will be sent as data-only [server-sent events](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events#Event_stream_format) as they become available, with the stream terminated by a `data: [DONE]` message.
    ///
    /// For streamed progress, use [`create_with_stream`](apis::completions::Completions::create_with_stream).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>, // default: false

    /// Include the log probabilities on the `logprobs` most likely tokens, as well the chosen tokens.
    /// For example, if `logprobs` is 5, the API will return a list of the 5 most likely tokens.
    /// The API will always return the `logprob` of the sampled token, so there may be up to `logprobs+1` elements in the response.
    ///
    /// The maximum value for `logprobs` is 5. If you need more than this, please contact us through our [Help center](https://help.openai.com/) and describe your use case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<u8>, // default: null

    /// Echo back the prompt in addition to the completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub echo: Option<bool>, // default: false

    /// Up to 4 sequences where the API will stop generating further tokens. The returned text will not contain the stop sequence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Stop>, // default: null

    /// Number between -2.0 and 2.0.
    /// Positive values penalize new tokens based on whether they appear in the text so far, increasing the model's likelihood to talk about new topics.
    ///
    /// [See more information about frequency and presence penalties.](https://platform.openai.com/docs/api-reference/parameter-details)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>, // min: -2.0, max: 2.0, default: 0

    /// Number between -2.0 and 2.0.
    /// Positive values penalize new tokens based on their existing frequency in the text so far, decreasing the model's likelihood to repeat the same line verbatim.
    ///
    /// [See more information about frequency and presence penalties.](https://platform.openai.com/docs/api-reference/parameter-details)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>, // min: -2.0, max: 2.0, default: 0

    /// Generates `best_of` completions server-side and returns the "best" (the one with the highest log probability per token).
    /// Results cannot be streamed.
    ///
    /// When used with `n`, `best_of` controls the number of candidate completions and n specifies how many to return – `best_of` must be greater than `n`.
    ///
    /// **Note**: Because this parameter generates many completions, it can quickly consume your token quota.
    /// Use carefully and ensure that you have reasonable settings for `max_tokens` and `stop`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_of: Option<f32>, // default: 1

    /// Modify the likelihood of specified tokens appearing in the completion.
    ///
    /// Accepts a json object that maps tokens (specified by their token ID in the GPT tokenizer) to an associated bias value from -100 to 100.
    /// You can use this [tokenizer](https://platform.openai.com/tokenizer?view=bpe) tool (which works for both GPT-2 and GPT-3) to convert text to token IDs.
    /// Mathematically, the bias is added to the logits generated by the model prior to sampling.
    /// The exact effect will vary per model, but values between -1 and 1 should decrease or increase likelihood of selection;
    /// values like -100 or 100 should result in a ban or exclusive selection of the relevant token.
    ///
    /// As an example, you can pass `{"50256": -100}` to prevent the <|endoftext|> token from being generated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<HashMap<String, serde_json::Value>>, // default: null

    /// A unique identifier representing your end-user, which can help OpenAI to monitor and detect abuse. [Learn more](https://platform.openai.com/docs/guides/safety-best-practices/end-user-ids).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u32,
    pub model: String,
    pub choices: Vec<CompletionChoice>,
    pub usage: Usage,
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
