//! ChatKit sessions: `POST /v1/chatkit/sessions` create,
//! `POST /v1/chatkit/sessions/{session_id}/cancel` cancel. Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct ChatKitSessions<'a> {
    openai: &'a OpenAI,
}

impl<'a> ChatKitSessions<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

#[allow(dead_code)]
pub struct ChatKitSession<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> ChatKitSession<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }
}
