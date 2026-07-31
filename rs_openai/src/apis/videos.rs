//! Sora video generation. Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct Videos<'a> {
    openai: &'a OpenAI,
}

impl<'a> Videos<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

#[allow(dead_code)]
pub struct Video<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> Video<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }
}
