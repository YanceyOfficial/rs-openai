use dotenvy::dotenv;
use rs_openai::OpenAI;
use std::env::var;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(&OpenAI {
        api_key,
        org_id: None,
    });

    // list
    let res = client.engines().list();
    println!("{:?}", res);

    // retrieve
    let res = client.engines().retrieve("text-davinci-003");
    println!("{:?}", res);

    Ok(())
}
