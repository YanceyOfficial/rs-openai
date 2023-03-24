use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone, Default)]
pub struct FileMeta {
    pub buffer: Vec<u8>,
    pub filename: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Stop {
    String(String),
    ArrayOfString(Vec<String>),
}
