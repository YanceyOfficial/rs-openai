use dotenvy::dotenv;
use futures::StreamExt;
use rs_openai::{fine_tuning::CreateFineTuneRequestBuilder, OpenAI};
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
    let req = CreateFineTuneRequestBuilder::default()
        .training_file("YOUR_FINE_TUNE_FILE")
        .model("davinci")
        .n_epochs(4u32)
        .batch_size(1u32)
        .learning_rate_multiplier(0.1)
        .prompt_loss_weight(0.01)
        .build()?;

    let res = client.fine_tuning().create(&req).await?;
    println!("{:?}", res);

    // list
    let res = client.fine_tuning().list().await?;
    println!("{:?}", res);

    // retrieve
    let res = client.fine_tuning().retrieve("").await?;
    println!("{:?}", res);

    // cancel
    let res = client.fine_tuning().cancel("").await?;
    println!("{:?}", res);

    // retrieve_content
    // TODO: Since free accounts cannot read fine-tune event content, I have to verify this api until purchase a Plus.
    let res = client.fine_tuning().retrieve_content("").await?;
    println!("{:?}", res);

    // retrieve_content_stream
    // TODO: Since free accounts cannot read fine-tune event content, I have to verify this api until purchase a Plus.
    let mut stream = client.fine_tuning().retrieve_content_stream("").await?;

    let mut lock = stdout().lock();
    while let Some(response) = stream.next().await {
        response.unwrap().data.iter().for_each(|choice| {
            write!(lock, "{}", choice.message).unwrap();
        });

        stdout().flush()?;
    }

    Ok(())
}
