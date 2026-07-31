//! ChatKit threads: `GET /v1/chatkit/threads` list, `GET .../{thread_id}` retrieve,
//! `DELETE .../{thread_id}` delete, `GET .../{thread_id}/items` list items. Not yet
//! implemented — see docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct ChatKitThreads<'a> {
    openai: &'a OpenAI,
}

impl<'a> ChatKitThreads<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

pub struct ChatKitThread<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> ChatKitThread<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Scoped accessor for this thread's items. Not yet implemented.
    pub fn items(&self) -> ChatKitThreadItems<'a> {
        ChatKitThreadItems::new(self.openai, self.id.clone())
    }
}

#[allow(dead_code)]
pub struct ChatKitThreadItems<'a> {
    openai: &'a OpenAI,
    thread_id: String,
}

impl<'a> ChatKitThreadItems<'a> {
    pub(crate) fn new(openai: &'a OpenAI, thread_id: String) -> Self {
        Self { openai, thread_id }
    }
}
