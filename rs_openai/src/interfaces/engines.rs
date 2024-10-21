use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct EngineResponse {
    pub id: String,
    pub object: String,
    pub owner: String,
    pub ready: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct EngineListResponse {
    pub data: Vec<EngineResponse>,
    pub object: String,
}
