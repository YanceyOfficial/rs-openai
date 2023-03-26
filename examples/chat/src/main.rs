use dotenvy::dotenv;
use rs_openai::{
    chat::{ChatCompletionMessageRequestBuilder, CreateChatRequestBuilder, Role},
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

    // let req = CreateChatRequestBuilder::default()
    //     .model("gpt-3.5-turbo")
    //     .messages(vec![ChatCompletionMessageRequestBuilder::default()
    //         .role(Role::User)
    //         .content("To Solve LeetCode's problem 81 in Rust.")
    //         .build()?])
    //     .build()?;

    // let res = client.chat().create(&req);
    // println!("{:?}", res);

    let req = CreateChatRequestBuilder::new(
        "gpt-3.5-turbo",
        vec![ChatCompletionMessageRequestBuilder::default()
            .role(Role::User)
            .content("To Solve LeetCode's problem 82 in Rust.")
            .build()?],
    )
    .top_p(1_f32)
    .n(1)
    .build()?;

    let res = client.chat().create(&req);
    println!("{:?}", res);

    Ok(())
}
