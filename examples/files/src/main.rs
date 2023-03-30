use dotenvy::dotenv;
use rs_openai::{files::UploadFileRequestBuilder, shared::types::FileMeta, OpenAI};
use std::io::prelude::*;
use std::{env::var, fs::File};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(&OpenAI {
        api_key,
        org_id: None,
    });

    // list
    let res = client.files().list().await?;
    println!("{:?}", res);

    // upload
    let mut file = File::open("./assets/books_training_file.jsonl").unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let req = UploadFileRequestBuilder::default()
        .file(FileMeta {
            filename: "books_training_file.jsonl".into(),
            buffer,
        })
        .purpose("fine-tune".to_string())
        .build()?;

    let res = client.files().upload(&req).await?;
    println!("{:?}", res);

    // delete
    let res = client.files().delete("YOUR_FILE_ID").await?;
    println!("{:?}", res);

    // retrieve
    let res = client.files().retrieve("YOUR_FILE_ID").await?;
    println!("{:?}", res);

    // retrieve_content
    // TODO: Since free accounts cannot download fine-tune training files, I have to verify this api until purchase a Plus.
    let res = client.files().retrieve_content("YOUR_FILE_ID").await?;
    println!("{:?}", res);

    Ok(())
}
