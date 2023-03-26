use dotenvy::dotenv;
use rs_openai::{completions::CreateCompletionRequestBuilder, OpenAI};
use std::env::var;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(&OpenAI {
        api_key: &api_key,
        org_id: None,
    });

    let req = CreateCompletionRequestBuilder::default()
        .model("text-davinci-003")
        .prompt(["Tell me a joke about the universe", "hello"])
        .max_tokens(40_u16)
        .build()?;

    let res = client.completions().create(&req);
    println!("{:?}", res);

    Ok(())
}
