use dotenvy::dotenv;
use rs_openai::{
    images::{
        CreateImageEditRequestBuilder, CreateImageRequestBuilder,
        CreateImageVariationRequestBuilder, ImageSize, ResponseFormat,
    },
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

    let origin_image = read_image("./assets/worldcup.png").unwrap();
    let transparent_image = read_image("./assets/worldcup-transparent.png").unwrap();

    create(&client)?;
    create_edit(&client, origin_image.clone(), transparent_image.clone())?;
    create_variations(&client, origin_image.clone())?;

    Ok(())
}

fn read_image(path: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut file = File::open(path).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    Ok(buffer)
}

fn create(client: &OpenAI) -> Result<(), Box<dyn std::error::Error>> {
    let req = CreateImageRequestBuilder::default()
        .prompt("An oil painting with beach and sunshine.")
        .response_format(ResponseFormat::Url)
        .size(ImageSize::S512x512)
        .n(2)
        .build()?;

    let res = client.images().create(&req).unwrap();
    println!("{:?}", res);

    Ok(())
}

#[allow(unused)]
fn create_edit(
    client: &OpenAI,
    origin_buffer: Vec<u8>,
    transparent_buffer: Vec<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let req = CreateImageEditRequestBuilder::default()
        .image(FileMeta {
            buffer: origin_buffer,
            filename: "worldcup.png".into(),
        })
        .mask(FileMeta {
            buffer: transparent_buffer,
            filename: "worldcup-transparent.png".into(),
        })
        .prompt("celebration ribbon")
        .n(2)
        .build()?;

    let res = client.images().create_edit(&req).unwrap();
    println!("{:?}", res);

    Ok(())
}

#[allow(unused)]
fn create_variations(client: &OpenAI, buffer: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    let req = CreateImageVariationRequestBuilder::default()
        .image(FileMeta {
            buffer,
            filename: "worldcup.png".into(),
        })
        .n(2)
        .build()?;

    let res = client.images().create_variations(&req).unwrap();
    println!("{:?}", res);

    Ok(())
}
