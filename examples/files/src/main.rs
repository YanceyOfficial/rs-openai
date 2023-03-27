use dotenvy::dotenv;
use rs_openai::{files::UploadFileRequestBuilder, shared::types::FileMeta, OpenAI};
use std::io::prelude::*;
use std::{env::var, fs::File};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(&OpenAI {
        api_key,
        org_id: None,
    });

    // list
    let res = client.files().list();
    println!("{:?}", res);

    // upload
    let mut file = File::open("./assets/training_file.jsonl").unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let req = UploadFileRequestBuilder::default()
        .file(FileMeta {
            filename: "training_file.jsonl".into(),
            buffer,
        })
        .purpose("fine-tune".to_string())
        .build()?;

    let res = client.files().upload(&req);
    println!("{:?}", res);

    // delete
    let res = client.files().delete("");
    println!("{:?}", res);

    // retrieve
    let res = client.files().retrieve("");
    println!("{:?}", res);

    // retrieve_content
    let res = client.files().retrieve_content("");
    println!("{:?}", res);

    Ok(())
}
