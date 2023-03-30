//! The OpenAI Rust library provides convenient access to the OpenAI API from Rust applications.
//!
//! ## Creating client
//!
//! ```ignore
//! use dotenvy::dotenv;
//! use rs_openai::{OpenAI};
//! use std::env::var;
//!
//! dotenv().ok();
//! let api_key = var("OPENAI_API_KEY").unwrap();
//!
//! let client = OpenAI::new(&OpenAI {
//!     api_key,
//!     org_id: None,
//! });
//! ```
//!
//! ## Making requests
//!
//!```ignore
//! use dotenvy::dotenv;
//! use rs_openai::{
//!     chat::{ChatCompletionMessageRequestBuilder, CreateChatRequestBuilder, Role},
//!     OpenAI,
//! };
//! use std::env::var;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     dotenv().ok();
//!     let api_key = var("OPENAI_API_KEY").unwrap();
//!
//!     let client = OpenAI::new(&OpenAI {
//!         api_key,
//!         org_id: None,
//!     });
//!
//!     let req = CreateChatRequestBuilder::default()
//!         .model("gpt-3.5-turbo")
//!         .messages(vec![ChatCompletionMessageRequestBuilder::default()
//!             .role(Role::User)
//!             .content("To Solve LeetCode's problem 81 in Rust.")
//!             .build()?])
//!         .build()?;
//!
//!     let res = client.chat().create(&req).await?;
//!     println!("{:?}", res);
//!
//!     Ok(())
//! }
//!```
//!
//! ## Examples
//! For full working examples for all supported features see [examples](https://github.com/YanceyOfficial/rs-openai/tree/master/examples) directory in the repository.
//!
pub mod apis;
pub mod client;
pub mod shared;

pub use apis::*;
pub use client::*;