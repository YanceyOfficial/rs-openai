//! Given a chat conversation, the model will return a chat completion response.

use crate::client::OpenAI;
use crate::interfaces::chat;
use crate::shared::response_wrapper::{OpenAIError, OpenAIResponse};
use crate::shared::utils::is_stream;
use futures::Stream;
use std::pin::Pin;

pub struct Chat<'a> {
    openai: &'a OpenAI,
}

impl<'a> Chat<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates a completion for the chat message.
    pub async fn create(
        &self,
        req: &chat::CreateChatRequest,
    ) -> OpenAIResponse<chat::ChatResponse> {
        if is_stream(req.stream) {
            return Err(OpenAIError::InvalidArgument(
                "When stream is true, use Chat::create_with_stream".into(),
            ));
        }

        self.openai.post("/chat/completions", req).await
    }

    /// Creates a completion for the chat message.
    pub async fn create_with_stream(
        &self,
        req: &chat::CreateChatRequest,
    ) -> Result<
        Pin<Box<dyn Stream<Item = OpenAIResponse<chat::ChatStreamResponse>> + Send>>,
        OpenAIError,
    > {
        if !is_stream(req.stream) {
            return Err(OpenAIError::InvalidArgument(
                "When stream is false, use Chat::create".into(),
            ));
        }

        Ok(self.openai.post_stream("/chat/completions", req).await)
    }
}
