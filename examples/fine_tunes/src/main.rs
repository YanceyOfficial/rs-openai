use dotenvy::dotenv;
use rs_openai::{fine_tunes::CreateFineTuneRequestBuilder, OpenAI};
use std::env::var;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(&OpenAI {
        api_key: &api_key,
        org_id: None,
    });

    let req = CreateFineTuneRequestBuilder::default()
        .training_file("")
        .validation_file("")
        .model("davinci")
        .n_epochs(4 as u32)
        .batch_size(1 as u32)
        .learning_rate_multiplier(0.1)
        .prompt_loss_weight(0.01)
        .compute_classification_metrics(false)
        .classification_n_classes(1 as u32)
        .classification_positive_class("")
        .classification_betas(vec![0.1, 0.2])
        .suffix("")
        .build()?;

    let create = client.fine_tunes().create(&req);
    println!("{:?}", create);

    let list = client.fine_tunes().list();
    println!("{:?}", list);

    let retrieve = client.fine_tunes().retrieve("");
    println!("{:?}", retrieve);

    let cancel = client.fine_tunes().cancel("");
    println!("{:?}", cancel);

    let event = client.fine_tunes().retrieve_content("");
    println!("{:?}", event);

    Ok(())
}
