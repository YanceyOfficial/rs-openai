//! Standalone grading-logic definitions used by Evals. Operation set not yet confirmed — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md. Not yet implemented.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct Graders<'a> {
    openai: &'a OpenAI,
}

impl<'a> Graders<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}
