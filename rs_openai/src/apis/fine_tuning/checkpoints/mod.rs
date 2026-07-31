//! A specific fine-tuning checkpoint's operations. Not yet implemented — see
//! docs/superpowers/research/2026-07-31-field-diff-images-audio-files-fine-tuning.md.

pub mod permissions;

use crate::client::OpenAI;

pub struct FineTuningCheckpoint<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> FineTuningCheckpoint<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Scoped accessor for which organizations can access this checkpoint. Not yet
    /// implemented.
    pub fn permissions(&self) -> permissions::FineTuningCheckpointPermissions<'a> {
        permissions::FineTuningCheckpointPermissions::new(self.openai, self.id.clone())
    }
}
