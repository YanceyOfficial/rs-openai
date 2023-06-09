use dotenvy::dotenv;
use futures::StreamExt;
use rs_openai::{completions::CreateCompletionRequestBuilder, OpenAI};
use std::env::var;
use std::io::{stdout, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(&OpenAI {
        api_key,
        org_id: None,
    });

    // create
    let req = CreateCompletionRequestBuilder::default()
        .model("text-davinci-003")
        .prompt("What's your name?")
        .max_tokens(40_u16)
        .build()?;

    let res = client.completions().create(&req).await?;
    println!("{:?}", res);

    // create_with_stream
    let req = CreateCompletionRequestBuilder::default()
        .model("text-davinci-003")
        .prompt("What's your name?")
        .max_tokens(40_u16)
        .stream(true)
        .build()?;

    let mut stream = client.completions().create_with_stream(&req).await?;

    let mut lock = stdout().lock();
    while let Some(data) = stream.next().await {
        data.unwrap().choices.iter().for_each(|choice| {
            write!(lock, "{}", choice.text).unwrap();
        });

        stdout().flush()?;
    }

    Ok(())
}
