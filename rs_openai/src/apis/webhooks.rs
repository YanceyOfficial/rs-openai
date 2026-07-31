//! Signature verification and typed event payloads for OpenAI's outbound webhooks (e.g.
//! batch completion, eval run completion). Not a CRUD resource — needs an "unwrap"-style
//! helper (verify signature + deserialize payload) rather than HTTP-calling methods. Not yet
//! implemented — see docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct Webhooks<'a> {
    openai: &'a OpenAI,
}

impl<'a> Webhooks<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}
