use dotenvy::dotenv;
use rs_openai::{
    audio::{
        AudioModel, CreateTranscriptionRequestBuilder, CreateTranslationRequestBuilder, ResponseFormat,
    },
    shared::errors::OpenAIResponseType,
    shared::types::FileMeta,
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

    let mut file = File::open("./assets/dear_abe_san.mp4").unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    create_transcription(&client, buffer.clone())?;
    create_translation(&client, buffer.clone())?;

    Ok(())
}

fn create_transcription(
    client: &OpenAI,
    buffer: Vec<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let request = CreateTranscriptionRequestBuilder::default()
        .file(FileMeta {
            buffer,
            filename: "dear_abe_san.mp4".into(),
        })
        .model(AudioModel::Whisper1)
        .response_format(ResponseFormat::Json)
        .build()?;

    let response = client.audio().create_transcription(&request).unwrap();

    match response {
        OpenAIResponseType::Json(j) => println!("{:?}", j),
        OpenAIResponseType::Text(t) => println!("{}", t),
    }

    Ok(())
}

fn create_translation(client: &OpenAI, buffer: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    let request = CreateTranslationRequestBuilder::default()
        .file(FileMeta {
            buffer,
            filename: "dear_abe_san.mp4".into(),
        })
        .model(AudioModel::Whisper1)
        .build()?;

    let response = client.audio().create_translation(&request);

    println!("{:?}", response.unwrap()); // "Happy New Year! I wish you a happy new year. I wish you a bright and prosperous year."

    Ok(())
}
