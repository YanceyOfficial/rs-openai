//! Consent records for voice cloning / custom-voice features
//! (`/v1/audio/voice_consents`). Not yet implemented — see
//! docs/superpowers/research/2026-07-31-field-diff-images-audio-files-fine-tuning.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct AudioVoiceConsents<'a> {
    openai: &'a OpenAI,
}

impl<'a> AudioVoiceConsents<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

#[allow(dead_code)]
pub struct AudioVoiceConsent<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> AudioVoiceConsent<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }
}
