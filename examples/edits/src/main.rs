use dotenvy::dotenv;
use rs_openai::{edits::CreateEditRequestBuilder, OpenAI};
use std::env::var;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(&OpenAI {
        api_key,
        org_id: None,
    });

    let req = CreateEditRequestBuilder::default()
        .model("text-davinci-edit-001")
        .input("Today is Monkey.")
        .instruction("Fix the grammer mistakes.")
        .build()?;

    let res = client.edits().create(&req).await?;
    println!("{:?}", res);

    Ok(())
}
