//! Given a input text, outputs if the model classifies it as violating OpenAI's content policy.
//!
//! Related guide: [Moderations](https://platform.openai.com/docs/guides/moderation)

use crate::client::OpenAI;
use crate::interfaces::moderations;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct Moderations<'a> {
    openai: &'a OpenAI,
}

impl<'a> Moderations<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Classifies if text violates OpenAI's Content Policy.
    pub async fn create(
        &self,
        req: &moderations::CreateModerationRequest,
    ) -> OpenAIResponse<moderations::ModerationResponse> {
        self.openai.post("/moderations", req).await
    }
}
