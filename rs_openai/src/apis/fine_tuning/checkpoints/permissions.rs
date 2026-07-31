//! Grants/revokes other organizations' access to a fine-tuning checkpoint
//! (`POST`/`GET`/`DELETE .../fine_tuning/checkpoints/{checkpoint}/permissions`,
//! admin-key-only). Not yet implemented — see
//! docs/superpowers/research/2026-07-31-field-diff-images-audio-files-fine-tuning.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct FineTuningCheckpointPermissions<'a> {
    openai: &'a OpenAI,
    checkpoint_id: String,
}

impl<'a> FineTuningCheckpointPermissions<'a> {
    pub(crate) fn new(openai: &'a OpenAI, checkpoint_id: String) -> Self {
        Self {
            openai,
            checkpoint_id,
        }
    }
}
