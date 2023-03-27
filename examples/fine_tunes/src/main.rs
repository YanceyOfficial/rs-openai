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

    // create
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

    let res = client.fine_tunes().create(&req);
    println!("{:?}", res);

    // list
    let res = client.fine_tunes().list();
    println!("{:?}", res);

    // retrieve
    let res = client.fine_tunes().retrieve("");
    println!("{:?}", res);

    // cancel
    let res = client.fine_tunes().cancel("");
    println!("{:?}", res);

    // retrieve_content
    let res = client.fine_tunes().retrieve_content("");
    println!("{:?}", res);

    Ok(())
}
