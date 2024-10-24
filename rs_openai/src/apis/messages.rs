//! Create messages that assistants can interact with.
//!
//! Related guide: [Assistants](https://platform.openai.com/docs/assistants/overview)

use crate::client::OpenAI;
use crate::interfaces::messages;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct Messages<'a> {
    openai: &'a OpenAI,
}

impl<'a> Messages<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Create a message.
    ///     
    /// # Path parameters
    ///
    /// - `thread_id` - The ID of the [thread](https://platform.openai.com/docs/api-reference/threads) to create a message for.
    pub async fn create(
        &self,
        thread_id: &str,
        req: &messages::CreateMessageRequest,
    ) -> OpenAIResponse<messages::MessageResponse> {
        self.openai
            .post(&format!("/threads/{thread_id}/messages"), req)
            .await
    }

    /// Returns a list of messages for a given thread.
    ///    
    /// # Path parameters
    ///
    /// - `thread_id` - The ID of the [thread](https://platform.openai.com/docs/api-reference/threads) to create a message for.
    pub async fn list(
        &self,
        thread_id: &str,
        req: &messages::ListMessageRequest,
    ) -> OpenAIResponse<messages::ListMessageResponse> {
        self.openai
            .get(&format!("/threads/{thread_id}/messages"), req)
            .await
    }

    /// Retrieves a message.
    ///   
    /// # Path parameters
    ///
    /// - `thread_id` - The ID of the [thread](https://platform.openai.com/docs/api-reference/threads) to create a message for.
    /// - `message_id` - The ID of the message to retrieve.
    pub async fn retrieve(
        &self,
        thread_id: &str,
        message_id: &str,
    ) -> OpenAIResponse<messages::MessageResponse> {
        self.openai
            .get(&format!("/threads/{thread_id}/messages/{message_id}"), &())
            .await
    }

    /// Modifies a message.
    ///
    /// # Path parameters
    ///
    /// - `thread_id` - The ID of the [thread](https://platform.openai.com/docs/api-reference/threads) to create a message for.
    /// - `message_id` - The ID of the message to retrieve.
    pub async fn modify(
        &self,
        thread_id: &str,
        message_id: &str,
        req: &messages::ModifyMessageRequest,
    ) -> OpenAIResponse<messages::MessageResponse> {
        self.openai
            .post(&format!("/threads/{thread_id}/messages/{message_id}"), req)
            .await
    }

    /// Delete a message.
    ///
    /// # Path parameters
    ///
    /// - `thread_id` - The ID of the [thread](https://platform.openai.com/docs/api-reference/threads) to create a message for.
    /// - `message_id` - The ID of the message to retrieve.
    pub async fn delete(
        &self,
        thread_id: &str,
        message_id: &str,
    ) -> OpenAIResponse<messages::DeleteMessageResponse> {
        self.openai
            .delete(&format!("/threads/{thread_id}/messages/{message_id}"), &())
            .await
    }
}
