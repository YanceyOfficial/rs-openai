//! Semantic search over uploaded files, used by the Responses API's `file_search` tool. Core
//! surface only this round: CRUD + search, plus a basic file-attach lifecycle -- see
//! docs/superpowers/specs/2026-08-12-vector-stores-design.md.

pub mod file_batches;
pub mod files;

use crate::client::OpenAI;
pub use crate::interfaces::vector_stores::*;

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

    /// Scoped accessor for files attached to this vector store.
    pub fn files(&self) -> files::VectorStoreFiles<'a> {
        files::VectorStoreFiles::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for batch file-attach operations on this vector store. Not yet
    /// implemented.
    pub fn file_batches(&self) -> file_batches::VectorStoreFileBatches<'a> {
        file_batches::VectorStoreFileBatches::new(self.openai, self.id.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::interfaces::vector_stores::{ChunkingStrategy, StaticChunkingStrategy};

    #[test]
    fn chunking_strategy_auto_serializes_with_type_tag_only() {
        let strategy = ChunkingStrategy::Auto;
        let json = serde_json::to_value(&strategy).unwrap();
        assert_eq!(json, serde_json::json!({"type": "auto"}));
    }

    #[test]
    fn chunking_strategy_static_serializes_with_nested_static_key() {
        let strategy = ChunkingStrategy::Static {
            config: StaticChunkingStrategy::builder()
                .max_chunk_size_tokens(400_u32)
                .chunk_overlap_tokens(100_u32)
                .build(),
        };
        let json = serde_json::to_value(&strategy).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "static",
                "static": {"max_chunk_size_tokens": 400, "chunk_overlap_tokens": 100}
            })
        );
    }

    #[test]
    fn chunking_strategy_other_deserializes_with_no_fields() {
        let value = serde_json::json!({"type": "other"});
        let strategy: ChunkingStrategy = serde_json::from_value(value).unwrap();
        assert!(matches!(strategy, ChunkingStrategy::Other));
    }
}
