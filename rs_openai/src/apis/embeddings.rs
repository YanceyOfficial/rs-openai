//! Get a vector representation of a given input that can be easily consumed by machine learning models and algorithms.
//!
//! Related guide: [Embeddings](https://platform.openai.com/docs/guides/embeddings)

use crate::client::OpenAI;
use crate::interfaces::embeddings;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct Embeddings<'a> {
    openai: &'a OpenAI,
}

impl<'a> Embeddings<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates an embedding vector representing the input text.
    pub async fn create(
        &self,
        req: &embeddings::CreateEmbeddingRequest,
    ) -> OpenAIResponse<embeddings::EmbeddingResponse> {
        self.openai.post("/embeddings", req).await
    }
}
