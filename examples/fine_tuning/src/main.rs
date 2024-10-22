use dotenvy::dotenv;
use futures::StreamExt;
use rs_openai::{fine_tuning::CreateFineTuningRequestBuilder, OpenAI};
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
    let req = CreateFineTuningRequestBuilder::default()
        .training_file("YOUR_FINE_TUNE_FILE")
        .model("davinci")
        .build()?;

    let res = client.fine_tuning().create(&req).await?;
    println!("{:?}", res);

    // list
    let res = client.fine_tuning().list().await?;
    println!("{:?}", res);

    // list events
    let res = client.fine_tuning().list_events("").await?;
    println!("{:?}", res);

    // list checkpoints
    let res = client.fine_tuning().list_checkpoints("").await?;
    println!("{:?}", res);

    // retrieve
    let res = client.fine_tuning().retrieve("").await?;
    println!("{:?}", res);

    // cancel
    let res = client.fine_tuning().cancel("").await?;
    println!("{:?}", res);

    Ok(())
}
