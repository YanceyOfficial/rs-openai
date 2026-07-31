//! Custom voice registration (`/v1/audio/voices`). Not yet implemented — see
//! docs/superpowers/research/2026-07-31-field-diff-images-audio-files-fine-tuning.md.
//! Only `create` is confirmed so far; add a singular, ID-scoped `AudioVoice` accessor
//! here once the full operation set is confirmed during implementation.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct AudioVoices<'a> {
    openai: &'a OpenAI,
}

impl<'a> AudioVoices<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}
