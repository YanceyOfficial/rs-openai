//! A container's files (`/v1/containers/{container_id}/files`), including content retrieve.
//! Not yet implemented — see docs/superpowers/research/2026-07-31-openai-api-structure.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct ContainerFiles<'a> {
    openai: &'a OpenAI,
    container_id: String,
}

impl<'a> ContainerFiles<'a> {
    pub(crate) fn new(openai: &'a OpenAI, container_id: String) -> Self {
        Self {
            openai,
            container_id,
        }
    }
}
