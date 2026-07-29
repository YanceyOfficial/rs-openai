use dotenvy::dotenv;
use rs_openai::{OpenAI, embeddings::CreateEmbeddingRequest};
use std::env::var;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(api_key);

    let req = CreateEmbeddingRequest::builder()
        .model("text-embedding-ada-002")
        .input([
            "The quick brown fox jumps over the lazy dog.",
            "Rust is a systems programming language.",
            "OpenAI provides a REST API for language models.",
            "This is a short paragraph used purely as embedding input.",
        ])
        .build();

    let res = client.embeddings().create(&req).await?;
    println!("{:?}", res);

    Ok(())
}
