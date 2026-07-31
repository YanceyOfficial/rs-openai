//! ChatKit (Beta): embeddable chat UI sessions and their threads. Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

pub mod sessions;
pub mod threads;

use crate::client::OpenAI;

pub struct ChatKit<'a> {
    openai: &'a OpenAI,
}

impl<'a> ChatKit<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Scoped accessor for ChatKit sessions. Not yet implemented.
    pub fn sessions(&self) -> sessions::ChatKitSessions<'a> {
        sessions::ChatKitSessions::new(self.openai)
    }

    /// Scoped accessor for a specific ChatKit session. Not yet implemented.
    pub fn session(&self, id: impl Into<String>) -> sessions::ChatKitSession<'a> {
        sessions::ChatKitSession::new(self.openai, id.into())
    }

    /// Scoped accessor for ChatKit threads. Not yet implemented.
    pub fn threads(&self) -> threads::ChatKitThreads<'a> {
        threads::ChatKitThreads::new(self.openai)
    }

    /// Scoped accessor for a specific ChatKit thread. Not yet implemented.
    pub fn thread(&self, id: impl Into<String>) -> threads::ChatKitThread<'a> {
        threads::ChatKitThread::new(self.openai, id.into())
    }
}
