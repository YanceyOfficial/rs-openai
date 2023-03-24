use dotenvy::dotenv;
use rs_openai::{
    images::{
        CreateImageEditRequestBuilder, CreateImageRequestBuilder,
        CreateImageVariationRequestBuilder, ResponseFormat,
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

    let mut file = File::open("./assets/worldcup.jpg").unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    create(&client)?;
    create_edit(&client, buffer.clone())?;
    create_variations(&client, buffer.clone())?;

    Ok(())
}

fn create(client: &OpenAI) -> Result<(), Box<dyn std::error::Error>> {
    let request = CreateImageRequestBuilder::default()
        .prompt("world cup 2022, argentina celebration.")
        .response_format(ResponseFormat::B64Json)
        .build()?;

    let response = client.images().create(&request).unwrap();

    match response {
        OpenAIResponseType::Json(j) => println!("{:?}", j),
        OpenAIResponseType::Text(t) => println!("{}", t),
    }

    Ok(())
}

#[allow(unused)]
fn create_edit(client: &OpenAI, buffer: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    let request = CreateImageEditRequestBuilder::default()
        .image(FileMeta {
            buffer,
            filename: "worldcup.jpg".into(),
        })
        .build()?;

    let response = client.images().create_edit(&request).unwrap();

    match response {
        OpenAIResponseType::Json(j) => println!("{:?}", j),
        OpenAIResponseType::Text(t) => println!("{}", t),
    }

    Ok(())
}

#[allow(unused)]
fn create_variations(client: &OpenAI, buffer: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    let request = CreateImageVariationRequestBuilder::default()
        .image(FileMeta {
            buffer,
            filename: "worldcup.jpg".into(),
        })
        .build()?;

    let response = client.images().create_variations(&request).unwrap();

    match response {
        OpenAIResponseType::Json(j) => println!("{:?}", j),
        OpenAIResponseType::Text(t) => println!("{}", t),
    }

    Ok(())
}
