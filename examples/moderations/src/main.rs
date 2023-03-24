use dotenvy::dotenv;
use rs_openai::{
    moderations::{CreateModerationRequestBuilder, Model},
    OpenAI,
};
use std::env::var;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(&OpenAI {
        api_key: &api_key,
        org_id: None,
    });

    let req = CreateModerationRequestBuilder::default()
        // The `input` parameter supports both of String and Vec<String>.
        .input(["Do you want to build a snowman?", "I will kill you."])

        // The `model` parameter is optional, defaults to "text-moderation-latest".
        .model(Model::Latest)
        .build()?;

    let list = client.moderations().create(&req);
    println!("{:?}", list);

    Ok(())
}
