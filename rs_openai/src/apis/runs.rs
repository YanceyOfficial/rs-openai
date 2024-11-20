//! Create runs that assistants can interact with.
//!
//! Related guide: [Assistants](https://platform.openai.com/docs/assistants/overview)

use crate::client::OpenAI;
use crate::interfaces::runs;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct Runs<'a> {
    openai: &'a OpenAI,
}

impl<'a> Runs<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Create a run.
    ///
    /// # Path parameters
    ///
    /// - `thread_id` - The ID of the thread to run.
    pub async fn create(
        &self,
        thread_id: &str,
        req: &runs::CreateRunRequest,
    ) -> OpenAIResponse<runs::RunResponse> {
        self.openai
            .post(&format!("/threads/{thread_id}/runs"), req)
            .await
    }

    /// Retrieves a thread.
    pub async fn retrieve(
        &self,
        thread_id: &str, // The thread object matching the specified ID.
    ) -> OpenAIResponse<runs::RunResponse> {
        self.openai.get(&format!("/runs/{thread_id}"), &()).await
    }

    /// Modifies a thread.
    pub async fn modify(
        &self,
        thread_id: &str, // The ID of the thread to modify. Only the `metadata` can be modified.
        req: &runs::ModifyRunRequest,
    ) -> OpenAIResponse<runs::RunResponse> {
        self.openai.post(&format!("/runs/{thread_id}"), req).await
    }

    /// Delete a thread.
    pub async fn delete(
        &self,
        thread_id: &str, // The ID of the thread to modify. Only the `metadata` can be modified.
    ) -> OpenAIResponse<runs::DeleteRunResponse> {
        self.openai.delete(&format!("/runs/{thread_id}"), &()).await
    }
}
