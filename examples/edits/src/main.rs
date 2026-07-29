use dotenvy::dotenv;
use rs_openai::{OpenAI, edits::CreateEditRequestBuilder};
use std::env::var;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(api_key);

    let req = CreateEditRequestBuilder::default()
        .model("text-davinci-edit-001")
        .input("Today is Monkey.")
        .instruction("Fix the grammar mistakes.")
        .build()?;

    let res = client.edits().create(&req).await?;
    println!("{:?}", res);

    Ok(())
}
