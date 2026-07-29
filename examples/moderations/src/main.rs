use dotenvy::dotenv;
use rs_openai::{OpenAI, moderations::CreateModerationRequest};
use std::env::var;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(api_key);

    let req = CreateModerationRequest::builder()
        .input(["Hello, how are you today?", "This is a plain test message."])
        .build();

    let res = client.moderations().create(&req).await?;
    println!("{:?}", res);

    Ok(())
}
