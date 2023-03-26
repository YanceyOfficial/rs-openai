//! List and describe the various models available in the API.
//! You can refer to the [Models](https://platform.openai.com/docs/models/overview) documentation to understand what models are available and the differences between them.

use crate::{OpenAI, OpenAIResponse};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct ModelPermission {
    pub id: String,
    pub object: String,
    pub created: u32,
    pub allow_create_engine: bool,
    pub allow_sampling: bool,
    pub allow_logprobs: bool,
    pub allow_search_indices: bool,
    pub allow_view: bool,
    pub allow_fine_tuning: bool,
    pub organization: String,
    pub group: Option<serde_json::Value>,
    pub is_blocking: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ModelResponse {
    pub id: String,
    pub object: String,
    pub created: u32,
    pub owned_by: String,
    pub permission: Vec<ModelPermission>,
    pub root: String,
    pub parent: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ListModelResponse {
    pub object: String,
    pub data: Vec<ModelResponse>,
}

pub struct Models<'a> {
    openai: &'a OpenAI<'a>,
}

impl<'a> Models<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Retrieves a model instance, providing basic information about the model such as the owner and permissioning.
    ///
    /// # Path parameters
    ///
    /// - `model` - The ID of the model to use for this request.
    #[tokio::main]
    pub async fn retrieve(&self, model: &str) -> OpenAIResponse<ModelResponse> {
        self.openai.get(&format!("/models/{model}"), &()).await
    }

    /// Lists the currently available models, and provides basic information about each one such as the owner and availability.
    #[tokio::main]
    pub async fn list(&self) -> OpenAIResponse<ListModelResponse> {
        self.openai.get("/models", &()).await
    }
}
