use serde::Serialize;

#[derive(Debug, Serialize, Clone, Default)]
pub struct File {
    pub buffer: Vec<u8>,
    pub filename: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(untagged)]
pub enum Stop {
    String(String),
    ArrayOfString(Vec<String>),
}
