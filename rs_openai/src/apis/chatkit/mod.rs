//! ChatKit (Beta): embeddable chat UI sessions and their threads. Sessions implemented;
//! Threads implemented at minimal fidelity (list/retrieve/delete only, Items sub-resource
//! still not implemented) — see docs/superpowers/specs/2026-08-14-chatkit-design.md.

pub mod sessions;
pub mod threads;

use crate::client::OpenAI;
pub use crate::interfaces::chatkit::*;

pub struct ChatKit<'a> {
    openai: &'a OpenAI,
}

impl<'a> ChatKit<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Scoped accessor for ChatKit sessions.
    pub fn sessions(&self) -> sessions::ChatKitSessions<'a> {
        sessions::ChatKitSessions::new(self.openai)
    }

    /// Scoped accessor for a specific ChatKit session.
    pub fn session(&self, id: impl Into<String>) -> sessions::ChatKitSession<'a> {
        sessions::ChatKitSession::new(self.openai, id.into())
    }

    /// Scoped accessor for ChatKit threads.
    pub fn threads(&self) -> threads::ChatKitThreads<'a> {
        threads::ChatKitThreads::new(self.openai)
    }

    /// Scoped accessor for a specific ChatKit thread.
    pub fn thread(&self, id: impl Into<String>) -> threads::ChatKitThread<'a> {
        threads::ChatKitThread::new(self.openai, id.into())
    }
}
