use serde::{Deserialize, Serialize};

/// Describes an OpenAI model offering that can be used with the API.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ModelResponse {
    // The model identifier, which can be referenced in the API endpoints.
    pub id: String,
    /// The object type, which is always "model".
    pub object: String,
    /// The Unix timestamp (in seconds) when the model was created.
    pub created: u32,
    /// The organization that owns the model.
    pub owned_by: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListModelResponse {
    pub object: String,
    pub data: Vec<ModelResponse>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteModelResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}
