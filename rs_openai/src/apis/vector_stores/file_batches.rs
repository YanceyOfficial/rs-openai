//! Batch file-attach operations for a vector store
//! (`/v1/vector_stores/{vector_store_id}/file_batches`). Not yet implemented — see
//! docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct VectorStoreFileBatches<'a> {
    openai: &'a OpenAI,
    vector_store_id: String,
}

impl<'a> VectorStoreFileBatches<'a> {
    pub(crate) fn new(openai: &'a OpenAI, vector_store_id: String) -> Self {
        Self {
            openai,
            vector_store_id,
        }
    }
}
