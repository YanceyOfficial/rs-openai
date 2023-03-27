use dotenvy::dotenv;
use rs_openai::{
    moderations::{CreateModerationRequestBuilder, ModerationModel},
    OpenAI,
};
use std::env::var;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(&OpenAI {
        api_key,
        org_id: None,
    });

    // create
    let req = CreateModerationRequestBuilder::default()
        .input(["Do you want to build a snowman?", "I will kill you."])
        .model(ModerationModel::Latest)
        .build()?;

    let res = client.moderations().create(&req);
    println!("{:?}", res);

    Ok(())
}
