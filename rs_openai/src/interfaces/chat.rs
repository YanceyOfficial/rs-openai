#[allow(unused)]
use crate::apis;
use crate::shared::response_wrapper::OpenAIError;
use crate::shared::types::Stop;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, Default, strum::Display)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    #[strum(serialize = "system")]
    System,
    #[default]
    #[strum(serialize = "user")]
    User,
    #[strum(serialize = "assistant")]
    Assistant,
}

#[derive(Builder, Default, Debug, Clone, Deserialize, Serialize)]
#[builder(name = "ChatCompletionMessageRequestBuilder")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct ChatCompletionMessage {
    /// The role of the author of this message. One of `system`, `user`, or `assistant`.
    pub role: Role,

    /// The contents of the message.
    pub content: String,

    /// The name of the author of this message. May contain a-z, A-Z, 0-9, and underscores, with a maximum length of 64 characters.
    pub name: Option<String>,
}

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "CreateChatRequestBuilder")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct CreateChatRequest {
    /// ID of the model to use.
    /// See the [model endpoint compatibility](https://platform.openai.com/docs/models/model-endpoint-compatibility) table for details on which models work with the Chat API.
    pub model: String,

    /// A list of messages describing the conversation so far.
    pub messages: Vec<ChatCompletionMessage>,

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

    /// How many chat completion choices to generate for each input message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u8>, // default: 1

    /// If set, partial message deltas will be sent, like in ChatGPT.
    /// Tokens will be sent as data-only [server-sent events](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events#Event_stream_format) as they become available, with the stream terminated by a `data: [DONE]` message.
    /// See the OpenAI Cookbook for [example code](https://github.com/openai/openai-cookbook/blob/main/examples/How_to_stream_completions.ipynb).
    ///
    /// For streamed progress, use [`create_with_stream`](apis::chat::Chat::create_with_stream).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>, // default: false

    /// Up to 4 sequences where the API will stop generating further tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Stop>, // default: null

    /// The maximum number of tokens to generate in the chat completion.
    ///
    /// The total length of input tokens and generated tokens is limited by the model's context length.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

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

    /// Modify the likelihood of specified tokens appearing in the completion.
    ///
    /// Accepts a json object that maps tokens (specified by their token ID in the tokenizer) to an associated bias value from -100 to 100.
    /// Mathematically, the bias is added to the logits generated by the model prior to sampling.
    /// The exact effect will vary per model, but values between -1 and 1 should decrease or increase likelihood of selection;
    /// values like -100 or 100 should result in a ban or exclusive selection of the relevant token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<HashMap<String, serde_json::Value>>, // default: null

    /// A unique identifier representing your end-user, which can help OpenAI to monitor and detect abuse. [Learn more](https://platform.openai.com/docs/guides/safety-best-practices/end-user-ids).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ChatUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ChatChoice {
    pub message: ChatCompletionMessage,
    pub finish_reason: String,
    pub index: u32,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ChatResponse {
    pub id: String,
    pub object: String,
    pub created: u32,
    pub choices: Vec<ChatChoice>,
    pub usage: ChatUsage,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Delta {
    pub content: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ChatChoiceStream {
    pub delta: Delta,
    pub finish_reason: Option<String>,
    pub index: u32,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ChatStreamResponse {
    pub id: String,
    pub object: String,
    pub model: String,
    pub created: u32,
    pub choices: Vec<ChatChoiceStream>,
}
