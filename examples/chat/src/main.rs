use dotenvy::dotenv;
use futures::StreamExt;
use rs_openai::{
    OpenAI,
    chat::{ChatCompletionMessage, CreateChatRequest, Role},
};
use std::env::var;
use std::io::{Write, stdout};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(api_key);

    let req = CreateChatRequest::builder()
        .model("gpt-3.5-turbo")
        .messages(vec![
            ChatCompletionMessage::builder()
                .role(Role::User)
                .content("To Solve LeetCode's problem 81 in Rust.")
                .build(),
        ])
        .build();

    let res = client.chat().create(&req).await?;
    println!("{:?}", res);

    // stream mode
    let req = CreateChatRequest::builder()
        .model("gpt-3.5-turbo")
        .messages(vec![
            ChatCompletionMessage::builder()
                .role(Role::User)
                .content("To Solve LeetCode's problem 81 in Rust.")
                .build(),
        ])
        .stream(true)
        .build();

    let mut stream = client.chat().create_with_stream(&req).await?;

    let mut lock = stdout().lock();
    while let Some(response) = stream.next().await {
        response.unwrap().choices.iter().for_each(|choice| {
            if let Some(ref content) = choice.delta.content {
                write!(lock, "{}", content).unwrap();
            }
        });

        stdout().flush()?;
    }

    Ok(())
}
