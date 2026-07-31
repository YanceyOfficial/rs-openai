//! Semantic search over uploaded files, used by the Responses API's `file_search` tool. Not
//! yet implemented — see docs/superpowers/research/2026-07-31-openai-api-structure.md.

pub mod file_batches;
pub mod files;

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct VectorStores<'a> {
    openai: &'a OpenAI,
}

impl<'a> VectorStores<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

pub struct VectorStore<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> VectorStore<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Scoped accessor for files attached to this vector store. Not yet implemented.
    pub fn files(&self) -> files::VectorStoreFiles<'a> {
        files::VectorStoreFiles::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for batch file-attach operations on this vector store. Not yet
    /// implemented.
    pub fn file_batches(&self) -> file_batches::VectorStoreFileBatches<'a> {
        file_batches::VectorStoreFileBatches::new(self.openai, self.id.clone())
    }
}
