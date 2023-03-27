use dotenvy::dotenv;
use rs_openai::{embeddings::CreateEmbeddingRequestBuilder, OpenAI};
use std::env::var;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(&OpenAI {
        api_key,
        org_id: None,
    });

    let req = CreateEmbeddingRequestBuilder::default()
        .model("text-embedding-ada-002")
        .input([
            "Some say love it is a river, That drowns the tender reed",
            "Some say love it is a razor, That leaves your soul to bleed",
            "Some say love it is a hunger, An endless aching need",
            "I say love it is a flower, And you it's only seed",
        ])
        .build()?;

    let res = client.embeddings().create(&req);
    println!("{:?}", res);

    Ok(())
}
