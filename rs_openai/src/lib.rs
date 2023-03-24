use reqwest::header::{HeaderMap, CONTENT_TYPE};
use reqwest::multipart::Form;
use reqwest::{Client, RequestBuilder};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;

pub mod apis;
pub mod shared;

#[doc(inline)]
pub use apis::{
    audio, chat, completions, edits, embeddings, engines, files, fine_tunes, images, models,
    moderations,
};

#[doc(inline)]
pub use shared::errors::{ApiErrorResponse, OpenAIError, OpenAIResponse, OpenAIResponseType};

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
    ) -> OpenAIResponse<T>
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

        self.resolve_response(request).await
    }

    async fn resolve_response<T>(&self, request: reqwest::RequestBuilder) -> OpenAIResponse<T>
    where
        T: DeserializeOwned + Debug,
    {
        let response = request.send().await?;
        let status = response.status();
        let is_text_plain = response
            .headers()
            .get(CONTENT_TYPE)
            .unwrap()
            .to_str()
            .unwrap()
            .to_lowercase()
            .contains("text/plain");
        let bytes = response.bytes().await?;

        if !status.is_success() {
            let api_error: ApiErrorResponse =
                serde_json::from_slice(bytes.as_ref()).map_err(OpenAIError::JSONDeserialize)?;

            return Err(OpenAIError::ApiError(api_error));
        }

        if is_text_plain {
            unsafe {
                let text = String::from_utf8_unchecked(bytes.as_ref().to_vec());
                return Ok(OpenAIResponseType::Text(text));
            }
        }

        let data: T =
            serde_json::from_slice(bytes.as_ref()).map_err(OpenAIError::JSONDeserialize)?;

        Ok(OpenAIResponseType::Json(data))
    }

    pub(crate) async fn get<T, F>(&self, route: &str, query: &F) -> OpenAIResponse<T>
    where
        T: DeserializeOwned + Debug,
        F: Serialize,
    {
        self.openai_request::<T, _>(reqwest::Method::GET, route, |request| request.query(query))
            .await
    }

    pub(crate) async fn post_form<T>(&self, route: &str, form_data: Form) -> OpenAIResponse<T>
    where
        T: DeserializeOwned + Debug,
    {
        self.openai_request::<T, _>(reqwest::Method::POST, route, |request| {
            request.multipart(form_data)
        })
        .await
    }

    pub(crate) async fn post<T, F>(&self, route: &str, json: &F) -> OpenAIResponse<T>
    where
        T: DeserializeOwned + Debug,
        F: Serialize,
    {
        self.openai_request::<T, _>(reqwest::Method::POST, route, |request| request.json(json))
            .await
    }

    #[allow(unused)]
    pub(crate) async fn put<T, F>(&self, route: &str, json: &F) -> OpenAIResponse<T>
    where
        T: DeserializeOwned + Debug,
        F: Serialize,
    {
        self.openai_request::<T, _>(reqwest::Method::PUT, route, |request| request.json(json))
            .await
    }

    #[allow(unused)]
    pub(crate) async fn delete<T, F>(&self, route: &str, json: &F) -> OpenAIResponse<T>
    where
        T: DeserializeOwned + Debug,
        F: Serialize,
    {
        self.openai_request::<T, _>(reqwest::Method::DELETE, route, |request| request.json(json))
            .await
    }

    pub fn audio(&self) -> audio::Audio {
        audio::Audio::new(self)
    }

    pub fn chat(&self) -> chat::Chat {
        chat::Chat::new(self)
    }

    pub fn completions(&self) -> completions::Completions {
        completions::Completions::new(self)
    }

    pub fn edits(&self) -> edits::Edits {
        edits::Edits::new(self)
    }

    pub fn embeddings(&self) -> embeddings::Embeddings {
        embeddings::Embeddings::new(self)
    }

    pub fn engines(&self) -> engines::Engines {
        engines::Engines::new(self)
    }

    pub fn files(&self) -> files::Files {
        files::Files::new(self)
    }

    pub fn fine_tunes(&self) -> fine_tunes::FineTunes {
        fine_tunes::FineTunes::new(self)
    }

    pub fn images(&self) -> images::Images {
        images::Images::new(self)
    }

    pub fn models(&self) -> models::Models {
        models::Models::new(self)
    }

    pub fn moderations(&self) -> moderations::Moderations {
        moderations::Moderations::new(self)
    }
}

#[cfg(test)]
mod tests {
    use apis::moderations::CreateModerationRequestBuilder;
    use dotenvy::dotenv;
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

        let req = CreateModerationRequestBuilder::default()
            .input(["Do You Want To Build A Snowman?", "I will kill you."])
            .build()
            .unwrap();
        let res = openai.moderations().create(&req);
        println!("{:?}", res);

        assert_eq!(1, 1);
    }
}
