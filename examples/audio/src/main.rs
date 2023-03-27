use dotenvy::dotenv;
use rs_openai::{
    audio::{
        AudioModel, CreateTranscriptionRequestBuilder, CreateTranslationRequestBuilder,
        ResponseFormat,Language
    },
    shared::response_wrapper::OpenAIResponseType,
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
    let req = CreateTranscriptionRequestBuilder::default()
        .file(FileMeta {
            buffer,
            filename: "dear_abe_san.mp4".into(),
        })
        .model(AudioModel::Whisper1)
        .response_format(ResponseFormat::Json)
        .language(Language::Japanese)
        .build()?;

    let res = client.audio().create_transcription(&req).unwrap();

    match res {
        OpenAIResponseType::Json(j) => println!("{:?}", j),
        OpenAIResponseType::Text(t) => println!("{}", t),
    }

    Ok(())
}

fn create_translation(client: &OpenAI, buffer: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    let req = CreateTranslationRequestBuilder::default()
        .file(FileMeta {
            buffer,
            filename: "dear_abe_san.mp4".into(),
        })
        .model(AudioModel::Whisper1)
        .build()?;

    let res = client.audio().create_translation(&req);

    println!("{:?}", res.unwrap());

    Ok(())
}
