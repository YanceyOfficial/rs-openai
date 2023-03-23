use reqwest::header::HeaderMap;
use reqwest::multipart::Form;
use reqwest::{Client, RequestBuilder};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;

pub mod audio;
pub mod chat;
pub mod completions;
pub mod edits;
pub mod embeddings;
pub mod engines;
pub mod files;
pub mod fine_tunes;
pub mod images;
pub mod models;
pub mod moderations;

pub use audio::Audio;
pub use chat::Chat;
pub use completions::Completions;
pub use edits::Edits;
pub use embeddings::Embeddings;
pub use engines::Engines;
pub use files::Files;
pub use fine_tunes::FineTunes;
pub use images::Images;
pub use models::Models;
pub use moderations::Moderations;

mod error;

mod r#macro;

use error::{ApiErrorResponse, OpenAIError};

type Response<T> = Result<T, error::OpenAIError>;

/// Default v1 API base url
pub const API_BASE: &str = "https://api.openai.com/v1";

/// Name for organization header
pub const ORGANIZATION_HEADER: &str = "OpenAI-Organization";

pub struct OpenAI<'a> {
    pub api_key: &'a str,
    pub org_id: Option<&'a str>,
}

impl<'a> OpenAI<'a> {
    pub fn new(&self) -> Self {
        Self {
            api_key: self.api_key,
            org_id: self.org_id,
        }
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();

        if let Some(org_id) = self.org_id {
            headers.insert(ORGANIZATION_HEADER, org_id.parse().unwrap());
        }

        headers
    }

    async fn openai_request<T, F>(
        &self,
        method: reqwest::Method,
        route: &str,
        builder: F,
    ) -> Response<T>
    where
        T: DeserializeOwned + Debug,
        F: FnOnce(RequestBuilder) -> RequestBuilder,
    {
        let client = Client::new();

        let mut request = client
            .request(method, API_BASE.to_string() + route)
            .headers(self.headers())
            .bearer_auth(self.api_key);

        request = builder(request);

        self.error_handler(request).await
    }

    async fn error_handler<T>(&self, request: reqwest::RequestBuilder) -> Result<T, OpenAIError>
    where
        T: DeserializeOwned + Debug,
    {
        let response = request.send().await?;
        let status = response.status();
        let bytes = response.bytes().await?;

        if !status.is_success() {
            let api_error: ApiErrorResponse =
                serde_json::from_slice(bytes.as_ref()).map_err(OpenAIError::JSONDeserialize)?;

            return Err(OpenAIError::ApiError(api_error));
        }

        let data: T =
            serde_json::from_slice(bytes.as_ref()).map_err(OpenAIError::JSONDeserialize)?;
        Ok(data)
    }

    pub(crate) async fn get<T, F>(&self, route: &str, query: &F) -> Response<T>
    where
        T: DeserializeOwned + Debug,
        F: Serialize,
    {
        self.openai_request::<T, _>(reqwest::Method::GET, route, |request| request.query(query))
            .await
    }

    pub(crate) async fn post_form<T>(&self, route: &str, form_data: Form) -> Response<T>
    where
        T: DeserializeOwned + Debug,
    {
        self.openai_request::<T, _>(reqwest::Method::POST, route, |request| {
            request.multipart(form_data)
        })
        .await
    }

    pub(crate) async fn post<T, F>(&self, route: &str, json: &F) -> Response<T>
    where
        T: DeserializeOwned + Debug,
        F: Serialize,
    {
        self.openai_request::<T, _>(reqwest::Method::POST, route, |request| request.json(json))
            .await
    }

    #[allow(unused)]
    pub(crate) async fn put<T, F>(&self, route: &str, json: &F) -> Response<T>
    where
        T: DeserializeOwned + Debug,
        F: Serialize,
    {
        self.openai_request::<T, _>(reqwest::Method::PUT, route, |request| request.json(json))
            .await
    }

    #[allow(unused)]
    pub(crate) async fn delete<T, F>(&self, route: &str, json: &F) -> Response<T>
    where
        T: DeserializeOwned + Debug,
        F: Serialize,
    {
        self.openai_request::<T, _>(reqwest::Method::DELETE, route, |request| request.json(json))
            .await
    }

    pub fn audio(&self) -> Audio {
        Audio::new(self)
    }

    pub fn chat(&self) -> Chat {
        Chat::new(self)
    }

    pub fn completions(&self) -> Completions {
        Completions::new(self)
    }

    pub fn edits(&self) -> Edits {
        Edits::new(self)
    }

    pub fn embeddings(&self) -> Embeddings {
        Embeddings::new(self)
    }

    pub fn engines(&self) -> Engines {
        Engines::new(self)
    }

    pub fn files(&self) -> Files {
        Files::new(self)
    }

    pub fn fine_tunes(&self) -> FineTunes {
        FineTunes::new(self)
    }

    pub fn images(&self) -> Images {
        Images::new(self)
    }

    pub fn models(&self) -> Models {
        Models::new(self)
    }

    pub fn moderations(&self) -> Moderations {
        Moderations::new(self)
    }
}

#[cfg(test)]
mod tests {
    use dotenvy::dotenv;
    use moderations::CreateModerationRequestArgs;
    use std::env;

    use super::*;

    #[test]
    fn it_works() {
        dotenv().ok();
        let api_key = env::var("OPENAI_API_KEY").unwrap();
        let openai = OpenAI::new(&OpenAI {
            api_key: api_key.as_str(),
            org_id: None,
        });

        // let res = openai.engines().list();
        // println!("{:?}", res);

        // let res = openai.engines().retrieve("text-davinci-003");
        // println!("{:?}", res);

        let req = CreateModerationRequestArgs::default()
            .input(["Do You Want To Build A Snowman?", "I will kill you."])
            .build()
            .unwrap();
        let res = openai.moderations().create(&req);
        println!("{:?}", res);

        assert_eq!(1, 1);
    }
}
