# OpenAI Rust Library

![Crates.io](https://img.shields.io/crates/v/rs_openai?style=flat-square&logo=appveyor)
![Crates.io](https://img.shields.io/crates/d/rs_openai?style=flat-square&logo=appveyor)
![Crates.io](https://img.shields.io/crates/l/rs_openai?style=flat-square&logo=appveyor)
![docs.rs](https://img.shields.io/docsrs/rs_openai?style=flat-square&logo=appveyor)
[![build](https://github.com/YanceyOfficial/rs-openai/actions/workflows/rust.yml/badge.svg)](https://github.com/YanceyOfficial/rs-openai/actions/workflows/rust.yml)
[![rust-clippy analyze](https://github.com/YanceyOfficial/rs-openai/actions/workflows/rust-clippy.yml/badge.svg)](https://github.com/YanceyOfficial/rs-openai/actions/workflows/rust-clippy.yml)

The OpenAI Rust library provides convenient access to the OpenAI API from Rust applications.

## Installation

```toml
[dependencies]
rs_openai = { version = "0.2.0" }
```

## Features

- [x] Audio
- [x] Chat (including SSE streaming)
- [x] Completions (including SSE streaming)
- [x] Edits
- [x] Embeddings
- [x] Engines (Already deprecated)
- [x] Files
- [x] Fine-Tunes (including SSE streaming)
- [x] Images
- [x] Models
- [x] Moderations
- [ ] Enhance backoff
- [ ] Supports Microsoft Azure Endpoints

## Usage

The library needs to be configured with your account's secret key, which is available on the [website](https://platform.openai.com/account/api-keys). We recommend setting it as an environment variable.

```bash
# .env
OPENAI_API_KEY=sk-...
OPENAI_API_ORGANIZATION=org-...
```

Here's an example of initializing the library with the API key loaded from an environment variable and creating a completion:

```rust
use dotenvy::dotenv;
use rs_openai::{
    chat::{ChatCompletionMessageRequestBuilder, CreateChatRequestBuilder, Role},
    OpenAI,
};
use std::env::var;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(&OpenAI {
        api_key,
        org_id: None,
    });

    let req = CreateChatRequestBuilder::default()
        .model("gpt-3.5-turbo")
        .messages(vec![ChatCompletionMessageRequestBuilder::default()
            .role(Role::User)
            .content("To Solve LeetCode's problem 81 in Rust.")
            .build()?])
        .build()?;

    let res = client.chat().create(&req).await?;
    println!("{:?}", res);

    Ok(())
}
```

### Stream

Like ChatGPT, we support `stream` mode for **Create chat completion**, **Create completion** and **List fine-tune events**. In these cases, tokens will be sent as data-only [server-sent events](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events#Event_stream_format) as they become available. Watch the [demo](https://edge.yancey.app/beg/qkzmqxyg-1680159198801.mp4) for the following code.

```rust
use dotenvy::dotenv;
use futures::StreamExt;
use rs_openai::{
    chat::{ChatCompletionMessageRequestBuilder, CreateChatRequestBuilder, Role},
    OpenAI,
};
use std::env::var;
use std::io::{stdout, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let api_key = var("OPENAI_API_KEY").unwrap();

    let client = OpenAI::new(&OpenAI {
        api_key,
        org_id: None,
    });


    // stream mode
    let req = CreateChatRequestBuilder::default()
        .model("gpt-3.5-turbo")
        .messages(vec![ChatCompletionMessageRequestBuilder::default()
            .role(Role::User)
            .content("To Solve LeetCode's problem 81 in Rust.")
            .build()?])
        .stream(true)
        .build()?;

    let mut stream = client.chat().create_with_stream(&req).await?;

    let mut lock = stdout().lock();
    while let Some(response) = stream.next().await {
        response.unwrap().choices.iter().for_each(|choice| {
            if let Some(ref content) = choice.delta.content {
                write!(lock, "{}", content).unwrap();
            }
        });

        stdout().flush()?;
    }

    Ok(())
}
```

Check out the full [API documentation](https://platform.openai.com/docs/api-reference/) for examples of all the available functions.

- Visit [examples](https://github.com/YanceyOfficial/rs-openai/tree/master/examples) directory on how to use async-openai.

- Visit [docs.rs/rs_openai](https://docs.rs/rs_openai) for docs.

## Requirements

In general, we want to support the versions of Rust that our customers are using. If you run into problems with any version issues, please let us know at on our support page.

## Contributing

The main purpose of this repository is to continue to evolve OpenAI Rust Library, making it faster and easier to use. Development of OpenAI Rust Library happens in the open on GitHub, and we are grateful to the community for contributing bugfixes and improvements. Read below to learn how you can take part in improving OpenAI Rust Library.

### [Code of Conduct](./CODE_OF_CONDUCT.md)

OpenAI Rust Library has adopted a Code of Conduct that we expect project participants to adhere to. Please read [the full text](./CODE_OF_CONDUCT.md) so that you can understand what actions will and will not be tolerated.

### [Contributing Guide](./CONTRIBUTING.md)

Read our [contributing guide](./CONTRIBUTING.md) to learn about our development process, how to propose bugfixes and improvements, and how to build and test your changes to OpenAI Rust Library.

### Good Issues

Please make sure to read the [Issue Reporting Checklist](./.github/ISSUE_TEMPLATE/bug_report.md) before opening an issue. Issues not conforming to the guidelines may be closed immediately.

## Thanks

As a Rust beginner, lots of experience, thoughts and idea are came from [64bit](https://github.com/64bit)'s [async-openai](https://github.com/64bit/async-openai), Thank you and your project!

## License

OpenAI Rust Library is licensed under the terms of the [MIT licensed](https://opensource.org/licenses/MIT).
