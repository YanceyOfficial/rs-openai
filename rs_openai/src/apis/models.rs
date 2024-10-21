//! List and describe the various models available in the API.
//! You can refer to the [Models](https://platform.openai.com/docs/models/overview) documentation to understand what models are available and the differences between them.

use crate::client::OpenAI;
use crate::interfaces::models;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct Models<'a> {
    openai: &'a OpenAI,
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
    pub async fn retrieve(&self, model: &str) -> OpenAIResponse<models::ModelResponse> {
        self.openai.get(&format!("/models/{model}"), &()).await
    }

    /// Lists the currently available models, and provides basic information about each one such as the owner and availability.
    pub async fn list(&self) -> OpenAIResponse<models::ListModelResponse> {
        self.openai.get("/models", &()).await
    }
}
