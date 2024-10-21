//! Given a prompt and an instruction, the model will return an edited version of the prompt.

use crate::client::OpenAI;
use crate::interfaces::edits;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct Edits<'a> {
    openai: &'a OpenAI,
}

impl<'a> Edits<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates a new edit for the provided input, instruction, and parameters.
    pub async fn create(
        &self,
        req: &edits::CreateEditRequest,
    ) -> OpenAIResponse<edits::EditResponse> {
        self.openai.post("/edits", req).await
    }
}
