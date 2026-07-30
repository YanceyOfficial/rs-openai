use dotenvy::dotenv;
use rs_openai::{
    OpenAI,
    images::{
        CreateImageEditRequest, CreateImageRequest, CreateImageVariationRequest, ImageSize,
        ResponseFormat,
    },
    shared::types::File,
};
use std::io::prelude::*;
use std::{env::var, fs::File as StdFile};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(api_key);

    let origin_image = read_image("./assets/worldcup.png").unwrap();
    let transparent_image = read_image("./assets/worldcup-transparent.png").unwrap();

    create(&client)?;
    create_edit(&client, origin_image.clone(), transparent_image)?;
    create_variations(&client, origin_image)?;

    Ok(())
}

#[tokio::main]
async fn read_image(path: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut file = StdFile::open(path).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    Ok(buffer)
}

#[tokio::main]
async fn create(client: &OpenAI) -> Result<(), Box<dyn std::error::Error>> {
    let req = CreateImageRequest::builder()
        .prompt("a hyperrealistic portrait photo of a city street at night")
        .response_format(ResponseFormat::Url)
        .size(ImageSize::S512x512)
        .n(10)
        .build();

    let res = client.images().create(&req).await?;
    println!("{:?}", res);

    Ok(())
}

#[tokio::main]
async fn create_edit(
    client: &OpenAI,
    origin_buffer: Vec<u8>,
    transparent_buffer: Vec<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let req = CreateImageEditRequest::builder()
        .image(File {
            buffer: origin_buffer,
            filename: "worldcup.png".into(),
        })
        .mask(File {
            buffer: transparent_buffer,
            filename: "worldcup-transparent.png".into(),
        })
        .prompt("celebration ribbon")
        .n(2)
        .build();

    let res = client.images().create_edit(&req).await?;
    println!("{:?}", res);

    Ok(())
}

#[tokio::main]
async fn create_variations(
    client: &OpenAI,
    buffer: Vec<u8>,
) -> Result<(), Box<dyn std::error::Error>> {
    let req = CreateImageVariationRequest::builder()
        .image(File {
            buffer,
            filename: "worldcup.png".into(),
        })
        .n(2)
        .build();

    let res = client.images().create_variations(&req).await?;
    println!("{:?}", res);

    Ok(())
}
