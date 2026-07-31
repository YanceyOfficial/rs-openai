//! Sandboxed containers backing the Code Interpreter tool. Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

pub mod files;

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct Containers<'a> {
    openai: &'a OpenAI,
}

impl<'a> Containers<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

pub struct Container<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> Container<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Scoped accessor for files inside this container. Not yet implemented.
    pub fn files(&self) -> files::ContainerFiles<'a> {
        files::ContainerFiles::new(self.openai, self.id.clone())
    }
}
