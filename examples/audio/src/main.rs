use dotenvy::dotenv;
use rs_openai::{
    audio::{AudioModel, CreateTranscriptionRequestArgs, FileMeta},
    OpenAI,
};
use std::io::prelude::*;
use std::{env::var, fs::File};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(&OpenAI {
        api_key: &api_key,
        org_id: None,
    });

    let mut file = File::open("./assets/dear_abe_san.mp4")?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let request = CreateTranscriptionRequestArgs::default()
        .file(FileMeta {
            file_content: buffer,
            filename: "xxxx".into(),
            content_type: "".into(),
        })
        .model(AudioModel::Whisper1)
        .build()?;

    let response = client.audio().transcribe(&request);

    println!("{:?}", response);

    Ok(())
}
