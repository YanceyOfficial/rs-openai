//! Get a vector representation of a given input that can be easily consumed by machine learning models and algorithms.
//!
//! Related guide: [Embeddings](https://platform.openai.com/docs/guides/embeddings)

use crate::shared::response_wrapper::OpenAIError;
use crate::{OpenAI, OpenAIResponse};
use derive_builder::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone)]
#[serde(untagged)]
pub enum EmbeddingInput {
    String(String),
    ArrayOfString(Vec<String>),
    ArrayOfTokens(Vec<u16>),
    ArrayOfTokenArrays(Vec<Vec<u16>>),
}

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "CreateEmbeddingRequestBuilder")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct CreateEmbeddingRequest {
    /// ID of the model to use.
    /// Use the [List models](https://platform.openai.com/docs/api-reference/models/list) API to see all of your available models, or see our [Model overview](https://platform.openai.com/docs/models/overview) for descriptions of them.
    pub model: String,

    /// Input text to get embeddings for, encoded as a string or array of tokens.
    /// To get embeddings for multiple inputs in a single request, pass an array of strings or array of token arrays. Each input must not exceed 8192 tokens in length.
    pub input: EmbeddingInput,

    /// A unique identifier representing your end-user, which can help OpenAI to monitor and detect abuse. [Learn more](https://platform.openai.com/docs/guides/safety-best-practices/end-user-ids).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingResponse {
    pub object: String,
    pub data: Vec<EmbeddingData>,
    pub model: String,
    pub usage: Usage,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingData {
    pub object: String,
    pub embedding: Vec<f32>,
    pub index: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub total_tokens: u32,
}

pub struct Embeddings<'a> {
    openai: &'a OpenAI,
}

impl<'a> Embeddings<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates an embedding vector representing the input text.
    #[tokio::main]
    pub async fn create(&self, req: &CreateEmbeddingRequest) -> OpenAIResponse<EmbeddingResponse> {
        self.openai.post("/embeddings", req).await
    }
}
