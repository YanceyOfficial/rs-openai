//! Create threads that assistants can interact with.
//!
//! Related guide: [Assistants](https://platform.openai.com/docs/assistants/overview)

use crate::client::OpenAI;
use crate::interfaces::threads;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct Threads<'a> {
    openai: &'a OpenAI,
}

impl<'a> Threads<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Create a thread.
    pub async fn create(
        &self,
        req: &threads::CreateThreadRequest,
    ) -> OpenAIResponse<threads::ThreadResponse> {
        self.openai.post("/threads", req).await
    }

    /// Retrieves a thread.
    pub async fn retrieve(
        &self,
        thread_id: &str, // The thread object matching the specified ID.
    ) -> OpenAIResponse<threads::ThreadResponse> {
        self.openai.get(&format!("/threads/{thread_id}"), &()).await
    }

    /// Modifies a thread.
    pub async fn modify(
        &self,
        thread_id: &str, // The ID of the thread to modify. Only the `metadata` can be modified.
        req: &threads::ModifyThreadRequest,
    ) -> OpenAIResponse<threads::ThreadResponse> {
        self.openai
            .post(&format!("/threads/{thread_id}"), req)
            .await
    }

    /// Delete a thread.
    pub async fn delete(
        &self,
        thread_id: &str, // The ID of the thread to modify. Only the `metadata` can be modified.
    ) -> OpenAIResponse<threads::DeleteThreadResponse> {
        self.openai
            .delete(&format!("/threads/{thread_id}"), &())
            .await
    }
}
