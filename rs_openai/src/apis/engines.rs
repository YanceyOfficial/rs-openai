//! These endpoints describe and provide access to the various engines available in the API.
//!
//! # Warning
//!
//! The Engines endpoints are deprecated.
//!
//! Please use their replacement, [Models](https://platform.openai.com/docs/api-reference/models), instead. [Learn more](https://help.openai.com/TODO).

use crate::client::OpenAI;
use crate::interfaces::engines;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct Engines<'a> {
    openai: &'a OpenAI,
}

impl<'a> Engines<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Lists the currently available (non-finetuned) models,
    /// and provides basic information about each one such as the owner and availability.
    #[deprecated(
        note = "The Engines endpoints are deprecated. Please use their replacement, Models, instead."
    )]
    pub async fn list(&self) -> OpenAIResponse<engines::EngineListResponse> {
        self.openai.get("/engines", &()).await
    }

    /// Retrieves a model instance, providing basic information about it such as the owner and availability.
    ///
    /// # Path parameters
    ///
    /// - `engine_id` - The ID of the engine to use for this request
    #[deprecated(
        note = "The Engines endpoints are deprecated. Please use their replacement, Models, instead."
    )]
    pub async fn retrieve(&self, engine_id: &str) -> OpenAIResponse<engines::EngineResponse> {
        self.openai.get(&format!("/engines/{engine_id}"), &()).await
    }
}
