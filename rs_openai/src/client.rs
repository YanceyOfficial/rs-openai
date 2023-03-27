pub use crate::apis::{
    audio, chat, completions, edits, embeddings, engines, files, fine_tunes, images, models,
    moderations,
};
use crate::shared::response_wrapper::{ApiErrorResponse, OpenAIError, OpenAIResponse};
use futures::StreamExt;
use reqwest::{header::HeaderMap, multipart::Form, Client, Method, RequestBuilder};
use reqwest_eventsource::{Event, EventSource, RequestBuilderExt};
use serde::{de::DeserializeOwned, Serialize};
use std::error::Error;
use std::fmt::Debug;
use std::marker::Send;
use tokio::sync::mpsc::{channel, Receiver};

// Default v1 API base url
pub const API_BASE: &str = "https://api.openai.com/v1";

/// Name for organization header
pub const ORGANIZATION_HEADER: &str = "OpenAI-Organization";

pub struct OpenAI {
    pub api_key: String,
    pub org_id: Option<String>,
}

impl OpenAI {
    pub fn new(&self) -> Self {
        Self {
            api_key: self.api_key.to_owned(),
            org_id: self.org_id.to_owned(),
        }
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();

        if let Some(org_id) = &self.org_id {
            headers.insert(ORGANIZATION_HEADER, org_id.parse().unwrap());
        }

        headers
    }

    fn openai_request<F>(&self, method: Method, route: &str, builder: F) -> RequestBuilder
    where
        F: FnOnce(RequestBuilder) -> RequestBuilder,
    {
        let client = Client::new();

        let mut request = client
            .request(method, API_BASE.to_string() + route)
            .headers(self.headers())
            .bearer_auth(&self.api_key);

        request = builder(request);
        request
    }

    async fn resolve_response<T>(&self, request: RequestBuilder) -> OpenAIResponse<T>
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

    async fn resolve_text_response(&self, request: RequestBuilder) -> OpenAIResponse<String> {
        let response = request.send().await?;
        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            let api_error: ApiErrorResponse =
                serde_json::from_slice(text.as_ref()).map_err(OpenAIError::JSONDeserialize)?;

            return Err(OpenAIError::ApiError(api_error));
        }

        Ok(text)
    }

    pub(crate) async fn get<T, F>(&self, route: &str, query: &F) -> OpenAIResponse<T>
    where
        T: DeserializeOwned + Debug,
        F: Serialize,
    {
        let request = self.openai_request(Method::GET, route, |request| request.query(query));
        self.resolve_response(request).await
    }

    pub(crate) async fn post<T, F>(&self, route: &str, json: &F) -> OpenAIResponse<T>
    where
        T: DeserializeOwned + Debug,
        F: Serialize,
    {
        let request = self.openai_request(Method::POST, route, |request| request.json(json));
        self.resolve_response(request).await
    }

    pub(crate) async fn post_form<T>(&self, route: &str, form_data: Form) -> OpenAIResponse<T>
    where
        T: DeserializeOwned + Debug,
    {
        let request =
            self.openai_request(Method::POST, route, |request| request.multipart(form_data));
        self.resolve_response(request).await
    }

    pub(crate) async fn post_stream<T, F>(
        &self,
        route: &str,
        json: &F,
    ) -> Result<Receiver<OpenAIResponse<T>>, Box<dyn Error>>
    where
        T: DeserializeOwned + Debug + Send + 'static,
        F: Serialize,
    {
        let event_source = self
            .openai_request(Method::POST, route, |request| request.json(json))
            .eventsource()
            .unwrap();
        self.stream_sse(event_source).await
    }

    pub(crate) async fn post_form_with_text_response(
        &self,
        route: &str,
        form_data: Form,
    ) -> OpenAIResponse<String> {
        let request =
            self.openai_request(Method::POST, route, |request| request.multipart(form_data));
        self.resolve_text_response(request).await
    }

    #[allow(unused)]
    pub(crate) async fn delete<T, F>(&self, route: &str, json: &F) -> OpenAIResponse<T>
    where
        T: DeserializeOwned + Debug,
        F: Serialize,
    {
        let request = self.openai_request(Method::DELETE, route, |request| request.json(json));
        self.resolve_response(request).await
    }

    async fn stream_sse<T>(
        &self,
        mut event_source: EventSource,
    ) -> Result<Receiver<OpenAIResponse<T>>, Box<dyn Error>>
    where
        T: DeserializeOwned + Debug + Send + 'static,
    {
        let (tx, mut rx) = channel::<OpenAIResponse<T>>(32);

        let sse_task = tokio::spawn(async move {
            while let Some(evt) = event_source.next().await {
                match evt {
                    Err(e) => {
                        if let Err(_) = tx.send(Err(OpenAIError::StreamError(e.to_string()))).await
                        {
                            break;
                        }
                    }
                    Ok(evt) => match evt {
                        Event::Message(message) => {
                            if message.data == "[DONE]" {
                                break;
                            }

                            let response = match serde_json::from_str::<T>(&message.data) {
                                Err(e) => Err(OpenAIError::JSONDeserialize(e)),
                                Ok(output) => Ok(output),
                            };

                            if let Err(_) = tx.send(response).await {
                                break;
                            }
                        }
                        _ => continue,
                    },
                }
            }

            event_source.close();
        });

        while let Some(res) = rx.recv().await {
            println!("{:?}", res);
        }

        sse_task.await?;

        Ok(rx)
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
