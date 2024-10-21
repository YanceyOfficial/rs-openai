//! Given a prompt, the model will return one or more predicted completions, and can also return the probabilities of alternative tokens at each position.

use crate::client::OpenAI;
use crate::interfaces::completions;
use crate::shared::response_wrapper::{OpenAIError, OpenAIResponse};
use crate::shared::utils::is_stream;
use futures::Stream;
use std::pin::Pin;

pub struct Completions<'a> {
    openai: &'a OpenAI,
}

impl<'a> Completions<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates a completion for the provided prompt and parameters.
    pub async fn create(
        &self,
        req: &completions::CreateCompletionRequest,
    ) -> OpenAIResponse<completions::CompletionResponse> {
        if is_stream(req.stream) {
            return Err(OpenAIError::InvalidArgument(
                "When stream is true, use Completions::create_with_stream".into(),
            ));
        }

        self.openai.post("/completions", req).await
    }

    /// Creates a completion for the provided prompt and parameters.
    pub async fn create_with_stream(
        &self,
        req: &completions::CreateCompletionRequest,
    ) -> Result<
        Pin<Box<dyn Stream<Item = OpenAIResponse<completions::CompletionStreamResponse>> + Send>>,
        OpenAIError,
    > {
        if !is_stream(req.stream) {
            return Err(OpenAIError::InvalidArgument(
                "When stream is false, use Completions::create".into(),
            ));
        }

        Ok(self.openai.post_stream("/completions", req).await)
    }
}
