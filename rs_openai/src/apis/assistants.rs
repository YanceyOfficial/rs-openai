//! Build assistants that can call models and use tools to perform tasks.
//!
//! [Get started with the Assistants API](https://platform.openai.com/docs/assistants)

use crate::client::OpenAI;
use crate::interfaces::assistants;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct Assistants<'a> {
    openai: &'a OpenAI,
}

impl<'a> Assistants<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Create an assistant with a model and instructions.
    pub async fn create_assistant(
        &self,
        req: &assistants::AssistantRequest,
    ) -> OpenAIResponse<assistants::AssistantResponse> {
        self.openai.post("/assistants", req).await
    }

    /// Returns a list of assistants.
    pub async fn list_assistants(
        &self,
        req: &assistants::ListAssistantRequest,
    ) -> OpenAIResponse<assistants::ListAssistantResponse> {
        self.openai.get("/assistants", req).await
    }

    /// Retrieves an assistant.
    pub async fn retrieve_assistant(
        &self,
        assistant_id: &str,
    ) -> OpenAIResponse<assistants::AssistantResponse> {
        self.openai
            .get(&format!("/assistants/{assistant_id}"), &())
            .await
    }

    /// Modifies an assistant.
    pub async fn modify_assistant(
        &self,
        assistant_id: &str,
        req: &assistants::AssistantRequest,
    ) -> OpenAIResponse<assistants::AssistantResponse> {
        self.openai
            .post(&format!("/assistants/{assistant_id}"), req)
            .await
    }

    /// Delete an assistant.
    pub async fn delete_assistant(
        &self,
        assistant_id: &str,
    ) -> OpenAIResponse<assistants::DeleteAssistantResponse> {
        self.openai
            .delete(&format!("/assistants/{assistant_id}"), &())
            .await
    }
}
